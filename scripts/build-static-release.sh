#!/usr/bin/env bash
set -euo pipefail

target="x86_64-unknown-linux-musl"
binary="target/${target}/release/runfossil"

if ! rustup target list --installed | grep -qx "$target"; then
    rustup target add "$target"
fi

if command -v musl-gcc >/dev/null 2>&1; then
    linker="musl-gcc"
elif command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    linker="x86_64-linux-musl-gcc"
else
    echo "missing musl linker; install musl-tools or provide musl-gcc" >&2
    exit 69
fi

export CC_x86_64_unknown_linux_musl="$linker"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$linker"

cargo build --release --target "$target" -p runfossil-cli
bash scripts/verify-static-binary.sh "$binary"
sha256sum "$binary" > "${binary}.sha256"
echo "$binary"
echo "${binary}.sha256"
