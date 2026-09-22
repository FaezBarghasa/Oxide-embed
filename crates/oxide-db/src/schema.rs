pub const INITIAL_SCHEMA_SURQL: &str = r#"
DEFINE TABLE project SCHEMAFULL;
DEFINE FIELD uuid ON project TYPE string;
DEFINE FIELD name ON project TYPE string;
DEFINE FIELD created_at ON project TYPE datetime;
DEFINE FIELD updated_at ON project TYPE datetime;

DEFINE TABLE file SCHEMAFULL;
DEFINE FIELD project_id ON file TYPE string;
DEFINE FIELD relative_path ON file TYPE string;
DEFINE FIELD language ON file TYPE option<string>;
DEFINE FIELD content_hash ON file TYPE option<string>;
DEFINE FIELD size_bytes ON file TYPE int;
DEFINE FIELD last_indexed_at ON file TYPE option<datetime>;
DEFINE INDEX file_path_unique ON file FIELDS relative_path UNIQUE;

DEFINE TABLE symbol SCHEMAFULL;
DEFINE FIELD file_id ON symbol TYPE string;
DEFINE FIELD kind ON symbol TYPE string;
DEFINE FIELD name ON symbol TYPE string;
DEFINE FIELD qualified_name ON symbol TYPE option<string>;
DEFINE FIELD start_line ON symbol TYPE int;
DEFINE FIELD end_line ON symbol TYPE int;
DEFINE FIELD signature ON symbol TYPE option<string>;
DEFINE FIELD doc ON symbol TYPE option<string>;
DEFINE FIELD fingerprint ON symbol TYPE string;
DEFINE FIELD is_macro_node ON symbol TYPE bool DEFAULT false;
DEFINE FIELD parent_id ON symbol TYPE option<string>;
DEFINE FIELD breadcrumbs ON symbol TYPE array<string> DEFAULT [];
DEFINE FIELD summary ON symbol TYPE option<string>;
DEFINE INDEX symbol_fingerprint_idx ON symbol FIELDS fingerprint;
DEFINE INDEX idx_symbol_macro ON symbol FIELDS is_macro_node;

DEFINE TABLE chunk SCHEMAFULL;
DEFINE FIELD file_id ON chunk TYPE string;
DEFINE FIELD symbol_id ON chunk TYPE option<string>;
DEFINE FIELD kind ON chunk TYPE string;
DEFINE FIELD text ON chunk TYPE string;
DEFINE FIELD outline ON chunk TYPE option<string>;
DEFINE FIELD content_hash ON chunk TYPE string;
DEFINE FIELD start_line ON chunk TYPE int;
DEFINE FIELD end_line ON chunk TYPE int;
DEFINE FIELD embedding ON chunk TYPE option<array<float>>;
DEFINE FIELD embedding_model ON chunk TYPE option<string>;
DEFINE FIELD embedding_dim ON chunk TYPE option<int>;
DEFINE FIELD vector_set_id ON chunk TYPE option<string>;
DEFINE FIELD updated_at ON chunk TYPE datetime;
DEFINE INDEX chunk_vector_idx ON chunk FIELDS embedding HNSW DIMENSION 384 DIST COSINE;

DEFINE TABLE oxide_migration SCHEMAFULL;
DEFINE FIELD from_version ON oxide_migration TYPE int;
DEFINE FIELD to_version ON oxide_migration TYPE int;
DEFINE FIELD started_at ON oxide_migration TYPE datetime;
DEFINE FIELD finished_at ON oxide_migration TYPE option<datetime>;
DEFINE FIELD status ON oxide_migration TYPE string;
DEFINE FIELD backup_path ON oxide_migration TYPE option<string>;

