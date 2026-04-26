//! Tests for the SPSS (.sav) reader. Generates the test fixture programmatically
//! using `ambers::write_sav` so we don't ship binary blobs.

use std::sync::Arc;
use tempfile::TempDir;

use ambers::{metadata::SpssMetadata, Compression};
use arrow57::array::{Float64Array, RecordBatch, StringArray};
use arrow57::datatypes::{DataType, Field, Schema};
use octa::data::CellValue;
use octa::formats::FormatRegistry;

fn make_sav(dir: &TempDir) -> std::path::PathBuf {
    let path = dir.path().join("sample.sav");

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Float64, true),
        Field::new("name", DataType::Utf8, true),
        Field::new("score", DataType::Float64, true),
    ]));

    let id = Float64Array::from(vec![Some(1.0), Some(2.0), Some(3.0), None]);
    let name = StringArray::from(vec![
        Some("Alice"),
        Some("Bob"),
        Some("Charlie"),
        Some("Dana"),
    ]);
    let score = Float64Array::from(vec![Some(90.5), Some(82.0), None, Some(77.3)]);

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(id), Arc::new(name), Arc::new(score)],
    )
    .expect("build SPSS RecordBatch");

    let metadata = SpssMetadata::from_arrow_schema(&schema);
    ambers::write_sav(&path, &batch, &metadata, Compression::None, None).expect("write .sav");

    path
}

#[test]
fn spss_reader_reads_basic_file() {
    let dir = TempDir::new().unwrap();
    let path = make_sav(&dir);

    let registry = FormatRegistry::new();
    let reader = registry.reader_for_path(&path).expect("reader for .sav");
    assert_eq!(reader.name(), "SPSS");

    let table = reader.read_file(&path).expect("read .sav");
    assert_eq!(table.format_name.as_deref(), Some("SPSS"));
    assert_eq!(table.col_count(), 3);
    assert_eq!(table.row_count(), 4);

    let cols: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(cols, vec!["id", "name", "score"]);

    // Row 0: (1.0, "Alice", 90.5)
    assert!(matches!(
        table.get(0, 0),
        Some(CellValue::Float(v)) if (*v - 1.0).abs() < 1e-9
    ));
    assert!(matches!(
        table.get(0, 1),
        Some(CellValue::String(s)) if s == "Alice"
    ));

    // Row 2: score is null
    assert!(matches!(table.get(2, 2), Some(CellValue::Null)));

    // Row 3: id is null, name "Dana"
    assert!(matches!(table.get(3, 0), Some(CellValue::Null)));
    assert!(matches!(
        table.get(3, 1),
        Some(CellValue::String(s)) if s == "Dana"
    ));
}

#[test]
fn spss_reader_supports_zsav_extension() {
    let registry = FormatRegistry::new();
    let dummy = std::path::Path::new("foo.zsav");
    let reader = registry.reader_for_path(dummy).expect("reader for .zsav");
    assert_eq!(reader.name(), "SPSS");
}

#[test]
fn spss_reader_does_not_support_write() {
    let registry = FormatRegistry::new();
    let dummy = std::path::Path::new("foo.sav");
    let reader = registry.reader_for_path(dummy).unwrap();
    assert!(!reader.supports_write());
}
