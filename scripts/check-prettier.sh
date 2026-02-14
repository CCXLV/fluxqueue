#!/bin/bash

set -euxo pipefail

PRETTIER_VERSION=3.8.1

pnpm dlx "prettier@${PRETTIER_VERSION}" --check docs/
