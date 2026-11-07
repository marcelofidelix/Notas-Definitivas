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

O editor Rust oferece: edição modal (Normal, Inserção, Visual caractere/linha, `:` comandos), `y`/`p`/`x`/`dd`/`u`/`Ctrl+R`, seleção por `Shift+Setas` e mouse, abas (`gt`/`gT`, `c`/`C`, clique do meio), lista lateral por urgência (`[1]…[O]`, filtros `0`–`6`, `m`/`!`), busca sem acentos com prévia (`Ctrl+F`), tabelas auto-alinhadas (`Tab`, `gqq`, `:table`, `:tableall`), dobras (`za`, `zM`, `zR`, `zc`, `zo`), links `[t](url)` e `[[wiki]]` (Enter, `gl`, `Ctrl+N`) — **inclusive execução de scripts** (`.bat`, `.cmd`, `.ps1`, `.sh`, `.exe`… com a pasta do script como diretório de trabalho) —, quebra de linha visual, 8 temas TrueColor, `:import`/`:export`, autosave a cada 1 min, histórico (`Backspace`), Game of Life (`:life`, ociosidade de 10 min) e cursor **fixo** (sem piscar) com quadros sincronizados (DEC 2026).

### Melhorias em relação à versão em C

- **Seleção com mouse:** duplo clique no mesmo ponto do texto seleciona a palavra (letras Unicode, números e `_`); triplo clique seleciona a linha inteira do documento, mesmo com quebra visual. Os cliques devem ocorrer com menos de 500 ms entre eles. A seleção entra em Visual caractere/linha e funciona com `y`/`Ctrl+C`, `x`, `p` e `Esc`; espaços e pontuação são selecionados separadamente das palavras.
- **Substituir seleção:** com texto selecionado, pressione `r`, digite o novo texto e confirme com `Enter`. `Esc` cancela mantendo a seleção; texto vazio remove o trecho. A substituição inteira é um passo de desfazer. Sem seleção, `r` continua substituindo um único caractere.

- **Segurança do terminal:** todo texto de nota/nome/prompt passa por `sanitize_for_terminal` — um `.md` com `ESC`/OSC embutidos não consegue escrever no clipboard, mudar o título nem bagunçar a tela.
- **Robustez comprovada por fuzz:** 10 milhões de teclas/cliques/redimensionamentos aleatórios sem pânico e sem violar invariantes (cursor em limite de caractere, índices válidos). O fuzz encontrou, ainda no desenvolvimento, três defeitos: fatiar string fora de um limite de caractere (nome terminando em ideograma), e cursor além do fim da linha após `:rename` e após `:tableall` (o código C também não reajusta o cursor nesses dois casos).
- **Sem nenhuma nota aberta** (banco vazio ou todas as abas fechadas), teclas como `w`, `G`, `dd` são ignoradas com segurança (no C várias leem a linha atual sem checar se há nota aberta).
- **Colagem sem perda:** colagem multilinha é uma operação única; texto colado nunca é executado como comando; limite de 16 MB.
- **Linha inteira cola abaixo, como no Vim:** com registro de linha inteira (`y`, `dd`, Visual Linha), `p` insere as linhas **abaixo** da atual e `P` acima; se o cursor está na última linha, a linha nova é criada no fim. O cursor fica no primeiro caractere da primeira linha colada e o bloco inteiro é um só passo de desfazer. Seleção por caractere (mesmo atravessando linhas) continua sendo colada no cursor.
- **Cache de busca correto após renomear** (no C o título antigo ficava no índice) e `Esc` na busca lateral **não** abre mais uma nota.
- **Seleção sempre visível** (`v`/`V`/`Shift+Setas`): V-Line realça a linha inteira (inclusive vazias e blocos de código), `v` realça o trecho exato e a quebra de linha, e a barra de status mostra quantas linhas/caracteres serão copiados.
- **Quebra de linha por largura real** (ideogramas/emoji = 2 colunas) e blocos de código sem duplicar linhas; recuo e TAB preservados.
- **Detalhes de edição:** `$` vai ao último caractere (como no Vim); `r`, `dd`, `gg`, `za`… são comandos de várias teclas não bloqueantes; busca lateral com edição completa (`←`, `→`, `Home`, `End`, `Ctrl+←/→`); `PageUp`/`PageDown`; tabelas sem limite de 128 linhas (continuam recusando > 32 colunas, sem perder dados); lista lateral e rolagem com a mesma altura.
- **Notas com mais de 9.999 linhas:** a calha do número de linha ganha uma coluna quando o contador passa de quatro dígitos. Antes o número de cinco dígitos empurrava o texto e deixava o cursor (e o clique do mouse) uma casa fora de lugar.
- **Clique abaixo da última linha:** em notas curtas, clicar no espaço vazio abaixo do texto leva o cursor ao **último caractere da última linha** (antes o clique era ignorado).
- Ao fechar abas, o editor salva as alterações pendentes. `:wq` salva todas e sai. `:q!` e `:qa!` descartam o que ainda está pendente. Em caso de pânico ou encerramento pelo sistema, o editor tenta um salvamento de emergência.

