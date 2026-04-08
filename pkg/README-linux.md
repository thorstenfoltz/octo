# Octa for Linux

A multi-format data viewer and editor for Parquet, CSV, JSON, Excel, and more.

## Install

Run the install script (installs to `/usr/local` by default, requires sudo):

```bash
sudo ./install.sh
```

To install to a custom prefix (e.g. `~/.local` for user-local, no sudo needed):

```bash
./install.sh ~/.local
```

This installs:

- Binary to `<prefix>/bin/octa`
- Icon to `<prefix>/share/icons/hicolor/scalable/apps/octa.svg`
- Desktop entry to `<prefix>/share/applications/octa.desktop`

## Uninstall

```bash
sudo ./uninstall.sh
```

Or with the same custom prefix used during install:

```bash
./uninstall.sh ~/.local
```

## Run without installing

```bash
./octa [file]
```

## Arch Linux

Octa is available on the AUR as `octa` (source) and `octa-bin` (pre-compiled).
