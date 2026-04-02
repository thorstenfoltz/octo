use crate::data::DataTable;
use crate::formats::FormatReader;
use anyhow::Result;
use std::path::Path;

pub struct YamlReader;

impl FormatReader for YamlReader {
    fn name(&self) -> &str {
        "YAML"
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }

    fn is_text_format(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let content = std::fs::read_to_string(path)?;
        let value: serde_yaml::Value = serde_yaml::from_str(&content)?;
        let json_value = yaml_to_json(&value);
        crate::formats::json_reader::json_to_table(json_value, path, "YAML")
    }
}

fn yaml_to_json(value: &serde_yaml::Value) -> serde_json::Value {
    match value {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::json!(i)
            } else if let Some(f) = n.as_f64() {
                serde_json::json!(f)
            } else {
                serde_json::Value::String(n.to_string())
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        serde_yaml::Value::String(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    (key, yaml_to_json(v))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CellValue;
    use serde_json::json;

    #[test]
    fn test_yaml_null() {
        assert_eq!(yaml_to_json(&serde_yaml::Value::Null), json!(null));
    }

    #[test]
    fn test_yaml_bool() {
        assert_eq!(
            yaml_to_json(&serde_yaml::Value::Bool(true)),
            json!(true)
        );
    }

    #[test]
    fn test_yaml_number_int() {
        let v: serde_yaml::Value = serde_yaml::from_str("42").unwrap();
        assert_eq!(yaml_to_json(&v), json!(42));
    }

    #[test]
    fn test_yaml_number_float() {
        let v: serde_yaml::Value = serde_yaml::from_str("3.14").unwrap();
        assert_eq!(yaml_to_json(&v), json!(3.14));
    }

    #[test]
    fn test_yaml_string() {
        let v = serde_yaml::Value::String("hello".into());
        assert_eq!(yaml_to_json(&v), json!("hello"));
    }

    #[test]
    fn test_yaml_sequence() {
        let v: serde_yaml::Value = serde_yaml::from_str("[1, 2, 3]").unwrap();
        assert_eq!(yaml_to_json(&v), json!([1, 2, 3]));
    }

    #[test]
    fn test_yaml_mapping() {
        let v: serde_yaml::Value = serde_yaml::from_str("name: Alice\nage: 30").unwrap();
        let j = yaml_to_json(&v);
        assert_eq!(j["name"], json!("Alice"));
        assert_eq!(j["age"], json!(30));
    }

    #[test]
    fn test_yaml_read_file() {
        let mut f = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
        std::io::Write::write_all(
            &mut f,
            b"- name: Alice\n  age: 30\n- name: Bob\n  age: 25\n",
        )
        .unwrap();
        let table = YamlReader.read_file(f.path()).unwrap();
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get(0, 0), Some(&CellValue::String("Alice".into())));
        assert_eq!(table.get(1, 1), Some(&CellValue::Int(25)));
    }
}
