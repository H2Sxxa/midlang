use anyhow::Result;
use midlang_preset::{PresetBuilder, redb_store};
use std::path::PathBuf;

fn main() {
    build().unwrap();
}

fn build() -> Result<()> {
    let export_to = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("presets");
    std::fs::create_dir_all(&export_to)?;

    PresetBuilder::new()
        // Basic Preset
        .kv("en", "hello", "Hello")?
        .kv("en", "world", "World")?
        .kv("zh", "hello", "你好")?
        .kv("zh", "world", "世界")?
        .export(&redb_store(export_to.join("basic.redb"))?)?
        // With Coverage Test Preset
        .kv("en", "coverage_test", "Coverage Test")?
        .export(&redb_store(export_to.join("coverage.redb"))?)?;
    Ok(())
}
