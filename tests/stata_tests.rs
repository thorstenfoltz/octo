//! Tests for the Stata (.dta) reader. Generates the test fixture programmatically
//! using `dta::DtaWriter`.

use tempfile::TempDir;

use dta::stata::dta::byte_order::ByteOrder;
use dta::stata::dta::dta_writer::DtaWriter;
use dta::stata::dta::header::Header;
use dta::stata::dta::release::Release;
use dta::stata::dta::schema::Schema;
use dta::stata::dta::value::Value;
use dta::stata::dta::variable::Variable;
use dta::stata::dta::variable_type::VariableType;
use dta::stata::missing_value::MissingValue;
use dta::stata::stata_double::StataDouble;
use dta::stata::stata_long::StataLong;
use octa::data::CellValue;
use octa::formats::FormatRegistry;

fn make_dta(dir: &TempDir) -> std::path::PathBuf {
    let path = dir.path().join("sample.dta");

    let header = Header::builder(Release::V118, ByteOrder::LittleEndian).build();
    let schema = Schema::builder()
        .add_variable(Variable::builder(VariableType::Long, "id").format("%12.0g"))
        .add_variable(Variable::builder(VariableType::FixedString(16), "name").format("%-16s"))
        .add_variable(Variable::builder(VariableType::Double, "score").format("%9.2f"))
        .build()
        .expect("schema");

    let mut record_writer = DtaWriter::new()
        .from_path(&path)
        .expect("open dta")
        .write_header(header)
        .expect("write header")
        .write_schema(schema)
        .expect("write schema")
        .into_record_writer()
        .expect("record writer");

    record_writer
        .write_record(&[
            Value::Long(StataLong::Present(1)),
            Value::string("Alice"),
            Value::Double(StataDouble::Present(90.5)),
        ])
        .expect("row 1");
    record_writer
        .write_record(&[
            Value::Long(StataLong::Present(2)),
            Value::string("Bob"),
            Value::Double(StataDouble::Present(82.0)),
        ])
        .expect("row 2");
    record_writer
        .write_record(&[
            Value::Long(StataLong::Present(3)),
            Value::string("Charlie"),
            Value::Double(StataDouble::Missing(MissingValue::System)),
        ])
        .expect("row 3");

    record_writer
        .into_long_string_writer()
        .expect("long string writer")
        .into_value_label_writer()
        .expect("value label writer")
        .finish()
        .expect("finish");

    path
}

#[test]
fn stata_reader_reads_basic_file() {
    let dir = TempDir::new().unwrap();
    let path = make_dta(&dir);

    let registry = FormatRegistry::new();
    let reader = registry.reader_for_path(&path).expect("reader for .dta");
    assert_eq!(reader.name(), "Stata");

    let table = reader.read_file(&path).expect("read .dta");
    assert_eq!(table.format_name.as_deref(), Some("Stata"));
    assert_eq!(table.col_count(), 3);
    assert_eq!(table.row_count(), 3);

    let cols: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(cols, vec!["id", "name", "score"]);

    // Types: Long → Int32, FixedString → Utf8, Double → Float64
    assert_eq!(table.columns[0].data_type, "Int32");
    assert_eq!(table.columns[1].data_type, "Utf8");
    assert_eq!(table.columns[2].data_type, "Float64");

    // Row 0: (1, "Alice", 90.5)
    assert!(matches!(table.get(0, 0), Some(CellValue::Int(1))));
    assert!(matches!(
        table.get(0, 1),
        Some(CellValue::String(s)) if s == "Alice"
    ));
    assert!(matches!(
        table.get(0, 2),
        Some(CellValue::Float(v)) if (*v - 90.5).abs() < 1e-9
    ));

    // Row 2: score is missing → CellValue::Null
    assert!(matches!(table.get(2, 2), Some(CellValue::Null)));
}

#[test]
fn stata_reader_does_not_support_write() {
    let registry = FormatRegistry::new();
    let dummy = std::path::Path::new("foo.dta");
    let reader = registry.reader_for_path(dummy).unwrap();
    assert!(!reader.supports_write());
}
