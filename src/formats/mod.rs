pub mod arrow_ipc_reader;
pub mod avro_reader;
pub mod csv_reader;
pub mod excel_reader;
pub mod json_reader;
pub mod markdown_reader;
pub mod parquet_reader;
pub mod pdf_reader;
pub mod text_reader;
pub mod toml_reader;
pub mod xml_reader;
pub mod yaml_reader;

use crate::data::DataTable;
use anyhow::Result;
use std::path::Path;

/// Trait that every format reader must implement.
/// To add a new format, create a struct that implements this trait
/// and register it in `FormatRegistry::default()`.
pub trait FormatReader: Send + Sync {
    /// Human-readable name of the format (e.g., "Parquet", "CSV").
    fn name(&self) -> &str;

    /// File extensions this reader handles (lowercase, without dot).
    fn extensions(&self) -> &[&str];

    /// Read a file into a DataTable.
    fn read_file(&self, path: &Path) -> Result<DataTable>;

    /// Optionally write a DataTable back to a file.
    /// Returns an error by default (read-only format).
    fn write_file(&self, _path: &Path, _table: &DataTable) -> Result<()> {
        anyhow::bail!("Writing is not supported for this format")
    }

    /// Whether this reader supports writing.
    fn supports_write(&self) -> bool {
        false
    }
}

/// Registry of all available format readers.
/// New formats are added here.
pub struct FormatRegistry {
    readers: Vec<Box<dyn FormatReader>>,
}

impl FormatRegistry {
    /// Create a registry with all built-in readers.
    pub fn new() -> Self {
        let mut registry = Self {
            readers: Vec::new(),
        };
        // Register built-in formats
        registry.register(Box::new(parquet_reader::ParquetReader));
        registry.register(Box::new(csv_reader::CsvReader));
        registry.register(Box::new(csv_reader::TsvReader));
        registry.register(Box::new(json_reader::JsonReader));
        registry.register(Box::new(json_reader::JsonlReader));
        registry.register(Box::new(excel_reader::ExcelReader));
        registry.register(Box::new(avro_reader::AvroReader));
        registry.register(Box::new(arrow_ipc_reader::ArrowIpcReader));
        registry.register(Box::new(xml_reader::XmlFormatReader));
        registry.register(Box::new(pdf_reader::PdfReader));
        registry.register(Box::new(toml_reader::TomlReader));
        registry.register(Box::new(yaml_reader::YamlReader));
        registry.register(Box::new(markdown_reader::MarkdownReader));
        registry.register(Box::new(text_reader::TextReader));
        registry
    }

    /// Register a new format reader.
    pub fn register(&mut self, reader: Box<dyn FormatReader>) {
        self.readers.push(reader);
    }

