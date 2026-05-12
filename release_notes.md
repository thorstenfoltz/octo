## Features

- **Parse cell / row / column / table in a new tab.** New entry under **Edit → "Parse in new tab"** with four direct entries (Cell / Row / Column / Whole table), mirrored by a submenu on the cell right-click context menu. A modal then lets you pick the target format (JSON, JSON Lines, YAML, TOML, XML, CSV, TSV, Markdown, Plain Text) and — for CSV/TSV — a delimiter. Behavior for multi-cell scopes mirrors the existing JSON parser: JSON wraps the cell payloads as `[c1,c2,…]` so each cell becomes one row; every other text format joins cells with newlines. Table scope serializes the active table through the chosen format's writer (same path as Save As) and reopens the dump in a new tab. The result is routed through the standard reader pipeline via a `tempfile::NamedTempFile` named with the format's primary extension, and the new tab's `source_path` is cleared so an accidental Save can't overwrite `/tmp`.
- **In-app window controls (close / minimize / maximize) in the toolbar.** New checkbox under **Settings → Appearance → "Window controls in toolbar"** (default off). When on, Octa requests an undecorated viewport at startup and paints close / maximize / minimize buttons at the right edge of the existing toolbar — same bar that holds the Octa logo, File, Edit, and View menus. Useful on tiling window managers and minimal compositors that don't draw their own controls. The maximize button reads back from `ctx.input(|i| i.viewport().maximized)` so its highlighted state matches the live window. Takes effect after restart since `with_decorations` is a viewport-builder flag.
- **Status-bar busy indicator.** The status bar now shows a small spinner plus a short label ("Loading rows…" or "Updating…") whenever a long-running operation is in flight: a background row-load draining into the active tab or the auto-update checker / installer. Idle frames stay silent, so launching the app no longer suggests work is happening when nothing is.
- **Suppressed window-manager startup-notify cursor.** `octa.desktop` now sets `StartupNotify=false`, so the bouncing clock cursor X11 / Wayland used to show next to the mouse pointer for ~5 s after launching Octa from a desktop entry is gone. Real work-in-progress still surfaces through the in-app status-bar spinner above.

## Fixes

- **Insert Column dialog: no more spinner-arrow artifacts on the position field.** The "Insert at position" field has been swapped from `egui::DragValue` (which renders ± hover arrows that looked out of place in this small modal) to a plain `egui::TextEdit::singleline`. Out-of-range input tints the text red; the buffer is cleared on dialog close so the next open re-derives its default from the actual column count.
- **Insert Column dialog: formula no longer silently treats non-numeric cells as 0.** The formula evaluator previously coerced any non-numeric referenced cell (strings like `"abc"`, dates, booleans, binaries) to `0.0`, quietly corrupting the result. It now reports the first offender via `FormulaOutcome { value, bad_cell }`. After adding the column the dialog raises a dismissible banner: `Formula skipped N of M row(s); first non-numeric reference: column "B" row 3 = "abc"`.
- **View → Read-only mode menu label no longer shows the shortcut.** The entry used to read `Read-only mode  (F8)`; it now just reads `Read-only mode`. F8 (or the user-rebound combo) still toggles it.

## Removals

- **None.** All format readers, view modes, and existing settings from previous releases are still present.

## Documentation

- This branch only changes `Makefile`, `release_notes.md`, and the source tree — no user-facing README/CLAUDE.md churn beyond the new behaviors above.

## Internals

- **`tempfile = "3"`** promoted from `[dev-dependencies]` to `[dependencies]` so the new Parse-in-new-tab modal can stage its parser input on a `NamedTempFile`.
- **New module `src/app/dialogs/parse_in_new_tab.rs`** owning `ParseModalState`, the format ComboBox + delimiter sub-option, the cell-string combiner (JSON-array-wrap vs newline-join), and the tempfile-based reopen flow.
- **`ParseScope` enum (Cell / Row / Column / Table)** in `src/ui/toolbar.rs`, threaded through `ToolbarAction.parse_in_new_tab` and `TableInteraction.ctx_parse_in_new_tab` so both menu surfaces produce the same dispatched action.
- **`OctaApp.pending_parse_modal: Option<ParseModalState>`** new state field on the app shell; `dialogs::parse_in_new_tab::build_modal_state` snapshots the relevant cells when the modal opens so live edits to the source tab can't leak into the in-flight parse.
- **`FormulaOutcome { value, bad_cell }` + `FormulaBadCell { row, col, content }`** new public types on `octa::data`. `evaluate_formula(_)` is now a thin wrapper around `evaluate_formula_with_diagnostics(_)`; `cell_as_f64` returns `Result<Option<f64>, FormulaBadCell>` and the recursive `eval_expression / eval_term / eval_factor` chain threads a `&mut Option<FormulaBadCell>` accumulator.
- **`AppSettings.use_custom_title_bar: bool`** new persisted setting (default `false`); `main.rs` flips `with_decorations(false).with_resizable(true)` on the viewport when it's on, and `ui::toolbar::draw_toolbar` grows a new `show_window_controls: bool` parameter that lights up the right-aligned button trio.
- **`draw_status_bar` gains `busy: bool` + `busy_hint: Option<&str>`.** The caller (`OctaApp::render_status_bar`) computes `busy` from `bg_loading_done == false || update_state ∈ {Checking, Updating}` and picks the hint to display.
- **`tests/formula_tests.rs`** grew four new tests covering `evaluate_formula_with_diagnostics`: string references rejected, numeric-string references still parse, multi-bad-cell pinning, and clean numeric paths leaving `bad_cell == None`. Total test count: 505 (down four from the previous branch state because the auto-decode work — and its tests — were rolled back; see below).
- **`Makefile` lint targets switched from a removed custom Dockerfile to `npx mega-linter-runner@v9 --flavor rust`.** The previous `lint` / `lint-fix` recipes still pointed at `.github/megalinter-rust/`, which doesn't exist anymore (CI now uses the stock `ghcr.io/oxsecurity/megalinter-rust:v9` image directly). The new recipes piggyback on the existing `check-npx` guard, rely on npm + Docker caches (no `--pull always`), and pick up the `REPOSITORY_BRANCH_NAME` plugin → `.linters/validate_branch_name.sh` through the standard `.mega-linter.yml` config.
- **`octa.desktop` adds `StartupNotify=false`.** Suppresses the WM startup-notify cursor on Linux launches.

## Removed mid-branch (no user-visible effect)

- **Auto-decode of JSON-shaped cell content.** An earlier iteration of this branch added a post-load pass that scanned every string cell, parsed any JSON-object content, and exploded it into `<source>.<key>` sibling columns — gated by a per-format opt-in set under Settings → File-Specific. After testing it became clear that the conceptual model (cell-level decode vs file-level structural flatten) was too easy to confuse, so the entire pass, its settings field, its UI, its tests, and the `JSONL ↔ "JSON Lines"` rename it required were rolled back to match `master`. The Parse-in-new-tab flow replaces it as the explicit on-demand mechanism.
