# libsqlite3 para o build Windows (MinGW)

## Por que existe esta pasta

A partir da **v6.29.0** as notas do Notas Definitivas vivem em um banco SQLite
(`notas.db`), em vez de arquivos `.md` dentro de `data/`. Isso significa que o
programa passa a depender da libsqlite3 nos dois alvos.

No Linux isso e trivial (`/usr/include/sqlite3.h` e `-lsqlite3` ja existem). No
Windows cross-compilado com MinGW-w64 **nao existe pacote correspondente no
repositorio do Arch** (procurei: nao ha `mingw-w64-sqlite3`), entao a
biblioteca precisa ser gerada.

## O que esta versionado aqui

| Arquivo | Descricao |
| :--- | :--- |
| `include/sqlite3.h` | cabecalho publico da amalgamation oficial |
| `include/sqlite3ext.h` | cabecalho de extensoes (exigido pelo sqlite3.h) |
| `lib/libsqlite3.a` | biblioteca estatica x86-64 para Windows (PE/COFF) |

Origem: `https://sqlite.org/2026/sqlite-amalgamation-3530400.zip`
(SQLite **3.53.4** — a mesma versao do SQLite instalado no Linux deste
projeto, para nao haver divergencia de comportamento entre as plataformas).

Compilada com:

```sh
x86_64-w64-mingw32-gcc -O2 -DSQLITE_OMIT_LOAD_EXTENSION -DSQLITE_THREADSAFE=1 \
    -c sqlite3.c -o sqlite3.o
x86_64-w64-mingw32-ar rcs libsqlite3.a sqlite3.o
```

`SQLITE_OMIT_LOAD_EXTENSION` remove o suporte a carregar extensoes dinamicas:
nao e usado pelo editor e evita dependencia de `dlopen` no Windows.
`SQLITE_THREADSAFE=1` mantem o modo serializado, ja que o editor usa uma thread
de segundo plano para a area de transferencia.

## Como regenerar (ou atualizar a versao)

```sh
./tools/build_sqlite_mingw.sh
```

O script baixa a amalgamation, cross-compila e substitui os arquivos desta
pasta. Para trocar de versao:

```sh
SQLITE_VER=3540000 SQLITE_YEAR=2026 ./tools/build_sqlite_mingw.sh
```

## Efeito no binario

A biblioteca entra **estaticamente** no executavel: o `notas.exe` fica com
~1,5 MB (contra ~315 KB antes) e **nao precisa de nenhum DLL** ao lado dele.

## Nota sobre tamanho no Git

Os tres arquivos somam ~1,3 MB. Foram versionados de proposito, para que
`./build.sh windows` funcione sem rede e de forma reproduzivel.
