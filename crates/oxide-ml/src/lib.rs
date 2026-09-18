pub mod candle_embedder;
pub mod device;
pub mod embedder;
pub mod mock;
pub mod model;
pub mod onnx_embedder;
pub mod qwen_embedder;
pub mod rerank;

pub use candle_embedder::{CandleBertEmbedder, cluster_embeddings, cosine_similarity};
pub use embedder::Embedder;
pub use mock::MockEmbedder;
pub use model::ModelManager;
pub use onnx_embedder::OnnxGemmaEmbedder;
pub use qwen_embedder::CandleQwenEmbedder;
pub use rerank::MmrReranker;
