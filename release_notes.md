# Release notes — `fix/several-bugs`

A small bug-fix batch plus a structural refactor of the five largest source
files. No public API or user-visible behaviour change from the refactor;
the bug fixes are user-facing.

## Fixes

- **Archive viewer: "Open selected entry" always opens a new tab.**
  Selecting an entry inside a `.zip` / `.tar` / `.tgz` and clicking
  **Open selected entry** sometimes replaced the archive listing tab with
  the extracted file's data — the tab title became "Untitled" and the
  button stayed disabled because the tab was no longer recognised as an
  archive. The extraction path now routes through a new
  `OctaApp::load_file_in_new_tab` helper that pushes an empty placeholder
  tab before calling `load_file`, so `apply_loaded_table`'s tab-reuse
  heuristic fills the placeholder rather than reusing the archive tab.
  The archive listing keeps its title, columns, and selection; the
  action bar's button stays clickable.

- **Multi-table picker: smaller default, configurable, no auto-grow.**
  The SQLite / DuckDB / GeoPackage table picker used to open at a fixed
  640×600 box. It now opens at a fit-to-content height capped by a new
  setting, **Settings → Performance → Tables visible in picker** (default
  **10**, TOML key `table_picker_visible_rows`). The dialog stays
  user-resizable — drag the bottom-right corner to grow it for databases
  with many tables. An auto-grow loop that previously expanded the
  window each frame until it filled the screen (the hand-computed footer
  height was a few pixels short of the actual chrome; `Resize`'s
  `desired_size = max(desired_size, last_content_size)` then ratcheted
  the window taller every frame) was fixed by restructuring with
  `egui::Panel::bottom` + `CentralPanel::show_inside` so egui — not us —
  computes the body / footer split.

## Code organization (no behaviour change)

Five files that had grown past 900 lines were split into subdirectory
modules. Every external import path (`use octa::ui::theme::ThemeColors`,
`use octa::data::FormulaOutcome`, …) resolves identically — re-exports
from each new `mod.rs` preserve the public surface.

| Before | After | Sizes (lines) |
|---|---|---|
| `src/ui/theme.rs` (1,462) | `ui/theme/{mod, palettes, visuals}.rs` | 444 / 389 / 656 |
| `src/ui/settings.rs` (2,109) | `ui/settings/{mod, dialog}.rs` | 984 / 1,140 |
| `src/ui/table_view.rs` (2,310) | `ui/table_view/{mod, state, header, rows}.rs` | 1,045 / 91 / 621 / 595 |
| `src/data/mod.rs` (1,996) | `data/mod.rs` + new `data/formulas.rs` | 1,693 / 316 |
| `src/app/dialogs/documentation.rs` (912) | `dialogs/documentation/{mod, content}.rs` | 135 / 785 |

- `theme/palettes.rs` holds the eleven `ThemeColors` palette builders
  (Dark / Light / Nord / Dracula / Gruvbox Dark / High Contrast / Manga /
  Gentleman / Deep Sea / Frost / Rainbow). `theme/visuals.rs` holds the
  matching per-theme `Style` decoration builders and background painters.
- `settings/dialog.rs` holds the `impl SettingsDialog` UI rendering block
  plus the private shortcut-capture helpers. The enums (`IconVariant`,
  `WindowSize`, etc.), `AppSettings` struct + serde defaults + load/save,
  and the TOML round-trip tests stay in `settings/mod.rs`.
- `table_view/state.rs` holds `impl TableViewState`. `header.rs` holds
  the column-header render path (sort arrows, drag-reorder, resize,
  right-click menu). `rows.rs` holds data-row rendering, inline-edit
  TextEdit, and the row context menu.
- `data/formulas.rs` extracts `FormulaBadCell`, `FormulaOutcome`, the
  recursive-descent parser, and the cell-coercion helper; re-exported
  from `data/mod.rs` so existing call sites (`add_column.rs`, the table
  view's formula display, the formula tests) are untouched.
- `dialogs/documentation/content.rs` holds the ~50 `const &str` Markdown
  section bodies that the F1 documentation dialog renders. The dialog
  rendering itself shrinks to ~130 lines in `mod.rs`.

## Documentation

- **`docs/reference/settings.md`** — new Performance row for
  *Tables visible in picker* (default 10, TOML key
  `table_picker_visible_rows`).
- **`docs/usage/tabs-and-sidebar.md`** — under "Multi-table databases", a
  paragraph explaining the fit-to-content sizing, the bottom-right
  resize, and the link back to the new setting.
