#!/usr/bin/env bash
# =============================================================================
# build.sh - Script de compilacao do Notas Definitivas (Linux / cross-Windows)
# -----------------------------------------------------------------------------
# Uso:
#   ./build.sh              compila para Linux           -> ./notas
#   ./build.sh linux        idem (explicito)
#   ./build.sh windows      cross-compila para Windows  -> ./notas.exe  (precisa
#                           do mingw-w64: x86_64-w64-mingw32-gcc)
#   ./build.sh all          compila os dois
#   ./build.sh run          compila para Linux e executa
#   ./build.sh clean        remove os binarios gerados
#
# Variaveis opcionais:
#   CC=clang ./build.sh linux     usa outro compilador
#   MINW=... ./build.sh windows   usa outro compilador de cross
# =============================================================================
set -euo pipefail

cd "$(dirname "$0")"

# O fonte vive na raiz na versao standalone, e em src/ na arvore completa.
SRC="notas.c"
if [ -f "src/notas.c" ]; then
    SRC="src/notas.c"
fi

CC="${CC:-gcc}"
MINW="${MINW:-x86_64-w64-mingw32-gcc}"

CFLAGS_COMMON="-O2 -Wall -Wextra"
LIBS_LINUX="-ldl"
LIBS_WINDOWS="-lkernel32 -luser32 -lshell32"

OUT_LINUX="notas"
OUT_WINDOWS="notas.exe"

if [ ! -f "$SRC" ]; then
    echo "ERRO: fonte nao encontrado ($SRC). Rode o script na pasta do projeto." >&2
    exit 1
fi

build_linux() {
    echo "==> Compilando para Linux com $CC ($SRC)"
    # shellcheck disable=SC2086
    $CC $CFLAGS_COMMON "$SRC" -o "$OUT_LINUX" $LIBS_LINUX
    echo "    OK -> $PWD/$OUT_LINUX  ($(stat -c%s "$OUT_LINUX" 2>/dev/null || echo '?') bytes)"
}

build_windows() {
    if ! command -v "$MINW" >/dev/null 2>&1; then
        echo "ERRO: $MINW nao encontrado. Instale o mingw-w64" >&2
        echo "      (Arch/Omarchy: sudo pacman -S mingw-w64-gcc)" >&2
        exit 1
    fi
    echo "==> Cross-compilando para Windows com $MINW ($SRC)"
    # shellcheck disable=SC2086
    $MINW $CFLAGS_COMMON "$SRC" -o "$OUT_WINDOWS" $LIBS_WINDOWS
    echo "    OK -> $PWD/$OUT_WINDOWS  ($(stat -c%s "$OUT_WINDOWS" 2>/dev/null || echo '?') bytes)"
}

case "${1:-linux}" in
    linux|"")
        build_linux
        ;;
    windows|win|exe)
        build_windows
        ;;
    all|both)
        build_linux
        build_windows
        ;;
    run)
        build_linux
        echo "==> Executando ./$OUT_LINUX"
        exec "./$OUT_LINUX"
        ;;
    clean)
        rm -f "$OUT_LINUX" "$OUT_WINDOWS"
        echo "==> Binarios removidos."
        ;;
    *)
        echo "Uso: $0 [linux|windows|all|run|clean]" >&2
        exit 1
        ;;
esac
