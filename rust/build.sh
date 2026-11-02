#!/usr/bin/env bash
# =============================================================================
# build.sh - Compila a versao em Rust do Notas Definitivas (Linux / macOS)
# -----------------------------------------------------------------------------
# Uso (dentro da pasta rust/):
#   ./build.sh          compila em release  -> ../notas_rs
#   ./build.sh run      compila e executa
#   ./build.sh test     roda os testes (unitarios + integracao + fuzz)
#   ./build.sh clean    remove target/ e ../notas_rs
#
# Requisitos: Rust (rustup) e um compilador C (gcc/clang) para o SQLite embutido.
# No Linux, o clipboard X11/Wayland usa a crate arboard (sem dependencias de dev).
# =============================================================================
set -euo pipefail
cd "$(dirname "$0")"

case "${1:-build}" in
    clean)
        rm -rf target ../notas_rs
        echo "Artefatos removidos."
        ;;
    test)
        cargo test --release
        ;;
    build|run)
        cargo build --release
        cp target/release/notas ../notas_rs
        echo "    OK -> ../notas_rs"
        [ "${1:-build}" = "run" ] && exec ../notas_rs
        ;;
    *)
        echo "uso: $0 [build|run|test|clean]" >&2
        exit 2
        ;;
esac
