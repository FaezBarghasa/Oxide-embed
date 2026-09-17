use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxide_core::FileRecord;
use oxide_core::id::{FileId, ProjectId, SymbolId};
use oxide_core::symbol::{CallEdge, SymbolKind, SymbolRecord};
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};

fn bench_database_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_surreal_operations");
    let rt = tokio::runtime::Runtime::new().unwrap();

    let store = rt.block_on(async {
        let path = std::env::temp_dir().join("oxide_bench_db");
        let _ = std::fs::remove_dir_all(&path);
        let s = SurrealProjectStore::open(&path).await.unwrap();

        let proj_id = ProjectId::new_v7();
        let file = FileRecord {
            id: FileId::from_relative_path("src/lib.rs"),
            project_id: proj_id.clone(),
            relative_path: "src/lib.rs".into(),
            language: Some("rust".into()),
            content_hash: Some("hash_alpha".into()),
            size_bytes: 1024,
            last_indexed_at: Some(chrono::Utc::now()),
        };
        s.upsert_file(&file).await.unwrap();

        for i in 0..50 {
            let sym = SymbolRecord {
                id: SymbolId::new(&file.id, &format!("sym_{}", i)),
                file_id: file.id.clone(),
                kind: SymbolKind::Function,
                name: format!("sym_{}", i),
                qualified_name: Some(format!("crate::sym_{}", i)),
                start_line: i * 5 + 1,
                end_line: i * 5 + 4,
                signature: Some(format!("pub fn sym_{}()", i)),
                doc: None,
                fingerprint: format!("fp_{}", i),
            };
            s.upsert_symbol(&sym).await.unwrap();

            if i > 0 {
                s.upsert_call_edge(&CallEdge {
                    caller_symbol_id: sym.id,
                    callee_name: format!("sym_{}", i - 1),
                    line: i * 5 + 2,
                })
                .await
                .unwrap();
            }
        }

        s
    });

    group.bench_function("search_lexical_and_vector", |b| {
        let query = SearchQuery {
            text: "sym_10".into(),
            embedding: Some(vec![0.05f32; 384]),
            limit: 5,
            language: None,
        };
        b.to_async(&rt)
            .iter(|| async { store.search(black_box(&query)).await.unwrap() });
    });

    group.bench_function("subgraph_traversal_hops", |b| {
        b.to_async(&rt).iter(|| async {
            GraphTraversalService::get_subgraph(
                black_box(&store),
                black_box("sym_10"),
                black_box(2),
            )
            .await
            .unwrap()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_database_operations);
criterion_main!(benches);
