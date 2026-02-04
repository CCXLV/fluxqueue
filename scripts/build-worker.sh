#!/bin/bash
set -e

TARGET=$1

if [ -z "$TARGET" ]; then
  echo "Target missing"
  exit 1
fi


cargo build -p fluxqueue-worker --release --target "$TARGET"

mkdir -p dist
cp "target/$TARGET/release/fluxqueue-worker" "dist/fluxqueue-worker-$TARGET"
