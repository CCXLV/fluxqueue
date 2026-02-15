#!/usr/bin/env bash
set -e

TARGET=$1

if [ -z "$TARGET" ]; then
  echo "Target missing"
  exit 1
fi

cargo build -p fluxqueue-worker --release --target "$TARGET"

BIN_NAME="fluxqueue-worker"
EXT=""

# Add .exe only for Windows targets
if [[ "$TARGET" == *"windows"* ]]; then
  EXT=".exe"
fi

mkdir -p dist
cp "target/$TARGET/release/${BIN_NAME}${EXT}" "dist/${BIN_NAME}-${TARGET}${EXT}"
