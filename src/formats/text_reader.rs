use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::Result;
use std::path::Path;

/// Reader for plain text files (.txt, .log, .cfg, .ini, .conf, .sh, .bat, .ps1, etc.)
/// Each line becomes a row with a single "Line" column.
pub struct TextReader;

impl FormatReader for TextReader {
    fn name(&self) -> &str {
        "Text"
    }

    fn extensions(&self) -> &[&str] {
        &["txt", "log", "cfg", "ini", "conf", "sh", "bat", "ps1", "env", "gitignore", "dockerignore", "editorconfig", "properties"]
    }

    fn supports_write(&self) -> bool {
        true
    }

    fn write_file(&self, path: &Path, table: &DataTable) -> Result<()> {
        let mut lines = Vec::with_capacity(table.row_count());
        for row in 0..table.row_count() {
            match table.get(row, 0) {
                Some(CellValue::String(s)) => lines.push(s.clone()),
                Some(v) => lines.push(v.to_string()),
                None => lines.push(String::new()),
            }
        }
        std::fs::write(path, lines.join("\n"))?;
        Ok(())
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        read_text_file(path)
    }
}

fn read_text_file(path: &Path) -> Result<DataTable> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    let columns = vec![ColumnInfo {
        name: "Line".to_string(),
        data_type: "Utf8".to_string(),
    }];

    let rows: Vec<Vec<CellValue>> = lines
        .iter()
        .map(|line| vec![CellValue::String(line.to_string())])
        .collect();

    let mut table = DataTable::empty();
    table.columns = columns;
    table.rows = rows;
    table.source_path = Some(path.to_string_lossy().to_string());
    table.format_name = Some("Text".to_string());
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "Hello World").unwrap();
            writeln!(f, "Second line").unwrap();
            write!(f, "Third line").unwrap();
        }
        let table = read_text_file(&path).unwrap();
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.col_count(), 1);
        assert_eq!(table.columns[0].name, "Line");
        assert_eq!(table.get(0, 0), Some(&CellValue::String("Hello World".to_string())));
        assert_eq!(table.get(1, 0), Some(&CellValue::String("Second line".to_string())));
        assert_eq!(table.get(2, 0), Some(&CellValue::String("Third line".to_string())));
    }

    #[test]
    fn test_write_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");

        let reader = TextReader;
        let mut table = DataTable::empty();
        table.columns = vec![ColumnInfo {
            name: "Line".to_string(),
            data_type: "Utf8".to_string(),
        }];
        table.rows = vec![
            vec![CellValue::String("first".to_string())],
            vec![CellValue::String("second".to_string())],
        ];

        reader.write_file(&path, &table).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first\nsecond");
    }

    #[test]
    fn test_empty_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, "").unwrap();

        let table = read_text_file(&path).unwrap();
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.col_count(), 1);
    }

    #[test]
    fn test_text_reader_metadata() {
        let reader = TextReader;
        assert_eq!(reader.name(), "Text");
        assert!(reader.extensions().contains(&"txt"));
        assert!(reader.extensions().contains(&"log"));
        assert!(reader.supports_write());
    }

    #[test]
    fn test_text_format_name_set() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello").unwrap();

        let table = read_text_file(&path).unwrap();
        assert_eq!(table.format_name, Some("Text".to_string()));
        assert!(table.source_path.is_some());
    }

    #[test]
    fn test_text_unicode_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unicode.txt");
        std::fs::write(&path, "Hallo Welt\nUnicode: \u{00e4}\u{00f6}\u{00fc}\u{00df}\n\u{1f600}").unwrap();

        let table = read_text_file(&path).unwrap();
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.get(2, 0), Some(&CellValue::String("\u{1f600}".to_string())));
    }

    #[test]
    fn test_text_single_line_no_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("single.txt");
        std::fs::write(&path, "just one line").unwrap();

        let table = read_text_file(&path).unwrap();
        assert_eq!(table.row_count(), 1);
        assert_eq!(table.get(0, 0), Some(&CellValue::String("just one line".to_string())));
    }

    #[test]
    fn test_write_then_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roundtrip.txt");

        let reader = TextReader;
        let mut table = DataTable::empty();
        table.columns = vec![ColumnInfo {
            name: "Line".to_string(),
            data_type: "Utf8".to_string(),
        }];
        table.rows = vec![
            vec![CellValue::String("alpha".to_string())],
            vec![CellValue::String("beta".to_string())],
            vec![CellValue::String("gamma".to_string())],
        ];

        reader.write_file(&path, &table).unwrap();
        let loaded = reader.read_file(&path).unwrap();
        assert_eq!(loaded.row_count(), 3);
        assert_eq!(loaded.get(0, 0), Some(&CellValue::String("alpha".to_string())));
        assert_eq!(loaded.get(2, 0), Some(&CellValue::String("gamma".to_string())));
    }
}
