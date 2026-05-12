#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    echo "usage: $0 <binary> <sha256-file> [signature-file]" >&2
    exit 64
fi

binary="$1"
checksum="$2"
signature="${3:-}"

bash scripts/verify-static-binary.sh "$binary"
sha256sum --check "$checksum"

if [ -n "$signature" ]; then
    gpg --verify "$signature" "$binary"
else
    echo "no signature file supplied; checksum and static-link verification passed"
fi
