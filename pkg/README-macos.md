# Octa for macOS

A multi-format data viewer and editor for Parquet, CSV, JSON, Excel, and more.

This build is for Apple Silicon (arm64) Macs.

## Install

Drag `Octa.app` into `/Applications` (or `~/Applications`).

Because the app is **not signed or notarized**, the first launch will be blocked by Gatekeeper. To open it anyway:

1. Control-click `Octa.app` in Finder and choose **Open**, then confirm in the dialog.

   *Or*, from a terminal, clear the quarantine attribute:

   ```bash
   xattr -dr com.apple.quarantine /Applications/Octa.app
   ```

2. After the first successful launch, double-clicking works normally.

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
