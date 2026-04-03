use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::Result;
use std::path::Path;

pub struct CsvReader;

impl FormatReader for CsvReader {
    fn name(&self) -> &str {
        "CSV"
    }

    fn extensions(&self) -> &[&str] {
        &["csv"]
    }

    fn supports_write(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let delimiter = detect_delimiter(path).unwrap_or(b',');
        read_delimited(path, delimiter, "CSV")
    }

    fn write_file(&self, path: &Path, table: &DataTable) -> Result<()> {
        write_delimited(path, b',', table)
    }
}

pub struct TsvReader;

impl FormatReader for TsvReader {
    fn name(&self) -> &str {
        "TSV"
    }

    fn extensions(&self) -> &[&str] {
        &["tsv", "tab"]
    }

    fn supports_write(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        read_delimited(path, b'\t', "TSV")
    }

    fn write_file(&self, path: &Path, table: &DataTable) -> Result<()> {
        write_delimited(path, b'\t', table)
    }
}

/// Auto-detect the delimiter used in a CSV file by checking consistency of
/// candidate delimiters across the first few lines.
fn detect_delimiter(path: &Path) -> Option<u8> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().take(20).collect();
    if lines.is_empty() {
        return None;
    }

    let candidates: &[u8] = &[b',', b';', b'|', b'\t'];
    let mut best: Option<(u8, usize)> = None; // (delimiter, count_per_line)

    for &delim in candidates {
        let delim_char = delim as char;
        let counts: Vec<usize> = lines.iter().map(|l| l.matches(delim_char).count()).collect();

        // Skip if header has zero occurrences
        if counts[0] == 0 {
            continue;
        }

        // Check consistency: all lines should have roughly the same count
        let header_count = counts[0];
        let consistent = counts.iter().all(|&c| c == header_count || c == 0);

        if consistent {
            if best.is_none() || header_count > best.unwrap().1 {
                best = Some((delim, header_count));
            }
        }
    }

    best.map(|(d, _)| d)
}

const MAX_ROWS: usize = 1_000_000;

pub fn infer_cell_value(s: &str) -> CellValue {
    if s.is_empty() {
        return CellValue::Null;
    }
    match s.to_lowercase().as_str() {
        "true" => return CellValue::Bool(true),
        "false" => return CellValue::Bool(false),
        _ => {}
    }
    if let Ok(i) = s.parse::<i64>() {
        return CellValue::Int(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return CellValue::Float(f);
    }
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return CellValue::Date(s.to_string());
    }
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok()
        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f").is_ok()
        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_ok()
    {
        return CellValue::DateTime(s.to_string());
    }
    // Timezone-aware timestamps (RFC3339, ISO8601 with offset)
    if chrono::DateTime::parse_from_rfc3339(s).is_ok()
        || chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%z").is_ok()
        || chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f%z").is_ok()
        || chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z").is_ok()
        || chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z").is_ok()
    {
        return CellValue::DateTime(s.to_string());
    }
    CellValue::String(s.to_string())
}

fn read_delimited(path: &Path, delimiter: u8, format_name: &str) -> Result<DataTable> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let headers: Vec<String> = rdr.headers()?.iter().map(|h| h.to_string()).collect();
    let columns: Vec<ColumnInfo> = headers
        .iter()
        .map(|h| ColumnInfo {
            name: h.clone(),
            data_type: "Utf8".to_string(),
        })
        .collect();

    let col_count = columns.len();
    let mut rows: Vec<Vec<CellValue>> = Vec::new();
    let mut truncated = false;
    for result in rdr.records() {
        if rows.len() >= MAX_ROWS {
            truncated = true;
            break;
        }
        let record = result?;
        let mut row: Vec<CellValue> = (0..col_count)
            .map(|i| {
                record
                    .get(i)
                    .map(infer_cell_value)
                    .unwrap_or(CellValue::Null)
            })
            .collect();
        row.resize(col_count, CellValue::Null);
        rows.push(row);
    }

    // If truncated, signal that more rows are available without reading the rest
    let total_rows = if truncated {
        Some(usize::MAX) // sentinel: unknown total, more rows available
    } else {
        None
    };

    // Refine column types based on actual data
    let mut refined_columns = columns;
    for (col_idx, col) in refined_columns.iter_mut().enumerate() {
        let mut has_int = false;
        let mut has_float = false;
        let mut has_bool = false;
        let mut has_date = false;
        let mut has_datetime = false;
        let mut has_string = false;

        for row in &rows {
            match row.get(col_idx) {
                Some(CellValue::Int(_)) => has_int = true,
                Some(CellValue::Float(_)) => has_float = true,
                Some(CellValue::Bool(_)) => has_bool = true,
                Some(CellValue::Date(_)) => has_date = true,
                Some(CellValue::DateTime(_)) => has_datetime = true,
                Some(CellValue::String(_)) => has_string = true,
                _ => {}
            }
        }

        col.data_type = if has_string {
            "Utf8".to_string()
        } else if has_datetime {
            "Timestamp(Microsecond, None)".to_string()
        } else if has_date {
            "Date32".to_string()
        } else if has_float || (has_int && has_float) {
            "Float64".to_string()
        } else if has_int {
            "Int64".to_string()
        } else if has_bool {
            "Boolean".to_string()
        } else {
            "Utf8".to_string()
        };
    }

    Ok(DataTable {
        columns: refined_columns,
        rows,
        edits: std::collections::HashMap::new(),
        source_path: Some(path.to_string_lossy().to_string()),
        format_name: Some(format_name.to_string()),
        structural_changes: false,
        total_rows,
        row_offset: 0,
    })
}

