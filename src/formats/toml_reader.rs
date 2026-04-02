use crate::data::DataTable;
use crate::formats::FormatReader;
use anyhow::Result;
use std::path::Path;

pub struct TomlReader;

impl FormatReader for TomlReader {
    fn name(&self) -> &str {
        "TOML"
    }

    fn extensions(&self) -> &[&str] {
        &["toml"]
    }

    fn is_text_format(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let content = std::fs::read_to_string(path)?;
        let value: toml::Value = content.parse()?;
        let json_value = toml_to_json(&value);
        crate::formats::json_reader::json_to_table(json_value, path, "TOML")
    }
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CellValue;
    use serde_json::json;

    #[test]
    fn test_toml_string() {
        let v = toml::Value::String("hello".into());
        assert_eq!(toml_to_json(&v), json!("hello"));
    }

    #[test]
    fn test_toml_integer() {
        let v = toml::Value::Integer(42);
        assert_eq!(toml_to_json(&v), json!(42));
    }

    #[test]
    fn test_toml_float() {
        let v = toml::Value::Float(3.14);
        assert_eq!(toml_to_json(&v), json!(3.14));
    }

    #[test]
    fn test_toml_boolean() {
        let v = toml::Value::Boolean(true);
        assert_eq!(toml_to_json(&v), json!(true));
    }

    #[test]
    fn test_toml_array() {
        let v: toml::Value = toml::from_str("arr = [1, 2, 3]").unwrap();
        let j = toml_to_json(&v);
        assert_eq!(j["arr"], json!([1, 2, 3]));
    }

    #[test]
    fn test_toml_table() {
        let v: toml::Value = toml::from_str("[server]\nhost = \"localhost\"\nport = 8080").unwrap();
        let j = toml_to_json(&v);
        assert_eq!(j["server"]["host"], json!("localhost"));
        assert_eq!(j["server"]["port"], json!(8080));
    }

    #[test]
    fn test_toml_read_file() {
        let mut f = tempfile::NamedTempFile::with_suffix(".toml").unwrap();
        std::io::Write::write_all(
            &mut f,
            b"[[items]]\nname = \"a\"\nval = 1\n\n[[items]]\nname = \"b\"\nval = 2\n",
        )
        .unwrap();
        let table = TomlReader.read_file(f.path()).unwrap();
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get(0, 0), Some(&CellValue::String("a".into())));
    }
}
