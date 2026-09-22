use oxide_core::id::FileId;
use oxide_core::symbol::SymbolKind;
use oxide_parser::Language;
use oxide_parser::chunker::Chunker;
use oxide_parser::doc_linker::DocLinker;
use oxide_parser::languages::get_extractor;
use oxide_parser::outline::OutlineGenerator;

#[test]
fn test_rust_extractor() {
    let code = r#"
pub struct DeviceManager {
    id: u32,
}

impl DeviceManager {
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn reset(&mut self) {
        self.id = 0;
    }
}
"#;
    let file_id = FileId::from_relative_path("src/device.rs");
    let ext = get_extractor(Language::Rust).expect("rust extractor");
    let symbols = ext.extract_symbols(&file_id, code);

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "DeviceManager" && s.kind == SymbolKind::Struct)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "new" && s.kind == SymbolKind::Method)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "reset" && s.kind == SymbolKind::Method)
    );

    let outline = OutlineGenerator::generate_outline(&symbols);
    assert!(outline.contains("struct DeviceManager"));
    assert!(outline.contains("fn:"));
}

#[test]
fn test_typescript_and_python_extractors() {
    let ts_code = r#"
export class MotorController {
    speed: number;
    constructor(speed: number) {
        this.speed = speed;
    }
    setSpeed(val: number): void {
        this.speed = val;
    }
}
"#;
    let fid_ts = FileId::from_relative_path("src/motor.ts");
    let ext_ts = get_extractor(Language::TypeScript).expect("ts extractor");
    let syms_ts = ext_ts.extract_symbols(&fid_ts, ts_code);
    assert!(
        syms_ts
            .iter()
            .any(|s| s.name == "MotorController" && s.kind == SymbolKind::Class)
    );
    assert!(
        syms_ts
            .iter()
            .any(|s| s.name == "setSpeed" && s.kind == SymbolKind::Method)
    );

    let py_code = r#"
class ThermalRegulator:
    def __init__(self, target_temp: float):
        self.target = target_temp

    def heat_up(self, amount: float):
        self.target += amount
"#;
    let fid_py = FileId::from_relative_path("src/thermal.py");
    let ext_py = get_extractor(Language::Python).expect("python extractor");
    let syms_py = ext_py.extract_symbols(&fid_py, py_code);
    assert!(
        syms_py
            .iter()
            .any(|s| s.name == "ThermalRegulator" && s.kind == SymbolKind::Class)
    );
    assert!(
        syms_py
            .iter()
            .any(|s| s.name == "heat_up" && s.kind == SymbolKind::Method)
    );
}

#[test]
fn test_doc_linker_extraction_and_symbol_linking() {
    let md = r#"
# Firmware Core Architecture
Overview of firmware design.

## Device Lifecycle
The DeviceManager manages initial boot and reset sequences.

## Motor Control
Detailed specifications for MotorController speed curves.
"#;
    let sections = DocLinker::extract_doc_sections("docs/ARCH.md", md);
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].heading, "Firmware Core Architecture");
    assert_eq!(sections[1].heading, "Device Lifecycle");
    assert_eq!(sections[2].heading, "Motor Control");

    let fid = FileId::from_relative_path("src/device.rs");
    let sym_dev = oxide_core::SymbolRecord {
        id: oxide_core::SymbolId::new(&fid, "DeviceManager"),
        file_id: fid.clone(),
        kind: SymbolKind::Struct,
        name: "DeviceManager".into(),
        qualified_name: Some("DeviceManager".into()),
        start_line: 1,
        end_line: 5,
        signature: Some("pub struct DeviceManager".into()),
        doc: None,
        fingerprint: "fp1".into(),
        is_macro_node: true,
        parent_id: None,
        breadcrumbs: vec!["DeviceManager".into()],
        summary: Some("struct DeviceManager".into()),
    };

    let edges = DocLinker::link_sections_to_symbols(&sections, &[sym_dev]);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].doc_section_id, "docs/ARCH.md:1");
}

#[test]
fn test_chunker_symbol_and_outline_chunks() {
    let code =
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }";
    let fid = FileId::from_relative_path("math.rs");
    let ext = get_extractor(Language::Rust).unwrap();
    let symbols = ext.extract_symbols(&fid, code);

    let chunker = Chunker::new(512);
    let chunks = chunker.chunk_file(&fid, code, &symbols);

    // 1 outline chunk + 2 symbol chunks = 3 chunks
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].kind, oxide_core::ChunkKind::FileOutline);
    assert_eq!(chunks[1].kind, oxide_core::ChunkKind::SymbolChunk);
    assert_eq!(chunks[2].kind, oxide_core::ChunkKind::SymbolChunk);
}

#[test]
fn test_c_and_cpp_extractors() {
    let c_code = r#"
#include <stdint.h>
#include "stm32f4xx_hal.h"

typedef struct {
    uint32_t baud_rate;
    uint8_t mode;
} UART_ConfigTypeDef;

void UART_Init(UART_ConfigTypeDef *config) {
    HAL_UART_Init(config);
}
"#;
    let fid_c = FileId::from_relative_path("drivers/uart.c");
    let ext_c = get_extractor(Language::C).expect("c extractor");
    let syms_c = ext_c.extract_symbols(&fid_c, c_code);
    assert!(
        syms_c
            .iter()
            .any(|s| s.name == "UART_ConfigTypeDef" && (s.kind == SymbolKind::Struct || s.kind == SymbolKind::TypeAlias))
    );
    assert!(
        syms_c
            .iter()
            .any(|s| s.name == "UART_Init" && s.kind == SymbolKind::Function)
    );

    let import_edges = ext_c.extract_import_edges(&fid_c, c_code);
    assert!(
        import_edges
            .iter()
            .any(|e| e.imported_path == "stm32f4xx_hal.h" || e.imported_path == "<stdint.h>")
    );

    let cpp_code = r#"
#include <iostream>

namespace Oxide {
    class EngineController {
    public:
        void startEngine() {
            initIgnition();
        }
    private:
        void initIgnition() {}
    };
}
"#;
    let fid_cpp = FileId::from_relative_path("src/engine.cpp");
    let ext_cpp = get_extractor(Language::Cpp).expect("cpp extractor");
    let syms_cpp = ext_cpp.extract_symbols(&fid_cpp, cpp_code);
    assert!(
        syms_cpp
            .iter()
            .any(|s| s.name == "EngineController" && s.kind == SymbolKind::Class)
    );
    assert!(
        syms_cpp
            .iter()
            .any(|s| s.name == "startEngine" && s.kind == SymbolKind::Method)
    );
}

