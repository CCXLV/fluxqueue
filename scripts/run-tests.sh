#!/bin/bash

set -euxo pipefail

source .venv/bin/activate

# Setting up LD_LIBRARY_PATH environment variable
# https://pyo3.rs/v0.28.2/building-and-distribution.html#dynamically-embedding-the-python-interpreter
OS="$(uname)"
LIB_DIR=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")

if [[ "$OS" == "Darwin" ]]; then
    export DYLD_LIBRARY_PATH="$LIB_DIR:${DYLD_LIBRARY_PATH:-}"
else
    export LD_LIBRARY_PATH="$LIB_DIR:${LD_LIBRARY_PATH:-}"
fi

pytest tests/

cargo test --workspace --locked
