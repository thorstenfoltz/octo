## Features

- **Read-only mode.** Toggle a session-only lock with **F8** (remappable in **Settings → Shortcuts**) or via **View → Read-only mode**. Every editing path short-circuits while the lock is on — cell edits, structural row/column changes, marks, undo/redo, the raw text editor, and the markdown editor all decline to mutate. Saving and copying still work. The status bar shows a `[Read-only]` pill while the lock is active; on toggle, a confirmation modal explains the change with a "Don't show again" checkbox (re-enable under **Settings → File-Specific → Read-only mode notice**).
- **NetCDF v3 (`.nc`) reader.** New pure-Rust reader powered by the `netcdf3` crate. All 1D variables sharing the largest dimension are grouped into one table — each variable becomes a column whose row index is the dimension axis. Multi-dimensional and 0D scalar variables are skipped, with a count surfaced in the format-name pill (e.g. `NetCDF (3 multi-D vars skipped)`). Type mapping: `i8`/`u8`/`i16`/`i32` → `Int32`, `f32` → `Float32`, `f64` → `Float64`. Read-only — NetCDF v4 (HDF5-based) is not supported here; use the HDF5 reader for those files.
- **YAML tree view.** `.yaml`/`.yml` files now offer a Firefox-style collapsible tree alongside the table view. Available from **View → YAML Tree** (or via F4 cycle). Shares the JSON tree's renderer and key-edit affordances; YAML is parsed once at load through `serde_yaml` then converted to a JSON `Value` for tree walking.
- **Editable keys + new-key affordance in the JSON / YAML tree.** Double-click a key to rename it inline (insertion order preserved); each expanded `{` row carries a small `+` button to add a new key at the end of the object. Edits flow back to `tab.raw_content` via the format-appropriate serializer and set the unsaved-changes flag. Drag-selection contrast in tree labels has been fixed — keys are no longer invisible against the accent-tinted selection rectangle.
- **Markdown view: Preview / Split / Edit toggle.** A new segmented control at the top of the Markdown view picks between rendered preview only, side-by-side editor + preview, or full-width editor only. Split mode is the default — typing in the editor pane updates the preview every frame. Long lines no longer wrap in either pane; both surfaces support horizontal scrolling.
- **Markdown bold actually renders bold.** The markdown view now uses `pulldown_cmark` with a custom egui renderer that swaps in a bundled `Roboto-Bold` font face for `**bold**` runs and headings. Previously `**text**` only changed color (egui's `RichText::strong()` is a color tweak, not a font-weight switch); the difference is now obvious. Italics, inline code, code blocks, lists, blockquotes, and horizontal rules also render through the new path.
- **Parse-error fallback to raw text.** When a text-format reader (CSV, TSV, JSON, JSONL, XML, YAML, TOML, Markdown, Jupyter, Text) fails to parse a file, the tab automatically opens the raw bytes in the Raw view with a dismissible orange banner naming the format and the parser's error message. Files larger than 500 MB skip the fallback and surface the error in the status bar instead. Binary formats (Parquet, Excel, PDF, SQLite, …) keep their existing status-bar behavior since raw bytes would render as garbage.
- **Three new icon variants: White, Black, Pink.** Pick them under **Settings → Visual → Icon variant**. The `Random` meta-variant now rolls between 15 concrete colors instead of 12.
- **`SELECT * FROM h2o` aquarium easter egg.** Joins the existing `octopuses` and `stars` family in the SQL view — returns a hand-crafted ocean-zone table with `Float64` numeric columns so `AVG(temperature_c) FROM h2o` works as expected.

## Fixes

- **Raw CSV per-column colors after alignment.** The second-and-later quoted fields in an aligned row used to render with one color per character because `format_delimited_text` joined cells with `delimiter + space`, pushing the opening quote past the tokenizer's strict `i == field_start` check. The tokenizer now skips ASCII whitespace at field start (with a `c != delimiter` guard so TSV tabs still split correctly) so a quoted cell like `"1,2,3,4,5"` keeps a single column color across the embedded commas regardless of position.
- **Tab close × no longer shifts on hover.** The leading whitespace that pushed the gray × to the right edge of the close button rect has been moved out of the label and into `ui.add_space`. Hover paints the red × at the same coordinates as the original glyph instead of jumping leftward.
- **Parse-error banner uses the standard close glyph.** Banner dismiss button switched from `\u{2715}` (✕) to `\u{00D7}` (×) so it matches the tab-close X.
- **Read-only "Don't show again" checkbox no longer flickers.** The checkbox state now lives on the `ReadOnlyNotice` struct itself instead of being re-derived from settings every frame, so a single click sticks.
- **macOS install instructions: no recursive `xattr`.** `pkg/README-macos.md` and `README.md` both now show a `find` step before the strip, drop the `-r` flag, and keep an `xattr -cr` fallback comment for the rare "Octa.app is damaged" case.

## Removals

- **None.** All format readers and view modes from previous releases are still present.

## Documentation

- `CLAUDE.md` updated for the NetCDF reader, YAML tree view, parse-error fallback, read-only mode (with chokepoint note), Markdown layout + custom pulldown_cmark renderer, JSON / YAML tree key editing, raw CSV color tokenizer fix, and the three new icon variants.
- `README.md` formats table now lists NetCDF v3; view-modes section describes the YAML tree, Markdown three-state toggle, parse-error banner, and read-only mode F8 binding.

## Internals

- **`pulldown-cmark = "0.12"`** added as a direct dependency (was already transitive via `egui_commonmark`); the markdown preview no longer uses `egui_commonmark::CommonMarkViewer` for rendering.
- **`netcdf3 = "0.6"`** added for the new NetCDF reader. Pure Rust, no system library dependency.
- **`assets/Roboto-Bold.ttf`** (~452 KB) bundled and embedded via `include_bytes!`; registered as `FontFamily::Name("bold")` in `apply_fonts` so the markdown renderer can address true bold glyphs.
- **`assets/octa-{white,black,pink}.svg`** added; `IconVariant` enum extended along with `ALL`, `CONCRETE`, `label`, `svg_source`, and `preview_color`.
- **`tests/fixtures/sample.nc`** seeded by `tests/common/mod.rs::write_netcdf3_fixture` (uses `netcdf3::FileWriter` to lay down a minimal 5-row, 2-variable file). Three new integration tests cover the NetCDF read path. Seven new unit tests cover `json_util::rename_object_key_at_path` and `add_object_key_at_path`. Total test count: 501 (up from 467).
