#!/usr/bin/env bash
# Entrada principal: a versao mantida do projeto e Rust.
set -euo pipefail
exec bash "$(dirname "$0")/rust/build.sh" "$@"