## 🖱️ Roda do mouse

No editor, girar a roda do mouse move o **cursor do texto** uma linha visual para baixo/cima, como `j`/`k` — inclusive dentro de linhas quebradas e de células de tabela. A visualização acompanha o cursor, então notas longas podem ser percorridas até o fim; antes a rolagem parava quando o cursor chegava ao topo da tela. Na lista lateral a roda continua rolando a lista e, na prévia de uma nota, rola o texto da prévia sem mexer no cursor da nota.

## ☑ Lista de tarefas

`- [ ] tarefa` aparece como **□ tarefa** (caixa vazia) e `- [x] tarefa` (ou `[X]`, também com `*`) como **▣ tarefa** (caixa marcada, texto esmaecido). A renderização vale nos modos Normal, Visual, Comando e na prévia; em **Inserção** o texto aparece cru (`- [ ] tarefa`) para você editar. O arquivo nunca é alterado pela exibição, então continua Markdown padrão (GitHub, Obsidian…). Para marcar/desmarcar, entre em Inserção (ou use `r` sobre o espaço/`x` dentro dos colchetes).

**Largura de uma célula, de verdade.** Os símbolos "óbvios" ☐ (U+2610) e ☑ (U+2611) **não existem** no Cascadia nem no Consolas: o terminal os pede à fonte *Segoe UI Symbol*, que os desenha ~45 % mais largos que uma célula (a caixa marcada "invadia" o texto). □ (U+25A1) e ▣ (U+25A3) existem nas duas fontes com a largura exata da célula, então foram os escolhidos. Se preferir outros, `NOTAS_TASK_GLYPHS=ab` (dois caracteres de 1 coluna cada: vazio e marcado) os troca. Cursor, cliques e quebra de linha levam em conta que `[ ]` ocupa 1 coluna quando renderizado. Não conta como tarefa: `- [x](url)` (é um link), `- [ ]texto` (sem espaço) e linhas em blocos de código.

## ▦ Tabelas

Duas linhas ou mais começando por `|` formam uma tabela (`|a|b|c|` na primeira linha e `|1|2|3|` na seguinte já bastam; a linha `|---|---|` é opcional).

**Edição (modo Inserção)**

- **Alinhamento automático:** ao sair de uma linha de tabela (setas, clique) ou do Inserção (`Esc`), a tabela é realinhada — só se algo mudar, com o cursor na mesma célula e no mesmo caractere, e como um passo próprio do `u`. `Tab` (alinha e vai à próxima célula, criando linha no fim), `gqq`, `:table` e `:tableall` continuam funcionando.
- **`Enter` numa tabela** cria uma linha nova **já alinhada**, com o cursor na primeira célula (também numa tabela de uma só linha).

**Visualização (fora do modo Inserção)** — o texto da nota não muda, só a exibição:

```
╭───────┬───────┬──────────────────────────────────╮
│ Item  │   Qtd │ Descricao                        │
╞═══════╪═══════╪══════════════════════════════════╡
│ Cafe  │    10 │ Esta e uma descricao bem longa   │
│       │       │ que precisa quebrar em varias    │
│       │       │ linhas dentro da celula para     │
│       │       │ caber na janela.                 │
├───────┼───────┼──────────────────────────────────┤
│ Pao   │     2 │ site e [Nota]                    │
╰───────┴───────┴──────────────────────────────────╯
```

