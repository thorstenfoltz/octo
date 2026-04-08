#!/usr/bin/env bash
set -euo pipefail

# Build a local Arch package from the current working tree for testing.
#
# Usage: ./pkg/build-local.sh [source|binary] [version]
#   source  — compiles from source during install (mirrors AUR experience)
#   binary  — pre-compiled, fast install
#   version defaults to 0.0.1
#
# Install: paru -U pkg/build/<type>/octa-*.pkg.tar.zst
# Remove:  paru -R octa  or  paru -R octa-bin

TYPE="${1:-}"
VERSION="${2:-0.0.1}"

if [[ "$TYPE" != "source" && "$TYPE" != "binary" ]]; then
	echo "Usage: $0 <source|binary> [version]"
	echo ""
	echo "  source  — compiles from source during install (mirrors AUR)"
	echo "  binary  — pre-compiled binary, fast install"
	exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$SCRIPT_DIR/build/$TYPE"
ARCH="$(uname -m)"

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

if [[ "$TYPE" == "source" ]]; then
	# -------------------------------------------------------------------
	# Source package — compiles during install
	# -------------------------------------------------------------------
	echo "==> Creating source tarball for version $VERSION..."
	tar czf "$BUILD_DIR/octa-$VERSION.tar.gz" \
		--transform "s,^,octa-$VERSION/," \
		--exclude='.git' \
		--exclude='target' \
		--exclude='pkg/build' \
		-C "$REPO_DIR" .

	cat >"$BUILD_DIR/PKGBUILD" <<'PKGBUILD_EOF'
# Local test build (source) — not for AUR
pkgname=octa
pkgver=VERSION_PLACEHOLDER
pkgrel=1
pkgdesc="A modular multi-format data viewer and editor"
arch=('x86_64')
url="https://github.com/thorstenfoltz/octa"
license=('MIT')
depends=('gtk3' 'libxcb' 'libxkbcommon' 'openssl' 'fontconfig' 'freetype2' 'harfbuzz' 'fribidi' 'libjpeg-turbo' 'openjpeg2' 'gumbo-parser' 'jbig2dec' 'mujs')
makedepends=('rust' 'cargo' 'clang' 'cmake' 'nasm' 'pkgconf')
conflicts=('octa-bin')
options=(!lto)
source=("octa-$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    sed -i "s/^version = .*/version = \"$pkgver\"/" Cargo.toml
    export RUSTUP_TOOLCHAIN=stable
    cargo update --workspace
    cargo fetch --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    export CARGO_BUILD_JOBS="$(nproc)"
    export MAKEFLAGS="-j$(nproc)"
    cargo build --frozen --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/octa" "$pkgdir/usr/bin/octa"
    install -Dm644 "assets/octa.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/octa.svg"
    install -Dm644 "octa.desktop" "$pkgdir/usr/share/applications/octa.desktop"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
PKGBUILD_EOF

	sed -i "s/VERSION_PLACEHOLDER/$VERSION/" "$BUILD_DIR/PKGBUILD"

	echo ""
	echo "Done! Source package prepared. Build and install with:"
	echo "  cd $BUILD_DIR && paru -Ui"

else
	# -------------------------------------------------------------------
	# Binary package — pre-compiled, just installs files
	# -------------------------------------------------------------------
	echo "==> Building release binary..."
	cd "$REPO_DIR"
	sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
	cargo build --release 2>&1
	sed -i 's/^version = .*/version = "0.0.0-dev"/' Cargo.toml

	cp "$REPO_DIR/target/release/octa" "$BUILD_DIR/octa"
	cp "$REPO_DIR/assets/octa.svg" "$BUILD_DIR/octa.svg"
	cp "$REPO_DIR/octa.desktop" "$BUILD_DIR/octa.desktop"
	cp "$REPO_DIR/LICENSE" "$BUILD_DIR/LICENSE"

	cat >"$BUILD_DIR/PKGBUILD" <<'PKGBUILD_EOF'
# Local test build (binary) — not for AUR
pkgname=octa-bin
pkgver=VERSION_PLACEHOLDER
pkgrel=1
pkgdesc="A modular multi-format data viewer and editor (pre-compiled)"
arch=('x86_64')
url="https://github.com/thorstenfoltz/octa"
license=('MIT')
depends=('gtk3' 'libxcb' 'libxkbcommon' 'openssl' 'fontconfig' 'freetype2' 'harfbuzz' 'fribidi' 'libjpeg-turbo' 'openjpeg2' 'gumbo-parser' 'jbig2dec' 'mujs')
provides=('octa')
conflicts=('octa')
source=('octa' 'octa.svg' 'octa.desktop' 'LICENSE')
sha256sums=('SKIP' 'SKIP' 'SKIP' 'SKIP')

package() {
    install -Dm755 "$srcdir/octa" "$pkgdir/usr/bin/octa"
    install -Dm644 "$srcdir/octa.svg" "$pkgdir/usr/share/icons/hicolor/scalable/apps/octa.svg"
    install -Dm644 "$srcdir/octa.desktop" "$pkgdir/usr/share/applications/octa.desktop"
    install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/octa/LICENSE"
}
PKGBUILD_EOF

	sed -i "s/VERSION_PLACEHOLDER/$VERSION/" "$BUILD_DIR/PKGBUILD"

	echo "==> Packaging binary..."
	cd "$BUILD_DIR"
	makepkg -sf

	PKG="$BUILD_DIR/octa-bin-${VERSION}-1-${ARCH}.pkg.tar.zst"
	echo ""
	echo "Done! Install with:"
	echo "  paru -U $PKG"
fi

PKGNAME="octa"
[[ "$TYPE" == "binary" ]] && PKGNAME="octa-bin"
echo ""
echo "Remove with:  paru -R $PKGNAME"
