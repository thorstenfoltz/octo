use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

pub struct JsonReader;

impl FormatReader for JsonReader {
    fn name(&self) -> &str {
        "JSON"
    }

    fn extensions(&self) -> &[&str] {
        &["json", "geojson"]
    }

    fn is_text_format(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let content = std::fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&content)?;
        json_to_table(value, path, "JSON")
    }
}

pub struct JsonlReader;

impl FormatReader for JsonlReader {
    fn name(&self) -> &str {
        "JSON Lines"
    }

    fn extensions(&self) -> &[&str] {
        &["jsonl", "ndjson"]
    }

    fn is_text_format(&self) -> bool {
        true
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let content = std::fs::read_to_string(path)?;
        let values: Vec<Value> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        json_to_table(Value::Array(values), path, "JSONL")
    }
}

/// Convert a JSON value to a DataTable. Public so TOML/YAML readers can reuse it.
/// Preserves original key order from the source data.
pub fn json_to_table(value: Value, path: &Path, format_name: &str) -> Result<DataTable> {
    let records = match value {
        Value::Array(arr) => arr,
        Value::Object(ref map) => {
            // Try to find the first array field in the object
            let mut found = None;
            for (_, v) in map {
                if let Value::Array(arr) = v {
                    if !arr.is_empty() {
                        found = Some(arr.clone());
                        break;
                    }
                }
            }
            found.unwrap_or_else(|| vec![value.clone()])
        }
        _ => vec![value],
    };

    if records.is_empty() {
        return Ok(DataTable {
            columns: Vec::new(),
            rows: Vec::new(),
            edits: std::collections::HashMap::new(),
            source_path: Some(path.to_string_lossy().to_string()),
            format_name: Some(format_name.to_string()),
            structural_changes: false,
            total_rows: None,
        });
    }

    // Collect all unique keys by flattening objects — preserve insertion order
    let mut all_keys: Vec<String> = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut flat_records: Vec<Vec<(String, Value)>> = Vec::new();

    for record in &records {
        let mut flat = Vec::new();
        flatten_value("", record, &mut flat);
        for (key, _) in &flat {
            if seen_keys.insert(key.clone()) {
                all_keys.push(key.clone());
            }
        }
        flat_records.push(flat);
    }

    let columns: Vec<ColumnInfo> = all_keys
        .iter()
        .map(|name| ColumnInfo {
            name: name.clone(),
            data_type: "Utf8".to_string(),
        })
        .collect();

    let mut rows: Vec<Vec<CellValue>> = Vec::new();
    for flat in &flat_records {
        let row: Vec<CellValue> = all_keys
            .iter()
            .map(|key| {
                flat.iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| json_value_to_cell(v))
                    .unwrap_or(CellValue::Null)
            })
            .collect();
        rows.push(row);
    }

    // Refine column types
    let mut final_columns = columns;
    for (col_idx, col) in final_columns.iter_mut().enumerate() {
        let mut types = HashSet::new();
        for row in &rows {
            match &row[col_idx] {
                CellValue::Null => {}
                CellValue::Int(_) => {
                    types.insert("int");
                }
                CellValue::Float(_) => {
                    types.insert("float");
                }
                CellValue::Bool(_) => {
                    types.insert("bool");
                }
                CellValue::Nested(_) => {
                    types.insert("nested");
                }
                _ => {
                    types.insert("string");
                }
            }
        }
        col.data_type = if types.contains("string") || types.contains("nested") {
            "Utf8".to_string()
        } else if types.contains("float") {
            "Float64".to_string()
        } else if types.contains("int") && !types.contains("float") {
            "Int64".to_string()
        } else if types.contains("int") {
            "Float64".to_string()
        } else if types.contains("bool") {
            "Boolean".to_string()
        } else {
            "Utf8".to_string()
        };
    }

    Ok(DataTable {
        columns: final_columns,
        rows,
        edits: std::collections::HashMap::new(),
        source_path: Some(path.to_string_lossy().to_string()),
        format_name: Some(format_name.to_string()),
        structural_changes: false,
        total_rows: None,
    })
}

fn flatten_value(prefix: &str, value: &Value, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                match val {
                    Value::Object(_) => flatten_value(&full_key, val, out),
                    _ => {
                        out.push((full_key, val.clone()));
                    }
                }
            }
        }
        _ => {
            let key = if prefix.is_empty() {
                "value".to_string()
            } else {
                prefix.to_string()
            };
            out.push((key, value.clone()));
        }
    }
}

