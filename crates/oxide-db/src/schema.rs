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
DEFINE INDEX symbol_fingerprint_idx ON symbol FIELDS fingerprint;

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

DEFINE TABLE oxide_migration SCHEMAFULL;
DEFINE FIELD from_version ON oxide_migration TYPE int;
DEFINE FIELD to_version ON oxide_migration TYPE int;
DEFINE FIELD started_at ON oxide_migration TYPE datetime;
DEFINE FIELD finished_at ON oxide_migration TYPE option<datetime>;
DEFINE FIELD status ON oxide_migration TYPE string;
DEFINE FIELD backup_path ON oxide_migration TYPE option<string>;
"#;
