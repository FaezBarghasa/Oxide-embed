use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxide_ml::Embedder;
use oxide_ml::candle_embedder::{CandleBertEmbedder, cluster_embeddings, cosine_similarity};

fn bench_candle_embedder(c: &mut Criterion) {
    let mut group = c.benchmark_group("ml_candle_embedder");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let embedder = CandleBertEmbedder::new_offline();

    let sample_text = "pub async fn handle_subgraph_traversal(store: &SurrealProjectStore, symbol: &str) -> Result<SubgraphContext>";

    group.bench_function("offline_384d_embedding_projection", |b| {
        b.to_async(&rt)
            .iter(|| async { embedder.embed(black_box(sample_text)).await.unwrap() });
    });

    let v1 = vec![0.1f32; 384];
    let v2 = vec![0.2f32; 384];

    group.bench_function("cosine_similarity_384d", |b| {
        b.iter(|| cosine_similarity(black_box(&v1), black_box(&v2)));
    });

    // Generate 50 sample vectors
    let mut vectors = Vec::new();
    for i in 0..50 {
        let mut v = vec![0.0f32; 384];
        v[i % 384] = 1.0;
        v[(i + 1) % 384] = 0.5;
        vectors.push(v);
    }

    group.bench_function("cluster_50_embeddings", |b| {
        b.iter(|| cluster_embeddings(black_box(&vectors), black_box(0.85)));
    });

    group.finish();
}

criterion_group!(benches, bench_candle_embedder);
criterion_main!(benches);