-- Cognee Cognitive Graph Tables
DEFINE TABLE calls SCHEMAFULL;
DEFINE FIELD in ON calls TYPE record<symbol>;
DEFINE FIELD out ON calls TYPE record<symbol>;
DEFINE FIELD caller_name ON calls TYPE option<string>;
DEFINE FIELD callee_name ON calls TYPE option<string>;
DEFINE FIELD line ON calls TYPE option<int>;
DEFINE FIELD weight ON calls TYPE float DEFAULT 1.0;
DEFINE FIELD traversal_count ON calls TYPE int DEFAULT 0;
DEFINE FIELD last_traversed_at ON calls TYPE option<datetime>;
DEFINE FIELD valid_from ON calls TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_to ON calls TYPE option<datetime>;
DEFINE INDEX idx_calls_active ON calls FIELDS valid_to;

DEFINE TABLE contains SCHEMAFULL;
DEFINE FIELD in ON contains TYPE record<file>;
DEFINE FIELD out ON contains TYPE record<symbol>;
DEFINE FIELD valid_from ON contains TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_to ON contains TYPE option<datetime>;

DEFINE TABLE imports SCHEMAFULL;
DEFINE FIELD in ON imports TYPE record<file>;
DEFINE FIELD out ON imports TYPE record<file>;
DEFINE FIELD imported_path ON imports TYPE option<string>;
DEFINE FIELD imported_symbols ON imports TYPE array<string>;
DEFINE FIELD weight ON imports TYPE float DEFAULT 1.0;

DEFINE TABLE doc_section SCHEMAFULL;
DEFINE FIELD file_path ON doc_section TYPE string;
DEFINE FIELD heading ON doc_section TYPE string;
DEFINE FIELD content ON doc_section TYPE string;
DEFINE FIELD embedding ON doc_section TYPE option<array<float>>;

DEFINE TABLE doc_reference SCHEMAFULL;
DEFINE FIELD in ON doc_reference TYPE record<doc_section>;
DEFINE FIELD out ON doc_reference TYPE record<symbol>;
DEFINE FIELD symbol_name ON doc_reference TYPE option<string>;
DEFINE FIELD context ON doc_reference TYPE string;
DEFINE FIELD created_at ON doc_reference TYPE datetime DEFAULT time::now();

DEFINE TABLE cerebrum_rule SCHEMAFULL;
DEFINE FIELD rule ON cerebrum_rule TYPE string;
DEFINE FIELD source_cluster ON cerebrum_rule TYPE option<string>;
DEFINE FIELD created_at ON cerebrum_rule TYPE datetime DEFAULT time::now();

DEFINE TABLE buglog SCHEMAFULL;
DEFINE FIELD description ON buglog TYPE string;
DEFINE FIELD resolution ON buglog TYPE option<string>;
DEFINE FIELD embedding ON buglog TYPE option<array<float>>;
DEFINE FIELD status ON buglog TYPE string DEFAULT "active";
DEFINE FIELD cluster_id ON buglog TYPE option<string>;
DEFINE FIELD consolidated_into ON buglog TYPE option<record<cerebrum_rule>>;
DEFINE FIELD created_at ON buglog TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_buglog_active ON buglog FIELDS status;

DEFINE TABLE memory_evolution_log SCHEMAFULL;
DEFINE FIELD action ON memory_evolution_log TYPE string;
DEFINE FIELD target_table ON memory_evolution_log TYPE string;
DEFINE FIELD records_affected ON memory_evolution_log TYPE int;
DEFINE FIELD executed_at ON memory_evolution_log TYPE datetime DEFAULT time::now();

-- OpenWolf Execution Context Hygiene Tables
DEFINE TABLE bash_cache SCHEMAFULL;
DEFINE FIELD command ON bash_cache TYPE string;
DEFINE FIELD exit_code ON bash_cache TYPE int;
DEFINE FIELD log_path ON bash_cache TYPE string;
DEFINE FIELD original_bytes ON bash_cache TYPE int;
DEFINE FIELD condensed_bytes ON bash_cache TYPE int;
DEFINE FIELD created_at ON bash_cache TYPE datetime DEFAULT time::now();

DEFINE TABLE session_read SCHEMAFULL;
DEFINE FIELD session_id ON session_read TYPE string;
DEFINE FIELD file_id ON session_read TYPE string;
DEFINE FIELD content_hash ON session_read TYPE string;
DEFINE FIELD read_count ON session_read TYPE int DEFAULT 1;
DEFINE FIELD last_read_at ON session_read TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_session_file ON session_read FIELDS session_id, file_id UNIQUE;

