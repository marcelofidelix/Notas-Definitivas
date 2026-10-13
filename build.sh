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
MINW_SQLITE_DIR="${MINW_SQLITE_DIR:-$PWD/tools/sqlite3-mingw}"

CFLAGS_COMMON="-O2 -Wall -Wextra"
# A partir da v6.29.0 as notas vivem em um banco SQLite (notas.db),
# entao libsqlite3 e obrigatoria nos dois alvos.
LIBS_LINUX="-lsqlite3 -ldl"
LIBS_WINDOWS="-lsqlite3 -lkernel32 -luser32 -lshell32"

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
    # O alvo Windows precisa da libsqlite3 compilada para MinGW (nao existe
    # no repositorio do Arch). Ver tools/BUILD_WINDOWS_SQLITE.md.
    if [ ! -f "$MINW_SQLITE_DIR/include/sqlite3.h" ]; then
        echo "ERRO: libsqlite3 para MinGW nao encontrada em $MINW_SQLITE_DIR" >&2
        echo "      Gere com:  ./tools/build_sqlite_mingw.sh" >&2
        echo "      (veja tools/BUILD_WINDOWS_SQLITE.md)" >&2
        exit 1
    fi
    LIBS_WINDOWS="$LIBS_WINDOWS -I$MINW_SQLITE_DIR/include -L$MINW_SQLITE_DIR/lib"
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
