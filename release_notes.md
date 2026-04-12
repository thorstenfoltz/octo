## Features

- Add multi-tab support: open multiple files in separate tabs, close with Ctrl+W, switch with Ctrl+Tab / Ctrl+Shift+Tab
- Add column type conversion: right-click a column header to convert between String, Int64, Float64, Boolean, Date32, and Timestamp types with full undo/redo support
- Add Ctrl+Arrow keyboard shortcuts to jump to first/last row (Ctrl+Up/Down) and first/last column (Ctrl+Left/Right)
- Add selectable text extraction for PDF pages (copy text from rendered PDFs)
- Add page headers with page numbers in PDF view
- Add copy support for notebook cells and whole-notebook text
- Add line number gutter in notebook code cells
- Add JSON Tree view: collapsible tree viewer for JSON/JSONL files (Firefox-style), available via View menu
- Add Exit button to File menu
- Improve notebook view layout and styling (horizontal scroll, cell backgrounds)

## Bug Fixes

- Fix Unicode search and replace: correctly handle umlauts (ä, ö, ü, ß) and other non-ASCII characters without byte-offset misalignment

## Internal

- Extract view mode rendering (PDF, Notebook, Markdown, Raw) from main.rs into view_modes.rs (~556 lines)
- Extract RowMatcher search logic from main.rs into data/search.rs for testability
- Add Unicode/Umlaut test coverage for search and replace operations