DEFINE TABLE handoff_checkpoint SCHEMAFULL;
DEFINE FIELD objective ON handoff_checkpoint TYPE string;
DEFINE FIELD active_branch ON handoff_checkpoint TYPE string;
DEFINE FIELD files_modified ON handoff_checkpoint TYPE array<string>;
DEFINE FIELD pending_errors ON handoff_checkpoint TYPE array<string>;
DEFINE FIELD next_step ON handoff_checkpoint TYPE string;
DEFINE FIELD created_at ON handoff_checkpoint TYPE datetime DEFAULT time::now();

DEFINE TABLE token_ledger SCHEMAFULL;
DEFINE FIELD agent_harness ON token_ledger TYPE string;
DEFINE FIELD model_id ON token_ledger TYPE string;
DEFINE FIELD input_tokens ON token_ledger TYPE int;
DEFINE FIELD cached_tokens ON token_ledger TYPE int;
DEFINE FIELD output_tokens ON token_ledger TYPE int;
DEFINE FIELD reasoning_tokens ON token_ledger TYPE int;
DEFINE FIELD estimated_cost_usd ON token_ledger TYPE float;
DEFINE FIELD recorded_at ON token_ledger TYPE datetime DEFAULT time::now();

-- Memanto Typed Semantic Memory Tables
DEFINE TABLE memory_record SCHEMAFULL;
DEFINE FIELD project_id ON memory_record TYPE string;
DEFINE FIELD session_id ON memory_record TYPE option<string>;
DEFINE FIELD kind ON memory_record TYPE string;
DEFINE FIELD title ON memory_record TYPE string;
DEFINE FIELD content ON memory_record TYPE string;
DEFINE FIELD tags ON memory_record TYPE array<string>;
DEFINE FIELD symbol_ref ON memory_record TYPE option<string>;
DEFINE FIELD status ON memory_record TYPE string DEFAULT "active";
DEFINE FIELD superseded_by ON memory_record TYPE option<string>;
DEFINE FIELD author ON memory_record TYPE option<string>;
DEFINE FIELD confidence ON memory_record TYPE float DEFAULT 1.0;
DEFINE FIELD source_hash ON memory_record TYPE option<string>;
DEFINE FIELD embedding ON memory_record TYPE option<array<float>>;
DEFINE FIELD created_at ON memory_record TYPE datetime DEFAULT time::now();
DEFINE FIELD valid_until ON memory_record TYPE option<datetime>;
DEFINE INDEX idx_memory_kind ON memory_record FIELDS kind;
DEFINE INDEX idx_memory_status ON memory_record FIELDS status;
DEFINE INDEX memory_vector_idx ON memory_record FIELDS embedding HNSW DIMENSION 384 DIST COSINE;

DEFINE TABLE governs SCHEMAFULL;
DEFINE FIELD in ON governs TYPE record<memory_record>;
DEFINE FIELD out ON governs TYPE record<symbol>;
DEFINE FIELD relation ON governs TYPE string DEFAULT "governs";
DEFINE FIELD created_at ON governs TYPE datetime DEFAULT time::now();

DEFINE TABLE replaces SCHEMAFULL;
DEFINE FIELD in ON replaces TYPE record<memory_record>;
DEFINE FIELD out ON replaces TYPE record<memory_record>;
DEFINE FIELD reason ON replaces TYPE option<string>;
DEFINE FIELD created_at ON replaces TYPE datetime DEFAULT time::now();

DEFINE TABLE derived_from SCHEMAFULL;
DEFINE FIELD in ON derived_from TYPE record<memory_record>;
DEFINE FIELD out ON derived_from TYPE option<string>;
DEFINE FIELD source_type ON derived_from TYPE string DEFAULT "git_commit";
DEFINE FIELD created_at ON derived_from TYPE datetime DEFAULT time::now();
"#;
