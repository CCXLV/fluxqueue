#!/bin/bash
set -e

TARGET=$1

if [ -z "$TARGET" ]; then
  echo "Target missing"
  exit 1
fi


cargo build -p fastqueue-worker --release --target "$TARGET"

mkdir -p dist
cp "target/$TARGET/release/fastqueue-worker" "dist/fastqueue-worker-$TARGET"
