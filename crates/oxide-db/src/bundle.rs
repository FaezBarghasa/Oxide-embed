use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};
use oxide_core::error::{OxideError, Result};
use oxide_core::OxideManifest;
use crate::store::ProjectStore;

pub struct OxemBundle;

impl OxemBundle {
    pub async fn export<S: ProjectStore, P: AsRef<Path>>(
        store: &S,
        manifest: &OxideManifest,
        output_path: P,
    ) -> Result<()> {
        let file = File::create(output_path)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // 1. Write manifest.json
        zip.start_file("manifest.json", options)
            .map_err(|e| OxideError::Other(e.to_string()))?;
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        zip.write_all(manifest_json.as_bytes())?;

        // 2. Write export.json (records)
        zip.start_file("export.json", options)
            .map_err(|e| OxideError::Other(e.to_string()))?;
        let dump_data = store.export_surql().await?;
        zip.write_all(dump_data.as_bytes())?;

        zip.finish().map_err(|e| OxideError::Other(e.to_string()))?;
        Ok(())
    }

    pub async fn import<S: ProjectStore, P: AsRef<Path>>(
        store: &S,
        input_path: P,
    ) -> Result<OxideManifest> {
        let file = File::open(input_path)?;
        let mut zip = ZipArchive::new(file).map_err(|e| OxideError::Other(e.to_string()))?;

        // 1. Read manifest.json
        let manifest: OxideManifest = {
            let mut manifest_file = zip
                .by_name("manifest.json")
                .map_err(|e| OxideError::Other(e.to_string()))?;
            let mut manifest_str = String::new();
            manifest_file.read_to_string(&mut manifest_str)?;
            let manifest: OxideManifest = serde_json::from_str(&manifest_str)?;
            manifest.validate()?;
            manifest
        };

        // 2. Read export.json
        let export_str = {
            let mut export_file = zip
                .by_name("export.json")
                .map_err(|e| OxideError::Other(e.to_string()))?;
            let mut export_str = String::new();
            export_file.read_to_string(&mut export_str)?;
            export_str
        };

        store.import_surql(&export_str).await?;
        Ok(manifest)
    }
}
