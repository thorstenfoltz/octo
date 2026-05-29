//! MCP tool: `validate_against_schema` - check a file's column schema
//! against a JSON Schema (e.g. one exported by `export_schema --target
//! json-schema`).
//!
//! The schema can come from disk (`schema_path`) or inline
//! (`schema_inline`). Exactly one of the two must be provided.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use serde_json::{Map, Value};

use octa::data::validate_schema::{ValidationReport, validate_against_json_schema};

use crate::mcp::OctaMcpServer;

use super::compare_schemas::diff_to_json;
use super::read_with_registry;

// Tool description lives inline at the `#[tool]` site in `src/mcp/mod.rs`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Path to the file whose schema is being validated.
    pub path: PathBuf,

    /// For multi-table sources, the specific table to inspect.
    #[serde(default)]
    pub table: Option<String>,

    /// Path to a JSON Schema file (typically one produced by
    /// `export_schema --target json-schema`). Exactly one of
    /// `schema_path` / `schema_inline` must be provided.
    #[serde(default)]
    pub schema_path: Option<PathBuf>,

    /// Inline JSON Schema string. Exactly one of `schema_path` /
    /// `schema_inline` must be provided.
    #[serde(default)]
    pub schema_inline: Option<String>,
}

pub async fn handle(_server: &OctaMcpServer, p: Params) -> Result<CallToolResult, McpError> {
    if p.schema_path.is_some() == p.schema_inline.is_some() {
        return Err(McpError::invalid_params(
            "exactly one of `schema_path` or `schema_inline` must be provided",
            None,
        ));
    }

    let path = p.path.clone();
    let table_name = p.table.clone();
    let schema_path = p.schema_path.clone();
    let schema_inline = p.schema_inline.clone();

    let report = tokio::task::spawn_blocking(move || -> anyhow::Result<ValidationReport> {
        let dt = read_with_registry(&path, table_name.as_deref())?;
        let schema_text = match (schema_path, schema_inline) {
            (Some(sp), None) => std::fs::read_to_string(&sp)
                .map_err(|e| anyhow::anyhow!("read schema_path {}: {e}", sp.display()))?,
            (None, Some(s)) => s,
            _ => unreachable!("xor checked above"),
        };
        validate_against_json_schema(&dt.columns, &schema_text)
    })
    .await
    .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
    .map_err(|e| McpError::invalid_params(format!("validate_schema failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(
        report_to_json(&report).to_string(),
    )]))
}

fn report_to_json(report: &ValidationReport) -> Value {
    let mut out = Map::new();
    out.insert("matches".to_string(), Value::Bool(report.matches));
    out.insert("diff".to_string(), diff_to_json(&report.diff));
    let unparsed: Vec<Value> = report
        .unparsed_types
        .iter()
        .map(|s| Value::String(s.clone()))
        .collect();
    out.insert("unparsed_types".to_string(), Value::Array(unparsed));
    Value::Object(out)
}
