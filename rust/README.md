# 🦀 Notas Definitivas — versão em Rust

Reescrita completa do editor em **Rust 2021** (sem `unsafe` fora de duas chamadas Win32), com **as mesmas funcionalidades** da versão em C (`../notas.c`) e o **mesmo `notas.db`**: as duas versões podem ser usadas alternadamente sobre o mesmo banco.

| | C (`../notas.c`) | Rust (`src/`) |
| :-- | :-- | :-- |
| Tamanho | ~11 mil linhas, um arquivo | ~10 mil linhas em 20 arquivos (cerca de 1/3 são testes) |
| Memória | `malloc`/`realloc` manuais | segura por construção (sem estouro, uso-após-liberação ou vazamento) |
| Texto | bytes + decodificação manual de UTF-8 | `String` + cursor sempre em limite de caractere |
| Terminal | VT manual (`termios` / Win32) | `crossterm` (Windows, Linux, macOS) |
| SQLite | `libsqlite3` externa | `rusqlite` com SQLite **embutido** (nenhuma DLL) |
| Clipboard | threads + `wl-copy`/`xclip`/`clip.exe` | `arboard` (Win32, X11, Wayland) + OSC 52 |

## Como compilar

```cmd
cd rust
build.bat            REM release -> ..\notas_rs.exe
build.bat run        REM compila e executa
build.bat test       REM unitarios + integracao + fuzz
```

```bash
cd rust && ./build.sh        # release -> ../notas_rs
```

Ou direto com o Cargo: `cargo build --release` (binário em `target/release/notas`).

- O banco fica **ao lado do executável** (`notas.db`), igual ao C; `NOTAS_DB=caminho` sobrescreve.
- `notas [arquivo.md]` abre um arquivo avulso (criado ao salvar, se não existir). `--version` e `--help` funcionam.
- Requisitos: Rust (rustup) + um compilador/linker C do mesmo alvo (o SQLite é compilado junto). **No Windows, use o alvo MSVC** (`rustup default stable-x86_64-pc-windows-msvc` + Visual Studio Build Tools com a carga de trabalho C++, instalável com `winget install Microsoft.VisualStudio.2022.BuildTools`); é o caminho testado e não depende do GCC.

### Problemas conhecidos

- **`collect2.exe: CreateProcess: No such file or directory` (Windows, alvo GNU).** Algumas máquinas com *Controle de Aplicativo / Smart App Control* bloqueiam o `collect2.exe` (não assinado) do GCC do WinLibs, e o Rust não consegue linkar — o mesmo vale para qualquer programa C. Não contorne a política do Windows. Solução adotada neste projeto: o alvo **MSVC** (Visual Studio Build Tools, assinado pela Microsoft). Alternativas: usar outra distribuição do MinGW-w64 cujos binários sejam permitidos; ou liberar o binário na política da sua organização.

## Funcionalidades (paridade com a versão em C)

Tudo o que está no [README principal](../README.md) existe aqui: edição modal (Normal, Inserção, Visual caractere/linha, `:` comandos), `y`/`p`/`x`/`dd`/`u`/`Ctrl+R`, seleção por `Shift+Setas` e mouse, abas (`gt`/`gT`, `c`/`C`, clique do meio), lista lateral por urgência (`[1]…[O]`, filtros `0`–`6`, `m`/`!`), busca sem acentos com prévia (`Ctrl+F`), tabelas auto-alinhadas (`Tab`, `gqq`, `:table`, `:tableall`), dobras (`za`, `zM`, `zR`, `zc`, `zo`), links `[t](url)` e `[[wiki]]` (Enter, `gl`, `Ctrl+N`) — **inclusive execução de scripts** (`.bat`, `.cmd`, `.ps1`, `.sh`, `.exe`… com a pasta do script como diretório de trabalho) —, quebra de linha visual, 8 temas TrueColor, `:import`/`:export`, autosave a cada 1 min, histórico (`Backspace`), Game of Life (`:life`, ociosidade de 10 min) e cursor **fixo** (sem piscar) com quadros sincronizados (DEC 2026).