/// Load a chunk of CSV/TSV rows in the background.
/// Skips `skip_rows` data records, then reads up to `max_rows` records.
/// Pushes rows into `buffer` in batches. Sets `done` to true when finished.
pub fn load_csv_rows_chunk(
    path: &Path,
    delimiter: u8,
    skip_rows: usize,
    max_rows: usize,
    num_cols: usize,
    buffer: std::sync::Arc<std::sync::Mutex<Vec<Vec<CellValue>>>>,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    exhausted: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let flush_threshold = 50_000;
    let mut batch_buf = Vec::with_capacity(flush_threshold);
    let mut skipped = 0usize;
    let mut loaded = 0usize;

    for result in rdr.records() {
        let record = result?;
        if skipped < skip_rows {
            skipped += 1;
            continue;
        }
        if loaded >= max_rows {
            break;
        }
        let mut row: Vec<CellValue> = (0..num_cols)
            .map(|i| {
                record
                    .get(i)
                    .map(infer_cell_value)
                    .unwrap_or(CellValue::Null)
            })
            .collect();
        row.resize(num_cols, CellValue::Null);
        batch_buf.push(row);
        loaded += 1;

        if batch_buf.len() >= flush_threshold {
            if let Ok(mut buf) = buffer.lock() {
                buf.append(&mut batch_buf);
            }
            batch_buf = Vec::with_capacity(flush_threshold);
        }
    }

    // Flush remaining
    if !batch_buf.is_empty() {
        if let Ok(mut buf) = buffer.lock() {
            buf.append(&mut batch_buf);
        }
    }

    if loaded < max_rows {
        exhausted.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

pub fn write_delimited(path: &Path, delimiter: u8, table: &DataTable) -> Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_path(path)?;

    let headers: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers)?;

    for row_idx in 0..table.row_count() {
        let record: Vec<String> = (0..table.col_count())
            .map(|col_idx| {
                table
                    .get(row_idx, col_idx)
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
            .collect();
        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // --- infer_cell_value ---

    #[test]
    fn test_infer_empty_is_null() {
        assert_eq!(infer_cell_value(""), CellValue::Null);
    }

    #[test]
    fn test_infer_bool() {
        assert_eq!(infer_cell_value("true"), CellValue::Bool(true));
        assert_eq!(infer_cell_value("false"), CellValue::Bool(false));
        assert_eq!(infer_cell_value("TRUE"), CellValue::Bool(true));
        assert_eq!(infer_cell_value("False"), CellValue::Bool(false));
    }

    #[test]
    fn test_infer_int() {
        assert_eq!(infer_cell_value("42"), CellValue::Int(42));
        assert_eq!(infer_cell_value("-7"), CellValue::Int(-7));
        assert_eq!(infer_cell_value("0"), CellValue::Int(0));
    }

    #[test]
    fn test_infer_float() {
        assert_eq!(infer_cell_value("3.14"), CellValue::Float(3.14));
        assert_eq!(infer_cell_value("-0.5"), CellValue::Float(-0.5));
    }

    #[test]
    fn test_infer_date() {
        assert_eq!(
            infer_cell_value("2024-01-15"),
            CellValue::Date("2024-01-15".into())
        );
    }

    #[test]
    fn test_infer_datetime() {
        assert_eq!(
            infer_cell_value("2024-01-15 10:30:00"),
            CellValue::DateTime("2024-01-15 10:30:00".into())
        );
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00"),
            CellValue::DateTime("2024-01-15T10:30:00".into())
        );
    }

    #[test]
    fn test_infer_datetime_with_timezone() {
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00Z"),
            CellValue::DateTime("2024-01-15T10:30:00Z".into())
        );
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00+01:00"),
            CellValue::DateTime("2024-01-15T10:30:00+01:00".into())
        );
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00.123Z"),
            CellValue::DateTime("2024-01-15T10:30:00.123Z".into())
        );
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00.123+05:30"),
            CellValue::DateTime("2024-01-15T10:30:00.123+05:30".into())
        );
    }

    #[test]
    fn test_infer_datetime_with_fractional_seconds() {
        assert_eq!(
            infer_cell_value("2024-01-15 10:30:00.123"),
            CellValue::DateTime("2024-01-15 10:30:00.123".into())
        );
        assert_eq!(
            infer_cell_value("2024-01-15T10:30:00.456789"),
            CellValue::DateTime("2024-01-15T10:30:00.456789".into())
        );
    }

    #[test]
    fn test_infer_string_fallback() {
        assert_eq!(
            infer_cell_value("hello world"),
            CellValue::String("hello world".into())
        );
    }

    // --- detect_delimiter ---

    #[test]
    fn test_detect_comma_delimiter() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "a,b,c").unwrap();
        writeln!(f, "1,2,3").unwrap();
        writeln!(f, "4,5,6").unwrap();
        assert_eq!(detect_delimiter(f.path()), Some(b','));
    }

    #[test]
    fn test_detect_semicolon_delimiter() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "a;b;c").unwrap();
        writeln!(f, "1;2;3").unwrap();
        assert_eq!(detect_delimiter(f.path()), Some(b';'));
    }

    #[test]
    fn test_detect_tab_delimiter() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "a\tb\tc").unwrap();
        writeln!(f, "1\t2\t3").unwrap();
        assert_eq!(detect_delimiter(f.path()), Some(b'\t'));
    }

    #[test]
    fn test_detect_empty_file_returns_none() {
        let f = NamedTempFile::new().unwrap();
        assert_eq!(detect_delimiter(f.path()), None);
    }

    // --- read/write round-trip ---

    #[test]
    fn test_csv_round_trip() {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "name,age,active").unwrap();
        writeln!(f, "Alice,30,true").unwrap();
        writeln!(f, "Bob,25,false").unwrap();

        let table = CsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.col_count(), 3);
        assert_eq!(table.columns[0].name, "name");
        assert_eq!(table.get(0, 0), Some(&CellValue::String("Alice".into())));
        assert_eq!(table.get(0, 1), Some(&CellValue::Int(30)));
        assert_eq!(table.get(1, 2), Some(&CellValue::Bool(false)));

        // Write back and re-read
        let out = NamedTempFile::with_suffix(".csv").unwrap();
        CsvReader.write_file(out.path(), &table).unwrap();
        let table2 = CsvReader.read_file(out.path()).unwrap();
        assert_eq!(table2.row_count(), 2);
        assert_eq!(table2.col_count(), 3);
        assert_eq!(table2.get(0, 0), Some(&CellValue::String("Alice".into())));
    }

    #[test]
    fn test_tsv_round_trip() {
        let mut f = NamedTempFile::with_suffix(".tsv").unwrap();
        writeln!(f, "x\ty").unwrap();
        writeln!(f, "1\t2").unwrap();

        let table = TsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.row_count(), 1);
        assert_eq!(table.get(0, 0), Some(&CellValue::Int(1)));

        let out = NamedTempFile::with_suffix(".tsv").unwrap();
        TsvReader.write_file(out.path(), &table).unwrap();
        let table2 = TsvReader.read_file(out.path()).unwrap();
        assert_eq!(table2.get(0, 0), Some(&CellValue::Int(1)));
    }

    // --- column type refinement ---

    #[test]
    fn test_column_type_refinement_int() {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "val").unwrap();
        writeln!(f, "1").unwrap();
        writeln!(f, "2").unwrap();
        writeln!(f, "3").unwrap();
        let table = CsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.columns[0].data_type, "Int64");
    }

    #[test]
    fn test_column_type_refinement_float() {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "val").unwrap();
        writeln!(f, "1.5").unwrap();
        writeln!(f, "2.5").unwrap();
        let table = CsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.columns[0].data_type, "Float64");
    }

    #[test]
    fn test_column_type_refinement_bool() {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "val").unwrap();
        writeln!(f, "true").unwrap();
        writeln!(f, "false").unwrap();
        let table = CsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.columns[0].data_type, "Boolean");
    }

    #[test]
    fn test_column_type_mixed_becomes_string() {
        let mut f = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(f, "val").unwrap();
        writeln!(f, "42").unwrap();
        writeln!(f, "hello").unwrap();
        let table = CsvReader.read_file(f.path()).unwrap();
        assert_eq!(table.columns[0].data_type, "Utf8");
    }
}
