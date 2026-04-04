FROM ghcr.io/oxsecurity/megalinter-rust:v9

# Install system libraries for mupdf-sys compilation and libclang for bindgen.
# The musl Rust target statically links build scripts by default, which prevents
# bindgen from dlopen-ing libclang.so. Setting -crt-static produces dynamically
# linked build scripts that can load shared libraries.
RUN apk add --no-cache \
    build-base \
    pkgconf \
    fontconfig-dev \
    freetype-dev \
    harfbuzz-dev \
    fribidi-dev \
    jpeg-dev \
    openjpeg-dev \
    gumbo-parser-dev \
    jbig2dec-dev \
    mujs-dev \
    clang-dev \
    clang-libs \
    llvm21-libs

ENV LIBCLANG_PATH=/usr/lib/llvm21/lib
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=-crt-static"