### Melhorias em relação à versão em C

- **Segurança do terminal:** todo texto de nota/nome/prompt passa por `sanitize_for_terminal` — um `.md` com `ESC`/OSC embutidos não consegue escrever no clipboard, mudar o título nem bagunçar a tela.
- **Robustez comprovada por fuzz:** 10 milhões de teclas/cliques/redimensionamentos aleatórios sem pânico e sem violar invariantes (cursor em limite de caractere, índices válidos). O fuzz encontrou, ainda no desenvolvimento, três defeitos: fatiar string fora de um limite de caractere (nome terminando em ideograma), e cursor além do fim da linha após `:rename` e após `:tableall` (o código C também não reajusta o cursor nesses dois casos).
- **Sem nenhuma nota aberta** (banco vazio ou todas as abas fechadas), teclas como `w`, `G`, `dd` são ignoradas com segurança (no C várias leem a linha atual sem checar se há nota aberta).
- **Colagem sem perda:** colagem multilinha é uma operação única; texto colado nunca é executado como comando; limite de 64 MB.
- **Cache de busca correto após renomear** (no C o título antigo ficava no índice) e `Esc` na busca lateral **não** abre mais uma nota.
- **Seleção sempre visível** (`v`/`V`/`Shift+Setas`): V-Line realça a linha inteira (inclusive vazias e blocos de código), `v` realça o trecho exato e a quebra de linha, e a barra de status mostra quantas linhas/caracteres serão copiados.
- **Quebra de linha por largura real** (ideogramas/emoji = 2 colunas) e blocos de código sem duplicar linhas; recuo e TAB preservados.
- **Detalhes de edição:** `$` vai ao último caractere (como no Vim); `r`, `dd`, `gg`, `za`… são comandos de várias teclas não bloqueantes; busca lateral com edição completa (`←`, `→`, `Home`, `End`, `Ctrl+←/→`); `PageUp`/`PageDown`; tabelas sem limite de 128 linhas (continuam recusando > 32 colunas, sem perder dados); lista lateral e rolagem com a mesma altura.
- **Falhas de gravação nunca perdem dados:** gravação atômica (`tmp` + `fsync` + `rename`), `COMMIT`/`ROLLBACK` verificados, fechar/sair salva tudo antes, e um pânico inesperado dispara salvamento de emergência.

## 🔗 Copiar o link da nota

O botão **`[🔗 Link]`** (barra de botões), o comando **`:link`** (`:copylink`, `:cl`) e a tecla **`y`** na lista lateral copiam o link da nota para a área de transferência — e também para o registrador interno, então `p`/`Ctrl+V` colam na hora:

- nota do banco → `[[Título da Nota]]` (wikilink; `Enter` sobre ele abre a nota);
- arquivo avulso (`notas arquivo.md`) → `[nome](<caminho completo>)`.

Vale a nota **selecionada na lista** (se o foco está nela) ou a da **aba ativa**. A barra de botões escolhe sozinha entre rótulos completos, curtos e só ícones conforme a largura da janela (11 botões sempre visíveis).

## ⚡ Revisão de desempenho e robustez

A revisão começou **medindo** (nota de 20 mil linhas longas, cursor no meio; `cargo test --release bench_ -- --ignored --nocapture`):

| Item | Antes | Depois |
| :-- | --: | --: |
| Quadro 120×30 | 234 ms | ~3 ms |
| Tecla digitada + quadro | 188 ms | ~3 ms |
| `Enter` (ponto de desfazer) | 4 ms | 0,5 ms |
| Memória de 100 níveis de desfazer | 358 MB | 75 MB (teto de 2 mi de linhas) |
| Bytes por tecla no terminal | 34 KB | ~1,3 KB |
| Início com 2.000 notas de 10 KB | 349 ms | ~80–110 ms |

O que foi feito:

