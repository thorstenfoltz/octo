# `octa --compare-schemas`

Diff the column schemas of two files and print the result as a
four-column table. Only the schemas are read.

## Synopsis

```bash
octa --compare-schemas FILE_A FILE_B [--table-a NAME] [--table-b NAME] [-f FORMAT]
```

| Flag                    | Required | Meaning                                              |
|-------------------------|----------|------------------------------------------------------|
| `--compare-schemas A B` | yes      | The two files to compare. Exactly two paths.         |
| `--table-a NAME`        | no       | Specific table on FILE_A (multi-table sources only). |
| `--table-b NAME`        | no       | Specific table on FILE_B (multi-table sources only). |
| `-f`, `--format FORMAT` | no       | Output format: `tsv` (default), `json`, or `csv`.    |

Column matching is by **exact, case-sensitive name**: `ID` and `id`
are different columns.

## Output

A four-column table with one row per finding:

| Column   | Meaning                                                     |
|----------|-------------------------------------------------------------|
| `status` | One of `common`, `only_in_a`, `only_in_b`, `type_mismatch`. |
| `column` | Column name.                                                |
| `type_a` | Data type on FILE_A (empty for `only_in_b`).                |
| `type_b` | Data type on FILE_B (empty for `only_in_a`).                |

## Examples

### Spotting drift between two file versions

```bash
$ octa --compare-schemas sales_2024.parquet sales_2025.parquet
status         column        type_a   type_b
common         id            Int64    Int64
common         region        Utf8     Utf8
only_in_a      legacy_flag   Boolean  
only_in_b      currency               Utf8
type_mismatch  amount        Float64  Utf8
```

### Comparing tables across two databases

```bash
$ octa --compare-schemas staging.sqlite prod.sqlite \
    --table-a users --table-b users \
    -f json
[
  { "status": "common", "column": "id", "type_a": "Int64", "type_b": "Int64" },
  …
]
```

### Just the JSON, for piping to `jq`

```bash
octa --compare-schemas a.parquet b.parquet -f json \
  | jq '[.[] | select(.status != "common")]'
```

## Exit codes

`--compare-schemas` always exits `0` on a successful read of both
files, regardless of whether the schemas match. (Use
[`--validate-schema`](validate-schema.md) for CI-pipeable
exit-code-based drift detection.)

Non-zero exits map to read failures: file not found, no reader
available, or, for `--table-a` / `--table-b`, a table not present in
the source.

## See also

- [`octa --validate-schema`](validate-schema.md): one-sided
  validation against a fixed expected schema, with CI-friendly exit
  codes.
- [`octa --export-schema`](export-schema.md): produce a JSON Schema
  to lock in.
- [MCP `compare_schemas`](../mcp/tools/compare_schemas.md): same
  feature over MCP.
