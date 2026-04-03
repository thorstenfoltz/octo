use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::Result;
use std::path::Path;

/// Reader for Markdown files (.md, .markdown).
/// Each line becomes a row with a single "Line" column (for table view).
/// The primary view is the rendered Markdown view.
pub struct MarkdownReader;

impl FormatReader for MarkdownReader {
    fn name(&self) -> &str {
        "Markdown"
    }

    fn extensions(&self) -> &[&str] {
        &["md", "markdown", "mdown", "mkd"]
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
        table.format_name = Some("Markdown".to_string());
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "# Heading").unwrap();
            writeln!(f, "").unwrap();
            write!(f, "Some **bold** text").unwrap();
        }
        let reader = MarkdownReader;
        let table = reader.read_file(&path).unwrap();
        assert_eq!(table.row_count(), 3);
        assert_eq!(table.format_name, Some("Markdown".to_string()));
        assert_eq!(
            table.get(0, 0),
            Some(&CellValue::String("# Heading".to_string()))
        );
        assert_eq!(
            table.get(2, 0),
            Some(&CellValue::String("Some **bold** text".to_string()))
        );
    }

    #[test]
    fn test_write_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.md");

        let reader = MarkdownReader;
        let mut table = DataTable::empty();
        table.columns = vec![ColumnInfo {
            name: "Line".to_string(),
            data_type: "Utf8".to_string(),
        }];
        table.rows = vec![
            vec![CellValue::String("# Title".to_string())],
            vec![CellValue::String("paragraph".to_string())],
        ];

        reader.write_file(&path, &table).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "# Title\nparagraph");
    }

    #[test]
    fn test_markdown_reader_metadata() {
        let reader = MarkdownReader;
        assert_eq!(reader.name(), "Markdown");
        assert!(reader.extensions().contains(&"md"));
        assert!(reader.extensions().contains(&"markdown"));
        assert!(reader.extensions().contains(&"mdown"));
        assert!(reader.extensions().contains(&"mkd"));
        assert!(reader.supports_write());
    }

    #[test]
    fn test_markdown_preserves_formatting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fmt.md");
        let content = "# Title\n\n- item 1\n- item 2\n\n```rust\nfn main() {}\n```\n\n> blockquote";
        std::fs::write(&path, content).unwrap();

        let reader = MarkdownReader;
        let table = reader.read_file(&path).unwrap();
        assert_eq!(table.row_count(), 10);
        assert_eq!(table.get(0, 0), Some(&CellValue::String("# Title".to_string())));
        assert_eq!(table.get(2, 0), Some(&CellValue::String("- item 1".to_string())));
        assert_eq!(table.get(5, 0), Some(&CellValue::String("```rust".to_string())));
        assert_eq!(table.get(9, 0), Some(&CellValue::String("> blockquote".to_string())));
    }

    #[test]
    fn test_markdown_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt.md");

        let reader = MarkdownReader;
        let mut table = DataTable::empty();
        table.columns = vec![ColumnInfo {
            name: "Line".to_string(),
            data_type: "Utf8".to_string(),
        }];
        table.rows = vec![
            vec![CellValue::String("## Section".to_string())],
            vec![CellValue::String("".to_string())],
            vec![CellValue::String("Content here.".to_string())],
        ];

        reader.write_file(&path, &table).unwrap();
        let loaded = reader.read_file(&path).unwrap();
        assert_eq!(loaded.row_count(), 3);
        assert_eq!(loaded.get(0, 0), Some(&CellValue::String("## Section".to_string())));
        assert_eq!(loaded.get(1, 0), Some(&CellValue::String("".to_string())));
        assert_eq!(loaded.get(2, 0), Some(&CellValue::String("Content here.".to_string())));
    }

    #[test]
    fn test_markdown_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.md");
        std::fs::write(&path, "").unwrap();

        let reader = MarkdownReader;
        let table = reader.read_file(&path).unwrap();
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.col_count(), 1);
        assert_eq!(table.format_name, Some("Markdown".to_string()));
    }
}
