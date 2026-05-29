# Release notes: `feature/mcp-extensions`

A feature batch that expands SQL across the whole app, the CLI, and
the MCP server, adds four new schema and profiling actions on both the CLI
and MCP surfaces, introduces per-column number formatting with save-time
rounding, and improves several file-loading and analysis workflows.

## SQL workspace (GUI, CLI, and MCP)

SQL is no longer a single-table, print-only feature. Each surface now
shares the same workspace engine (`src/sql/workspace.rs`, `engine.rs`).

- **Multi-table queries and JOINs.** A query can reference more than the
  active file. Extra files are registered as named workspace tables and a
  single query can JOIN across formats (CSV, Parquet, JSON, Excel, SQLite,
  and so on).
- **Database attachments.** Whole DuckDB or SQLite databases can be
  `ATTACH`-ed so every inner table is reachable. DuckDB tables appear as
  `alias.schema.tbl`; SQLite tables as `alias.tbl` (via the bundled sqlite
  extension when present, otherwise a per-table fallback).
- **Write-back.** A SELECT result can be persisted to a DuckDB or SQLite
  file instead of being printed, with create / replace / append modes and
  an optional target schema for DuckDB.
- **GUI panel.** The SQL panel gained a Workspace section that lists
  registered tables and attached databases with per-row add / remove /
  detach controls, a refresh button for the active table, and a
  "Write result to DB..." dialog (`src/app/dialogs/sql_write_back.rs`).
  Each tab owns its own session-scoped workspace.
- **CLI flags.** `octa --sql` grew `--sql-table NAME=PATH` (repeatable),
  `--sql-attach ALIAS=PATH` (repeatable), and `--sql-write-to PATH` with
  `--sql-write-table`, `--sql-write-schema`, and `--sql-write-mode`.
- **MCP `run_sql`.** Mirrors the CLI: `extra_tables`, `attach`, and
  `write_to` parameters. A write-back returns a `write_back` response
  shape instead of rows. Each call builds and tears down a fresh
  workspace so state never leaks between calls.

## New schema and profiling tooling

Four new actions are exposed identically on the CLI and over MCP, all
built on pure library functions in `src/data/`.

- **`octa --compare-schemas A B`** / MCP `compare_schemas`. Diffs two
  files' column schemas (`common`, `only_in_a`, `only_in_b`,
  `type_mismatch`), with `--table-a` / `--table-b` for multi-table
  sources.
- **`octa --validate-schema FILE --expect-schema SCHEMA_FILE`** / MCP
  `validate_against_schema`. Checks a file's columns against an expected
  JSON Schema. Exit code is `0` on a clean match and `1` on any drift, so
  it slots straight into a CI pipeline.
- **`octa --describe FILE`** / MCP `describe_file`. A one-shot orientation
  snapshot (format, file size, row count, schema, sample rows) that
  replaces the usual `--schema` then `--head` two-step. `--sample-rows N`
  controls the sample size.
- **`octa --unique-columns FILE`** / MCP `unique_columns`. Finds columns,
  and optional small combinations, whose values are unique across a file,
  useful for spotting primary-key candidates. `--max-combo N` controls the
  combination size.

## Number formatting and save-time rounding

- **Per-column number format.** A new dialog
  (`src/app/dialogs/column_format.rs`, `src/data/num_format.rs`) sets
  decimal places and rounding per numeric column, opened from a column
  header right-click or Edit -> Number format. Edits apply live.
- **Thousands separators.** Numeric cells can be grouped, with an English
  or European separator style (the European decimal comma applies even
  with grouping off). This is display-only and never touches saved,
  exported, CLI, or MCP output.
- **Rounding is display-only.** The in-memory table keeps full precision.
  If any column rounds values, Save prompts with Save rounded values /
  Save full precision / Cancel (`src/app/dialogs/round_save_prompt.rs`),
  so the choice only affects the bytes on disk.

## File-loading and analysis improvements

- **Whitespace trim on load.** String cells and column titles are trimmed
  of leading and trailing whitespace on load (`src/data/trim.rs`), with a
  dismissible warning banner and two settings to control the behaviour.
  For database-backed tables the trim is reconciled so it is not seen as
  an edit or schema change on save.
- **Excel multi-sheet selection.** Workbooks open every sheet in its own
  tab up to a configurable cap; above it, a multi-select sheet picker
  appears (`src/app/dialogs/sheet_picker.rs`).
- **Value Frequency column picker.** Opening the dialog without a column
  context now raises a column picker
  (`src/app/dialogs/value_frequency_picker.rs`), and numeric columns
  support custom bin counts.

## Settings

New settings back the above features: the multi-table picker default
height (`table_picker_visible_rows`), the Excel auto-open cap
(`excel_max_auto_sheets`), whitespace-trim toggles
(`trim_whitespace_on_load`, `warn_on_whitespace_trim`), the thousands
separator toggle and style, and the MCP response caps.

