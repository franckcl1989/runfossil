#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <binary>" >&2
    exit 64
fi

binary="$1"

if [ ! -f "$binary" ]; then
    echo "static binary not found: $binary" >&2
    exit 66
fi

file_output="$(file "$binary")"
echo "$file_output"

if ! printf '%s\n' "$file_output" | grep -Eqi 'statically linked|static-pie linked'; then
    echo "binary is not reported as statically linked or static PIE by file(1)" >&2
    exit 1
fi

ldd_output="$(ldd "$binary" 2>&1 || true)"
echo "$ldd_output"

if ! printf '%s\n' "$ldd_output" | grep -Eqi 'not a dynamic executable|statically linked'; then
    echo "ldd reported dynamic dependencies for $binary" >&2
    exit 1
fi