- **Quebra de linha dentro da célula:** as colunas ocupam a largura natural; se a tabela não cabe na janela, as mais largas encolhem (preservando palavras inteiras sempre que possível) e o texto quebra em várias linhas. Quando alguma célula quebra, filetes separam as linhas para a leitura ficar clara. Palavras maiores que a coluna são cortadas; nada se perde.
- **Cabeçalho** em negrito e cor de destaque, com filete duplo, quando a segunda linha é `|---|---|`.
- **Alinhamento** pela linha delimitadora (`:--` esquerda, `:-:` centro, `--:` direita); colunas só de números ficam à direita sozinhas.
- **Marcação dentro das células:** `**negrito**`, `*itálico*`, `[link](url)` e `[[wikilink]]` são exibidos como no resto da nota (sem os delimitadores).
- **Cursor e mouse** seguem as células: `j`/`k` percorrem o texto quebrado da célula e depois passam para a linha seguinte/anterior mantendo a coluna; um clique cai no texto sob o ponteiro; seleção `v`/`V` realça normalmente.
- **Ordenar coluna (botão direito):** clique com o botão direito sobre uma célula da tabela — ou selecione o texto de uma coluna e clique com o direito — para abrir o menu **`[1] A→Z`** / **`[2] Z→A`**. A ordenação usa a coluna clicada, mantém o cabeçalho e a linha delimitadora no lugar, ignora maiúsculas e acentos, põe células vazias no fim e é um único passo do `u`. No teclado: `↓`/`Tab` alterna a opção, `1`/`2` aplicam direto, `Enter` confirma e `Esc` cancela.
- Linhas alternadas ganham um fundo suave; a linha do cursor é realçada.
- Se a janela é tão estreita que nem a largura mínima de cada coluna cabe, ou com `:nowrap`, a tabela volta ao texto cru (cortado na borda) em vez de ficar ilegível. Tabelas com mais de 32 colunas também.
## 🔢 Prefixo numérico (como no Vim)

No modo Normal (e no Visual), um número antes do comando o repete: `100l` anda 100 casas à direita, `5j` desce 5 linhas, `3w` avança 3 palavras. O número aparece na barra de status enquanto você digita; `Esc` (ou um clique) o cancela. O `0` sozinho continua indo ao início da linha; depois de outro dígito (`10l`) ele faz parte do número.

| Comando | Com prefixo `N` |
| :-- | :-- |
| `h` `l` `j` `k` `w` `b`, setas, `PageUp/PageDown` | repete `N` vezes (para ao esbarrar no fim da linha/nota — `j`/`k` andam por linhas da tela) |
| `G` / `gg` | vai para a linha `N` (`5G`, `2gg`); além do fim, a última |
| `$` | desce `N-1` linhas e vai ao fim |
| `x` | recorta `N` caracteres (um só passo de desfazer) |
| `dd`, `d3d`, `3d2d` | apaga `N` linhas; os dois números se multiplicam; um só passo de desfazer |
| `y` | copia `N` linhas |
| `p` | cola `N` cópias do registro (um só passo); linha inteira desce abaixo da atual (`P` sobe) |
| `u` / `Ctrl+R` | desfaz/refaz `N` passos |
| `n` / `N` | `N`-ésima ocorrência da busca, para frente / para trás |
| `rx` | `3rx` troca `N` caracteres (se a linha não tiver `N`, nada acontece) |
| `i` `a` `I` `A` `o` `O` | `3ia<Esc>` repete o texto digitado `N` vezes ao sair da Inserção (`3ox<Esc>` cria 3 linhas); se você digitou mais de uma linha, não repete |
| `gt` / `gT` | `2gt` vai para a aba 2; `3gT` volta 3 abas |

Os demais comandos ignoram o prefixo. Números enormes são seguros (limite de 1.000.000; toda repetição para quando deixa de ter efeito, e `3p`/`5ix` produzem no máximo 16 MB). Na lista de notas os dígitos continuam sendo os filtros de urgência (`0`–`6`).

## 🔎 Busca dentro da nota (`/`, `n`, `N`)

`/texto` (ou o botão **[Localizar]**) procura no texto da nota **ignorando maiúsculas**: leva o cursor ao início da ocorrência e **realça todas as ocorrências da nota** de uma vez. A ocorrência onde o cursor parou tem cor própria (texto escuro em negrito sobre o fundo de destaque), então dá para ver onde você está e quantas faltam.

