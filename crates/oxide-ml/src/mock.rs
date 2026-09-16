use async_trait::async_trait;
use sha2::{Digest, Sha256};
use oxide_core::error::Result;
use crate::embedder::Embedder;

pub struct MockEmbedder {
    dimension: usize,
}

impl MockEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for MockEmbedder {
    fn default() -> Self {
        Self { dimension: 1024 }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        let mut vec = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let byte = hash[i % hash.len()];
            let val = (byte as f32 / 255.0) * 2.0 - 1.0;
            vec.push(val);
        }

        // Normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut vec {
                *val /= norm;
            }
        }

        Ok(vec)
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
