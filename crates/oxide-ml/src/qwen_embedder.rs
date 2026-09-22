use crate::device::select_device;
use crate::embedder::Embedder;
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::qwen2::{Config, ModelForCausalLM};
use oxide_core::error::{OxideError, Result};
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

pub struct CandleQwenEmbedder {
    model: Mutex<ModelForCausalLM>,
    tokenizer: Tokenizer,
    device: Device,
    dimension: usize,
}

impl CandleQwenEmbedder {
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
            .map_err(|e| OxideError::Ml(format!("Failed to read Qwen config: {e}")))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| OxideError::Ml(format!("Failed to parse Qwen config: {e}")))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to load Qwen tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights_path.as_ref()],
                candle_core::DType::F32,
                &device,
            )
            .map_err(|e| OxideError::Ml(format!("Failed to load safetensors: {e}")))?
        };

        let model = ModelForCausalLM::new(&config, vb)
            .map_err(|e| OxideError::Ml(format!("Failed to construct Qwen model: {e}")))?;

        let dimension = config.hidden_size;

        Ok(Self {
            model: Mutex::new(model),
            tokenizer,
            device,
            dimension,
        })
    }

    pub fn load_default() -> Result<Self> {
        Self::load_default_with_device("auto")
    }

    pub fn load_default_with_device(device_pref: &str) -> Result<Self> {
        let base_dir =
            crate::model::ModelManager::default_models_dir()?.join("qwen3-embedding-0.6b");
        let weights = base_dir.join("model.safetensors");
        let config = base_dir.join("config.json");
        let tokenizer = base_dir.join("tokenizer.json");

        if weights.exists() && config.exists() && tokenizer.exists() {
            Self::load_with_device(weights, config, tokenizer, device_pref)
        } else {
            Err(OxideError::Ml(format!(
                "Qwen3-Embedding-0.6B model not found in {}",
                base_dir.display()
            )))
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[async_trait]
impl Embedder for CandleQwenEmbedder {
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

        let batch_size = texts.len();
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| OxideError::Ml(format!("Qwen tokenizer error: {e}")))?;

        let mut actual_token_lengths = Vec::with_capacity(batch_size);
        let mut max_seq_len = 1;
        for encoding in &encodings {
            let len = encoding.get_ids().len().clamp(1, 2048);
            actual_token_lengths.push(len);
            if len > max_seq_len {
                max_seq_len = len;
            }
        }

        let mut all_ids = Vec::with_capacity(batch_size * max_seq_len);
        for (encoding, &actual_len) in encodings.iter().zip(actual_token_lengths.iter()) {
            let ids = encoding.get_ids();
            for &id in ids.iter().take(actual_len) {
                all_ids.push(id);
            }
            if max_seq_len > actual_len {
                all_ids.extend(std::iter::repeat_n(0, max_seq_len - actual_len));
            }
        }

        let input_ids = Tensor::from_vec(all_ids, (batch_size, max_seq_len), &self.device)
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        // Forward through Qwen causal model
        let mut model_guard = self
            .model
            .lock()
            .map_err(|_| OxideError::Ml("Failed to lock Qwen model mutex".into()))?;

        let logits = model_guard
            .forward(&input_ids, 0)
            .map_err(|e| OxideError::Ml(format!("Qwen forward error: {e}")))?;

        let (_b, seq_len, hidden_size) = logits
            .dims3()
            .map_err(|e| OxideError::Ml(format!("Qwen logits shape error: {e}")))?;

        let flat = logits
            .to_dtype(candle_core::DType::F32)
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .flatten_all()
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .to_vec1::<f32>()
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        let dim = self.dimension.min(hidden_size);
        let mut output = Vec::with_capacity(batch_size);

        for (b, &token_count) in actual_token_lengths.iter().enumerate() {
            let mut pooled = vec![0.0f32; dim];
            for t in 0..token_count {
                let offset = (b * seq_len + t) * hidden_size;
                for (h, item) in pooled.iter_mut().enumerate().take(dim) {
                    *item += flat[offset + h];
                }
            }
            let denom = token_count as f32;
            for item in pooled.iter_mut().take(dim) {
                *item /= denom;
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
