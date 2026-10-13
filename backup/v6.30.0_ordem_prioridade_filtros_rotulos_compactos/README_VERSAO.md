# Notas Definitivas — v6.30.0

**Data do snapshot:** 13/10/2026
**Estado:** programa funcional, compilado e testado nas duas plataformas.

## Onde as notas ficam agora

As notas vivem em um banco **SQLite** (`notas.db`), criado ao lado do executável —
não mais em arquivos `.md` dentro de `data/`. Na primeira execução o banco nasce
**vazio**. Para trazer um acervo de `.md`, use o botão **[⬇ Importar]** na barra
inferior; para gerar `.md` a partir do banco, **[⬆ Exportar]**.

> Atenção: o programa faz `chdir()` para o diretório do executável, então
> `notas.db` fica sempre ao lado do binário. Junto dele aparecem `notas.db-wal` e
> `notas.db-shm` (modo WAL) — são normais.

## O que esta versão consolida

Este snapshot consolida três etapas que foram feitas em sequência e ainda não
tinham sido fotografadas separadamente (não existe cópia do estado intermediário
delas):

### v6.29.0 — Backend SQLite, importação e exportação
- Armazenamento migrado para SQLite. O `filepath` interno passou a usar o
  caminho virtual `db:<id>`, o que preservou toda a interface existente (abas,
  undo, preview, histórico, busca, wikilinks) sem reescrevê-la.
- O nível de urgência deixou de morar em `data/.attention` e virou a coluna
  `attention` da tabela `notes`.
- Banco inicia vazio: o programa não cria mais `Bem-vindo.md`.
- **[⬇ Importar]**: pasta de `.md` (`.markdown`/`.mdown`/`.txt`/`.text` também) →
  banco. Reimportar a mesma pasta **atualiza** em vez de duplicar.
- **[⬆ Exportar]**: banco → pasta de `.md` (criada se não existir).
- Varredura de pasta: `opendir/readdir` no Linux, `FindFirstFileW` no Windows.
- SQLite 3.53.4 (mesma versão do sistema) embutido **estaticamente** no `.exe`;
  a `libsqlite3.a` para MinGW é gerada por `tools/build_sqlite_mingw.sh` e fica em
  `tools/sqlite3-mingw/` (não há pacote mingw-w64 de SQLite no Arch).

### v6.29.1 — Correção: não era possível sair sem abas abertas
- `:q` e `:q!` não funcionavam quando **nenhuma** aba estava aberta: o bloco
  `if (E.num_tabs == 0)` de `editor_process_keypress()` aceitava apenas
  `i/I/a/A/o/O/n` e descartava todo o resto — inclusive o `:`. Como o banco novo
  passou a ser o estado inicial, o programa ficava impossível de fechar na
  primeira execução.
- Correção: a linha de comando (`:`) passou a ser aceita nesse estado, o que
  também reabilita `:life`, `:theme`, `:help` etc. com a lista vazia.

### v6.30.0 — Ordem por prioridade, filtros e rótulos compactos
- A lista lateral é exibida em **grupos de prioridade**: nível 1, depois 2,
  depois 3, e por último as sem marcação. **Dentro de cada grupo, a nota editada
  mais recentemente aparece primeiro** (desempate por nome).
- Marcar/desmarcar (`m` ou clique na caixa) **reordena na hora** e a **seleção
  acompanha a nota**, sem o cursor pular para outra linha.
- **Correção dos filtros:** os botões `[1]`, `[2]` e `[3]` da lateral deixavam de
  responder quando o painel esquerdo estava estreito, porque o clique estava
  preso ao limite `E.mouse_col <= E.left_width` enquanto os botões são
  desenhados até a coluna 20. Quanto mais estreito o painel, mais botões
  morriam — primeiro o `[3]`, depois o `[2]`. Sintoma que aparecia como "no
  Windows não filtra", porque o console lá era mais estreito.
- **Rótulos compactos:** com o painel abaixo de 20 colunas, os rótulos viram
  ` [T] [1] [2] [3]` (16 colunas) em vez de ` [Todas] [1] [2] [3]` (20).
- Correção de avisos de truncagem no build Windows (`combined`/`wcomb`/`url_buf`)
  e no carregamento de nota (`nm`/`fn`), com os buffers consumidores alinhados
  (`MultiByteToWideChar` falha em vez de truncar).

## Limitação conhecida

As marcações de urgência **não** são gravadas nos arquivos `.md` exportados: elas
existem apenas na coluna `attention` do banco. Consequência: exportar e depois
importar em um banco **novo** faz as notas voltarem **sem marcação**. Reimportar
sobre um banco que já tem a nota preserva a marcação (só o conteúdo é
atualizado). **Para backup fiel, copie o `notas.db`.**

## Arquivos deste snapshot

| Arquivo | Descrição |
| :--- | :--- |
| `notas.c` | Fonte único (C99), ~10.100 linhas |
| `notas` | Binário Linux (202.528 bytes, linka libsqlite3 dinâmica) |
| `notas.exe` | Binário Windows (1.466.832 bytes, SQLite estático, sem DLLs) |
| `build.sh` | Build Linux/Windows (`linux`, `windows`, `all`, `run`, `clean`) |
| `build.bat` | Build/execução no Windows |
| `README.md` | Documentação de uso |
| `tools/` | `sqlite3-mingw/` (headers + `libsqlite3.a`), `build_sqlite_mingw.sh`, `test/` |

## Como compilar

```sh
./build.sh all        # Linux (gcc) + Windows (mingw-w64), sem warnings
./tools/test/run_tests.sh   # testes de unidade nas duas plataformas
```

Windows: `notas.exe` precisa apenas de si mesmo — não há DLL para acompanhar
(o SQLite é estático). Requer Windows 10/11 (usa Virtual Terminal).

## Verificação desta versão

| Suíte | Resultado |
| :--- | :--- |
| Unidade da camada de armazenamento — Linux | 24/24 |
| Unidade da camada de armazenamento — Windows (wine) | 24/24 |
| Funcional de interface (criar/salvar/importar/exportar) | 31/31 |
| Ordenação por prioridade | 17/17 |
| Rótulos compactos e cliques | 14/14 |
| Filtros com painel largo e estreito | 4/4 + 4/4 |

Compilação **sem warnings** com `-O2 -Wall -Wextra` em `gcc` e em
`x86_64-w64-mingw32-gcc`.