    /// Find a reader that can handle the given file path based on extension.
    /// Falls back to the Text reader for unknown extensions.
    pub fn reader_for_path(&self, path: &Path) -> Option<&dyn FormatReader> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        if let Some(ref ext) = ext {
            if let Some(reader) = self.readers
                .iter()
                .find(|r| r.extensions().contains(&ext.as_str()))
            {
                return Some(reader.as_ref());
            }
        }
        // Fallback: use Text reader for unknown/missing extensions
        self.readers
            .iter()
            .find(|r| r.name() == "Text")
            .map(|r| r.as_ref())
    }

    /// Get format filter labels and their extensions for file dialogs.
    /// Labels use dotted extensions (e.g. ".csv, .tsv") instead of format names.
    pub fn format_descriptions(&self) -> Vec<(String, Vec<String>)> {
        self.readers
            .iter()
            .map(|r| {
                let exts: Vec<String> = r.extensions().iter().map(|e| e.to_string()).collect();
                let label = exts.iter().map(|e| format!(".{}", e)).collect::<Vec<_>>().join(", ");
                (label, exts)
            })
            .collect()
    }

    /// Get individual extension filters for save dialogs.
    /// Each extension is its own entry (e.g. ".csv", ".json", ".xlsx" separately).
    pub fn save_format_descriptions(&self) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();
        for r in &self.readers {
            if r.supports_write() {
                for ext in r.extensions() {
                    result.push((format!(".{}", ext), vec![ext.to_string()]));
                }
            }
        }
        result
    }

    /// Build a combined filter string with all supported extensions.
    pub fn all_extensions(&self) -> Vec<String> {
        self.readers
            .iter()
            .flat_map(|r| r.extensions().iter().map(|e| e.to_string()))
            .collect()
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_registry_has_readers() {
        let reg = FormatRegistry::new();
        assert!(!reg.readers.is_empty());
    }

    #[test]
    fn test_reader_for_csv() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.csv"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "CSV");
    }

    #[test]
    fn test_reader_for_json() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.json"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "JSON");
    }

    #[test]
    fn test_reader_for_jsonl() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.jsonl"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "JSON Lines");
    }

    #[test]
    fn test_reader_for_parquet() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.parquet"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "Parquet");
    }

    #[test]
    fn test_reader_for_tsv() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.tsv"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "TSV");
    }

    #[test]
    fn test_reader_for_xlsx() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.xlsx"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "Excel");
    }

    #[test]
    fn test_reader_for_toml() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("config.toml"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "TOML");
    }

    #[test]
    fn test_reader_for_yaml() {
        let reg = FormatRegistry::new();
        for ext in &["data.yaml", "data.yml"] {
            let reader = reg.reader_for_path(&PathBuf::from(ext));
            assert!(reader.is_some(), "No reader for {}", ext);
            assert_eq!(reader.unwrap().name(), "YAML");
        }
    }

    #[test]
    fn test_reader_for_xml() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.xml"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "XML");
    }

    #[test]
    fn test_reader_for_avro() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.avro"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "Avro");
    }

    #[test]
    fn test_reader_case_insensitive() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("DATA.CSV"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "CSV");
    }

    #[test]
    fn test_reader_unknown_extension_falls_back_to_text() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("data.xyz"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "Text");
    }

    #[test]
    fn test_reader_no_extension_falls_back_to_text() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("noext"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "Text");
    }

    #[test]
    fn test_all_extensions() {
        let reg = FormatRegistry::new();
        let exts = reg.all_extensions();
        assert!(exts.contains(&"csv".to_string()));
        assert!(exts.contains(&"json".to_string()));
        assert!(exts.contains(&"parquet".to_string()));
        assert!(exts.contains(&"xlsx".to_string()));
    }

    #[test]
    fn test_format_descriptions() {
        let reg = FormatRegistry::new();
        let descs = reg.format_descriptions();
        assert!(!descs.is_empty());
        // Labels should now be dotted extensions
        let labels: Vec<&str> = descs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(labels.iter().any(|l| l.contains(".csv")));
        assert!(labels.iter().any(|l| l.contains(".parquet")));
    }

    #[test]
    fn test_reader_for_markdown() {
        let reg = FormatRegistry::new();
        for ext in &["doc.md", "doc.markdown", "doc.mdown", "doc.mkd"] {
            let reader = reg.reader_for_path(&PathBuf::from(ext));
            assert!(reader.is_some(), "No reader for {}", ext);
            assert_eq!(reader.unwrap().name(), "Markdown");
        }
    }

    #[test]
    fn test_reader_for_text() {
        let reg = FormatRegistry::new();
        for ext in &["file.txt", "file.log", "file.cfg", "file.ini", "file.conf"] {
            let reader = reg.reader_for_path(&PathBuf::from(ext));
            assert!(reader.is_some(), "No reader for {}", ext);
            assert_eq!(reader.unwrap().name(), "Text");
        }
    }

    #[test]
    fn test_reader_for_arrow_ipc() {
        let reg = FormatRegistry::new();
        for ext in &["data.arrow", "data.ipc", "data.feather"] {
            let reader = reg.reader_for_path(&PathBuf::from(ext));
            assert!(reader.is_some(), "No reader for {}", ext);
            assert_eq!(reader.unwrap().name(), "Arrow IPC");
        }
    }

    #[test]
    fn test_reader_for_pdf() {
        let reg = FormatRegistry::new();
        let reader = reg.reader_for_path(&PathBuf::from("report.pdf"));
        assert!(reader.is_some());
        assert_eq!(reader.unwrap().name(), "PDF");
    }

    #[test]
    fn test_all_extensions_includes_new_formats() {
        let reg = FormatRegistry::new();
        let exts = reg.all_extensions();
        assert!(exts.contains(&"txt".to_string()));
        assert!(exts.contains(&"md".to_string()));
        assert!(exts.contains(&"log".to_string()));
    }

    #[test]
    fn test_format_descriptions_includes_all() {
        let reg = FormatRegistry::new();
        let descs = reg.format_descriptions();
        let labels: Vec<&str> = descs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(labels.iter().any(|l| l.contains(".txt")));
        assert!(labels.iter().any(|l| l.contains(".md")));
        assert!(labels.iter().any(|l| l.contains(".parquet")));
        assert!(labels.iter().any(|l| l.contains(".csv")));
        assert!(labels.iter().any(|l| l.contains(".json")));
        assert!(labels.iter().any(|l| l.contains(".xlsx")));
    }

    #[test]
    fn test_fallback_various_unknown_extensions() {
        let reg = FormatRegistry::new();
        for ext in &["data.xyz", "file.zzz", "test.banana", "doc.rs"] {
            let reader = reg.reader_for_path(&PathBuf::from(ext));
            assert!(reader.is_some(), "No fallback reader for {}", ext);
            assert_eq!(reader.unwrap().name(), "Text", "Wrong fallback for {}", ext);
        }
    }
}
