#!/bin/bash

LIB_DIR=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")

if [ -n "$LIB_DIR" ]; then
    export LD_LIBRARY_PATH="$LIB_DIR:$LD_LIBRARY_PATH"
fi

exec "./fluxqueue-worker" "$@"
