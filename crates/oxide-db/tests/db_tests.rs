use oxide_core::FileRecord;
use oxide_core::chunk::{ChunkKind, ChunkRecord};
use oxide_core::cognify::{DocReferenceEdge, DocSection};
use oxide_core::id::{FileId, ProjectId, SymbolId};
use oxide_core::symbol::{CallEdge, SymbolKind, SymbolRecord};
use oxide_db::traversal::GraphTraversalService;
use oxide_db::{ProjectStore, SearchQuery, SurrealProjectStore};

#[tokio::test]
async fn test_surreal_db_crud_and_traversal() {
    let temp_dir = std::env::temp_dir().join("oxide_test_db_crud");
    let _ = std::fs::remove_dir_all(&temp_dir);

    let store = SurrealProjectStore::open(&temp_dir)
        .await
        .expect("open surreal project store");

    let proj_id = ProjectId::new_v7();
    let file = FileRecord {
        id: FileId::from_relative_path("src/core/driver.rs"),
        project_id: proj_id.clone(),
        relative_path: "src/core/driver.rs".into(),
        language: Some("rust".into()),
        content_hash: Some("hash123".into()),
        size_bytes: 512,
        last_indexed_at: Some(chrono::Utc::now()),
    };

    store.upsert_file(&file).await.expect("upsert file");
    assert_eq!(store.count_files().await.unwrap(), 1);

    let sym_driver = SymbolRecord {
        id: SymbolId::new(&file.id, "init_hardware"),
        file_id: file.id.clone(),
        kind: SymbolKind::Function,
        name: "init_hardware".into(),
        qualified_name: Some("driver::init_hardware".into()),
        start_line: 10,
        end_line: 25,
        signature: Some("pub fn init_hardware() -> Result<()>".into()),
        doc: Some("Initializes SPI and UART controllers".into()),
        fingerprint: "fp_init".into(),
        is_macro_node: false,
        parent_id: None,
        breadcrumbs: vec!["driver".into(), "init_hardware".into()],
        summary: None,
    };

    let sym_caller = SymbolRecord {
        id: SymbolId::new(&file.id, "main_bootstrap"),
        file_id: file.id.clone(),
        kind: SymbolKind::Function,
        name: "main_bootstrap".into(),
        qualified_name: Some("driver::main_bootstrap".into()),
        start_line: 30,
        end_line: 45,
        signature: Some("pub fn main_bootstrap()".into()),
        doc: None,
        fingerprint: "fp_boot".into(),
        is_macro_node: false,
        parent_id: None,
        breadcrumbs: vec!["driver".into(), "main_bootstrap".into()],
        summary: None,
    };

    store
        .upsert_symbol(&sym_driver)
        .await
        .expect("upsert symbol 1");
    store
        .upsert_symbol(&sym_caller)
        .await
        .expect("upsert symbol 2");
    assert_eq!(store.count_symbols().await.unwrap(), 2);

    // Call edge: main_bootstrap calls init_hardware
    let call_edge = CallEdge {
        caller_symbol_id: sym_caller.id.clone(),
        callee_name: "init_hardware".into(),
        line: 35,
    };
    store
        .upsert_call_edge(&call_edge)
        .await
        .expect("upsert call edge");

    // Doc section and reference edge
    let doc_sec = DocSection {
        id: "docs/HARDWARE.md:0".into(),
        file_path: "docs/HARDWARE.md".into(),
        heading: "Hardware Initialization".into(),
        content: "Call init_hardware before configuring motors.".into(),
        embedding: Some(vec![0.05f32; 384]),
    };
    store
        .upsert_doc_section(&doc_sec)
        .await
        .expect("upsert doc section");

    let doc_ref = DocReferenceEdge {
        doc_section_id: doc_sec.id.clone(),
        symbol_id: sym_driver.id.clone(),
        context: "Call init_hardware before configuring motors.".into(),
    };
    store
        .upsert_doc_reference(&doc_ref)
        .await
        .expect("upsert doc ref");

    // Chunk upsert
    let chunk = ChunkRecord {
        id: oxide_core::ChunkId::new(&file.id, Some(&sym_driver.id), "symbol_chunk", "chunkhash1"),
        file_id: file.id.clone(),
        symbol_id: Some(sym_driver.id.clone()),
        kind: ChunkKind::SymbolChunk,
        text: "pub fn init_hardware() -> Result<()> { Ok(()) }".into(),
        outline: Some("pub fn init_hardware()".into()),
        content_hash: "chunkhash1".into(),
        start_line: 10,
        end_line: 25,
        embedding: Some(vec![0.1f32; 384]),
        embedding_model: Some("bge-small-en-v1.5".into()),
        embedding_dim: Some(384),
        vector_set_id: Some("default".into()),
        updated_at: chrono::Utc::now(),
    };
    store.upsert_chunk(&chunk).await.expect("upsert chunk");
    assert_eq!(store.count_chunks().await.unwrap(), 1);

    // Test Search
    let search_res = store
        .search(&SearchQuery {
            text: "init_hardware".into(),
            embedding: Some(vec![0.1f32; 384]),
            limit: 5,
            language: None,
        })
        .await
        .expect("search");

    assert!(!search_res.is_empty());

    // Test Subgraph Traversal
    let subgraph = GraphTraversalService::get_subgraph(&store, "init_hardware", 2)
        .await
        .expect("get subgraph");

    assert!(subgraph.is_some());
    let ctx = subgraph.unwrap();
    assert_eq!(ctx.target_symbol, "init_hardware");
    assert!(ctx.callers.contains(&sym_caller.id.0));
    assert!(!ctx.doc_references.is_empty());

    let compact = ctx.to_compact_string();
    assert!(compact.contains("[Target] init_hardware"));
    assert!(compact.contains("[Callers]"));
}

