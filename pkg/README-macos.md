# Octa for macOS

A multi-format data viewer and editor for Parquet, CSV, JSON, Excel, and more.

This build is for Apple Silicon (arm64) Macs.

## Install

Drag `Octa.app` into `/Applications` (or `~/Applications`).

The app is **ad-hoc signed but not notarized**, so macOS Gatekeeper will warn the first time you launch it. You have two options to bypass the warning:

**Option A — clear the quarantine attribute (always works):**

```bash
# Locate the bundle and confirm its quarantine attribute is present
find /Applications -maxdepth 1 -name "Octa.app" -exec xattr {} \;

# Strip the attribute (top-level only — macOS only quarantines the bundle)
xattr -d com.apple.quarantine /Applications/Octa.app

# If the strip above fails with "No such xattr: …" but the warning persists,
# fall back to the recursive form once (handles "Octa.app is damaged"):
# xattr -cr /Applications/Octa.app
```

After that, double-clicking opens the app normally.

**Option B — right-click → Open:**

Control-click `Octa.app` in Finder and choose **Open**, then confirm in the Gatekeeper dialog. After the first successful launch, double-clicking works normally.

> ⚠️  If you see "Octa.app is damaged and can't be opened" with only a "Move to Trash" button, your copy was extracted before the ad-hoc signature was applied — use the `xattr -cr` fallback shown above.

## Run from the command line

```bash
/Applications/Octa.app/Contents/MacOS/octa [file]
```

Optionally symlink it onto your `PATH`:

```bash
ln -s /Applications/Octa.app/Contents/MacOS/octa /usr/local/bin/octa
```

## Uninstall

```bash
rm -rf /Applications/Octa.app
```
