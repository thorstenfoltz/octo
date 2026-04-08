# Octa for Windows

A multi-format data viewer and editor for Parquet, CSV, JSON, Excel, and more.

## Install

Run `install.bat` as Administrator (right-click, "Run as administrator").

This will:

- Copy `octa.exe` to `C:\Program Files\Octa`
- Add it to your user PATH
- Create a Start Menu shortcut

You may need to restart your terminal for PATH changes to take effect.

## Run without installing

Double-click `octa.exe` or run from the command line:

```bash
octa.exe [file]
```

## Uninstall

1. Delete `C:\Program Files\Octa`
2. Remove `C:\Program Files\Octa` from your PATH (Settings > System > About > Advanced system settings > Environment Variables)
3. Delete the Start Menu shortcut at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Octa.lnk`
