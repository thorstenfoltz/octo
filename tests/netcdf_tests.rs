mod common;

use common::{ensure_fixtures, fixture_path};
use octa::data::CellValue;
use octa::formats::FormatRegistry;

#[test]
fn netcdf_reader_loads_two_variables_into_columns() {
    ensure_fixtures();
    let path = fixture_path("sample.nc");
    let registry = FormatRegistry::new();
    let reader = registry
        .reader_for_path(&path)
        .expect("reader registered for .nc");
    assert_eq!(reader.name(), "NetCDF");

    let table = reader.read_file(&path).expect("reading sample.nc");
    assert_eq!(table.row_count(), 5);
    assert_eq!(table.col_count(), 2);

    let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"temperature"));
    assert!(names.contains(&"count"));

    let temp_idx = names.iter().position(|n| *n == "temperature").unwrap();
    let count_idx = names.iter().position(|n| *n == "count").unwrap();

    assert_eq!(table.columns[temp_idx].data_type, "Float64");
    assert_eq!(table.columns[count_idx].data_type, "Int32");

    // Verify the first row's values match what we wrote into the fixture.
    match table.get(0, temp_idx) {
        Some(CellValue::Float(f)) => assert!((f - 20.0).abs() < 1e-9),
        other => panic!("expected float, got {other:?}"),
    }
    match table.get(0, count_idx) {
        Some(CellValue::Int(i)) => assert_eq!(*i, 10),
        other => panic!("expected int, got {other:?}"),
    }
}

#[test]
fn netcdf_reader_does_not_advertise_write_support() {
    let registry = FormatRegistry::new();
    let dummy = std::path::PathBuf::from("dummy.nc");
    let reader = registry.reader_for_path(&dummy).unwrap();
    assert!(
        !reader.supports_write(),
        "NetCDF reader should be read-only"
    );
}

#[test]
fn netcdf_reader_format_name_propagates() {
    ensure_fixtures();
    let path = fixture_path("sample.nc");
    let registry = FormatRegistry::new();
    let reader = registry.reader_for_path(&path).unwrap();
    let table = reader.read_file(&path).unwrap();
    assert!(
        table
            .format_name
            .as_deref()
            .unwrap_or("")
            .starts_with("NetCDF")
    );
}
