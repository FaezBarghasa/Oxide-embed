use oxide_ml::candle_embedder::{CandleBertEmbedder, cluster_embeddings, cosine_similarity};
use oxide_ml::{Embedder, MockEmbedder};

#[tokio::test]
async fn test_offline_embedder_dimension_and_normalization() {
    let embedder = CandleBertEmbedder::new_offline();
    assert_eq!(embedder.dimension(), 384);

    let text1 = "function calculate_sum(a, b) { return a + b; }";
    let text2 = "function calculate_sum(a, b) { return a + b; }";
    let text3 = "struct CompletelyDifferentDatabaseEngine;";

    let emb1 = embedder.embed(text1).await.expect("embed text1");
    let emb2 = embedder.embed(text2).await.expect("embed text2");
    let emb3 = embedder.embed(text3).await.expect("embed text3");

    assert_eq!(emb1.len(), 384);
    assert_eq!(emb1, emb2);

    // Check unit length / normalization: sqrt(sum(x^2)) ~= 1.0
    let norm: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4);

    let sim_same = cosine_similarity(&emb1, &emb2);
    assert!((sim_same - 1.0).abs() < 1e-4);

    let sim_diff = cosine_similarity(&emb1, &emb3);
    assert!(sim_diff < 0.99);
}

#[tokio::test]
async fn test_mock_embedder() {
    let mock = MockEmbedder::new(384);
    let emb = mock.embed("test mock text").await.expect("embed mock");
    assert_eq!(emb.len(), 384);
}

#[test]
fn test_vector_clustering() {
    let mut v1 = vec![0.0f32; 384];
    v1[0] = 1.0;
    let mut v2 = vec![0.0f32; 384];
    v2[0] = 0.99;
    v2[1] = 0.05;

    let mut v3 = vec![0.0f32; 384];
    v3[100] = 1.0;

    let clusters = cluster_embeddings(&[v1, v2, v3], 0.85);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0], vec![0, 1]);
    assert_eq!(clusters[1], vec![2]);
}

#[tokio::test]
async fn test_onnx_gemma_embedder() {
    if let Ok(embedder) = oxide_ml::OnnxGemmaEmbedder::load_default() {
        assert_eq!(embedder.dimension(), 768);

        let text1 = "fn parse_ast_tree(node: &Node) -> Result<Ast>";
        let text2 = "fn parse_ast_tree(node: &Node) -> Result<Ast>";
        let text3 = "const CSS_COLOR_BACKGROUND: &str = #ffffff;";

        let emb1 = embedder.embed(text1).await.expect("embed text1");
        let emb2 = embedder.embed(text2).await.expect("embed text2");
        let emb3 = embedder.embed(text3).await.expect("embed text3");

        assert_eq!(emb1.len(), 768);
        assert_eq!(emb1, emb2);

        let norm: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);

        let sim_same = cosine_similarity(&emb1, &emb2);
        assert!((sim_same - 1.0).abs() < 1e-4);

        let sim_diff = cosine_similarity(&emb1, &emb3);
        assert!(sim_diff < 0.99);
    }
}

#[tokio::test]
async fn test_candle_qwen_embedder() {
    if let Ok(embedder) = oxide_ml::CandleQwenEmbedder::load_default() {
        assert_eq!(embedder.dimension(), 1024);

        let text1 = "fn handle_index(store: &SurrealProjectStore) -> Result<()>";
        let text2 = "fn handle_index(store: &SurrealProjectStore) -> Result<()>";
        let text3 = "let margin_top = 24px;";

        let emb1 = embedder.embed(text1).await.expect("embed text1");
        let emb2 = embedder.embed(text2).await.expect("embed text2");
        let emb3 = embedder.embed(text3).await.expect("embed text3");

        assert_eq!(emb1.len(), 1024);
        assert_eq!(emb1, emb2);

        let norm: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);

        let sim_same = cosine_similarity(&emb1, &emb2);
        assert!((sim_same - 1.0).abs() < 1e-4);

        let sim_diff = cosine_similarity(&emb1, &emb3);
        assert!(sim_diff < 0.99);
    }
}

#[tokio::test]
async fn test_batched_embedding_parity() {
    let embedder = CandleBertEmbedder::new_offline();
    let text1 = "function calculate_sum(a, b) { return a + b; }".to_string();
    let text2 = "struct ProjectStoreEngine;".to_string();
    let text3 = "impl ProjectWalker for LocalFileSystem {}".to_string();

    let single1 = embedder.embed(&text1).await.expect("embed single 1");
    let single2 = embedder.embed(&text2).await.expect("embed single 2");
    let single3 = embedder.embed(&text3).await.expect("embed single 3");

    let batch = embedder
        .embed_batch(&[text1, text2, text3])
        .await
        .expect("embed batch");
    assert_eq!(batch.len(), 3);

    let sim1 = cosine_similarity(&single1, &batch[0]);
    let sim2 = cosine_similarity(&single2, &batch[1]);
    let sim3 = cosine_similarity(&single3, &batch[2]);

    assert!(sim1 > 0.999, "batch item 0 similarity was {sim1}");
    assert!(sim2 > 0.999, "batch item 1 similarity was {sim2}");
    assert!(sim3 > 0.999, "batch item 2 similarity was {sim3}");
}
