#!/bin/bash

LIB_DIR=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")
export LD_LIBRARY_PATH="$LIB_DIR:$LD_LIBRARY_PATH"
