#!/usr/bin/env bash
# =============================================================================
# build_sqlite_mingw.sh - Gera a libsqlite3 ESTATICA para MinGW (alvo Windows)
# -----------------------------------------------------------------------------
# A partir da v6.29.0 as notas vivem em um banco SQLite, entao o notas.exe
# precisa da libsqlite3 compilada para Windows. O repositorio do Arch nao tem
# esse pacote, portanto ele e gerado aqui a partir da amalgamation oficial.
#
# Resultado (versionado no repositorio para o build nao depender de rede):
#   tools/sqlite3-mingw/include/sqlite3.h, sqlite3ext.h
#   tools/sqlite3-mingw/lib/libsqlite3.a
#
# Uso:
#   ./tools/build_sqlite_mingw.sh
#   SQLITE_VER=3540000 SQLITE_YEAR=2026 ./tools/build_sqlite_mingw.sh
# =============================================================================
set -euo pipefail

cd "$(dirname "$0")/.."

# SQLite 3.53.4 = mesma versao usada no Linux deste projeto.
SQLITE_VER="${SQLITE_VER:-3530400}"
SQLITE_YEAR="${SQLITE_YEAR:-2026}"
URL="https://sqlite.org/${SQLITE_YEAR}/sqlite-amalgamation-${SQLITE_VER}.zip"

MINGW_CC="${MINGW_CC:-x86_64-w64-mingw32-gcc}"
MINGW_AR="${MINGW_AR:-x86_64-w64-mingw32-ar}"
DEST="tools/sqlite3-mingw"

for tool in "$MINGW_CC" "$MINGW_AR" unzip curl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "ERRO: '$tool' nao encontrado." >&2
        echo "      Arch/Omarchy: sudo pacman -S mingw-w64-gcc unzip curl" >&2
        exit 1
    fi
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> Baixando a amalgamation: $URL"
curl -fsSL -o "$WORK/amalg.zip" "$URL"
unzip -q "$WORK/amalg.zip" -d "$WORK"
SRCDIR="$(find "$WORK" -maxdepth 1 -type d -name 'sqlite-amalgamation-*' | head -1)"
if [ -z "$SRCDIR" ]; then
    echo "ERRO: pacote inesperado (nao achei sqlite-amalgamation-*)" >&2
    exit 1
fi

mkdir -p "$DEST/include" "$DEST/lib"
cp "$SRCDIR/sqlite3.h" "$SRCDIR/sqlite3ext.h" "$DEST/include/"

echo "==> Cross-compilando sqlite3.c com $MINGW_CC"
"$MINGW_CC" -O2 -DSQLITE_OMIT_LOAD_EXTENSION -DSQLITE_THREADSAFE=1 \
    -c "$SRCDIR/sqlite3.c" -o "$WORK/sqlite3.o"
"$MINGW_AR" rcs "$DEST/lib/libsqlite3.a" "$WORK/sqlite3.o"

echo "==> OK:"
ls -la "$DEST/include" "$DEST/lib"
echo
echo "Agora rode:  ./build.sh windows"
