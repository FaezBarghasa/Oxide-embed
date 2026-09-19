use crate::device::select_device;
use crate::embedder::Embedder;
use crate::mock::MockEmbedder;
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use oxide_core::error::{OxideError, Result};
use std::path::Path;
use tokenizers::Tokenizer;

pub struct CandleBertEmbedder {
    model: Option<BertModel>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    fallback: MockEmbedder,
    dimension: usize,
}

impl CandleBertEmbedder {
    pub fn new_offline() -> Self {
        Self::new_offline_with_device("auto")
    }

    pub fn new_offline_with_device(device_pref: &str) -> Self {
        if let Ok(embedder) = Self::load_default_with_device(device_pref) {
            embedder
        } else {
            let device = select_device(device_pref);
            Self {
                model: None,
                tokenizer: None,
                device,
                fallback: MockEmbedder::new(384),
                dimension: 384,
            }
        }
    }

    pub fn load_default() -> Result<Self> {
        Self::load_default_with_device("auto")
    }

    pub fn load_default_with_device(device_pref: &str) -> Result<Self> {
        let base_dir = crate::model::ModelManager::default_models_dir()?.join("bge-small-en-v1.5");
        let weights = base_dir.join("model.safetensors");
        let config = base_dir.join("config.json");
        let tokenizer = base_dir.join("tokenizer.json");

        if weights.exists() && config.exists() && tokenizer.exists() {
            Self::load_with_device(weights, config, tokenizer, device_pref)
        } else {
            Err(OxideError::Ml(format!(
                "Default model files not found in {}",
                base_dir.display()
            )))
        }
    }

    pub fn load<P: AsRef<Path>>(
        weights_path: P,
        config_path: P,
        tokenizer_path: P,
    ) -> Result<Self> {
        Self::load_with_device(weights_path, config_path, tokenizer_path, "auto")
    }

    pub fn load_with_device<P: AsRef<Path>>(
        weights_path: P,
        config_path: P,
        tokenizer_path: P,
        device_pref: &str,
    ) -> Result<Self> {
        let device = select_device(device_pref);
        let config_str = std::fs::read_to_string(config_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to read BERT config: {e}")))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| OxideError::Ml(format!("Failed to parse BERT config: {e}")))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to load tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights_path.as_ref()],
                candle_core::DType::F32,
                &device,
            )
            .map_err(|e| OxideError::Ml(format!("Failed to load safetensors: {e}")))?
        };

        let model = BertModel::load(vb, &config)
            .map_err(|e| OxideError::Ml(format!("Failed to construct BertModel: {e}")))?;

        let dimension = config.hidden_size;

        Ok(Self {
            model: Some(model),
            tokenizer: Some(tokenizer),
            device,
            fallback: MockEmbedder::new(dimension),
            dimension,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[async_trait]
impl Embedder for CandleBertEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.embed_batch(&[text.to_string()]).await?;
        results
            .pop()
            .ok_or_else(|| OxideError::Ml("Empty embedding result".into()))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let (Some(model), Some(tokenizer)) = (&self.model, &self.tokenizer) else {
            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                results.push(self.fallback.embed(text).await?);
            }
            return Ok(results);
        };

        let batch_size = texts.len();
        let encodings = tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| OxideError::Ml(format!("Batch tokenization error: {e}")))?;

        let mut actual_token_lengths = Vec::with_capacity(batch_size);
        let mut max_seq_len = 1;
        for encoding in &encodings {
            let len = encoding.get_ids().len().min(512).max(1);
            actual_token_lengths.push(len);
            if len > max_seq_len {
                max_seq_len = len;
            }
        }

        let mut all_ids = Vec::with_capacity(batch_size * max_seq_len);
        let mut all_masks = Vec::with_capacity(batch_size * max_seq_len);
        for (encoding, &actual_len) in encodings.iter().zip(actual_token_lengths.iter()) {
            let ids = encoding.get_ids();
            for i in 0..actual_len {
                all_ids.push(ids[i]);
                all_masks.push(1u32);
            }
            for _ in actual_len..max_seq_len {
                all_ids.push(0); // Pad token ID
                all_masks.push(0u32);
            }
        }

        let input_ids = Tensor::from_vec(all_ids, (batch_size, max_seq_len), &self.device)
            .map_err(|e| OxideError::Ml(format!("Tensor creation error: {e}")))?;
        let token_type_ids = input_ids
            .zeros_like()
            .map_err(|e| OxideError::Ml(format!("Token type ids error: {e}")))?;
        let attention_mask = Tensor::from_vec(all_masks, (batch_size, max_seq_len), &self.device)
            .map_err(|e| OxideError::Ml(format!("Attention mask error: {e}")))?;

        let embeddings = model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| OxideError::Ml(format!("BERT forward error: {e}")))?;

        let (_b, seq_len, hidden_size) = embeddings
            .dims3()
            .map_err(|e| OxideError::Ml(format!("Embedding shape error: {e}")))?;

        let flat_embeddings = embeddings
            .to_dtype(candle_core::DType::F32)
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .flatten_all()
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .to_vec1::<f32>()
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        let mut output = Vec::with_capacity(batch_size);
        for (b, &token_count) in actual_token_lengths.iter().enumerate() {
            let mut pooled = vec![0.0f32; hidden_size];
            for t in 0..token_count {
                let offset = (b * seq_len + t) * hidden_size;
                for h in 0..hidden_size {
                    pooled[h] += flat_embeddings[offset + h];
                }
            }
            let denom = token_count as f32;
            for h in 0..hidden_size {
                pooled[h] /= denom;
            }

            // L2 normalize
            let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                output.push(pooled.into_iter().map(|x| x / norm).collect());
            } else {
                output.push(pooled);
            }
        }

        Ok(output)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

pub fn cluster_embeddings(embeddings: &[Vec<f32>], threshold: f32) -> Vec<Vec<usize>> {
    let n = embeddings.len();
    let mut visited = vec![false; n];
    let mut clusters = Vec::new();

    for i in 0..n {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let mut cluster = vec![i];

        for j in (i + 1)..n {
            if !visited[j] {
                let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
                if sim >= threshold {
                    visited[j] = true;
                    cluster.push(j);
                }
            }
        }
        clusters.push(cluster);
    }

    clusters
}
