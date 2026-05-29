//! MCP tool: `run_sql` - run a DuckDB SQL query against one or more files
//! using the multi-table SQL workspace.
//!
//! Single-file invocations stay identical to the original behaviour: the
//! `path` file is registered as `data` and the query runs against a fresh
//! workspace. The new optional fields let callers JOIN across multiple
//! sources (`extra_tables`), browse and query whole DBs without copying
//! rows (`attach`), and write the SELECT result back to a DuckDB or
//! SQLite file (`write_to`).

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use serde_json::{Map, Value};

use octa::sql::{AttachKind, QueryKind, SqlWorkspace, WriteMode, WriteTarget, sanitize_sql_name};

use crate::mcp::OctaMcpServer;

use super::{read_with_registry, table_to_json};

// Tool description lives inline at the `#[tool]` site in `src/mcp/mod.rs`.

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Params {
    /// Path to the primary file (registered as `data`).
    pub path: PathBuf,

    /// SQL query string. The primary file is exposed as `data`.
    pub query: String,

    /// Maximum rows to return. Default is the server's configured limit
    /// (1000 unless changed via Octa's Settings -> MCP). Pass 0 for unlimited.
    /// Slices the *response* - set `unlimited` to also lift the file-loader
    /// cap so the query sees every row.
    #[serde(default)]
    pub limit: Option<usize>,

    /// For multi-table sources, load this specific table as `data`.
    #[serde(default)]
    pub table: Option<String>,

    /// Lift the streaming initial-load cap so the query operates on every
    /// row in every loaded file. Default `false`.
    #[serde(default)]
    pub unlimited: bool,

    /// Additional tables to register into the workspace before the query
    /// runs. Each entry loads a file and exposes it under the chosen SQL
    /// name so the query can JOIN it against `data`. The SQL name is
    /// sanitised (lowercase, non-alphanumerics replaced with `_`).
    #[serde(default)]
    pub extra_tables: Vec<ExtraTable>,

    /// Databases to ATTACH for the duration of the call. After attachment
    /// every inner table is queryable as `alias.schema.tbl` (DuckDB) or
    /// `alias.tbl` (SQLite via the DuckDB sqlite extension when present;
    /// otherwise the workspace falls back to per-table loading under names
    /// like `alias__table`).
    #[serde(default)]
    pub attach: Vec<AttachSpec>,

    /// When set, write the SELECT result to a DuckDB or SQLite file
    /// instead of returning rows. The response shape becomes
    /// `{ "kind": "write_back", "rows_written": N, "target": "..." }`.
    #[serde(default)]
    pub write_to: Option<WriteSpec>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtraTable {
    /// SQL identifier to register the file under (e.g. `customers`).
    pub name: String,
    /// Path to the file to read via the format registry.
    pub path: PathBuf,
    /// Inner-table picker for multi-table sources (SQLite, DuckDB, Excel,
    /// ODS, GeoPackage). Defaults to the reader's `read_file` behaviour.
    #[serde(default)]
    pub table: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AttachSpec {
    /// SQL alias to attach the database under (e.g. `analytics`).
    pub alias: String,
    /// Path to the database file. The extension picks DuckDB vs. SQLite
    /// (`.duckdb` / `.ddb` -> DuckDB; everything else -> SQLite).
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteSpec {
    /// Path to the target DuckDB or SQLite file. Created if missing.
    pub path: PathBuf,
    /// Target schema (DuckDB only). `null` writes to `main`. SQLite has
    /// no schemas; passing anything other than `null` or `"main"` errors.
    #[serde(default)]
    pub schema: Option<String>,
    /// Target table name.
    pub table: String,
    /// Write mode: `create` (default; errors if the table already
    /// exists), `replace` (drop + recreate), or `append` (INSERT into
    /// existing table).
    #[serde(default = "default_write_mode")]
    pub mode: String,
    /// Create the target schema if it doesn't already exist (DuckDB only).
    #[serde(default)]
    pub create_schema_if_missing: bool,
}

fn default_write_mode() -> String {
    "create".to_string()
}

pub async fn handle(server: &OctaMcpServer, p: Params) -> Result<CallToolResult, McpError> {
    let row_cap = server.resolve_row_cap(p.limit);
    let cell_cap = server.cell_byte_cap;
    let path = p.path.clone();
    let table_name = p.table.clone();
    let query = p.query.clone();
    let unlimited = p.unlimited;
    let extras = p.extra_tables;
    let attachments = p.attach;
    let write_to = p.write_to;

    let outcome = tokio::task::spawn_blocking(move || -> anyhow::Result<Value> {
        let _g = unlimited.then(|| octa::formats::InitialLoadRowsGuard::new(usize::MAX));

        let active = read_with_registry(&path, table_name.as_deref())?;
        let mut ws = SqlWorkspace::new()?;
        ws.set_active_table(&active)?;

        for entry in &extras {
            let sql_name = sanitize_sql_name(&entry.name);
            ws.add_table_from_file(&entry.path, entry.table.as_deref(), &sql_name)?;
        }
        for entry in &attachments {
            let kind = AttachKind::from_path(&entry.path);
            ws.attach(&entry.path, &entry.alias, kind)?;
        }

        if let Some(spec) = write_to {
            let mode = WriteMode::parse(&spec.mode)?;
            let report = ws.write_result_to_db(&WriteTarget {
                path: spec.path.clone(),
                kind: AttachKind::from_path(&spec.path),
                schema: spec.schema.clone(),
                table: spec.table.clone(),
                mode,
                source_query: query,
                create_schema_if_missing: spec.create_schema_if_missing,
            })?;
            let mut out = Map::new();
            out.insert("kind".to_string(), Value::String("write_back".to_string()));
            out.insert("rows_written".to_string(), Value::from(report.rows_written));
            out.insert(
                "created_schema".to_string(),
                Value::Bool(report.created_schema),
            );
            out.insert(
                "target".to_string(),
                Value::String(report.target_display.clone()),
            );
            return Ok(Value::Object(out));
        }

        let qo = ws.execute(&query)?;
        let kind_str = match qo.kind {
            QueryKind::Select => "select",
            QueryKind::Mutation => "mutation",
        };
        let table_value = table_to_json(&qo.table, row_cap, cell_cap);
        let mut out = Map::new();
        out.insert("kind".to_string(), Value::String(kind_str.to_string()));
        if let Some(n) = qo.affected {
            out.insert("affected".to_string(), Value::from(n));
        }
        out.insert("result".to_string(), table_value);
        Ok(Value::Object(out))
    })
    .await
    .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
    .map_err(|e| McpError::invalid_params(format!("run_sql failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(
        outcome.to_string(),
    )]))
}
