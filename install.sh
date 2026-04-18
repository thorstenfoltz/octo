#!/usr/bin/env bash
set -euo pipefail

# Detect default prefix: Arch Linux uses /usr, others use /usr/local
if [ -z "${1:-}" ]; then
	if [ -f /etc/arch-release ]; then
		PREFIX="/usr"
	else
		PREFIX="/usr/local"
	fi
else
	PREFIX="$1"
fi
BIN_DIR="$PREFIX/bin"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
DESKTOP_DIR="$PREFIX/share/applications"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# If a pre-built binary exists next to this script, use it; otherwise build from source
if [[ -f "$SCRIPT_DIR/octa" ]]; then
	BINARY="$SCRIPT_DIR/octa"
	echo "Using pre-built binary."
elif [[ -f "$SCRIPT_DIR/target/release/octa" ]]; then
	BINARY="$SCRIPT_DIR/target/release/octa"
	echo "Using previously built binary at target/release/octa."
else
	if ! command -v cargo &>/dev/null; then
		echo "Error: No pre-built binary found next to this script and Rust/Cargo is not installed."
		echo
		echo "You have two options:"
		echo "  1. Download a pre-built release from"
		echo "     https://github.com/thorstenfoltz/octa/releases"
		echo "     and either place the 'octa' binary next to this script and rerun,"
		echo "     or just copy it to a directory on your PATH (no install needed)."
		echo "  2. Install the Rust toolchain from https://rustup.rs/ and rerun this script."
		exit 1
	fi
	echo "Building Octa (release)..."
	cargo build --release
	BINARY="$SCRIPT_DIR/target/release/octa"
fi

echo "Installing binary to $BIN_DIR..."
install -Dm755 "$BINARY" "$BIN_DIR/octa"

echo "Installing icon to $ICON_DIR..."
install -Dm644 "$SCRIPT_DIR/assets/octa.svg" "$ICON_DIR/octa.svg"

echo "Installing desktop entry to $DESKTOP_DIR..."
install -Dm644 "$SCRIPT_DIR/octa.desktop" "$DESKTOP_DIR/octa.desktop"

echo "Updating icon cache..."
if command -v gtk-update-icon-cache &>/dev/null; then
	gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

if command -v update-desktop-database &>/dev/null; then
	update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi

echo "Octa installed successfully."
echo "  Binary:  $BIN_DIR/octa"
echo "  Icon:    $ICON_DIR/octa.svg"
echo "  Desktop: $DESKTOP_DIR/octa.desktop"
