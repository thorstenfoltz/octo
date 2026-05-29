//! MCP tool: `describe_file` - one-shot orientation snapshot of a
//! tabular file. Format, file size, row count, schema, and a small
//! sample of rows in a single call.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use serde_json::{Map, Value};

use octa::data::CellValue;
use octa::data::describe::{FileDescription, describe_file};

use crate::mcp::OctaMcpServer;

// Tool description lives inline at the `#[tool]` site in `src/mcp/mod.rs`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Path to the file.
    pub path: PathBuf,

    /// For multi-table sources, the specific table to describe.
    #[serde(default)]
    pub table: Option<String>,

    /// Number of sample rows to include (default 5, max 100).
    #[serde(default)]
    pub sample_rows: Option<usize>,

    /// Lift the streaming initial-load cap so the row count reflects
    /// every row in the file. Without this, the count is bounded by
    /// the cap and `initial_load_capped` flags `true`. Default `false`.
    #[serde(default)]
    pub unlimited: bool,
}

pub async fn handle(server: &OctaMcpServer, p: Params) -> Result<CallToolResult, McpError> {
    let path = p.path.clone();
    let table = p.table.clone();
    let sample_rows = p.sample_rows;
    let unlimited = p.unlimited;
    let cell_cap = server.cell_byte_cap;

    let description = tokio::task::spawn_blocking(move || -> anyhow::Result<FileDescription> {
        let _g = unlimited.then(|| octa::formats::InitialLoadRowsGuard::new(usize::MAX));
        describe_file(&path, table.as_deref(), sample_rows)
    })
    .await
    .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
    .map_err(|e| McpError::invalid_params(format!("describe failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(
        description_to_json(&description, cell_cap).to_string(),
    )]))
}

fn description_to_json(d: &FileDescription, cell_cap: usize) -> Value {
    let mut out = Map::new();
    out.insert("path".to_string(), Value::String(d.path.clone()));
    out.insert(
        "format_name".to_string(),
        d.format_name
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
    );
    out.insert(
        "file_size_bytes".to_string(),
        d.file_size_bytes.map(Value::from).unwrap_or(Value::Null),
    );
    out.insert(
        "table".to_string(),
        d.table
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
    );
    out.insert("row_count".to_string(), Value::from(d.row_count));
    out.insert(
        "initial_load_capped".to_string(),
        Value::Bool(d.initial_load_capped),
    );
    out.insert(
        "initial_load_cap".to_string(),
        Value::from(d.initial_load_cap),
    );

    let columns: Vec<Value> = d
        .columns
        .iter()
        .map(|c| {
            let mut m = Map::new();
            m.insert("name".to_string(), Value::String(c.name.clone()));
            m.insert("type".to_string(), Value::String(c.data_type.clone()));
            Value::Object(m)
        })
        .collect();
    out.insert("columns".to_string(), Value::Array(columns));
    out.insert("column_count".to_string(), Value::from(d.columns.len()));

    let mut cell_truncated = false;
    let sample: Vec<Value> = d
        .sample_rows
        .iter()
        .map(|row| {
            let arr: Vec<Value> = row
                .iter()
                .map(|cell| {
                    let (v, t) = cell_to_json(cell, cell_cap);
                    if t {
                        cell_truncated = true;
                    }
                    v
                })
                .collect();
            Value::Array(arr)
        })
        .collect();
    out.insert("sample_rows".to_string(), Value::Array(sample));
    out.insert(
        "sample_row_count".to_string(),
        Value::from(d.sample_rows.len()),
    );
    out.insert("cell_truncated".to_string(), Value::Bool(cell_truncated));
    Value::Object(out)
}

/// Same cell-to-JSON conversion as `tools::table_to_json` (kept local
/// so the describe tool can reuse it without re-running the full
/// `table_to_json` machinery, which expects a `DataTable` we don't
/// hold after passing the sample through).
fn cell_to_json(cell: &CellValue, cell_byte_cap: usize) -> (Value, bool) {
    let v = match cell {
        CellValue::Null => Value::Null,
        CellValue::Bool(b) => Value::Bool(*b),
        CellValue::Int(i) => Value::from(*i),
        CellValue::Float(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
        CellValue::String(s)
        | CellValue::Date(s)
        | CellValue::DateTime(s)
        | CellValue::Nested(s) => Value::String(s.clone()),
        CellValue::Binary(b) => {
            let mut s = String::with_capacity(b.len() * 2);
            for byte in b {
                use std::fmt::Write;
                let _ = write!(&mut s, "{byte:02x}");
            }
            Value::String(s)
        }
    };
    if cell_byte_cap == 0 {
        return (v, false);
    }
    let Value::String(s) = &v else {
        return (v, false);
    };
    if s.len() <= cell_byte_cap {
        return (v, false);
    }
    let marker = format!(
        "[truncated: {} bytes; cap {} bytes. Slice the value with --sql / run_sql to fetch the rest.]",
        s.len(),
        cell_byte_cap
    );
    (Value::String(marker), true)
}
