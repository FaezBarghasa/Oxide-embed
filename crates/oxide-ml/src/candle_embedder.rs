use std::path::Path;
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;
use oxide_core::error::{OxideError, Result};
use crate::device::select_device;
use crate::embedder::Embedder;
use crate::mock::MockEmbedder;

pub struct CandleBertEmbedder {
    model: Option<BertModel>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    fallback: MockEmbedder,
    dimension: usize,
}

impl CandleBertEmbedder {
    pub fn new_offline() -> Self {
        Self {
            model: None,
            tokenizer: None,
            device: select_device(),
            fallback: MockEmbedder::new(384),
            dimension: 384,
        }
    }

    pub fn load<P: AsRef<Path>>(
        weights_path: P,
        config_path: P,
        tokenizer_path: P,
    ) -> Result<Self> {
        let device = select_device();
        let config_str = std::fs::read_to_string(config_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to read BERT config: {e}")))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| OxideError::Ml(format!("Failed to parse BERT config: {e}")))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to load tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.as_ref()], candle_core::DType::F32, &device)
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
}

#[async_trait]
impl Embedder for CandleBertEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if let (Some(model), Some(tokenizer)) = (&self.model, &self.tokenizer) {
            let tokens = tokenizer
                .encode(text, true)
                .map_err(|e| OxideError::Ml(format!("Tokenizer error: {e}")))?;
            let token_ids = tokens.get_ids();
            let input_ids = Tensor::new(token_ids, &self.device)
                .map_err(|e| OxideError::Ml(e.to_string()))?
                .unsqueeze(0)
                .map_err(|e| OxideError::Ml(e.to_string()))?;
            let token_type_ids = input_ids
                .zeros_like()
                .map_err(|e| OxideError::Ml(e.to_string()))?;

            let embeddings = model
                .forward(&input_ids, &token_type_ids, None)
                .map_err(|e| OxideError::Ml(format!("Bert forward error: {e}")))?;

            // Mean pooling over token sequence
            let (_b, seq_len, _h) = embeddings
                .dims3()
                .map_err(|e| OxideError::Ml(e.to_string()))?;
            let mean = (embeddings.sum(1).map_err(|e| OxideError::Ml(e.to_string()))? / (seq_len as f64))
                .map_err(|e| OxideError::Ml(e.to_string()))?;
            let vec: Vec<f32> = mean
                .squeeze(0)
                .map_err(|e| OxideError::Ml(e.to_string()))?
                .to_vec1()
                .map_err(|e| OxideError::Ml(e.to_string()))?;

            // L2 normalize
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                Ok(vec.into_iter().map(|x| x / norm).collect())
            } else {
                Ok(vec)
            }
        } else {
            self.fallback.embed(text).await
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
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
