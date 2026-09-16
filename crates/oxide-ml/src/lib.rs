pub mod candle_embedder;
pub mod device;
pub mod embedder;
pub mod mock;
pub mod model;

pub use candle_embedder::{cluster_embeddings, cosine_similarity, CandleBertEmbedder};
pub use embedder::Embedder;
pub use mock::MockEmbedder;
pub use model::ModelManager;
