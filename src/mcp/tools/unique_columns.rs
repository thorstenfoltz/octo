//! MCP tool: `unique_columns` - find columns (or small combinations)
//! whose values are unique across a tabular file. Useful for
//! primary-key reconnaissance on undocumented sources.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use serde_json::{Map, Value};

use octa::data::unique_columns::{UniqueAnalysis, find_unique_columns};

use crate::mcp::OctaMcpServer;

use super::read_with_registry;

// Tool description lives inline at the `#[tool]` site in `src/mcp/mod.rs`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Path to the file.
    pub path: PathBuf,

    /// For multi-table sources, the specific table to inspect.
    #[serde(default)]
    pub table: Option<String>,

    /// Maximum combo size to test (1 = single columns only, 2 = +
    /// pairs, 3 = + triples). Clamped to `[1, 3]`. Default 1.
    #[serde(default)]
    pub max_combo_size: Option<usize>,

    /// Lift the streaming initial-load cap so every row in the file
    /// is considered. Default `false`.
    #[serde(default)]
    pub unlimited: bool,
}

pub async fn handle(_server: &OctaMcpServer, p: Params) -> Result<CallToolResult, McpError> {
    let path = p.path.clone();
    let table = p.table.clone();
    let combo = p.max_combo_size.unwrap_or(1);
    let unlimited = p.unlimited;

    let analysis = tokio::task::spawn_blocking(move || -> anyhow::Result<UniqueAnalysis> {
        let _g = unlimited.then(|| octa::formats::InitialLoadRowsGuard::new(usize::MAX));
        let dt = read_with_registry(&path, table.as_deref())?;
        Ok(find_unique_columns(&dt, combo))
    })
    .await
    .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
    .map_err(|e| McpError::invalid_params(format!("unique_columns failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(
        analysis_to_json(&analysis).to_string(),
    )]))
}

fn analysis_to_json(a: &UniqueAnalysis) -> Value {
    let single: Vec<Value> = a
        .single
        .iter()
        .map(|r| {
            let mut m = Map::new();
            m.insert("column".to_string(), Value::String(r.column.clone()));
            m.insert("distinct_count".to_string(), Value::from(r.distinct_count));
            m.insert("null_count".to_string(), Value::from(r.null_count));
            m.insert("is_unique".to_string(), Value::Bool(r.is_unique));
            Value::Object(m)
        })
        .collect();
    let combos: Vec<Value> = a
        .combos
        .iter()
        .map(|c| {
            let mut m = Map::new();
            let names: Vec<Value> = c.columns.iter().map(|n| Value::String(n.clone())).collect();
            m.insert("columns".to_string(), Value::Array(names));
            m.insert("distinct_count".to_string(), Value::from(c.distinct_count));
            m.insert("is_unique".to_string(), Value::Bool(c.is_unique));
            Value::Object(m)
        })
        .collect();
    let mut out = Map::new();
    out.insert("total_rows".to_string(), Value::from(a.total_rows));
    out.insert("single".to_string(), Value::Array(single));
    out.insert("combos".to_string(), Value::Array(combos));
    Value::Object(out)
}
