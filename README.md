# Octo

A native desktop application for viewing and editing tabular data files. Built with Rust and [egui](https://github.com/emilk/egui).

## Supported Formats

| Format | Read | Write |
|--------|------|-------|
| Parquet | Yes | Yes |
| CSV / TSV | Yes | Yes |
| JSON / JSON Lines | Yes | No |
| Excel (.xlsx, .xls) | Yes | No |
| Arrow IPC / Feather | Yes | Yes |
| Avro | Yes | No |
| XML | Yes | No |
| TOML | Yes | No |
| YAML | Yes | No |
| PDF | Yes | Yes |
| Markdown | Yes | Yes |
| Plain Text | Yes | Yes |

Unknown file extensions are opened as plain text.

## Features

- Virtual table rendering with smooth scrolling for large datasets
- Lazy row loading for Parquet files (millions of rows)
- Inline cell editing with type-aware parsing
- Column resize, drag-and-drop reorder, and sorting
- Multi-cell, multi-row, and multi-column selection
- Copy/paste with OS clipboard integration (tab-separated)
- Row and column insert, delete, and move operations
- Search/filter across all columns
- Raw text view with line numbers for text-based formats
- Rendered Markdown view with CommonMark support
- PDF page rendering
- Dark and light themes
- CSV delimiter auto-detection and manual selection

## Building

Requires Rust 1.70+ and a C compiler (for native dependencies like mupdf).

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Run directly
cargo run

# Open a file
cargo run -- path/to/file.parquet
```

## Installation

### Linux

```bash
# System-wide (installs to /usr/local)
sudo ./install.sh

# User-local (no sudo needed)
./install.sh ~/.local
```

This installs the binary, SVG icon, and desktop entry so Octo appears in your application launcher with file associations.

### Windows

Run `install.bat` as Administrator. This builds a release binary, copies it to `%ProgramFiles%\Octo`, adds it to your PATH, and creates a Start Menu shortcut.

## Testing

```bash
cargo test
```

## License

MIT