fn json_value_to_cell(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(b) => CellValue::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CellValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                CellValue::Float(f)
            } else {
                CellValue::String(n.to_string())
            }
        }
        Value::String(s) => CellValue::String(s.clone()),
        Value::Array(_) | Value::Object(_) => {
            CellValue::Nested(serde_json::to_string(value).unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn test_path() -> PathBuf {
        PathBuf::from("/tmp/test.json")
    }

    // --- json_value_to_cell ---

    #[test]
    fn test_json_null() {
        assert_eq!(json_value_to_cell(&json!(null)), CellValue::Null);
    }

    #[test]
    fn test_json_bool() {
        assert_eq!(json_value_to_cell(&json!(true)), CellValue::Bool(true));
        assert_eq!(json_value_to_cell(&json!(false)), CellValue::Bool(false));
    }

    #[test]
    fn test_json_int() {
        assert_eq!(json_value_to_cell(&json!(42)), CellValue::Int(42));
        assert_eq!(json_value_to_cell(&json!(-1)), CellValue::Int(-1));
    }

    #[test]
    fn test_json_float() {
        assert_eq!(json_value_to_cell(&json!(3.14)), CellValue::Float(3.14));
    }

    #[test]
    fn test_json_string() {
        assert_eq!(
            json_value_to_cell(&json!("hello")),
            CellValue::String("hello".into())
        );
    }

    #[test]
    fn test_json_array_becomes_nested() {
        let val = json!([1, 2, 3]);
        match json_value_to_cell(&val) {
            CellValue::Nested(s) => assert_eq!(s, "[1,2,3]"),
            other => panic!("Expected Nested, got {:?}", other),
        }
    }

    #[test]
    fn test_json_object_becomes_nested() {
        let val = json!({"a": 1});
        match json_value_to_cell(&val) {
            CellValue::Nested(s) => assert!(s.contains("\"a\":1") || s.contains("\"a\": 1")),
            other => panic!("Expected Nested, got {:?}", other),
        }
    }

    // --- flatten_value ---

    #[test]
    fn test_flatten_simple_object() {
        let val = json!({"name": "Alice", "age": 30});
        let mut out = Vec::new();
        flatten_value("", &val, &mut out);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|(k, _)| k == "name"));
        assert!(out.iter().any(|(k, _)| k == "age"));
    }

    #[test]
    fn test_flatten_nested_object() {
        let val = json!({"user": {"name": "Alice", "address": {"city": "Berlin"}}});
        let mut out = Vec::new();
        flatten_value("", &val, &mut out);
        assert!(out.iter().any(|(k, _)| k == "user.name"));
        assert!(out.iter().any(|(k, _)| k == "user.address.city"));
    }

    #[test]
    fn test_flatten_scalar() {
        let val = json!(42);
        let mut out = Vec::new();
        flatten_value("", &val, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "value");
    }

    // --- json_to_table ---

    #[test]
    fn test_json_array_of_objects() {
        let val = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.col_count(), 2);
        assert_eq!(table.get(0, 0), Some(&CellValue::Int(1)));
        assert_eq!(table.get(1, 1), Some(&CellValue::String("Bob".into())));
    }

    #[test]
    fn test_json_object_with_array_field() {
        let val = json!({
            "data": [
                {"x": 1},
                {"x": 2}
            ]
        });
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get(0, 0), Some(&CellValue::Int(1)));
    }

    #[test]
    fn test_json_empty_array() {
        let val = json!([]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.col_count(), 0);
    }

    #[test]
    fn test_json_scalar_value() {
        let val = json!(42);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.row_count(), 1);
        assert_eq!(table.get(0, 0), Some(&CellValue::Int(42)));
    }

    #[test]
    fn test_json_sparse_objects() {
        let val = json!([
            {"a": 1},
            {"b": 2},
            {"a": 3, "b": 4}
        ]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.col_count(), 2);
        assert_eq!(table.row_count(), 3);
        // First row: a=1, b=null
        assert_eq!(table.get(0, 0), Some(&CellValue::Int(1)));
        assert_eq!(table.get(0, 1), Some(&CellValue::Null));
        // Second row: a=null, b=2
        assert_eq!(table.get(1, 0), Some(&CellValue::Null));
        assert_eq!(table.get(1, 1), Some(&CellValue::Int(2)));
    }

    // --- column type refinement ---

    #[test]
    fn test_json_int_column_type() {
        let val = json!([{"v": 1}, {"v": 2}]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.columns[0].data_type, "Int64");
    }

    #[test]
    fn test_json_float_column_type() {
        let val = json!([{"v": 1.5}, {"v": 2.5}]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.columns[0].data_type, "Float64");
    }

    #[test]
    fn test_json_bool_column_type() {
        let val = json!([{"v": true}, {"v": false}]);
        let table = json_to_table(val, &test_path(), "JSON").unwrap();
        assert_eq!(table.columns[0].data_type, "Boolean");
    }
}