- **`n`** vai para a próxima ocorrência; **`N`** volta para a anterior. As duas dão a volta no fim/início da nota e aceitam prefixo (`3n`, `2N`).
- A barra de status mostra **`Ocorrência 3/17 na linha 42`** — posição e total.
- O realce vale no texto, nas **tabelas desenhadas** (dentro das células) e nos **blocos de código**; em linhas de código o realce vem sem a colorização de sintaxe, para o destaque ficar exato.
- **`:noh`** (`:nohlsearch`) desliga o realce e **mantém o padrão**: `n`/`N` continuam navegando. Uma busca nova religa o realce.
- **`Esc` encerra a busca da nota:** o realce some e `n`/`N` param de navegar (equivale a apagar o padrão). Para apenas esconder o realce mantendo `n`/`N`, use `:noh`.
- A ocorrência pode estar em qualquer coluna da linha (antes, `n` parava no início da linha). A comparação é por caixa, não por acentos: `/acao` não acha `ação` — para isso existe a busca lateral (`Ctrl+F`), que ignora acentos.
## 🔗 Copiar o link da nota

O botão **`[🔗 Link]`** (barra de botões), o comando **`:link`** (`:copylink`, `:cl`) e a tecla **`y`** na lista lateral copiam o link da nota para a área de transferência — e também para o registrador interno, então `p`/`Ctrl+V` colam na hora:

- nota do banco → `[[Título da Nota]]` (wikilink; `Enter` sobre ele abre a nota);
- arquivo avulso (`notas arquivo.md`) → `[nome](<caminho completo>)`.

Vale a nota **selecionada na lista** (se o foco está nela) ou a da **aba ativa**. A barra de botões escolhe sozinha entre rótulos completos, curtos e só ícones conforme a largura da janela (11 botões sempre visíveis).

## ⚡ Revisão de desempenho e robustez

A revisão começou **medindo** (nota de 20 mil linhas longas, cursor na linha 10.000; `cargo test --release bench_ -- --ignored --nocapture`). Números atuais, medidos no Windows/MSVC:

| Item | Antes da 1ª revisão | Depois | Agora |
| :-- | --: | --: | --: |
| Quadro 120×30 | 234 ms | ~3 ms | **0,33 ms** |
| Tecla digitada + quadro | 188 ms | ~3 ms | **0,35 ms** |
| `Enter` (ponto de desfazer) | 4 ms | 0,5 ms | **0,30 ms** |
| Memória de 100 níveis de desfazer | 358 MB | **~1 MB** | ~949 KB |
| Bytes por tecla no terminal | 34 KB | ~1,3 KB | ~1,3 KB |
| Início com 2.000 notas de 10 KB | 349 ms | ~80–110 ms | **3 ms** (+23 ms na primeira busca) |

O que foi feito:

- **Posição do cursor sem refazer a quebra de todas as linhas** — o número de linhas visuais de cada linha é memorizado. Essa memória é descartada quando o texto ou o bloco de tabela da linha mudam, em vez de validada por um hash recalculado a cada consulta (era o custo dominante do quadro).
- **Índice de busca no banco (`content_norm`)**: o texto normalizado de cada nota é gravado junto com o conteúdo e carregado **só na primeira busca**. Abrir o programa com 2.000 notas de 10 KB caiu de 63–95 ms para 3 ms; a primeira busca paga ~23 ms uma vez.
- **Texto das linhas compartilhado (`Rc<str>`)** entre os estados de desfazer: copiar o documento custa um contador por linha, não o texto.
- **Histórico de desfazer por diferenças (`history.rs`):** em cada pilha (desfazer e refazer) só o estado mais recente é completo; os anteriores guardam apenas o trecho que mudou (linhas iguais no início e no fim são reconhecidas por ponteiro, sem comparar texto) e são reconstruídos ao desfazer. Uma nota de 20 mil linhas com 150 passos passou de 91 MB para ~1 MB, e o ponto de desfazer ficou mais barato. Verificado contra um modelo de cópias completas em edições aleatórias.
- **Redesenho incremental:** só as linhas da tela que mudaram são enviadas (ótimo em SSH); o batimento de 30 s, o redimensionamento e a limpeza repintam tudo. Testado: nenhuma linha alterada é omitida, nenhuma igual é reenviada.
- **Linhas gigantes** (> 20 mil bytes) são tratadas como texto simples (sem varrer links/estilos a cada quadro).
- No SQLite, salvar e renomear conferem a versão carregada e gravam na mesma transação. Um conflito mantém a edição na aba. Em arquivos avulsos, o editor compara o conteúdo no disco imediatamente antes da substituição. Se o arquivo estava por criar, a gravação exige que o destino continue livre. Use `:e!` para recarregar ou `:w!` para sobrescrever deliberadamente.
- A importação de uma pasta é integral. Uma falha de leitura ou gravação em um arquivo elegível desfaz o lote e mostra o caminho e o motivo.
- **Autosave que falha não entra em laço** (antes repetiria a cada 100 ms): tenta de novo em 1 minuto.
- **Fechar a janela do console (Windows)** salva as notas pendentes antes de sair (logoff/desligamento também). No Linux/macOS, `SIGTERM`, `SIGHUP` (queda de SSH) e `SIGQUIT` levantam a mesma flag de encerramento.
- **Pânico deixa rastro:** `notas-crash.log`, ao lado do banco, guarda data, local e mensagem antes de a tela ser restaurada (antes a mensagem do Rust era apagada junto com a tela alternativa).
- **Degradação não é silenciosa:** WAL indisponível, pasta do executável inacessível e exportação parcial aparecem no status inicial.
- **Exportação nunca perde nota:** nomes diferentes que higienizam para o mesmo arquivo (`a:b` e `a*b`) recebem sufixo em vez de se sobrescreverem.
- **Menos desperdícios:** o laço de eventos espera até o próximo prazo interno (letreiro, status, autosave, batimento) em vez de acordar a cada 100 ms; o clipboard reaproveita o handle de leitura e tenta 3 vezes em vez de 10 (congelamento máximo de 50 ms para 15 ms); a colagem tem teto de 16 MB (era 64 MB, que podia custar ~200 MB de pico); busca de texto sem acento não aloca por linha.

