#!/usr/bin/env bash
# =============================================================================
# run_tests.sh - Testes de unidade da camada de armazenamento (SQLite)
# -----------------------------------------------------------------------------
# Compila tools/test/storage_test.c (que reusa notas.c como unidade de traducao)
# e roda a suite nas duas plataformas:
#   - Linux nativo                       -> ramo opendir/readdir
#   - Windows via MinGW (e wine, se houver) -> ramo FindFirstFileW
#
# Diferente dos testes de interface (que precisam de um terminal), este suite
# nao abre TUI: exercita diretamente db_open/CRUD/importacao/exportacao.
# =============================================================================
set -uo pipefail

cd "$(dirname "$0")/../.."          # raiz do projeto
ROOT="$PWD"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

FIX="$WORK/src"
# Diretorios de execucao separados: cada plataforma tem o proprio notas.db,
# senao a segunda execucao encontraria o banco ja populado pela primeira.
mkdir -p "$FIX" "$WORK/run_linux" "$WORK/run_win"
printf '# Alfa\n\nconteudo alfa\n'            > "$FIX/Alfa.md"
printf '# Beta\n\n## Secao\n\nbeta: \xc3\xa7\xc3\xa3o\n' > "$FIX/Beta.md"
printf '# Gama\n\ngama\n'                     > "$FIX/Gama.md"
printf 'nao markdown\n'                       > "$FIX/ignorar.pdf"

rc_total=0

echo "############################################################################"
echo "# LINUX"
echo "############################################################################"
gcc -O1 -Wall -Wextra -Wno-unused-function tools/test/storage_test.c \
    -o "$WORK/storage_test_linux" -lsqlite3 -ldl || { echo "FALHA no build Linux"; exit 1; }
( cd "$WORK/run_linux" && "$WORK/storage_test_linux" "$FIX" "$WORK/out_linux" ) || rc_total=1
echo "arquivos exportados:"; ls "$WORK/out_linux" 2>/dev/null | sed 's/^/    /'

if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 && [ -f tools/sqlite3-mingw/lib/libsqlite3.a ]; then
    echo
    echo "############################################################################"
    echo "# WINDOWS (MinGW)"
    echo "############################################################################"
    x86_64-w64-mingw32-gcc -O1 -Wall -Wextra -Wno-unused-function tools/test/storage_test.c \
        -o "$WORK/storage_test.exe" -Itools/sqlite3-mingw/include \
        tools/sqlite3-mingw/lib/libsqlite3.a || { echo "FALHA no build Windows"; rc_total=1; }

    if command -v wine >/dev/null 2>&1; then
        echo "--- executando sob wine ---"
        ( cd "$WORK/run_win" && WINEDEBUG=-all wine "$WORK/storage_test.exe" \
              "Z:${FIX//\//\\}" "Z:${WORK//\//\\}\\out_win" 2>/dev/null ) || rc_total=1
        echo "arquivos exportados:"; ls "$WORK/out_win" 2>/dev/null | sed 's/^/    /'
    else
        echo "(wine nao instalado: o .exe foi compilado mas nao executado)"
    fi
else
    echo
    echo "(pulando o alvo Windows: falta mingw-w64 ou tools/sqlite3-mingw/)"
fi

echo
if [ "$rc_total" -eq 0 ]; then
    echo "RESULTADO GERAL: TUDO OK"
else
    echo "RESULTADO GERAL: HOUVE FALHAS"
fi
exit "$rc_total"
