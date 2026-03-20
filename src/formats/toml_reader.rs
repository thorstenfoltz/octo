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
