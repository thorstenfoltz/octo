//! MCP tool: `compare_schemas` - diff the column schemas of two files.
//!
//! Reads both files through the shared format registry, then delegates
//! the actual comparison to `octa::data::compare_schemas`. No rows are
//! serialised; the response is column metadata only.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use serde_json::{Map, Value};

use octa::data::compare_schemas::{SchemaDiff, compare_schemas};

use crate::mcp::OctaMcpServer;

use super::read_with_registry;

// Tool description lives inline at the `#[tool]` site in `src/mcp/mod.rs`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Path to the first file.
    pub path_a: PathBuf,

    /// Path to the second file.
    pub path_b: PathBuf,

    /// For multi-table sources, the table name to read from file A.
    #[serde(default)]
    pub table_a: Option<String>,

    /// For multi-table sources, the table name to read from file B.
    #[serde(default)]
    pub table_b: Option<String>,
}

pub async fn handle(_server: &OctaMcpServer, p: Params) -> Result<CallToolResult, McpError> {
    let path_a = p.path_a.clone();
    let path_b = p.path_b.clone();
    let table_a = p.table_a.clone();
    let table_b = p.table_b.clone();

    let diff = tokio::task::spawn_blocking(move || -> anyhow::Result<SchemaDiff> {
        let dt_a = read_with_registry(&path_a, table_a.as_deref())?;
        let dt_b = read_with_registry(&path_b, table_b.as_deref())?;
        Ok(compare_schemas(&dt_a.columns, &dt_b.columns))
    })
    .await
    .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
    .map_err(|e| McpError::invalid_params(format!("compare_schemas failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(
        diff_to_json(&diff).to_string(),
    )]))
}

/// Render a `SchemaDiff` as the JSON shape documented in the plan:
///   `{ identical, common, only_in_a, only_in_b, type_mismatches }`.
/// Columns are emitted as `{name, type}`; type mismatches as
/// `{name, a, b}`.
pub fn diff_to_json(diff: &SchemaDiff) -> Value {
    fn cols_to_json(cols: &[octa::data::ColumnInfo]) -> Value {
        let arr: Vec<Value> = cols
            .iter()
            .map(|c| {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(c.name.clone()));
                m.insert("type".to_string(), Value::String(c.data_type.clone()));
                Value::Object(m)
            })
            .collect();
        Value::Array(arr)
    }

    let mismatches: Vec<Value> = diff
        .type_mismatches
        .iter()
        .map(|m| {
            let mut obj = Map::new();
            obj.insert("name".to_string(), Value::String(m.name.clone()));
            obj.insert("a".to_string(), Value::String(m.type_a.clone()));
            obj.insert("b".to_string(), Value::String(m.type_b.clone()));
            Value::Object(obj)
        })
        .collect();

    let mut out = Map::new();
    out.insert("identical".to_string(), Value::Bool(diff.identical));
    out.insert("common".to_string(), cols_to_json(&diff.common));
    out.insert("only_in_a".to_string(), cols_to_json(&diff.only_in_a));
    out.insert("only_in_b".to_string(), cols_to_json(&diff.only_in_b));
    out.insert("type_mismatches".to_string(), Value::Array(mismatches));
    Value::Object(out)
}
