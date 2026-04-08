# Local Package Testing

Build and install Octa as an Arch Linux package from the local working tree before publishing to the AUR.

## Prerequisites

Clean up any previous manual installs (via `install.sh`) so they don't conflict with the pacman-managed package:

```bash
sudo rm -f /usr/share/applications/octa.desktop /usr/share/icons/hicolor/scalable/apps/octa.svg /usr/bin/octa
rm -f ~/.local/share/applications/octa.desktop ~/.local/share/icons/hicolor/scalable/apps/octa.svg ~/.local/bin/octa
```

## Usage

```bash
./pkg/build-local.sh <source|binary> [version]
```

The version argument is optional and defaults to `0.0.1`.

### Binary package (fast, for testing desktop integration)

Compiles once upfront, then packages the binary. Install is instant.

```bash
./pkg/build-local.sh binary 0.1.0
paru -U pkg/build/binary/octa-bin-0.1.0-1-x86_64.pkg.tar.zst
```

### Source package (mirrors AUR experience)

Prepares the PKGBUILD and source tarball. Compilation happens when paru builds and installs, exactly like an AUR install would.

```bash
./pkg/build-local.sh source 0.1.0
cd pkg/build/source && paru -Ui
```

The two packages conflict with each other and cannot be installed simultaneously.

## What to verify

- **Launch from terminal:** `octa` and `octa path/to/file.parquet`
- **Application menu:** Octa appears once (not duplicated) under the correct category
- **Taskbar icon:** opening Octa groups under the Octa icon, not a generic Wayland icon
- **File associations:** double-clicking a `.csv` or `.parquet` file opens Octa
- **Uninstall is clean:** removes everything without leftovers

## Uninstall

```bash
paru -R octa
# or
paru -R octa-bin
```

## Notes

- The source package includes uncommitted changes from the working tree — useful for testing before committing.
- Neither package is uploaded anywhere. Both use `sha256sums=('SKIP')` since the tarball is generated locally.
