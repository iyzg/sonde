#!/bin/sh
set -e

if command -v cargo >/dev/null 2>&1; then
    cargo install --git https://github.com/iyzg/sonde
else
    echo "cargo not found. install rust first: https://rustup.rs"
    exit 1
fi
