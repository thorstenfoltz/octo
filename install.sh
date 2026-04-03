#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-/usr/local}"
BIN_DIR="$PREFIX/bin"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
DESKTOP_DIR="$PREFIX/share/applications"

echo "Building Octo (release)..."
cargo build --release

echo "Installing binary to $BIN_DIR..."
install -Dm755 target/release/octo "$BIN_DIR/octo"

echo "Installing icon to $ICON_DIR..."
install -Dm644 assets/octo.svg "$ICON_DIR/octo.svg"

echo "Installing desktop entry to $DESKTOP_DIR..."
install -Dm644 octo.desktop "$DESKTOP_DIR/octo.desktop"

echo "Updating icon cache..."
if command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi

echo "Octo installed successfully."
echo "  Binary:  $BIN_DIR/octo"
echo "  Icon:    $ICON_DIR/octo.svg"
echo "  Desktop: $DESKTOP_DIR/octo.desktop"