- **Posição do cursor sem refazer a quebra de todas as linhas** — o número de linhas visuais de cada linha é memorizado (validado por hash do conteúdo, correto para qualquer forma de edição). Era o gargalo dos 234 ms.
- **Texto das linhas compartilhado (`Rc<str>`)** entre os snapshots de desfazer: copiar o documento custa um contador por linha, não o texto.
- **Redesenho incremental:** só as linhas da tela que mudaram são enviadas (ótimo em SSH); o batimento de 30 s, o redimensionamento e a limpeza repintam tudo. Testado: nenhuma linha alterada é omitida, nenhuma igual é reenviada.
- **Índice de busca:** normalização ASCII por bytes e sem cópia intermediária.
- **Linhas gigantes** (> 20 mil bytes) são tratadas como texto simples (sem varrer links/estilos a cada quadro).
- **Nunca sobrescrever trabalho alheio:** cada aba guarda o `mtime` que carregou; se outra instância (ou uma importação) alterou a nota, `:w`, o autosave e o fechamento **recusam** com aviso. `:w!` sobrescreve de propósito e `:e!` recarrega do banco.
- **Autosave que falha não entra em laço** (antes repetiria a cada 100 ms): tenta de novo em 1 minuto.
- **Fechar a janela do console (Windows)** salva as notas pendentes antes de sair (logoff/desligamento também).

Oportunidades ainda abertas (não feitas, por serem maiores ou não testáveis aqui): indexar a busca em segundo plano para bancos com dezenas de milhares de notas; tratar `SIGTERM`/`SIGHUP` no Linux/macOS (o tratador de encerramento atual é só Windows); reduzir o tamanho de `Line`.

## Arquitetura (`src/`)

| Módulo | Responsabilidade |
| :-- | :-- |
| `main.rs` | entrada, terminal (raw/alternativa/mouse/colagem, restaurado por `Drop`), laço de eventos, salvamento de emergência |
| `app.rs` | estado global, catálogo de notas, abas, histórico, criar/renomear/excluir, importar/exportar |
| `doc.rs` | `Buffer`/`Line`: edição, dobras, seleção, desfazer/refazer (com agrupamento de digitação) |
| `layout.rs` | quebra visual, coluna do cursor, mapeamento clique → byte, `VRowIter` (varredura O(linhas) por quadro) |
| `md.rs` / `table.rs` | links, negrito/itálico, listas; alinhamento de tabelas |
| `input.rs` / `cmd.rs` / `mouse.rs` | teclado por modo/foco, comandos `:`, mouse |
| `edit.rs` | copiar/colar, substituir, tabelas, links, busca no texto |
| `render.rs` | desenho TrueColor de um quadro inteiro, geometria de cliques, cursor |
| `db.rs` / `fsio.rs` | SQLite (`rusqlite`), arquivos, importação/exportação atômicas |
| `notes.rs` / `text.rs` / `theme.rs` | ordem/busca de notas, largura/UTF-8/normalização, 8 temas (gerados do C) |
| `clip.rs` / `links.rs` / `life.rs` | clipboard, abertura de links e scripts, Game of Life |

## Testes

```bash
cargo test                       # ~105 testes (unitarios + integracao, rapidos)
NOTAS_FUZZ_SEEDS=2000 NOTAS_FUZZ_STEPS=5000 cargo test --release fuzz   # campanha longa
cargo test dump_frames -- --ignored    # NOTAS_DUMP_DIR=pasta: grava quadros de varios estados
```

Os testes de integração (`src/tests.rs`) dirigem o `App` inteiro por teclas e mouse com banco em memória, conferem o SQLite e o texto dos quadros, e cobrem: banco vazio, edição/undo/redo, UTF-8, comandos de várias teclas, visual, colagem, busca, criar/renomear/excluir, urgência/filtros, tabelas, wikilinks, `:q`/`:wq`, import/export, nota de 20 mil linhas, mouse (botões, abas, duplo clique, arrasto), execução de script e arquivo avulso. Os testes **nunca tocam no clipboard real**.
