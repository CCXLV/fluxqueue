#!/bin/bash

set -euxo pipefail

# Running Python tests
uv run pytest tests/

# Setting up LD_LIBRARY_PATH environment variable
# https://pyo3.rs/v0.28.2/building-and-distribution.html#dynamically-embedding-the-python-interpreter
LIB_DIR=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")
export LD_LIBRARY_PATH="$LIB_DIR:${LD_LIBRARY_PATH:-}"

uv run cargo test --workspace --locked
