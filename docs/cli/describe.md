# `octa --describe`

One-shot orientation snapshot of a tabular file: format, file size,
row count, schema, and a small sample of rows. Saves the usual
`--schema` then `--head` two-step.

## Synopsis

```bash
octa --describe FILE [--table NAME] [--sample-rows N] [-f FORMAT]
```

| Flag                    | Required | Meaning                                                       |
|-------------------------|----------|---------------------------------------------------------------|
| `--describe FILE`       | yes      | The file to describe.                                         |
| `--table NAME`          | no       | Specific table for multi-table sources (SQLite, DuckDB, ...). |
| `--sample-rows N`       | no       | Sample-row count (default 5, clamped to 100).                 |
| `-f`, `--format FORMAT` | no       | Output format: `tsv` (default), `json`, or `csv`.             |

## Output

`-f tsv` and `-f csv` print a vertical `field / value` table that
shells / `awk` can grep easily:

```bash
$ octa --describe sales.parquet
field                value
path                 /home/me/data/sales.parquet
format_name          Parquet
file_size_bytes      1048576
table                
row_count            47832
initial_load_capped  false
column_count         5
column[id]           Int64
column[region]       Utf8
column[amount]       Float64
column[quarter]      Utf8
column[currency]     Utf8
sample_row[0]        1, EU, 1234.5, Q1, EUR
sample_row[1]        2, US, 9876.0, Q1, USD
…
```

`-f json` returns the same data as a structured object that mirrors
the [MCP `describe_file` shape](../mcp/tools/describe_file.md):

```bash
$ octa --describe sales.parquet -f json
{
  "path": "/home/me/data/sales.parquet",
  "format_name": "Parquet",
  "file_size_bytes": 1048576,
  "row_count": 47832,
  "columns": [
    { "name": "id", "type": "Int64" },
    …
  ],
  "sample_rows": [
    ["1", "EU", "1234.5", "Q1", "EUR"],
    …
  ]
}
```

## Examples

### First look at an unfamiliar file

```bash
octa --describe data.parquet
```

### Bigger preview

```bash
octa --describe data.csv --sample-rows 20
```

### Pick a specific table inside a multi-table source

```bash
octa --describe users.sqlite --table customers
```

### Machine-readable output for downstream tooling

```bash
octa --describe data.parquet -f json | jq '.row_count, .column_count'
```

## See also

- [`octa --schema`](schema.md): schema-only, no sample rows.
- [`octa --head`](head.md): sample rows only, no schema summary.
- [MCP `describe_file`](../mcp/tools/describe_file.md): same feature
  over MCP.
