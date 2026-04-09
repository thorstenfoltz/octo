use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// Reader for Jupyter Notebook files (.ipynb).
/// Each cell becomes a row with columns: cell_number, cell_type, source, and outputs.
pub struct JupyterReader;

impl FormatReader for JupyterReader {
    fn name(&self) -> &str {
        "Jupyter Notebook"
    }

    fn extensions(&self) -> &[&str] {
        &["ipynb"]
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let content = std::fs::read_to_string(path)?;
        let notebook: Value = serde_json::from_str(&content)?;
        parse_notebook(&notebook, path)
    }

    fn supports_write(&self) -> bool {
        true
    }

    fn write_file(&self, path: &Path, table: &DataTable) -> Result<()> {
        write_notebook(path, table)
    }
}

fn parse_notebook(notebook: &Value, path: &Path) -> Result<DataTable> {
    let cells = notebook
        .get("cells")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid notebook: missing 'cells' array"))?;

    let columns = vec![
        ColumnInfo {
            name: "Cell".to_string(),
            data_type: "Int64".to_string(),
        },
        ColumnInfo {
            name: "Type".to_string(),
            data_type: "Utf8".to_string(),
        },
        ColumnInfo {
            name: "Source".to_string(),
            data_type: "Utf8".to_string(),
        },
        ColumnInfo {
            name: "Output".to_string(),
            data_type: "Utf8".to_string(),
        },
    ];

    let mut rows = Vec::new();

    for (idx, cell) in cells.iter().enumerate() {
        let cell_type = cell
            .get("cell_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let source = extract_multiline(cell.get("source"));
        let output = extract_outputs(cell.get("outputs"));

        rows.push(vec![
            CellValue::Int((idx + 1) as i64),
            CellValue::String(cell_type.to_string()),
            CellValue::String(source),
            CellValue::String(output),
        ]);
    }

    let mut table = DataTable::empty();
    table.columns = columns;
    table.rows = rows;
    table.source_path = Some(path.to_string_lossy().to_string());
    table.format_name = Some("Jupyter Notebook".to_string());
    Ok(table)
}

/// Extract text from a notebook multiline field (string or array of strings).
fn extract_multiline(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(""),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Extract text output from a cell's outputs array.
fn extract_outputs(value: Option<&Value>) -> String {
    let outputs = match value {
        Some(Value::Array(arr)) => arr,
        _ => return String::new(),
    };

    let mut parts = Vec::new();
    for output in outputs {
        let output_type = output
            .get("output_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match output_type {
            "stream" => {
                let text = extract_multiline(output.get("text"));
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            "execute_result" | "display_data" => {
                // Prefer text/plain from the data dict
                if let Some(data) = output.get("data") {
                    if let Some(text) = data.get("text/plain") {
                        let t = extract_multiline(Some(text));
                        if !t.is_empty() {
                            parts.push(t);
                        }
                    }
                }
            }
            "error" => {
                let ename = output
                    .get("ename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Error");
                let evalue = output
                    .get("evalue")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                parts.push(format!("{}: {}", ename, evalue));
            }
            _ => {}
        }
    }

    parts.join("\n")
}

/// Write a DataTable back to a Jupyter Notebook (.ipynb) file.
fn write_notebook(path: &Path, table: &DataTable) -> Result<()> {
    let mut cells = Vec::new();

    for row in 0..table.row_count() {
        let cell_type = match table.get(row, 1) {
            Some(CellValue::String(s)) => s.clone(),
            _ => "code".to_string(),
        };

        let source = match table.get(row, 2) {
            Some(CellValue::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };

        let source_lines: Vec<Value> = source.lines().map(|l| Value::String(format!("{}\n", l))).collect();
        // Fix last line: don't add trailing newline if source didn't end with one
        let source_array = if source_lines.is_empty() {
            Value::Array(vec![])
        } else {
            let mut lines = source_lines;
            if !source.ends_with('\n') {
                if let Some(last) = lines.last_mut() {
                    if let Value::String(s) = last {
                        // Remove the trailing \n we added
                        s.pop();
                    }
                }
            }
            Value::Array(lines)
        };

        let mut cell_obj = serde_json::Map::new();
        cell_obj.insert("cell_type".to_string(), Value::String(cell_type.clone()));
        cell_obj.insert(
            "metadata".to_string(),
            Value::Object(serde_json::Map::new()),
        );
        cell_obj.insert("source".to_string(), source_array);

        if cell_type == "code" {
            cell_obj.insert("execution_count".to_string(), Value::Null);
            cell_obj.insert("outputs".to_string(), Value::Array(vec![]));
        }

        cells.push(Value::Object(cell_obj));
    }

    let notebook = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "kernelspec": {
                "display_name": "Python 3",
                "language": "python",
                "name": "python3"
            },
            "language_info": {
                "name": "python",
                "version": "3.10.0"
            }
        },
        "cells": cells
    });

    let content = serde_json::to_string_pretty(&notebook)?;
    std::fs::write(path, content)?;
    Ok(())
}