#[tokio::test]
async fn test_typed_semantic_memory_crud_and_recall() {
    let temp_dir = std::env::temp_dir().join("oxide_test_memory_crud");
    let _ = std::fs::remove_dir_all(&temp_dir);

    let store = SurrealProjectStore::open(&temp_dir)
        .await
        .expect("open surreal store");

    let proj_id = ProjectId::new_v7();

    // 1. Upsert a Decision Memory
    let mut mem_decision = oxide_core::MemoryRecord::new(
        proj_id.clone(),
        oxide_core::MemoryKind::Decision,
        "Embedded HAL Architecture",
        "Always use RTIC v2 and pure no_std for STM32 targets",
    );
    mem_decision.tags = vec!["stm32".into(), "embedded".into()];
    mem_decision.symbol_ref = Some("init_hardware".into());

    let emb1 = vec![0.8f32; 384];
    store
        .upsert_memory(&mem_decision, Some(emb1.clone()))
        .await
        .expect("upsert decision memory");

    // 2. Fetch by ID
    let fetched = store
        .get_memory(&mem_decision.id)
        .await
        .expect("get memory")
        .expect("memory exists");
    assert_eq!(fetched.title, "Embedded HAL Architecture");
    assert_eq!(fetched.kind, oxide_core::MemoryKind::Decision);
    assert_eq!(fetched.tags, vec!["stm32", "embedded"]);

    // 3. Upsert a Preference Memory
    let mem_pref = oxide_core::MemoryRecord::new(
        proj_id.clone(),
        oxide_core::MemoryKind::Preference,
        "Zero Unwrap Rule",
        "Avoid unwrap in production code; use typed Result<T, E>",
    );
    store
        .upsert_memory(&mem_pref, Some(vec![0.1f32; 384]))
        .await
        .expect("upsert pref memory");

    // 4. Recall memories filtered by kind and vector
    let recalled_decisions = store
        .recall_memories(
            Some(&emb1),
            Some(oxide_core::MemoryKind::Decision),
            &[],
            None,
            5,
        )
        .await
        .expect("recall decisions");
    assert_eq!(recalled_decisions.len(), 1);
    assert_eq!(recalled_decisions[0].title, "Embedded HAL Architecture");

    // 5. Active rules listing
    let active_rules = store.list_active_rules().await.expect("list active rules");
    assert_eq!(active_rules.len(), 2);

    // 6. Conflict detection
    let conflicting_emb = vec![0.79f32; 384];
    let conflicts = store
        .find_conflicts(oxide_core::MemoryKind::Decision, &conflicting_emb, 0.75)
        .await
        .expect("find conflicts");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].id, mem_decision.id);

    // 7. Link memory to symbol
    let sym_id = SymbolId::new(&FileId::from_relative_path("src/core.rs"), "init_hardware");
    store
        .link_memory_to_symbol(&mem_decision.id, &sym_id, "governs")
        .await
        .expect("link memory to symbol");
}