Oportunidades ainda abertas (maiores, não medidas como problema hoje): custo do quadro ainda cresce com o número de linhas até o cursor (duas varreduras por quadro — invisível até ~50 mil linhas, resolveria com índice de prefixo); indexar a busca em segundo plano para bancos com dezenas de milhares de notas; teste automatizado do terminal real (pty).

## Arquitetura (`src/`)

| Módulo | Responsabilidade |
| :-- | :-- |
| `main.rs` | entrada, terminal (raw/alternativa/mouse/colagem, restaurado por `Drop`), laço de eventos, salvamento de emergência |
| `app.rs` | estado global, catálogo de notas, abas, histórico, criar/renomear/excluir, importar/exportar |
| `doc.rs` | `Buffer`/`Line`: edição, dobras, seleção, desfazer/refazer (com agrupamento de digitação) |
| `history.rs` | pilhas de desfazer/refazer por diferenças (só o último estado é completo) |
| `layout.rs` | quebra visual, coluna do cursor, mapeamento clique → byte, `VRowIter` (varredura O(linhas) por quadro) |
| `md.rs` / `table.rs` | links, negrito/itálico, listas, tarefas; alinhamento de tabelas (texto) |
| `tbl.rs` | tabelas desenhadas: análise das células, larguras, quebra por célula, cursor/clique ↔ byte |
| `input.rs` / `cmd.rs` / `mouse.rs` | teclado por modo/foco, comandos `:`, mouse |
| `count.rs` | prefixo numérico do Vim (`100l`, `3dd`, `5G`...): repetição segura de comandos |
| `edit.rs` | copiar/colar, substituir, tabelas, links, busca no texto |
| `render.rs` | desenho TrueColor de um quadro inteiro, geometria de cliques, cursor |
| `db.rs` / `fsio.rs` | SQLite (`rusqlite`), arquivos, importação/exportação atômicas |
| `notes.rs` / `text.rs` / `theme.rs` | ordem/busca de notas, largura/UTF-8/normalização, 8 temas (gerados do C) |
| `clip.rs` / `links.rs` / `life.rs` | clipboard, abertura de links e scripts, Game of Life |

## Testes

```bash
cargo test                       # suíte padrão (unitários, integração e fuzz)
NOTAS_FUZZ_SEEDS=2000 NOTAS_FUZZ_STEPS=5000 cargo test --release fuzz   # campanha longa
cargo test dump_frames -- --ignored    # NOTAS_DUMP_DIR=pasta: grava quadros de varios estados
```

Os testes de integração (`src/tests.rs`) dirigem o `App` inteiro por teclas e mouse com banco em memória, conferem o SQLite e o texto dos quadros, e cobrem: banco vazio, edição/undo/redo, UTF-8, comandos de várias teclas, visual, colagem, busca, criar/renomear/excluir, urgência/filtros, tabelas, wikilinks, `:q`/`:wq`, import/export, nota de 20 mil linhas, mouse (botões, abas, duplo clique, arrasto), execução de script e arquivo avulso. Os testes **nunca tocam no clipboard real**.
