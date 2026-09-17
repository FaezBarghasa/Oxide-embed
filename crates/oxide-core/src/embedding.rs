use crate::id::bytes_to_hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolingMethod {
    Mean,
    LastToken,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub model: String,
    pub file: String,
    pub dimension: usize,
    pub quantization: String,
    pub pooling: PoolingMethod,
    pub normalized: bool,
    pub vector_set_id: String,
}

impl EmbeddingMetadata {
    pub fn qwen3_0_6b_default() -> Self {
        let model = "qwen3-embedding-0.6b".to_string();
        let file = "Qwen3-Embedding-0.6B-Q4_K_M.gguf".to_string();
        let dimension = 1024;
        let quantization = "q4_k_m".to_string();
        let pooling = PoolingMethod::LastToken;
        let normalized = true;

        let vector_set_id =
            Self::compute_vector_set_id(&model, &quantization, dimension, &pooling, normalized);

        Self {
            model,
            file,
            dimension,
            quantization,
            pooling,
            normalized,
            vector_set_id,
        }
    }

    pub fn compute_vector_set_id(
        model: &str,
        quantization: &str,
        dimension: usize,
        pooling: &PoolingMethod,
        normalized: bool,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model.as_bytes());
        hasher.update(quantization.as_bytes());
        hasher.update(dimension.to_string().as_bytes());
        hasher.update(format!("{:?}", pooling).as_bytes());
        if normalized {
            hasher.update(b"norm:true");
        } else {
            hasher.update(b"norm:false");
        }
        bytes_to_hex(&hasher.finalize())
    }
}
