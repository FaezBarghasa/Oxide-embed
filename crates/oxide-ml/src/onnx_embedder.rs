use crate::embedder::Embedder;
use async_trait::async_trait;
use ndarray::Array2;
use ort::session::Session;
use ort::value::Tensor;
use oxide_core::error::{OxideError, Result};
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

pub struct OnnxGemmaEmbedder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    dimension: usize,
}

impl OnnxGemmaEmbedder {
    pub fn load<P: AsRef<Path>>(model_path: P, tokenizer_path: P) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to load Gemma tokenizer: {e}")))?;

        let session = Session::builder()
            .map_err(|e| OxideError::Ml(format!("Failed to create ONNX session builder: {e}")))?
            .commit_from_file(model_path.as_ref())
            .map_err(|e| OxideError::Ml(format!("Failed to load ONNX model: {e}")))?;

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimension: 768,
        })
    }

    pub fn load_default() -> Result<Self> {
        let base_dir =
            crate::model::ModelManager::default_models_dir()?.join("embeddinggemma-300m");
        let model_path = base_dir.join("model_q4.onnx");
        let tokenizer_path = base_dir.join("tokenizer.json");

        if model_path.exists() && tokenizer_path.exists() {
            Self::load(model_path, tokenizer_path)
        } else {
            Err(OxideError::Ml(format!(
                "EmbeddingGemma-300M ONNX model not found in {}",
                base_dir.display()
            )))
        }
    }
}

#[async_trait]
impl Embedder for OnnxGemmaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| OxideError::Ml(format!("Tokenizer error: {e}")))?;

        let raw_ids = encoding.get_ids();
        let max_len = raw_ids.len().min(2048);
        let token_ids: Vec<i64> = raw_ids[..max_len].iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask()[..max_len]
            .iter()
            .map(|&m| m as i64)
            .collect();
        let seq_len = token_ids.len();

        let shape = [1, seq_len];
        let input_ids_array = Array2::from_shape_vec(shape, token_ids)
            .map_err(|e| OxideError::Ml(format!("Shape error for input_ids: {e}")))?;
        let mask_array = Array2::from_shape_vec(shape, attention_mask)
            .map_err(|e| OxideError::Ml(format!("Shape error for attention_mask: {e}")))?;

        let input_ids_tensor = Tensor::from_array(input_ids_array)
            .map_err(|e| OxideError::Ml(format!("Tensor conversion error: {e}")))?;
        let mask_tensor = Tensor::from_array(mask_array)
            .map_err(|e| OxideError::Ml(format!("Tensor conversion error: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| OxideError::Ml("Failed to lock ONNX session".into()))?;

        let inputs = ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => mask_tensor,
        ];

        let outputs = session
            .run(inputs)
            .map_err(|e| OxideError::Ml(format!("ONNX run error: {e}")))?;

        // Output tensor name is typically "last_hidden_state" or index 0
        let (_name, output_tensor) = outputs
            .iter()
            .next()
            .ok_or_else(|| OxideError::Ml("No output tensor returned from ONNX".into()))?;

        let (shape, data) = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| OxideError::Ml(format!("Failed to extract output tensor: {e}")))?;

        let shape_slice: &[i64] = &shape[..];
        let hidden_dim = if shape_slice.len() == 3 {
            shape_slice[2] as usize
        } else {
            self.dimension
        };

        // Mean-pooling over token sequence
        let mut pooled = vec![0.0f32; hidden_dim];
        let num_tokens = seq_len.max(1);

        for t in 0..seq_len {
            for h in 0..hidden_dim {
                let idx = t * hidden_dim + h;
                if idx < data.len() {
                    pooled[h] += data[idx];
                }
            }
        }
        for h in 0..hidden_dim {
            pooled[h] /= num_tokens as f32;
        }

        // L2 Normalization
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            Ok(pooled.into_iter().map(|x| x / norm).collect())
        } else {
            Ok(pooled)
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
