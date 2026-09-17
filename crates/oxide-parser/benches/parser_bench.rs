use criterion::{Criterion, criterion_group, criterion_main};
use oxide_core::id::FileId;
use oxide_parser::Language;
use oxide_parser::chunker::Chunker;
use oxide_parser::doc_linker::DocLinker;
use oxide_parser::languages::get_extractor;
use oxide_parser::outline::OutlineGenerator;
use std::hint::black_box;

const RUST_SAMPLE: &str = r#"
pub struct EmbeddedDriver {
    pin_tx: u8,
    pin_rx: u8,
    baud_rate: u32,
}

impl EmbeddedDriver {
    pub fn new(pin_tx: u8, pin_rx: u8, baud_rate: u32) -> Self {
        Self { pin_tx, pin_rx, baud_rate }
    }

    pub fn transmit(&mut self, payload: &[u8]) -> Result<(), DriverError> {
        if payload.is_empty() {
            return Err(DriverError::EmptyBuffer);
        }
        Ok(())
    }

    pub fn receive(&mut self, buf: &mut [u8]) -> Result<usize, DriverError> {
        Ok(0)
    }
}
"#;

const TS_SAMPLE: &str = r#"
export interface SensorConfig {
    id: string;
    sampleRateHz: number;
    enabled: boolean;
}

export class SensorTelemetryManager {
    private config: SensorConfig;

    constructor(config: SensorConfig) {
        this.config = config;
    }

    public recordSample(val: number): void {
        console.log(`Sample: ${val}`);
    }

    public flush(): Promise<boolean> {
        return Promise.resolve(true);
    }
}
"#;

const PY_SAMPLE: &str = r#"
class HardwarePipeline:
    def __init__(self, endpoint: str):
        self.endpoint = endpoint
        self.active = False

    def connect(self) -> bool:
        self.active = True
        return True

    def stream_telemetry(self, frames: list):
        for frame in frames:
            yield frame * 2
"#;

const MD_SAMPLE: &str = r#"
# Embedded Driver Architecture
High performance bare-metal driver for UART & SPI communication.

## Transmission Protocol
Describes how EmbeddedDriver transmits buffers over GPIO pins.

## Telemetry Pipeline
SensorTelemetryManager coordinates with HardwarePipeline for stream processing.
"#;

fn bench_ast_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_ast_extraction");
    let file_id = FileId::from_relative_path("driver.rs");

    group.bench_function("rust_ast_extractor", |b| {
        let ext = get_extractor(Language::Rust).unwrap();
        b.iter(|| ext.extract_symbols(black_box(&file_id), black_box(RUST_SAMPLE)));
    });

    group.bench_function("typescript_ast_extractor", |b| {
        let ext = get_extractor(Language::TypeScript).unwrap();
        b.iter(|| ext.extract_symbols(black_box(&file_id), black_box(TS_SAMPLE)));
    });

    group.bench_function("python_ast_extractor", |b| {
        let ext = get_extractor(Language::Python).unwrap();
        b.iter(|| ext.extract_symbols(black_box(&file_id), black_box(PY_SAMPLE)));
    });

    group.finish();
}

fn bench_outline_and_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_outline_and_chunking");
    let file_id = FileId::from_relative_path("driver.rs");
    let ext = get_extractor(Language::Rust).unwrap();
    let symbols = ext.extract_symbols(&file_id, RUST_SAMPLE);

    group.bench_function("outline_generation", |b| {
        b.iter(|| OutlineGenerator::generate_outline(black_box(&symbols)));
    });

    let chunker = Chunker::new(512);
    group.bench_function("chunk_file_with_symbols", |b| {
        b.iter(|| {
            chunker.chunk_file(
                black_box(&file_id),
                black_box(RUST_SAMPLE),
                black_box(&symbols),
            )
        });
    });

    group.finish();
}

fn bench_doc_linking_and_anatomy(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_doc_and_anatomy");
    let file_id = FileId::from_relative_path("driver.rs");
    let ext = get_extractor(Language::Rust).unwrap();
    let symbols = ext.extract_symbols(&file_id, RUST_SAMPLE);

    group.bench_function("doc_section_extraction", |b| {
        b.iter(|| DocLinker::extract_doc_sections("README.md", black_box(MD_SAMPLE)));
    });

    let sections = DocLinker::extract_doc_sections("README.md", MD_SAMPLE);
    group.bench_function("doc_to_symbol_linking", |b| {
        b.iter(|| DocLinker::link_sections_to_symbols(black_box(&sections), black_box(&symbols)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ast_extraction,
    bench_outline_and_chunking,
    bench_doc_linking_and_anatomy
);
criterion_main!(benches);
