use crate::device::select_device;
use crate::embedder::Embedder;
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::qwen2::{Config, ModelForCausalLM};
use oxide_core::error::{OxideError, Result};
use std::path::Path;
use tokenizers::Tokenizer;

use std::sync::Mutex;

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
        let device = select_device("auto");
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
        let base_dir =
            crate::model::ModelManager::default_models_dir()?.join("qwen3-embedding-0.6b");
        let weights = base_dir.join("model.safetensors");
        let config = base_dir.join("config.json");
        let tokenizer = base_dir.join("tokenizer.json");

        if weights.exists() && config.exists() && tokenizer.exists() {
            Self::load(weights, config, tokenizer)
        } else {
            Err(OxideError::Ml(format!(
                "Qwen3-Embedding-0.6B model not found in {}",
                base_dir.display()
            )))
        }
    }
}

#[async_trait]
impl Embedder for CandleQwenEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| OxideError::Ml(format!("Qwen tokenizer error: {e}")))?;
        let raw_token_ids = tokens.get_ids();
        let token_ids = &raw_token_ids[..raw_token_ids.len().min(2048)];
        let seq_len = token_ids.len().max(1);

        let input_ids = Tensor::new(token_ids, &self.device)
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        // Forward through Qwen causal model
        let mut model_guard = self
            .model
            .lock()
            .map_err(|_| OxideError::Ml("Failed to lock Qwen model mutex".into()))?;

        let logits = model_guard
            .forward(&input_ids, 0)
            .map_err(|e| OxideError::Ml(format!("Qwen forward error: {e}")))?;

        // Mean pool across sequence dimension
        let mean = (logits.sum(1).map_err(|e| OxideError::Ml(e.to_string()))? / (seq_len as f64))
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        let vec: Vec<f32> = mean
            .squeeze(0)
            .map_err(|e| OxideError::Ml(e.to_string()))?
            .to_vec1()
            .map_err(|e| OxideError::Ml(e.to_string()))?;

        // Truncate/slice to embedding dimension if logits projection differs
        let dim = self.dimension.min(vec.len());
        let slice = &vec[..dim];

        // L2 normalize
        let norm: f32 = slice.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            Ok(slice.iter().map(|x| x / norm).collect())
        } else {
            Ok(slice.to_vec())
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
