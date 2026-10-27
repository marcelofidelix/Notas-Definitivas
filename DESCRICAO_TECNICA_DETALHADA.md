# Arquitetura e Manual Técnico — Notas Definitivas (pós-v6.33.0)
> **Documento de Engenharia de Software e Especificação Técnica Integral**  
> **Base descrita**: `notas.c` — **11.237 linhas** de C99 em arquivo único, estado de trabalho posterior à v6.33.0
> **Dependência externa**: `libsqlite3` (obrigatória desde a v6.29.0)  
> **Escopo**: descreve a base de código atual, o que mudou nos ciclos **v6.28.0**, **v6.29.0**, **v6.30.0** e **v6.33.0**, e a rodada de **robustez/desempenho** documentada na seção 9.

---

## 1. Visão Geral e Filosofia de Design

O **Notas Definitivas** é um editor pessoal de notas em Markdown voltado para terminais de texto (TUI), construído em **C99 puro como uma unidade de tradução única (*Single Translation Unit*)**, sem dependência de `ncurses`, `glib` ou de bibliotecas de interface de terceiros. A única biblioteca além da C padrão e das APIs nativas do sistema operacional é o **SQLite**, que desde a v6.29.0 é o motor de armazenamento das notas.

### 1.1 Princípios Fundamentais da Arquitetura

1. **Armazenamento em banco SQLite (`notas.db`)**: todas as notas vivem em uma única tabela do banco, criado ao lado do executável (o programa faz `chdir()` para o diretório do binário). O banco nasce **vazio** — nenhuma nota de exemplo. O diretório `data/` deixou de ser armazenamento e passou a ser apenas **origem e destino** das rotinas de importação e exportação. O nível de urgência (`[1]`/`[2]`/`[3]`) vive na coluna `attention` da tabela, não mais no arquivo `data/.attention`.
2. **Unidade Única de Tradução (Single-File C Architecture)**: todo o código-fonte executável reside em `notas.c`, compilável diretamente com:
   - **Linux**: `gcc -O2 -Wall -Wextra notas.c -o notas -lsqlite3 -ldl`
   - **Windows**: `gcc -O2 -Wall -Wextra notas.c -o notas.exe -lsqlite3 -lkernel32 -luser32 -lshell32`
   - Os scripts `build.sh` / `build.bat` encapsulam esses comandos (`all`, `linux`, `windows`, `run`, `clean`).
3. **Renderização ANSI TrueColor (24 bits) sem Ncurses**: emissão de sequências de 24 bits (`\x1b[38;2;R;G;Bm` e `\x1b[48;2;R;G;Bm`) compostas em um buffer dinâmico de memória (`ABuffer`) e despejadas no terminal em **uma única escrita atômica**, eliminando cintilação.
4. **Redesenho por *dirty-flag* (v6.28.0)**: o quadro só é reconstruído quando existe mudança real de estado ou algo animado na tela. Quem decide é `should_refresh_now()`, consultada a cada iteração do loop principal — o modelo antigo, de redesenho incondicional a cada 150 ms, foi removido.
5. **Paridade Multiplataforma Total (POSIX Linux vs Windows Win32)**: no Linux, `termios`, `SIGWINCH` e multiplexação de descritores; no Windows, o Virtual Terminal do Windows 10/11 (`ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT`), com suporte a nomes Unicode via APIs Wide (`FindFirstFileW`, `DeleteFileW`, `MoveFileExW`, `_wfopen`).
6. **Ergonomia Híbrida (Vim + Produtividade Moderna)**: modelo modal completo, abas com marquee de título longo, painel lateral com busca difusa em tempo real (`Ctrl+F`) e prévia, suporte total a mouse (SGR 1006), autosave e 8 temas embutidos.
7. **Ordem da lista = prioridade, depois data (v6.30.0)**: a lista lateral é exibida em grupos de urgência (nível 1, 2, 3 e por último as notas sem marcação) e, **dentro de cada grupo**, a nota modificada mais recentemente primeiro, com desempate por nome.

### 1.2 O que mudou em relação à v6.27.0

| Versão | Mudança estrutural |
| :--- | :--- |
| **v6.28.0** | *Dirty-flag* de renderização (`should_refresh_now()`, `g_needs_redraw`, marquee a 300 ms, expiração da mensagem de status em 5 s e batimento de manutenção de 30 s) e scripts de build portáveis (`build.sh`/`build.bat`). |
| **v6.29.0** | Backend **SQLite**: as notas saíram de `data/*.md` para a tabela `notes` de `notas.db`; identidade de nota passou a ser o caminho virtual `"db:<id>"`; entraram a camada de acesso ao banco, as rotinas de importação/exportação e os botões `[Importar]`/`[Exportar]` com os modos `MODE_PROMPT_IMPORT_DIR`/`MODE_PROMPT_EXPORT_DIR`. |
| **v6.30.0** | Lista lateral ordenada por **prioridade e depois data** (`compare_notes_priority()`, `notes_resort_keep()`); filtro de urgência efetivamente aplicado em `apply_note_search()`; correção do clique dos filtros `[2]`/`[3]` com painel estreito; rótulos compactos nos controles que não cabem. |
| **v6.33.0** | Robustez, UTF-8 e CLI: exclusão atômica de caracteres multibyte UTF-8 (`line_delete_char`) e inserção atômica; tolerância a BOM UTF-8 (`\xEF\xBB\xBF`); criação de pastas com suporte a Unicode no Windows (`mkdir_utf8` / `CreateDirectoryW`); transações em lote SQLite na importação (`BEGIN TRANSACTION`); novos comandos `:import [dir]` e `:export [dir]`; proteção contra reescrita na exclusão de notas abertas em abas; resolução do título real em prévias virtuais `db:<id>`; correção na renomeação de notas do banco em abas inativas. |
| **Pós-v6.33.0** | Rodada de endurecimento e refatoração limpa (seção 9): escrita atômica de arquivos, validação estrita de IDs, transações de importação com COMMIT/ROLLBACK verificados, correção do Base64/OSC 52, eliminação de estouro de pilha em tabelas, renderização incremental (`RenderCursor`), undo coalescido para digitação, timers de 64 bits, saída de terminal com escrita completa, desduplicação e sincronização total dos prompts modais (`prompt_label`), correção do clique de mouse em importação/exportação, correção de `:s/` e `:sub/` com campos longos, endurecimento de buffers de escape (`cpos[64]`), `safe_strcat()`, `strtol` para `LINES`/`COLUMNS`, zero avisos em compilação estrita e suíte de regressão de edição (`editor_test.c`). |

Consequências diretas para quem lê o documento antigo: não existe mais armazenamento em `data/`, não existe mais `data/.attention`, a ordenação não é apenas por data de modificação, e o pipeline de renderização não é mais acionado por tempo fixo. Este documento substitui aquele — e, para não envelhecer de novo, **não carrega mais a versão no nome do arquivo**.

---

## 2. Funcionalidades de Destaque

### 2.1 Renderização de Bullet Lists (`*` e `-`) em Cores e Negrito (v6.27.0)
- **Identificação Precisa**: `is_bullet_list_line()` analisa a linha, ignora recuos (espaços e tabulações) e detecta se o primeiro caractere visível é hífen (`-`) ou asterisco (`*`) seguido de espaço ou início de checklist (`- [ ]`, `* [x]`).
- **Destaque Visual**: o marcador `-` é renderizado em negrito com `TH_FG_YELLOW`; o marcador `*` em negrito com `TH_FG_ACCENT`. O texto subsequente permanece na cor de leitura padrão (`TH_FG_MAIN`).
- **Filtro de Falsos Positivos**: linhas horizontais divisórias (`---`, `***`, `- - -`), marcadores inline de formatação (`*texto*`, `**texto**`) e parâmetros de comando iniciados por traço são deliberadamente desconsiderados.

### 2.2 Cópia e Colagem Bidirecional com o Sistema (v6.27.0, com threads assíncronas)
- **Cópia não bloqueante**: `y`, `yy`, `Y`, seleção no Modo Visual e seleção no Modo Inserção (mouse/`Shift+Setas`) escrevem no clipboard em uma **thread destacada** — `CreateThread` no Windows, `pthread` (detachável) no POSIX — de modo que o loop da interface retorna imediatamente e a mensagem de confirmação aparece em 0 ms.
- **OSC 52 + clipboard nativo**: antes da thread, `copy_to_system_clipboard()` emite a sequência OSC 52 com o texto em **Base64** (`base64_encode()`), que o terminal intercepta (Linux, WSL, SSH, tmux). No Windows local a sequência é omitida, porque o texto já vai por `CF_UNICODETEXT` e o OSC 52 geraria disputa de lock com o Windows Terminal.
- **Alvos do POSIX**: host Windows (WSL/Git Bash) usa `clip.exe`; Wayland usa `wl-copy`; X11 usa `xclip`/`xsel`.
- **Colagem**: `p`/`P` no Modo Normal, `Ctrl+V` no Modo Inserção e `p` no Modo Visual (substituindo a seleção). No Windows, `CF_UNICODETEXT` é convertido para UTF-8, preservando acentos (`ç`, `ã`, `é`, `ó`, `ú`) e emojis. No POSIX, a leitura usa `wl-paste`, `xclip`/`xsel` ou `powershell Get-Clipboard`.

### 2.3 Backend SQLite, Importação e Exportação de Markdown (v6.29.0)
- **Uma tabela, uma fonte de verdade**: `notes(id, name, content, mtime, attention)`, com `name` `UNIQUE`, mais o índice `idx_notes_mtime` e `PRAGMA user_version=1` (ver seção 3.8).
- **WAL**: `journal_mode=WAL` (com `synchronous=NORMAL` e `busy_timeout` de 3 s) permite leitura e escrita concorrentes sem travar a interface.
- **Banco nasce vazio**: na primeira execução nenhuma nota de exemplo é criada; a interface exibe `[New Note]`/instruções e o rodapé aponta para `[Nova]` ou `[Importar]`.
- **`[⬇ Importar]` — pasta de `.md` ➔ banco**: clique no botão, informe a pasta e pressione `Enter`. Aceita `.md`, `.markdown`, `.mdown`, `.txt` e `.text` (ignora o resto, como `.pdf`); reconhece acentos e emojis (UTF-8 preservado byte a byte); **reimportar a mesma pasta atualiza** as notas existentes em vez de duplicar (`UNIQUE` pelo nome, e a marcação de urgência fica intocada); caminhos com `~` são expandidos e aspas coladas são removidas.
- **`[⬆ Exportar]` — banco ➔ pasta de `.md`**: exporta tudo como `<nome>.md` na pasta informada (criada se não existir); o **texto** da nota faz a ida e volta sem alteração (escrito byte a byte como está no banco); nomes de nota são higienizados ao virar arquivo (`/ \ : * ? " < > |` viram `_`).
- **Prompts com edição completa**: os dois campos aceitam setas, `Home`/`End`, `Ctrl+A`/`Ctrl+E`, `Ctrl+←/→`, `Backspace`, `Delete`, `Ctrl+U` e `Ctrl+K`; `Esc` cancela.
- **⚠️ Limitação conhecida**: as marcações de urgência **não viajam** nos arquivos `.md` exportados (é markdown puro, sem metadados). Exportar e reimportar em um banco **novo** devolve todas as notas sem marcação; reimportar sobre notas existentes preserva a marcação. Para não perder nada, o caminho seguro é copiar o próprio `notas.db`.

### 2.4 Redesenho por *Dirty-Flag* (v6.28.0)
- **Problema**: o loop principal redesenhava a tela inteira a cada 150 ms (o timeout da leitura de tecla), mesmo sem nenhuma mudança de estado — cerca de **100 KB/s de tráfego ANSI** só para ficar parado, com o cursor e a barra de status piscando de forma sutil em terminais lentos.
- **Solução**: `should_refresh_now()` é consultada uma vez por iteração, **antes** de bloquear na leitura de teclado, e só autoriza o desenho em cinco casos: (1) `g_needs_redraw` ligado, porque algo mudou o estado visual; (2) janela redimensionada (`g_window_resized`) ou tela pedindo limpeza (`g_screen_clear_needed`); (3) marquee de título longo ativo **e** com o passo de 300 ms avançado; (4) a mensagem de status acabou de expirar (5 s) e o rodapé precisa voltar à dica de atalhos; (5) venceu o batimento de manutenção de 30 s, que mantém o relógio relativo da barra lateral (`agora`, `5m`, `2h`) atualizado mesmo ocioso.
- **Quem liga a flag**: qualquer rotina que altere o estado visual chama `g_needs_redraw = 1` (via `set_status_msg()`, digitação, troca de tema/aba, filtros etc.). `refresh_screen()` é o único ponto que **baixa** a flag e reinicia o batimento.
- **Regra de ouro**: todo efeito novo que dependa de tempo (animações, contadores, expirações) **precisa** entrar como condição em `should_refresh_now()`, senão a tela congela até a próxima tecla. `IDLE_HEARTBEAT_MS` pode ser sobrescrito por `-D` para testes de batimento rápido.

### 2.5 Ordem por Prioridade, Rótulos [H], [R] e [O] e Filtros Responsivos (v6.32.0)
- **Ordem da lista**: grupos na sequência **nível 1** (crítico/vermelho) ➔ **nível 2** (médio/laranja) ➔ **nível 3** (baixo/amarelo) ➔ **nível 4 - [H]** (azul/ciano) ➔ **nível 5 - [R]** (roxo/violeta) ➔ **nível 6 - [O]** (verde) ➔ **sem marcação**; dentro do grupo, a mais recente primeiro; desempate por nome de arquivo.
- **Alternância de marcação com agrupamento no topo**: `m` (na lista ou no editor) ou clique do mouse na caixa da nota cicla a criticidade (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[H]` ➔ `[R]` ➔ `[O]` ➔ `[ ]`) e grava atomicamente no SQLite (`db_set_attention`), reorganizando a lista por prioridade para agrupar as notas no topo da lista lateral e mantendo a seleção sobre a nota para evitar aberturas involuntárias.
- **Rótulos responsivos dos filtros**: os botões de filtro na barra inferior adaptam-se dinamicamente à largura do painel:
  - $\ge$ 32 colunas: rótulos completos espaçados (` [Todas] [1] [2] [3] [H] [R] [O]`).
  - 28 a 31 colunas: `[Todas]` abreviado para `[T]` espaçado (` [T] [1] [2] [3] [H] [R] [O]`).
  - 21 a 27 colunas: modo ultracompacto colado (`[T][1][2][3][H][R][O]`).
  Os limites de clique de cada botão são calculados no desenho e tratados independentemente da largura da lista.

---

## 3. Arquitetura de Dados e Estruturas Fundamentais

Todas as estruturas de dados estão declaradas no topo de `notas.c`:

### 3.1 `Theme` — Paleta TrueColor (24 bits)
Define a identidade cromática completa dos **8 temas** embutidos (`NUM_THEMES`): Tokyo Night, One Dark Pro, Dracula, Monokai, GitHub Dark, GitHub Light, VS Code Light+, Solarized Light:
```c
typedef struct {
    const char *name;            /* Nome do tema para exibição na UI */
    bool is_dark;                /* Flag: true para escuro, false para claro */
    const char *bg_main;         /* Cor de fundo do editor principal */
    const char *bg_sidebar;      /* Cor de fundo do painel lateral esquerdo */
    const char *bg_status;       /* Cor de fundo da barra de status inferior */
    const char *bg_select;       /* Cor de fundo de seleção no modo visual / botoes */
    const char *bg_cur_line;     /* Cor de realce da linha atual do cursor */
    const char *bg_tab_active;   /* Cor de fundo da aba atualmente aberta */
    const char *bg_tab_inactive; /* Cor de fundo de abas em segundo plano */
    const char *bg_search;       /* Cor de realce de termos na busca */
    const char *bg_badge_nor;    /* Fundo do badge do modo NORMAL */
    const char *bg_badge_ins;    /* Fundo do badge do modo INSERT */
    const char *bg_badge_vis;    /* Fundo do badge do modo VISUAL */
    const char *bg_badge_cmd;    /* Fundo do badge do modo COMMAND */
    const char *fg_main;         /* Cor do texto principal */
    const char *fg_muted;        /* Cor de texto secundário / esmaecido */
    const char *fg_border;       /* Cor da divisória vertical entre painéis */
    const char *fg_h1;           /* Cor para títulos H1 (#) */
    const char *fg_h2;           /* Cor para títulos H2 (##) */
    const char *fg_h3;           /* Cor para títulos H3 (###) */
    const char *fg_link;         /* Cor para links Markdown [texto](url) */
    const char *fg_wiki;         /* Cor para wikilinks [[nota]] */
    const char *fg_date;         /* Cor para timestamps e metadados */
    const char *fg_dark;         /* Cor escura auxiliar */
    const char *fg_white;        /* Cor branca de alto contraste */
    const char *fg_warn;         /* Cor de alerta/erro */
    const char *fg_yellow;       /* Cor amarela para marcadores de lista '-' */
    const char *fg_accent;       /* Cor de acento para marcadores de lista '*' */
} Theme;
```
A paleta é acessada por macros (`TH_FG_MAIN`, `TH_BG_STATUS`, `TH_FG_YELLOW`, `TH_FG_ACCENT`, ...) que resolvem para o tema corrente em `themes[E.current_theme]`. O ciclo de temas é `(E.current_theme + 1) % NUM_THEMES`.

### 3.2 `ABuffer` — Buffer Dinâmico de Renderização
Acumula em memória todas as sequências de escape e caracteres antes da escrita atômica no terminal:
```c
typedef struct {
    char *b;  /* Bloco de bytes alocado dinamicamente */
    int len;  /* Quantidade de bytes utilizados */
    int cap;  /* Capacidade máxima alocada */
} ABuffer;
```
`ab_init()` zera os campos; a primeira alocação real acontece em `ab_append()`, que começa em 1024 bytes e dobra conforme a necessidade (com `realloc`).

### 3.3 `Line` — Linha do Documento Markdown
```c
typedef struct {
    char *chars;        /* String UTF-8 alocada dinamicamente (com '\0') */
    int len;            /* Tamanho da linha em bytes */
    int heading_lvl;    /* 1 a 6 se iniciar com '#', caso contrário 0 */
    bool folded;        /* true se este cabeçalho estiver colapsado (dobrado) */
    bool hidden;        /* true se a linha estiver oculta dentro de uma dobra */
    bool is_code_fence; /* true se a linha for um delimitador de código (```) */
    bool in_code_block; /* true se a linha estiver no interior de um bloco de código */
} Line;
```
`line_init(l, s, len)` copia exatamente `len` bytes (não usa `strlen`), o que permite apontar para dentro de um buffer maior sem copiar a linha.

### 3.4 `UndoState` — Histórico de Desfazer e Refazer
Snapshots completos das linhas do documento e da posição do cursor (até `MAX_UNDO` = 100 níveis por aba):
```c
typedef struct {
    Line *lines;   /* Cópia em profundidade das linhas */
    int num_lines; /* Quantidade de linhas */
    int cursor_y;  /* Posição vertical do cursor */
    int cursor_x;  /* Posição horizontal do cursor */
} UndoState;
```
Cada nível continua sendo um snapshot completo das linhas. A diferença da rodada atual é a **coalescência de digitação**: `save_undo_for_typing()` reaproveita o snapshot quando teclas de caractere consecutivas ocorrem na mesma nota/linha/posição esperada dentro de `TYPING_UNDO_COALESCE_MS` (800 ms). Enter, Backspace, Delete, colar, tabela, troca de aba, undo/redo e demais edições chamam `save_undo()` e encerram o agrupamento. Assim, digitar uma frase consome um nível de undo, não um por tecla.

### 3.5 `Tab` — Estado Isolado por Aba
Até `MAX_TABS` = 32 abas simultâneas:
```c
typedef struct {
    char cur_filename[256];        /* Nome de exibição (ex: "Projeto.md") */
    char cur_filepath[512];        /* Chave da nota: "db:<id>" (ou caminho real, no modo legado) */
    char title[256];               /* Título extraído do cabeçalho ou do nome */
    Line *lines;                   /* Array de linhas do documento */
    int num_lines;                 /* Quantidade de linhas ativas */
    int cap_lines;                 /* Capacidade alocada do array */
    bool modified;                 /* Flag de modificação não salva */
    int64_t first_unsaved_time;    /* Timestamp monotônico (ms) da primeira alteração não salva */
    int cursor_y, cursor_x;        /* Coordenadas do cursor no texto */
    int scroll_y, scroll_x;        /* Deslocamentos de rolagem */
    int visual_start_y, visual_start_x; /* Início de seleção visual */
    bool has_selection;            /* Se há seleção ativa */
    UndoState undo_stack[MAX_UNDO];/* Pilha de desfazer da aba */
    int undo_count;
    UndoState redo_stack[MAX_UNDO];/* Pilha de refazer da aba */
    int redo_count;
} Tab;
```
O campo `cur_filepath` **continua sendo a identidade da nota** em toda a interface (abas, undo, prévia, histórico, busca, links) — só que desde a v6.29.0 ele normalmente carrega um caminho virtual `"db:<id>"`, e não mais `data/<nome>.md`.

### 3.6 `NoteInfo` — Catálogo de Notas
Índice em memória das notas existentes (usado pela lista lateral, pela busca e pelos filtros):
```c
typedef struct {
    char filename[512];      /* Nome de exibição: "<nome no banco>.md" */
    char filepath[512];      /* Chave da nota: "db:<id>" (ou caminho real, no modo legado) */
    char title[256];         /* Título limpo ou extraído do cabeçalho # */
    time_t mtime;            /* Data de modificação (coluna mtime do banco) */
    char rel_time[32];       /* Texto formatado (ex: "10m atrás", "09/10/26") */
    char *search_norm;       /* Cache textual normalizado (minúsculas, sem acentos) */
    int search_norm_len;     /* Tamanho do cache de busca */
    int attention_level;     /* Urgência: 0 = nenhuma, 1 = Crítica, 2 = Média, 3 = Baixa, 4 = [H], 5 = [R], 6 = [O] */
    bool needs_attention;    /* true se attention_level > 0 */
} NoteInfo;
```
Não há campo de id: o id vem embutido no próprio `filepath` (`note_id_from_path()`), o que mantém toda a lógica de comparação de notas intacta desde antes do banco.

### 3.7 `Editor` — O Estado Global `E`
Encapsula o estado completo da aplicação durante o ciclo de execução:
- **Janela e Layout**: `screen_rows`, `screen_cols`, `left_width` (largura ajustável do painel esquerdo, mínimo `MIN_LEFT_WIDTH` = 18).
- **Foco e Modos**: `focus` (`FOCUS_LEFT`/`FOCUS_RIGHT`), `mode` (ver 4.1), `wrap_mode` (quebra de linha visual).
- **Abas**: `tabs`, `num_tabs`, `cap_tabs`, `active_tab` e `tab_coords[]` (geometria das abas na tela, para o clique do mouse).
- **Espelho do buffer ativo**: `cur_filename`, `cur_filepath`, `lines`, `num_lines`, `cursor_*`, `scroll_*`, `visual_start_*`, `has_selection`, pilhas de undo/redo — sincronizados com `tabs[active_tab]` por `tab_save_active()`/`tab_load()`.
- **Tema**: `current_theme`, `theme_btn_start_col`, `theme_btn_end_col`.
- **Barra de status e botões** (`_start`/`_end` de cada um): `btn_new`, `btn_rename`, `btn_delete`, `btn_wrap`, `btn_find`, `btn_replace`, `btn_life`, **`btn_import`** (`[Importar]`) e **`btn_export`** (`[Exportar]`), mais `status_bar_lines` e `force_status_bar_wrap` (0 = automático, 1 = sempre compacto, 2 = nunca).
- **Filtros de urgência**: `filter_urgency` (0 = todas, 1, 2, 3, 4 ou 5) e `btn_filter_all`, `btn_filter_1`, `btn_filter_2`, `btn_filter_3`, `btn_filter_h`, `btn_filter_r`.
- **Busca e substituição**: `replace_find_buf`, `replace_with_buf`, `search_query`, `search_cursor`, `filtered_indices`, `num_filtered`, `selected_note_idx`, `sidebar_scroll_y`.
- **Histórico de navegação**: `history[MAX_HISTORY][512]`, `history_count`, `history_idx`.
- **Prompts e linha de comando**: `prompt_buf`, `prompt_cursor`, `rename_target_path`, `cmd_buf`, `cmd_cursor`, `status_msg`, `status_msg_time`, `last_search_pat`.
- **Mouse e encerramento**: `mouse_col`, `mouse_row`, `mouse_btn`, `mouse_released`, `quit_requested`, além do buffer de *yank* (`yank_buf`, `yank_is_linewise`) e do operador pendente (`pending_op`).

Além de `E`, a base mantém globais de processo ligadas à renderização e à atividade, usadas pelo *dirty-flag* (seção 4.3): `g_needs_redraw`, `g_screen_clear_needed`, `g_window_resized`, `g_marquee_active`, `g_last_marquee_tick`, `g_last_heartbeat_draw`, `g_last_activity_time`, `g_preview`, `g_db` (handle do SQLite).

### 3.8 Esquema do Banco SQLite (`notas.db`)

```sql
CREATE TABLE IF NOT EXISTS notes (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT    NOT NULL UNIQUE,      -- nome lógico da nota (chave)
    content   TEXT    NOT NULL DEFAULT '',  -- markdown completo, UTF-8 byte a byte
    mtime     INTEGER NOT NULL DEFAULT 0,   -- data de modificação (segundos)
    attention INTEGER NOT NULL DEFAULT 0    -- 0 = nenhuma, 1, 2, 3, 4 (H), 5 (R) ou 6 (O)
);
CREATE INDEX IF NOT EXISTS idx_notes_mtime ON notes(mtime DESC);
PRAGMA user_version = 1;   /* DB_SCHEMA_VERSION */
```
Ajustes aplicados na abertura (`db_open()`): `busy_timeout` de 3000 ms, `journal_mode=WAL`, `synchronous=NORMAL` e `foreign_keys=ON`. Em modo WAL aparecem os arquivos satélites `notas.db-wal` e `notas.db-shm` ao lado do executável. Se o filesystem não suportar WAL, a SQLite mantém o journal padrão e o editor continua funcionando.

`db_open()` agora lê `PRAGMA user_version` antes de qualquer gravação: versão maior que `DB_SCHEMA_VERSION` é recusada; versão menor é migrada para a v1. Qualquer falha de `CREATE TABLE`, índice ou versão fecha o handle com `sqlite3_close_v2()` e devolve `false`, em vez de deixar um banco meio-aberto. Níveis de `attention` são validados no CRUD e ao ler o banco (0..6); valores corrompidos viram 0.

Ao acrescentar uma coluna: atualize o `CREATE TABLE` de `db_open()`, incremente `DB_SCHEMA_VERSION` / `PRAGMA user_version`, adicione o CRUD `db_*` correspondente, monte o campo em `rescan_notes()` (que traduz linhas do banco em `NoteInfo`) e trate a persistência por nota como fazem `db_set_attention()` / `db_insert_note()`.

### 3.9 Identidade de Nota e o Caminho Virtual `"db:<id>"`

Este é o ponto arquitetural que permitiu trocar o backend sem reescrever a interface. Toda a UI opera sobre uma **string de caminho**; desde a v6.29.0 essa string é o caminho virtual `"db:<id>"`:

| Operação | Regra |
| :--- | :--- |
| Criar caminho | `note_path_from_id(id, out, sz)` → `"db:<id>"` |
| Ler o id | `note_id_from_path(path)` → id > 0, ou **-1** se ausente, não numérico, com lixo, negativo ou 0 |
| Ler conteúdo | `note_read_content(path, &len)` — banco (`db_get_content`) **ou** arquivo real (`read_entire_file`) |
| Gravar conteúdo | `note_write_content(path, data, len)` — banco (`db_update_content`) **ou** arquivo real com escrita atômica |
| Nome de exibição | `db_get_name(id)` → `cur_filename = "<nome>.md"` |

O *fallback* para arquivo real mantém funcionando o argumento de linha de comando (`./notas algum.md`), a prévia de um `.md` avulso e as rotinas de importação/exportação — sem que nenhuma verificação de igualdade de caminho precise ser alterada. `note_id_from_path()` usa `strtol` com checagem de `ERANGE`/caracteres restantes: `db:abc`, `db:12lixo`, `db:-1` e `db:0` são rejeitados em vez de virarem IDs válidos por acidente.

---

## 4. Máquina de Estados, Modos e Globais de Renderização

### 4.1 Modos Suportados (`enum EditorMode`)
```c
enum EditorMode {
    MODE_NORMAL = 0,           /* Navegação modal Vim clássica */
    MODE_INSERT,               /* Inserção direta de texto no buffer */
    MODE_VISUAL_CHAR,          /* Seleção visual caractere a caractere */
    MODE_VISUAL_LINE,          /* Seleção visual linha a linha */
    MODE_COMMAND,              /* Linha de comando inferior iniciada por ':' */
    MODE_SEARCH_LEFT,          /* Campo de busca 'Ctrl+F' no painel lateral */
    MODE_CONFIRM_DELETE_NOTE,  /* Janela de confirmação de exclusão (s/n) */
    MODE_PROMPT_NEW_NOTE,      /* Diálogo interativo para criação de nota */
    MODE_PROMPT_RENAME_NOTE,   /* Diálogo interativo para renomeação de nota */
    MODE_PROMPT_FIND,          /* Prompt de busca textual na nota ativa */
    MODE_PROMPT_REPLACE_FIND,  /* Prompt 1: termo a localizar na substituição */
    MODE_PROMPT_REPLACE_WITH,  /* Prompt 2: novo termo a ser inserido */
    MODE_PROMPT_IMPORT_DIR,    /* v6.29.0: pasta de .md que será importada para o banco */
    MODE_PROMPT_EXPORT_DIR     /* v6.29.0: pasta de destino da exportação */
};
```
Os dois últimos modos são **a faixa de prompts**, e o intervalo `MODE_PROMPT_NEW_NOTE .. MODE_PROMPT_EXPORT_DIR` aparece em quatro lugares: guarda de colagem (`editor_paste_string`), guarda do leitor de teclas (Alt), insulação do mouse e despachante de teclas. Os rótulos desses dois prompts (`PROMPT_IMPORT_LABEL`, `PROMPT_EXPORT_LABEL`) são usados tanto no desenho quanto no cálculo da coluna do cursor — sempre pela mesma macro.

### 4.2 Foco da Aplicação (`enum EditorFocus`)
- **`FOCUS_LEFT`**: foco no painel lateral. `j`/`k` e setas navegam na lista, `Enter` ou `l` abre a nota selecionada em aba, `m` cicla a urgência (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[H]` ➔ `[R]` ➔ `[O]` ➔ `[ ]`), `d` pede confirmação de exclusão, `/` ou `Ctrl+F` foca a busca e `1`, `2`, `3`, `4`/`H`, `5`/`R`, `6`/`O`, `0` ativam os filtros de urgência correspondentes.
- **`FOCUS_RIGHT`**: foco na área de edição, com os atalhos Vim (`hjkl`, `w`, `b`, `gg`, `G`, `x`, `d`, `y`, `p`, `u`, `Ctrl+R`, etc.) e os recursos de produtividade (bullets coloridas, tabelas, wikilinks, dobras, marquee). O atalho `m` ou `!` também cicla a urgência da nota da aba ativa atual.

### 4.3 Globais de Renderização e Atividade (v6.28.0)
| Global | Papel |
| :--- | :--- |
| `g_needs_redraw` | Ligada por toda rotina que altera o estado visual; é o sinal primário de que há o que desenhar. Baixada por `refresh_screen()`. |
| `g_screen_clear_needed` | Pede limpeza total da tela (`\x1b[2J\x1b[3J`) no próximo quadro, em vez de apenas reposicionar o cursor. Ligada em troca de nota/aba/tema/filtro e no `SIGWINCH`. |
| `g_window_resized` | Ligada pelo tratador de `SIGWINCH`; força redesenho e recálculo de layout no próximo quadro. |
| `g_marquee_active` / `g_last_marquee_tick` | Indicam que existe título longo rolando na barra de abas e qual passo de 300 ms já foi desenhado. |
| `g_last_heartbeat_draw` | Instante do último desenho de manutenção; `IDLE_HEARTBEAT_MS` (30 s, sobrescritível por `-D`) define o intervalo. |
| `g_last_activity_time` | Instante da última atividade do usuário, usado pelo *screensaver* de Game of Life (`GAMEOFLIFE_IDLE_TIMEOUT_MS` = 10 min). |

---

## 5. Camada Multiplataforma (POSIX Linux vs Win32)

O código compila nativamente em Linux e Windows sem emuladores adicionais. A tabela reflete a base atual:

| Recurso | Linux / POSIX | Windows / Win32 |
| :--- | :--- | :--- |
| **Entrada do Terminal** | `tcsetattr` em modo raw não-bloqueante | `SetConsoleMode` com flags de entrada virtual VT |
| **Saída ANSI / Cores** | Sequências ANSI VT100 / TrueColor 24-bit | `ENABLE_VIRTUAL_TERMINAL_PROCESSING` ativo |
| **Dimensões da Tela** | `ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws)` | `GetConsoleScreenBufferInfo(hOut, &csbi)` |
| **Rastreamento de Mouse** | Sequências ANSI SGR 1006 | Mesmo padrão via processamento VT nativo |
| **Banco de Dados** | `sqlite3_open("notas.db")` + `-lsqlite3` | Idem, linkando a libsqlite3 cross-compilada de `tools/sqlite3-mingw/` |
| **Abertura de Arquivo** | `fopen` | `_wfopen` via `fopen_utf8` (caminho UTF-8 ➔ WideChar) |
| **Criação de Pasta** | `mkdir(dir, 0755)` (`mkdir_cross` / `mkdir_utf8`) | `CreateDirectoryW` via `mkdir_utf8` (suporte a UTF-8/Wide) |
| **Exclusão de Arquivos** | `unlink(path)` | `DeleteFileW` com conversão UTF-8 ➔ Wide |
| **Renomeação** | `rename(old, new)` | `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)` |
| **Varredura de Diretório** | `opendir()`, `readdir()`, `closedir()` (importação) | `FindFirstFileW`, `FindNextFileW`, `FindClose` (importação) |
| **Cópia p/ Clipboard** | Thread `pthread` detachável: `wl-copy` (Wayland), `xclip`/`xsel` (X11), `clip.exe` (host Windows); + OSC 52 Base64 | Thread `CreateThread` gravando `CF_UNICODETEXT`; OSC 52 apenas em sessão SSH |
| **Colagem do Clipboard** | `wl-paste`, `xclip`/`xsel`, ou `powershell Get-Clipboard` (WSL/Git Bash) | `GetClipboardData(CF_UNICODETEXT)` ➔ UTF-8, com fallback ANSI (`CF_TEXT`) |
| **Abertura de Links** | `xdg-open -- <alvo>` via `fork`/`exec` (sem shell, sem injeção) | `ShellExecuteW(NULL, "open", wurl, ...)` com URL convertida para UTF-16 e validada |
| **Diretório de Trabalho** | `chdir()` para o diretório do executável (via `/proc/self/exe`, buffer de 4096) | `SetCurrentDirectoryW` com `GetModuleFileNameW` (caminho Unicode, buffer de 32768 WCHAR) |

---

## 6. Pipeline Gráfico de Renderização (`refresh_screen`)

O desenho só acontece quando `should_refresh_now()` autoriza (seção 2.4). Autorizado, `refresh_screen()` executa a sequência abaixo, sempre compondo **tudo** em um `ABuffer` e emitindo **uma única escrita atômica** no fim:

1. **Baixa a *dirty-flag***: `g_needs_redraw = 0` e `g_last_heartbeat_draw = get_current_time_ms()` — este é o ponto único que reinicia o batimento de manutenção.
2. **Recalcula o layout**: redetecta o tamanho da janela; se houve redimensionamento (`g_window_resized`), pede limpeza total; limita `left_width` (mínimo 18, nunca ocupando menos de 15 colunas da área de texto) e fixa `status_bar_lines = 2`.
3. **Decide limpeza x reposicionamento**: com `g_screen_clear_needed` (ou redimensionamento) emite `\x1b[2J\x1b[3J\x1b[H` e baixa a flag; caso contrário só `\x1b[H`. Em seguida oculta o cursor físico (`\x1b[?25l`).
4. **Painel Esquerdo (`render_left_panel`)**: título, caixa de busca rápida (`Ctrl+F`), a lista de notas na **ordem de prioridade** (tempo relativo, caixa de urgência colorida por `get_attention_color()`, marcadores de nota aberta `●` e selecionada `▸`) e, na última linha, os botões responsivos de filtro ` [Todas] [1] [2] [3] [H] [R] [O]` (com rótulo abreviado `[T]` ou modo colado conforme a largura).
5. **Divisória Vertical**: linha separadora entre painel lateral e área de edição, na cor `TH_FG_BORDER`.
6. **Barra de Abas (`render_tab_bar`, linha 1)**: abas abertas, indicador de modificado, botão de fechar (`x`), botão `[+]`, seletor de tema e animação de marquee para títulos longos.
7. **Painel Direito de Edição (`render_right_panel`)**: números de linha e indicadores de dobra; cores de H1/H2/H3; bullet lists (`-` amarelo negrito, `*` acento negrito); ocultação de URLs de links e de estilos Markdown no Modo Normal; tabelas alinhadas; fundo de blocos de código; realce de seleção visual e de termos buscados; quebra de linha visual quando `wrap_mode` está ligado. O mapeamento linha visual ➔ linha/segmento é feito pelo `RenderCursor` incremental (seção 9.3), resetado a cada quadro.
8. **Barra de Status (`render_status_bar`, duas linhas)**: badge do modo com fundo colorido (NORMAL, INSERT, `INSERT [SEL]`, VISUAL, V-LINE, COMMAND, SEARCH, DELETE?, NEW NOTE, RENAME) + nome da nota ativa (ou `[Prévia] nome`) + `[+]` de modificado + posição do cursor/linhas na primeira linha; na segunda, os botões centralizados e clicáveis — `[➕ Nova]`, `[✏️ Renomear]`, `[🗑️ Excluir]`, `[Wrap]`, `[🔍 Buscar]`, `[🔄 Subst]`, `[🎨 Tema]`, `[🧬 Life]`, `[⬇ Importar]`, `[⬆ Exportar]` — com rótulos compactos abaixo de 108 colunas.
9. **Linha de Comando e Prompts (`render_command_line`)**: comando `:` do Vim ou o campo de digitação do diálogo ativo, incluindo os prompts de importação/exportação.
10. **Descarga Atômica**: posiciona o cursor físico na coluna calculada (considerando a largura do rótulo do prompt em uso), reexibe a visibilidade do cursor e emite todo o bloco acumulado com `write()`/`WriteFile()` em laço, tratando escritas parciais e `EINTR`. Se `ABuffer` falhar, o quadro não é emitido e a limpeza/redesenho é reagendada.

---

## 7. Catálogo das Funções da Base v6.30.0

Abaixo estão catalogadas as **161** funções que existiam em `notas.c` na base v6.30.0, em ordem de aparição e com o número de linha daquela versão. `get_system_clipboard` aparece duas vezes porque tem implementações mutuamente exclusivas (Win32 e POSIX). As funções novas da rodada de endurecimento (`utf8_to_wide_alloc`, `write_file_atomic`, `save_undo_for_typing`, `RenderCursor`, `table_insert_blank_row`, `any_tab_modified`, `save_all_modified_tabs`, entre outras) estão descritas na seção 9. Os números de linha do catálogo abaixo ficaram defasados após esta rodada; use-os como referência histórica, não como localização atual.

### 7.1 `safe_strcpy` (Linha 61)
- **Protótipo**: `static void safe_strcpy(char *dest, size_t dest_sz, const char *src)`
- **Descrição**: Copia strings com segurança garantindo que nunca ocorra buffer overflow e assegurando terminação nula '\0'.
- **Retorno**: `static void`

### 7.2 `ab_init` (Linha 455)
- **Protótipo**: `static void ab_init(ABuffer *ab)`
- **Descrição**: Inicializa o `ABuffer` zerando ponteiro, tamanho e capacidade. A primeira alocação real acontece em `ab_append()`, que começa em 1024 bytes e dobra conforme a necessidade.
- **Retorno**: `static void`

### 7.3 `ab_append` (Linha 461)
- **Protótipo**: `static void ab_append(ABuffer *ab, const char *s, int len)`
- **Descrição**: Anexa uma sequência de bytes ao buffer acumulador dinâmico, expandindo a capacidade com realloc quando necessário.
- **Retorno**: `static void`

### 7.4 `ab_free` (Linha 478)
- **Protótipo**: `static void ab_free(ABuffer *ab)`
- **Descrição**: Libera a memória alocada dinamicamente pelo ABuffer e zera os ponteiros internos.
- **Retorno**: `static void`

### 7.5 `fopen_utf8` (Linha 727)
- **Protótipo**: `static FILE *fopen_utf8(const char *path, const char *mode)`
- **Descrição**: Abre um arquivo em modo binário a partir de um caminho UTF-8. No Windows converte o caminho para WideChar antes de chamar a API Wide (`_wfopen`), garantindo nomes de arquivo acentuados.
- **Retorno**: `static FILE *`

### 7.6 `unlink_utf8` (Linha 741)
- **Protótipo**: `static int unlink_utf8(const char *path)`
- **Descrição**: Remove um arquivo Markdown do disco. No Windows converte o caminho UTF-8 para WideChar e invoca DeleteFileW.
- **Retorno**: `static int`

### 7.7 `rename_utf8` (Linha 753)
- **Protótipo**: `static int rename_utf8(const char *oldpath, const char *newpath)`
- **Descrição**: Renomeia um arquivo de nota no disco de forma atômica, garantindo suporte a caracteres acentuados no Windows e Linux.
- **Retorno**: `static int`

### 7.8 `filetime_to_timet` (Linha 767)
- **Protótipo**: `static inline time_t filetime_to_timet(const FILETIME *ft)`
- **Descrição**: Converte a estrutura FILETIME da API Win32 para o formato time_t Unix para comparação de datas.
- **Retorno**: `static inline time_t`

### 7.9 `stat_cross` (Linha 776)
- **Protótipo**: `static int stat_cross(const char *path, struct stat *st)`
- **Descrição**: Obtém metadados de arquivos (tamanho e data de modificação) de forma multiplataforma com suporte pleno a UTF-8.
- **Retorno**: `static int`

### 7.10 `disable_raw_mode` (Linha 797)
- **Protótipo**: `static void disable_raw_mode(void)`
- **Descrição**: Restaura o terminal para o modo canônico original, desativa o rastreamento do mouse e reexibe o cursor.
- **Retorno**: `static void`

### 7.11 `handle_sigwinch` (Linha 849)
- **Protótipo**: `static void handle_sigwinch(int sig)`
- **Descrição**: Trata o `SIGWINCH` no Linux: marca `g_window_resized` e pede limpeza da tela (`g_screen_clear_needed`), para o redesenho seguinte recalcular o layout.
- **Retorno**: `static void`

### 7.12 `enable_raw_mode` (Linha 856)
- **Protótipo**: `static void enable_raw_mode(void)`
- **Descrição**: Coloca o terminal em modo Raw, desativa eco e quebra de linha canônica, e ativa o rastreamento de mouse SGR 1006.
- **Retorno**: `static void`

### 7.13 `get_cursor_position` (Linha 897)
- **Protótipo**: `static int get_cursor_position(int *rows, int *cols)`
- **Descrição**: Consulta a posição física atual do cursor emitindo a sequência DSR '\x1b[6n' e decodificando a resposta.
- **Retorno**: `static int`

### 7.14 `get_window_size` (Linha 921)
- **Protótipo**: `static void get_window_size(int *rows, int *cols)`
- **Descrição**: Obtém as dimensões de linhas e colunas da tela via ioctl (Linux) ou GetConsoleScreenBufferInfo (Windows).
- **Retorno**: `static void`

### 7.15 `utf8_charlen` (Linha 981)
- **Protótipo**: `static int utf8_charlen(unsigned char ch)`
- **Descrição**: Retorna o tamanho em bytes (1 a 4) de um caractere UTF-8 com base no seu byte inicial.
- **Retorno**: `static int`

### 7.16 `utf8_decode_codepoint` (Linha 989)
- **Protótipo**: `static int utf8_decode_codepoint(const char *s, int *bytes_read)`
- **Descrição**: Decodifica uma sequência de bytes UTF-8 em um codepoint Unicode de 32 bits.
- **Retorno**: `static int`

### 7.17 `utf8_codepoint_width` (Linha 1024)
- **Protótipo**: `static int utf8_codepoint_width(int cp)`
- **Descrição**: Retorna a largura visual de um codepoint Unicode em colunas de terminal (0 para diacríticos, 2 para CJK, 1 para texto comum).
- **Retorno**: `static int`

### 7.18 `utf8_width` (Linha 1050)
- **Protótipo**: `static int utf8_width(const char *s)`
- **Descrição**: Calcula a largura total em colunas visuais que uma string UTF-8 ocupará na tela.
- **Retorno**: `static int`

### 7.19 `utf8_align_start` (Linha 1072)
- **Protótipo**: `static int utf8_align_start(const char *s, int len, int idx)`
- **Descrição**: Ajusta um índice de byte para o início do caractere UTF-8 correspondente, evitando cortar caracteres no meio.
- **Retorno**: `static int`

### 7.20 `utf8_align_end` (Linha 1081)
- **Protótipo**: `static int utf8_align_end(const char *s, int len, int idx)`
- **Descrição**: Ajusta um índice de byte para o final do caractere UTF-8 correspondente.
- **Retorno**: `static int`

### 7.21 `utf8_width_len` (Linha 1093)
- **Protótipo**: `static int utf8_width_len(const char *s, int len)`
- **Descrição**: Calcula a largura visual em colunas de um trecho de texto UTF-8 delimitado por 'len' bytes.
- **Retorno**: `static int`

### 7.22 `byte_offset_for_col` (Linha 1117)
- **Protótipo**: `static int byte_offset_for_col(const char *s, int len, int target_col)`
- **Descrição**: Mapeia uma coluna visual de tela para o deslocamento exato de byte correspondente na string UTF-8.
- **Retorno**: `static int`

### 7.23 `utf8_truncate_display` (Linha 1141)
- **Protótipo**: `static void utf8_truncate_display(const char *src, int max_cols, char *dst, size_t dst_sz)`
- **Descrição**: Trunca uma string para caber exatamente em 'max_cols' colunas sem corromper caracteres multibyte.
- **Retorno**: `static void`

### 7.24 `utf8_marquee_slice` (Linha 1168)
- **Protótipo**: `static void utf8_marquee_slice(const char *title, int step, int avail_w, char *out, size_t out_sz)`
- **Descrição**: Gera uma fatia com animação de letreiro (marquee) para títulos de notas longos na barra de abas ou painel lateral.
- **Retorno**: `static void`

### 7.25 `get_current_time_ms` (Linha 1208)
- **Protótipo**: `static long get_current_time_ms(void)`
- **Descrição**: Retorna o timestamp atual em milissegundos de alta precisão através de relógio monotônico.
- **Retorno**: `static long`

### 7.26 `base64_encode` (Linha 1223)
- **Protótipo**: `static char *base64_encode(const unsigned char *data, size_t input_len)`
- **Descrição**: Codifica um bloco de bytes em Base64. Usado para montar a sequência OSC 52 (`\x1b]52;c;<base64>\x07`) de cópia para a área de transferência do terminal (funciona em Linux, WSL e sessões SSH).
- **Retorno**: `static char *`

### 7.27 `win32_clipboard_thread` (Linha 1250)
- **Protótipo**: `static DWORD WINAPI win32_clipboard_thread(LPVOID param)`
- **Descrição**: Thread auxiliar do Windows que grava dados na área de transferência assincronamente sem congelar a UI.
- **Retorno**: `static DWORD WINAPI`

### 7.28 `posix_clipboard_thread` (Linha 1291)
- **Protótipo**: `static void *posix_clipboard_thread(void *param)`
- **Descrição**: Thread POSIX destacável de cópia assíncrona. Escolhe o utilitário conforme o ambiente (`clip.exe` no WSL/Git Bash com host Windows, `wl-copy` no Wayland, `xclip`/`xsel` no X11), escreve os bytes no pipe e o fecha.
- **Retorno**: `static void *`

### 7.29 `copy_to_system_clipboard` (Linha 1337)
- **Protótipo**: `static void copy_to_system_clipboard(const char *text, int len)`
- **Descrição**: Copia texto para a área de transferência do sistema via Win32 API, sequência OSC 52 Base64 ou utilitários POSIX (wl-copy/xclip).
- **Retorno**: `static void`

### 7.30 `get_system_clipboard` (Linha 1409)
- **Protótipo**: `static char *get_system_clipboard(int *out_len)`
- **Descrição**: Lê a área de transferência do sistema e devolve um buffer alocado pelo chamador. Existem duas implementações mutuamente exclusivas: a Win32 (linha 1409) lê `CF_UNICODETEXT` e converte para UTF-8, com fallback para o formato ANSI `CF_TEXT`; a POSIX (linha 1453) lê via `wl-paste`, `xclip`/`xsel` ou `powershell Get-Clipboard` (WSL/Git Bash), drenando o pipe em blocos.
- **Retorno**: `static char *`

### 7.31 `get_system_clipboard` (Linha 1453)
- **Protótipo**: `static char *get_system_clipboard(int *out_len)`
- **Descrição**: Lê a área de transferência do sistema e devolve um buffer alocado pelo chamador. Existem duas implementações mutuamente exclusivas: a Win32 (linha 1409) lê `CF_UNICODETEXT` e converte para UTF-8, com fallback para o formato ANSI `CF_TEXT`; a POSIX (linha 1453) lê via `wl-paste`, `xclip`/`xsel` ou `powershell Get-Clipboard` (WSL/Git Bash), drenando o pipe em blocos.
- **Retorno**: `static char *`

### 7.32 `open_link_target` (Linha 1511)
- **Protótipo**: `static void open_link_target(const char *raw_target)`
- **Descrição**: Abre um link da web no navegador padrão ou um arquivo/pasta local no explorador de arquivos do sistema operacional.
- **Retorno**: `static void`

### 7.33 `detect_heading` (Linha 1672)
- **Protótipo**: `static void detect_heading(Line *l)`
- **Descrição**: Analisa se a linha começa com um cabeçalho Markdown ('#' a '######') e preenche o campo heading_lvl.
- **Retorno**: `static void`

### 7.34 `line_init` (Linha 1682)
- **Protótipo**: `static void line_init(Line *l, const char *s, int len)`
- **Descrição**: Aloca e inicializa uma struct Line com a string UTF-8 especificada e terminador nulo.
- **Retorno**: `static void`

### 7.35 `line_free` (Linha 1694)
- **Protótipo**: `static void line_free(Line *l)`
- **Descrição**: Desaloca a memória da string interna de uma struct Line.
- **Retorno**: `static void`

### 7.36 `mark_modified` (Linha 1700)
- **Protótipo**: `static void mark_modified(void)`
- **Descrição**: Marca a nota ativa como modificada e registra o instante para o temporizador de autosave.
- **Retorno**: `static void`

### 7.37 `buf_insert_line` (Linha 1714)
- **Protótipo**: `static void buf_insert_line(int at, const char *s, int len)`
- **Descrição**: Insere uma nova linha no buffer do editor na posição especificada 'at'.
- **Retorno**: `static void`

### 7.38 `buf_delete_line` (Linha 1727)
- **Protótipo**: `static void buf_delete_line(int at)`
- **Descrição**: Remove uma linha do buffer do editor na posição especificada 'at'.
- **Retorno**: `static void`

### 7.39 `line_insert_char` (Linha 1739)
- **Protótipo**: `static void line_insert_char(Line *l, int at, char c)`
- **Descrição**: Insere um caractere/byte na linha na posição 'at'.
- **Retorno**: `static void`

### 7.40 `line_delete_char` (Linha 1750)
- **Protótipo**: `static void line_delete_char(Line *l, int at)`
- **Descrição**: Remove de forma atômica o caractere UTF-8 na posição 'at' (calculando a largura do codepoint de 1 a 4 bytes e invocando `line_replace_range`), impedindo a criação de bytes de continuação órfãos em caracteres multibyte (como acentos e emojis).
- **Retorno**: `static void`

### 7.41 `line_replace_range` (Linha 1759)
- **Protótipo**: `static void line_replace_range(Line *l, int at, int del_len, const char *new_str, int new_len)`
- **Descrição**: Substitui um intervalo de bytes na linha por uma nova sequência de caracteres.
- **Retorno**: `static void`

### 7.42 `line_append_string` (Linha 1779)
- **Protótipo**: `static void line_append_string(Line *l, const char *s, int len)`
- **Descrição**: Concatena uma string ao final da linha especificada.
- **Retorno**: `static void`

### 7.43 `editor_paste_string` (Linha 1790)
- **Protótipo**: `static void editor_paste_string(const char *text, int len)`
- **Descrição**: Manipulador robusto de colagem multi-linha que previne efeito escada e normaliza quebras de linha CRLF/LF.
- **Retorno**: `static void`

### 7.44 `update_lines_syntax` (Linha 1878)
- **Protótipo**: `static void update_lines_syntax(Line *lines, int num_lines)`
- **Descrição**: Verifica blocos de código cercados por crases (```) e atualiza os flags is_code_fence e in_code_block.
- **Retorno**: `static void`

### 7.45 `update_folding` (Linha 1911)
- **Protótipo**: `static void update_folding(void)`
- **Descrição**: Recalcula a visibilidade das linhas ocultas com base nos cabeçalhos atualmente dobrados (colapsados).
- **Retorno**: `static void`

### 7.46 `toggle_fold_at_cursor` (Linha 1915)
- **Protótipo**: `static void toggle_fold_at_cursor(void)`
- **Descrição**: Alterna o estado de dobra (folding) do cabeçalho sob a posição do cursor (atalho 'za').
- **Retorno**: `static void`

### 7.47 `collapse_all_headings` (Linha 1940)
- **Protótipo**: `static void collapse_all_headings(void)`
- **Descrição**: Recolhe todas as seções de cabeçalho da nota (atalho 'zM').
- **Retorno**: `static void`

### 7.48 `uncollapse_all_headings` (Linha 1961)
- **Protótipo**: `static void uncollapse_all_headings(void)`
- **Descrição**: Expande todas as seções de cabeçalho da nota (atalho 'zR').
- **Retorno**: `static void`

### 7.49 `free_undo_state` (Linha 1983)
- **Protótipo**: `static void free_undo_state(UndoState *us)`
- **Descrição**: Libera a memória alocada nas linhas de um snapshot da pilha de desfazer.
- **Retorno**: `static void`

### 7.50 `save_undo` (Linha 1993)
- **Protótipo**: `static void save_undo(void)`
- **Descrição**: Salva um snapshot completo do buffer e do cursor na pilha de desfazer antes de alterações no texto.
- **Retorno**: `static void`

### 7.51 `apply_undo` (Linha 2016)
- **Protótipo**: `static void apply_undo(void)`
- **Descrição**: Restaura o estado anterior da pilha de desfazer (atalho 'u').
- **Retorno**: `static void`

### 7.52 `apply_redo` (Linha 2053)
- **Protótipo**: `static void apply_redo(void)`
- **Descrição**: Reaplica a alteração desfeita (atalho 'Ctrl+R').
- **Retorno**: `static void`

### 7.53 `format_rel_time` (Linha 2094)
- **Protótipo**: `static void format_rel_time(time_t mtime, time_t now, char *buf, size_t sz)`
- **Descrição**: Formata a diferença de tempo de modificação de forma amigável ('agora', '5m atrás', '2h atrás', 'data').
- **Retorno**: `static void`

### 7.54 `normalize_text` (Linha 2117)
- **Protótipo**: `static void normalize_text(const char *src, char *dst, size_t dst_cap)`
- **Descrição**: Normaliza texto removendo acentos e convertendo para minúsculas para busca ultra-rápida em português.
- **Retorno**: `static void`

### 7.55 `extract_note_title` (Linha 2182)
- **Protótipo**: `static void extract_note_title(const char *filepath, const char *fname, char *out_title, size_t sz)`
- **Descrição**: Extrai o título da nota: primeiro cabeçalho H1 (`# Título`) do conteúdo (lido por `note_read_content()`) ou o nome de arquivo higienizado, como fallback.
- **Retorno**: `static void`

### 7.56 `compare_notes_priority` (Linha 2202)
- **Protótipo**: `static int compare_notes_priority(const void *a, const void *b)`
- **Descrição**: Comparador de ordenação da lista lateral (v6.30.0): agrupa por prioridade (níveis 1, 2, 3 e por último as notas sem marcação, que caem no grupo 3), dentro do grupo coloca a mais recente primeiro (`compare_notes_mtime`) e desempata por nome de arquivo, para a ordem não oscilar entre redesenhos.
- **Retorno**: `static int`

### 7.57 `compare_notes_mtime` (Linha 2217)
- **Protótipo**: `static int compare_notes_mtime(const void *a, const void *b)`
- **Descrição**: Função de comparação para qsort ordenando as notas da mais recentemente modificada para a mais antiga.
- **Retorno**: `static int`

### 7.58 `read_entire_file` (Linha 2225)
- **Protótipo**: `static char *read_entire_file(const char *path, long *out_len)`
- **Descrição**: Lê um arquivo inteiro para um buffer alocado em memória (uma única alocação de `len + 1` bytes, com terminador '\0') e devolve o tamanho lido em `out_len`. É o caminho de leitura das notas que ainda são arquivos reais (importação e argumento de linha de comando).
- **Retorno**: `static char *`

### 7.59 `db_is_open` (Linha 2261)
- **Protótipo**: `static inline bool db_is_open(void)`
- **Descrição**: Retorna `true` se o banco SQLite está aberto (`g_db != NULL`).
- **Retorno**: `static inline bool`

### 7.60 `db_close` (Linha 2263)
- **Protótipo**: `static void db_close(void)`
- **Descrição**: Fecha o banco SQLite e zera o handle global `g_db`. É a última operação de `free_editor()`: as rotinas de limpeza anteriores ainda podem consultar o banco.
- **Retorno**: `static void`

### 7.61 `db_exec` (Linha 2270)
- **Protótipo**: `static bool db_exec(const char *sql)`
- **Descrição**: Executa um SQL sem resultado (DDL/PRAGMA) no banco aberto, liberando a mensagem de erro da SQLite. Retorna `false` em qualquer falha.
- **Retorno**: `static bool`

### 7.62 `db_open` (Linha 2282)
- **Protótipo**: `static bool db_open(void)`
- **Descrição**: Abre (criando se necessário) `notas.db` ao lado do executável e garante o schema: `busy_timeout` de 3 s, `journal_mode=WAL`, `synchronous=NORMAL`, `CREATE TABLE IF NOT EXISTS notes`, índice `idx_notes_mtime` e `PRAGMA user_version=1`. O banco nasce VAZIO — nenhuma nota de exemplo é criada.
- **Retorno**: `static bool`

### 7.63 `note_id_from_path` (Linha 2315)
- **Protótipo**: `static int note_id_from_path(const char *path)`
- **Descrição**: Extrai o id numérico de um caminho virtual no formato `db:<id>`. Retorna -1 quando o caminho não é do banco (nota aberta como arquivo real).
- **Retorno**: `static int`

### 7.64 `note_path_from_id` (Linha 2320)
- **Protótipo**: `static void note_path_from_id(int id, char *out, size_t out_sz)`
- **Descrição**: Formata o caminho virtual `db:<id>` em `out`.
- **Retorno**: `static void`

### 7.65 `db_find_id_by_name` (Linha 2327)
- **Protótipo**: `static int db_find_id_by_name(const char *name)`
- **Descrição**: Consulta o id da nota cujo nome lógico é `name` (coluna `name`, que é `UNIQUE`). Devolve -1 se a nota não existir.
- **Retorno**: `static int`

### 7.66 `db_insert_note` (Linha 2341)
- **Protótipo**: `static int db_insert_note(const char *name, const char *content, time_t mtime, int attention)`
- **Descrição**: Insere uma nota (nome, conteúdo, mtime, nível de atenção) e devolve o id gerado por `last_insert_rowid`. mtime 0 vira a hora atual. Retorna -1 em falha (por exemplo, nome já existente, barrado pelo `UNIQUE`).
- **Retorno**: `static int`

### 7.67 `db_update_content` (Linha 2359)
- **Protótipo**: `static bool db_update_content(int id, const char *content, time_t mtime)`
- **Descrição**: Grava o conteúdo e o mtime de uma nota existente. Retorna `false` se nenhuma linha foi alterada.
- **Retorno**: `static bool`

### 7.68 `db_update_name` (Linha 2376)
- **Protótipo**: `static bool db_update_name(int id, const char *newname)`
- **Descrição**: Renomeia a nota (coluna `name`). Como o nome é `UNIQUE`, um nome já em uso faz o UPDATE falhar e nada muda — é assim que o rename detecta conflito sem precisar de consulta prévia.
- **Retorno**: `static bool`

### 7.69 `db_delete_note` (Linha 2392)
- **Protótipo**: `static bool db_delete_note(int id)`
- **Descrição**: Remove a nota pelo id. Retorna `false` se nenhuma linha foi apagada.
- **Retorno**: `static bool`

### 7.70 `db_set_attention` (Linha 2405)
- **Protótipo**: `static bool db_set_attention(int id, int level)`
- **Descrição**: Grava o nível de urgência (0 a 3) da nota na coluna `attention`.
- **Retorno**: `static bool`

### 7.71 `db_get_name` (Linha 2420)
- **Protótipo**: `static bool db_get_name(int id, char *out, size_t out_sz)`
- **Descrição**: Lê o nome lógico da nota pelo id para um buffer do chamador. Retorna `false` se a nota não existir.
- **Retorno**: `static bool`

### 7.72 `db_get_content` (Linha 2437)
- **Protótipo**: `static char *db_get_content(int id, long *out_len)`
- **Descrição**: Lê o conteúdo da nota pelo id, copiando exatamente `sqlite3_column_bytes()` bytes para um buffer próprio do chamador — o conteúdo pode conter qualquer byte UTF-8, então nada de `strlen`.
- **Retorno**: `static char *`

### 7.73 `db_count_notes` (Linha 2463)
- **Protótipo**: `static int db_count_notes(void)`
- **Descrição**: Retorna a quantidade de notas da tabela (`SELECT COUNT(*)`). Usado nas mensagens de estado, na inicialização e para decidir se há o que exportar.
- **Retorno**: `static int`

### 7.74 `note_read_content` (Linha 2476)
- **Protótipo**: `static char *note_read_content(const char *path, long *out_len)`
- **Descrição**: Ponto único de LEITURA de nota: caminho virtual (`db:<id>`) lê do banco por `db_get_content()`; caso contrário lê o arquivo real com `read_entire_file()`. É o que mantém funcionando o argumento de linha de comando (abrir um `.md` avulso) e a prévia.
- **Retorno**: `static char *`

### 7.75 `note_write_content` (Linha 2482)
- **Protótipo**: `static bool note_write_content(const char *path, const char *data, long len)`
- **Descrição**: Ponto único de GRAVAÇÃO de nota: caminho virtual grava no banco por `db_update_content()` (atualizando o mtime); caso contrário escreve o arquivo real.
- **Retorno**: `static bool`

### 7.76 `lines_to_content` (Linha 2494)
- **Protótipo**: `static char *lines_to_content(Line *lines, int num_lines, long *out_len)`
- **Descrição**: Serializa o buffer de linhas do editor em texto Markdown (cada linha seguida de '\n') usando o `ABuffer`; devolve o buffer com a posse transferida ao chamador (não chamar `ab_free` depois).
- **Retorno**: `static char *`

### 7.77 `editor_load_content` (Linha 2513)
- **Protótipo**: `static void editor_load_content(const char *content)`
- **Descrição**: Carrega um conteúdo que já está em memória como linhas do editor ativo — equivalente ao antigo laço de `fgets()`, mas sem limite de tamanho por linha e removendo o `\r` dos finais CRLF.
- **Retorno**: `static void`

### 7.78 `is_markdown_ext` (Linha 2529)
- **Protótipo**: `static bool is_markdown_ext(const char *fname)`
- **Descrição**: Diz se o nome de arquivo tem extensão aceita na importação (`.md`, `.markdown`, `.mdown`, `.txt`, `.text`); nomes sem extensão também contam, mesma regra da varredura antiga.
- **Retorno**: `static bool`

### 7.79 `sanitize_note_name` (Linha 2538)
- **Protótipo**: `static void sanitize_note_name(const char *in, char *out, size_t out_sz)`
- **Descrição**: Troca por `_` os caracteres inválidos em nome de arquivo (`/ \ : * ? " < > |`). O nome lógico da nota passa por aqui antes de virar chave no banco e nome de arquivo na exportação.
- **Retorno**: `static void`

### 7.80 `expand_user_path` (Linha 2553)
- **Protótipo**: `static void expand_user_path(const char *in, char *out, size_t out_sz)`
- **Descrição**: Expande `~` no início do caminho usando `HOME` (ou `USERPROFILE` no Windows).
- **Retorno**: `static void`

### 7.81 `import_notes_from_dir` (Linha 2570)
- **Protótipo**: `static int import_notes_from_dir(const char *dirpath, int *out_added, int *out_updated, char *errbuf, size_t errsz)`
- **Descrição**: Importa uma pasta de arquivos Markdown para o banco: varre o diretório (Win32 ou POSIX), filtra por extensão, tira a extensão do nome e usa o resto como chave. Nota existente é ATUALIZADA com o conteúdo do arquivo (mantendo a marcação de urgência); nota nova é INSERIDA com atenção 0. Devolve o total processado e preenche `out_added`/`out_updated`.
- **Retorno**: `static int`

### 7.82 `export_notes_to_dir` (Linha 2679)
- **Protótipo**: `static int export_notes_to_dir(const char *dirpath, char *errbuf, size_t errsz)`
- **Descrição**: Exporta todas as notas do banco como `<nome>.md` na pasta indicada (criada com `mkdir_cross` se ainda não existir), higienizando o nome com `sanitize_note_name()` e escrevendo o texto byte a byte como está no banco. Devolve quantos arquivos foram gravados.
- **Retorno**: `static int`

### 7.83 `ui_import_from_dir` (Linha 2731)
- **Protótipo**: `static void ui_import_from_dir(const char *raw_dir)`
- **Descrição**: Ponto de entrada do botão `[Importar]`: expande `~`, apara espaços e remove aspas das pontas do caminho, chama `import_notes_from_dir()`, refaz a varredura e relata no rodapé quantas notas entraram (novas x atualizadas).
- **Retorno**: `static void`

### 7.84 `ui_export_to_dir` (Linha 2768)
- **Protótipo**: `static void ui_export_to_dir(const char *raw_dir)`
- **Descrição**: Ponto de entrada do botão `[Exportar]`: normaliza o caminho, recusa a operação com o banco vazio e chama `export_notes_to_dir()`, informando no rodapé quantas notas foram gravadas.
- **Retorno**: `static void`

### 7.85 `note_update_cache` (Linha 2800)
- **Protótipo**: `static void note_update_cache(NoteInfo *n)`
- **Descrição**: Monta o texto normalizado de busca da nota (nome + título + cabeçalho `#` + conteúdo), lendo o conteúdo por `note_read_content()`. O cache só é descartado quando a nota muda de data de modificação.
- **Retorno**: `static void`

### 7.86 `get_attention_color`
- **Protótipo**: `static const char *get_attention_color(int level)`
- **Descrição**: Retorna a sequência ANSI do nível de urgência, com variante clara e escura conforme o tema atual (1 = vermelho, 2 = laranja, 3 = amarelo, 4 = azul/ciano, 5 = roxo/violeta); sem marcação (0) devolve a cor de texto principal.
- **Retorno**: `static const char *`

### 7.87 `load_attention_flags`
- **Protótipo**: `static void load_attention_flags(void)`
- **Descrição**: Deriva o atalho booleano `needs_attention` a partir de `attention_level`, que já vem carregado da coluna `attention` do banco por `rescan_notes()`. Não lê mais o antigo arquivo `data/.attention`.
- **Retorno**: `static void`

### 7.88 `save_attention_flags`
- **Protótipo**: `static void save_attention_flags(void)`
- **Descrição**: Persiste no banco o nível de urgência de todas as notas em memória (`db_set_attention` por caminho `db:<id>`).
- **Retorno**: `static void`

### 7.89 `toggle_note_attention_at`
- **Protótipo**: `static void toggle_note_attention_at(int actual_idx)`
- **Descrição**: Alterna ciclicamente o nível de urgência da nota selecionada ([ ] ➔ [1] ➔ [2] ➔ [3] ➔ [H] ➔ [R] ➔ [O] ➔ [ ]), gravando atomicamente no banco SQLite, reorganizando a lista por prioridade para agrupar as notas no topo e mantendo a seleção sobre a nota.
- **Retorno**: `static void`

### 7.91 `rescan_notes` (Linha 2936)
- **Protótipo**: `static void rescan_notes(void)`
- **Descrição**: Varre a tabela `notes` do banco (não mais o diretório `data/`), monta o catálogo `NoteInfo` com caminho virtual `db:<id>`, reaproveita o cache de busca das notas cujo mtime não mudou, deriva os flags de atenção e ordena tudo com `compare_notes_priority()` (prioridade e, dentro do grupo, a mais recente primeiro).
- **Retorno**: `static void`

### 7.92 `str_contains_caseless` (Linha 3021)
- **Protótipo**: `static bool str_contains_caseless(const char *haystack, const char *needle)`
- **Descrição**: Verifica se uma substring existe dentro de outra sem distinguir maiúsculas e minúsculas.
- **Retorno**: `static bool`

### 7.93 `apply_note_search` (Linha 3037)
- **Protótipo**: `static void apply_note_search(const char *query)`
- **Descrição**: Recalcula `filtered_indices` aplicando o filtro de urgência e a busca multi-palavras sobre o cache normalizado, e ajusta seleção e prévia. Desde a v6.30.0 é o ponto onde o filtro de urgência é aplicado de fato, e ele é reaplicado a cada reordenação da lista.
- **Retorno**: `static void`

### 7.94 `free_preview` (Linha 3102)
- **Protótipo**: `static void free_preview(void)`
- **Descrição**: Libera as linhas do buffer de pré-visualização rápida do painel esquerdo.
- **Retorno**: `static void`

### 7.95 `load_preview` (Linha 3116)
- **Protótipo**: `static void load_preview(const char *path)`
- **Descrição**: Carrega a nota selecionada no painel de prévia do painel direito, sem abrir aba, lendo o conteúdo por `note_read_content()`.
- **Retorno**: `static void`

### 7.96 `tab_save_active` (Linha 3160)
- **Protótipo**: `static void tab_save_active(void)`
- **Descrição**: Sincroniza o estado atual do editor com a estrutura Tab da aba ativa.
- **Retorno**: `static void`

### 7.97 `tab_load` (Linha 3183)
- **Protótipo**: `static void tab_load(int idx)`
- **Descrição**: Carrega o estado de uma aba específica para as variáveis ativas do editor.
- **Retorno**: `static void`

### 7.98 `tab_switch` (Linha 3208)
- **Protótipo**: `static void tab_switch(int idx)`
- **Descrição**: Alterna a exibição para a aba indicada por índice.
- **Retorno**: `static void`

### 7.99 `tab_next` (Linha 3215)
- **Protótipo**: `static void tab_next(void)`
- **Descrição**: Avança para a próxima aba aberta (Ctrl+Tab ou :tabnext).
- **Retorno**: `static void`

### 7.100 `tab_prev` (Linha 3221)
- **Protótipo**: `static void tab_prev(void)`
- **Descrição**: Retorna para a aba anterior.
- **Retorno**: `static void`

### 7.101 `tab_close` (Linha 3227)
- **Protótipo**: `static void tab_close(int idx)`
- **Descrição**: Fecha a aba especificada. Se houver modificações não salvas, solicita confirmação do usuário.
- **Retorno**: `static void`

### 7.102 `close_all_tabs` (Linha 3283)
- **Protótipo**: `static void close_all_tabs(void)`
- **Descrição**: Fecha todas as abas abertas de uma só vez (atalho 'C' ou comando :closeall).
- **Retorno**: `static void`

### 7.103 `push_history` (Linha 3320)
- **Protótipo**: `static void push_history(const char *path)`
- **Descrição**: Registra uma nota visitada na pilha de histórico de navegação (até 64 notas).
- **Retorno**: `static void`

### 7.104 `navigate_history` (Linha 3334)
- **Protótipo**: `static void navigate_history(int steps)`
- **Descrição**: Navega para trás (Backspace) ou para frente (Ctrl+Backspace) no histórico de navegação entre notas.
- **Retorno**: `static void`

### 7.105 `open_note_by_path` (Linha 3346)
- **Protótipo**: `static void open_note_by_path(const char *path, bool record_history)`
- **Descrição**: Abre a nota: se já estiver aberta em alguma aba, apenas troca para ela; caso contrário cria a aba, resolve o nome pelo banco (`db_get_name()`, quando o caminho é `db:<id>`) ou pelo próprio caminho (nota de arquivo real), carrega o conteúdo com `note_read_content()` + `editor_load_content()` e pede limpeza de tela.
- **Retorno**: `static void`

### 7.106 `save_current_note` (Linha 3429)
- **Protótipo**: `static bool save_current_note(void)`
- **Descrição**: Salva a nota ativa: serializa as linhas com `lines_to_content()` e grava por `note_write_content()` (banco ou arquivo), atualiza o mtime/cache da nota na lista, reordena por prioridade e informa quantas linhas foram gravadas.
- **Retorno**: `static bool`

### 7.107 `save_tab_by_idx` (Linha 3474)
- **Protótipo**: `static bool save_tab_by_idx(int t_idx)`
- **Descrição**: Salva uma aba específica pelo índice (a aba ativa delega para `save_current_note()`), com o mesmo caminho de serialização/gravação e atualização do cache da lista.
- **Retorno**: `static bool`

### 7.108 `check_autosave` (Linha 3504)
- **Protótipo**: `static void check_autosave(void)`
- **Descrição**: Verifica se há alterações não salvas há mais de `AUTOSAVE_DELAY_MS` (60 s) em qualquer aba e executa o salvamento automático silencioso (`save_tab_by_idx`), no banco ou no arquivo real.
- **Retorno**: `static void`

### 7.109 `open_or_create_wikilink_note` (Linha 3542)
- **Protótipo**: `static void open_or_create_wikilink_note(const char *note_name)`
- **Descrição**: Abre a nota referenciada por `[[wikilink]]`; se ela não existir, cria uma nota nova no banco (com cabeçalho `# Nome`) através do mesmo caminho de `create_new_note()`.
- **Retorno**: `static void`

### 7.110 `create_new_note` (Linha 3577)
- **Protótipo**: `static void create_new_note(const char *raw_name)`
- **Descrição**: Cria uma nota nova: apara espaços do nome, higieniza com `sanitize_note_name()`, e se já existir nota com esse nome apenas abre a existente; caso contrário insere no banco (`db_insert_note`) com o cabeçalho `# <título>`, refaz a varredura e abre a nota em aba.
- **Retorno**: `static void`

### 7.111 `replace_in_current_note` (Linha 3648)
- **Protótipo**: `static void replace_in_current_note(const char *find_str, const char *rep_str)`
- **Descrição**: Executa substituição de todas as ocorrências de um termo por outro na nota ativa.
- **Retorno**: `static void`

### 7.112 `rename_note` (Linha 3694)
- **Protótipo**: `static bool rename_note(const char *old_path, const char *raw_new_name)`
- **Descrição**: Renomeia a nota. No banco apenas a coluna `name` muda — o id (e portanto o caminho virtual) permanece, o que mantém as abas abertas válidas — e o cabeçalho `# ...` da primeira linha é reescrito nas abas afetadas. Notas de arquivo real continuam no caminho legado com `rename_utf8()`. Nome já existente é recusado nos dois casos.
- **Retorno**: `static bool`

### 7.113 `should_hide_links` (Linha 3903)
- **Protótipo**: `static inline bool should_hide_links(bool in_preview)`
- **Descrição**: Informa se os alvos de links Markdown devem ficar ocultos na exibição no Modo Normal.
- **Retorno**: `static inline bool`

### 7.114 `should_hide_markdown_styles` (Linha 3928)
- **Protótipo**: `static inline bool should_hide_markdown_styles(bool in_preview)`
- **Descrição**: Informa se os caracteres de estilo (** ou *) devem ficar ocultos no Modo Normal.
- **Retorno**: `static inline bool`

### 7.115 `find_markdown_styles` (Linha 3933)
- **Protótipo**: `static int find_markdown_styles(const Line *l, MarkdownStyleSpan *spans, int max_spans)`
- **Descrição**: Localiza spans de negrito (**texto**) e itálico (*texto*) dentro de uma linha de texto.
- **Retorno**: `static int`

### 7.116 `find_links_in_line` (Linha 3993)
- **Protótipo**: `static int find_links_in_line(const Line *l, LinkSpan *spans, int max_spans)`
- **Descrição**: Localiza links Markdown [texto](url) e [[wikilinks]] dentro de uma linha de texto.
- **Retorno**: `static int`

### 7.117 `activate_link_at_cursor` (Linha 4080)
- **Protótipo**: `static void activate_link_at_cursor(void)`
- **Descrição**: Executa a ação correspondente ao link sob o cursor (abre página web ou carrega nota local).
- **Retorno**: `static void`

### 7.118 `yank_visual_selection` (Linha 4099)
- **Protótipo**: `static void yank_visual_selection(bool linewise)`
- **Descrição**: Copia (yank) o texto selecionado no Modo Visual para a área de transferência do editor e do sistema.
- **Retorno**: `static void`

### 7.119 `delete_visual_selection` e `delete_visual_selection_ex`
- **Protótipo**: `static void delete_visual_selection(void)` / `static void delete_visual_selection_ex(bool do_yank)`
- **Descrição**: Remove o texto selecionado no Modo Visual. O parâmetro `do_yank` define se a seleção removida deve sobrescrever o buffer de cópia (para operações de corte/delete) ou preservá-lo intacto (para operações de colagem/substituição).
- **Retorno**: `static void`

### 7.119b `paste_replace_selection`
- **Protótipo**: `static void paste_replace_selection(void)`
- **Descrição**: Substitui a seleção ativa (Modo Visual, Shift+Setas ou Mouse no Modo Normal) pelo conteúdo da área de transferência com os atalhos `'y'` ou `'p'`, sem perda do texto a ser colado.
- **Retorno**: `static void`

### 7.120 `jump_to_next_link` (Linha 4275)
- **Protótipo**: `static void jump_to_next_link(void)`
- **Descrição**: Move o cursor do editor diretamente para o próximo link Markdown ou wikilink na nota.
- **Retorno**: `static void`

### 7.121 `is_table_line` (Linha 4361)
- **Protótipo**: `static bool is_table_line(const Line *l)`
- **Descrição**: Detecta se uma linha faz parte de uma tabela formatada com pipes ('|').
- **Retorno**: `static bool`

### 7.122 `is_delimiter_cell` (Linha 4369)
- **Protótipo**: `static bool is_delimiter_cell(const char *s)`
- **Descrição**: Detecta se uma célula de tabela é delimitadora ('---', ':---:', '---:').
- **Retorno**: `static bool`

### 7.123 `table_trim_copy` (Linha 4380)
- **Protótipo**: `static char *table_trim_copy(const char *src, int len)`
- **Descrição**: Copia uma célula de tabela Markdown aparando os espaços das pontas; usado pelo alinhamento de tabelas.
- **Retorno**: `static char *`

### 7.124 `align_table_lines` (Linha 4403)
- **Protótipo**: `static void align_table_lines(Line *lines, int start_y, int end_y)`
- **Descrição**: Calcula a largura de cada coluna e reformata um bloco de tabela Markdown para alinhamento uniforme.
- **Retorno**: `static void`

### 7.125 `table_align_at` (Linha 4566)
- **Protótipo**: `static bool table_align_at(int line_idx)`
- **Descrição**: Alinha a tabela situada na linha de índice informada.
- **Retorno**: `static bool`

### 7.126 `table_align_all` (Linha 4594)
- **Protótipo**: `static void table_align_all(void)`
- **Descrição**: Alinha todas as tabelas presentes no documento aberto.
- **Retorno**: `static void`

### 7.127 `table_jump_next_cell` (Linha 4618)
- **Protótipo**: `static void table_jump_next_cell(bool create_if_at_end)`
- **Descrição**: Pula o cursor para a próxima célula da tabela (tecla Tab no modo normal/tabela).
- **Retorno**: `static void`

### 7.128 `table_insert_row_and_advance` (Linha 4718)
- **Protótipo**: `static bool table_insert_row_and_advance(void)`
- **Descrição**: Insere uma nova linha na tabela preservando a formatação das colunas ao pressionar Enter.
- **Retorno**: `static bool`

### 7.129 `get_line_segments` (Linha 4774)
- **Protótipo**: `static int get_line_segments(const Line *l, int max_cols, LineSegment *segs, int max_segs, bool hide_links, bool hide_styles)`
- **Descrição**: Divide uma linha longa em segmentos visuais para exibição no modo de quebra de linha suave (wrap mode).
- **Retorno**: `static int`

### 7.130 `get_visual_col_in_seg` (Linha 4903)
- **Protótipo**: `static int get_visual_col_in_seg(const Line *l, int seg_start, int x_in_seg, bool hide_links, bool hide_styles)`
- **Descrição**: Retorna a coluna visual de tela correspondente a um deslocamento de byte dentro de um segmento.
- **Retorno**: `static int`

### 7.131 `get_byte_offset_in_seg` (Linha 4981)
- **Protótipo**: `static int get_byte_offset_in_seg(const Line *l, int seg_start, int seg_len, int target_col, bool hide_links, bool hide_styles)`
- **Descrição**: Retorna o deslocamento de byte dentro de um segmento a partir de uma coluna visual de tela.
- **Retorno**: `static int`

### 7.132 `find_visual_row` (Linha 5048)
- **Protótipo**: `static bool find_visual_row(bool in_preview, int target_vrow, int avail_w, int *out_line_idx, int *out_sub_idx, LineSegment *out_seg, int *out_total_vrows)`
- **Descrição**: Converte uma linha lógica (respeitando dobras e quebra de linha visual) na linha visual correspondente, usada para posicionar cursor e seleção.
- **Retorno**: `static bool`

### 7.133 `get_cursor_visual_pos` (Linha 5084)
- **Protótipo**: `static void get_cursor_visual_pos(int avail_w, int *out_vrow, int *out_vcol)`
- **Descrição**: Calcula a linha e coluna visual do cursor considerando o modo de quebra de linha.
- **Retorno**: `static void`

### 7.134 `move_cursor_visual_down` (Linha 5128)
- **Protótipo**: `static void move_cursor_visual_down(void)`
- **Descrição**: Move o cursor para baixo respeitando quebras de linhas visuais virtuais.
- **Retorno**: `static void`

### 7.135 `move_cursor_visual_up` (Linha 5174)
- **Protótipo**: `static void move_cursor_visual_up(void)`
- **Descrição**: Move o cursor para cima respeitando quebras de linhas visuais virtuais.
- **Retorno**: `static void`

### 7.136 `render_left_panel` (Linha 5224)
- **Protótipo**: `static void render_left_panel(ABuffer *ab, int row)`
- **Descrição**: Desenha o painel esquerdo: título, caixa de busca, a lista de notas na ordem de prioridade (tempo relativo, caixa de urgência colorida e marcadores de aberta/selecionada) e, na última linha, os botões de filtro ` [Todas] [1] [2] [3]` — com o rótulo compacto `[T]` quando o painel tem menos de 20 colunas.
- **Retorno**: `static void`

### 7.137 `render_tab_bar` (Linha 5652)
- **Protótipo**: `static void render_tab_bar(ABuffer *ab)`
- **Descrição**: Desenha a barra superior de abas de arquivos, botão de fechar, botão [+] e seletor de temas.
- **Retorno**: `static void`

### 7.138 `render_code_line` (Linha 5797)
- **Protótipo**: `static void render_code_line(ABuffer *ab, Line *l, int *cols_printed, int max_w)`
- **Descrição**: Desenha uma linha localizada dentro de um bloco de código com formatação e fundo escurecido.
- **Retorno**: `static void`

### 7.139 `is_bullet_list_line` (Linha 5987)
- **Protótipo**: `static bool is_bullet_list_line(const Line *l, int *bullet_pos, char *bullet_char)`
- **Descrição**: Detecta se uma linha é um item de lista não-ordenada (* ou -), identificando posição e caractere para coloração.
- **Retorno**: `static bool`

### 7.140 `render_right_panel` (Linha 6017)
- **Protótipo**: `static void render_right_panel(ABuffer *ab, int screen_row)`
- **Descrição**: Desenha a área principal de edição de texto, aplicando cores de tema, realces de busca e estilos Markdown.
- **Retorno**: `static void`

### 7.141 `render_status_bar` (Linha 6377)
- **Protótipo**: `static void render_status_bar(ABuffer *ab)`
- **Descrição**: Desenha a barra de status em duas linhas: badge do modo + nome da nota (ou `[Prévia] nome`) + posição do cursor na primeira; botões centralizados e clicáveis na segunda (`[➕ Nova]`, `[✏️ Renomear]`, `[🗑️ Excluir]`, wrap, `[🔍 Buscar]`, `[🔄 Subst]`, tema, `[🧬 Life]`, `[⬇ Importar]` e `[⬆ Exportar]`). Abaixo de 108 colunas (ou com `:wrapbar`) os rótulos ficam compactos.
- **Retorno**: `static void`

### 7.142 `render_command_line` (Linha 6614)
- **Protótipo**: `static void render_command_line(ABuffer *ab)`
- **Descrição**: Desenha a linha inferior de comando ':' ou prompts interativos com cursor de edição.
- **Retorno**: `static void`

### 7.143 `editor_scroll` (Linha 6737)
- **Protótipo**: `static void editor_scroll(void)`
- **Descrição**: Ajusta as variáveis de scroll vertical e horizontal para garantir que o cursor permaneça visível na janela.
- **Retorno**: `static void`

### 7.144 `refresh_screen` (Linha 6797)
- **Protótipo**: `static void refresh_screen(void)`
- **Descrição**: Único ponto que de fato desenha o quadro: baixa a dirty-flag (`g_needs_redraw = 0`), reinicia o batimento de manutenção, redetecta o tamanho da janela, resolve a limpeza de tela (`g_screen_clear_needed` / redimensionamento), fixa `status_bar_lines = 2`, compõe painel esquerdo, barra de abas, painel direito, barra de status e linha de comando no `ABuffer`, e termina posicionando o cursor com uma única escrita atômica.
- **Retorno**: `static void`

### 7.145 `set_status_msg` (Linha 6987)
- **Protótipo**: `static void set_status_msg(const char *fmt, ...)`
- **Descrição**: Define uma mensagem de notificação temporária na barra de status com temporizador de expiração.
- **Retorno**: `static void`

### 7.146 `queue_input_byte` (Linha 7008)
- **Protótipo**: `static void queue_input_byte(unsigned char b)`
- **Descrição**: Enfileira um byte no buffer interno de entrada do editor.
- **Retorno**: `static void`

### 7.147 `has_pending_input` (Linha 7014)
- **Protótipo**: `static bool has_pending_input(void)`
- **Descrição**: Verifica se há bytes pendentes aguardando leitura no buffer de entrada.
- **Retorno**: `static bool`

### 7.148 `read_raw_byte` (Linha 7046)
- **Protótipo**: `static int read_raw_byte(int timeout_ms)`
- **Descrição**: Lê um byte do terminal com controle de timeout em milissegundos sem bloqueio permanente.
- **Retorno**: `static int`

### 7.149 `convert_ansi_to_utf8` (Linha 7155)
- **Protótipo**: `static int convert_ansi_to_utf8(unsigned char c, char *out)`
- **Descrição**: Converte sequências ANSI antigas de caracteres estendidos para UTF-8 válido.
- **Retorno**: `static int`

### 7.150 `editor_read_key` (Linha 7187)
- **Protótipo**: `static int editor_read_key(void)`
- **Descrição**: Decodifica teclas pressionadas e sequências de escape ANSI/VT100 e eventos de mouse SGR 1006.
- **Retorno**: `static int`

### 7.151 `execute_delete_selected_note` (Linha 7415)
- **Protótipo**: `static void execute_delete_selected_note(void)`
- **Descrição**: Exclui a nota selecionada na lista: apaga a linha do banco (`db_delete_note`, ou o arquivo quando o caminho é real), fecha as abas dessa nota, descarta a prévia se for a nota exibida e refaz a varredura, ajustando a seleção.
- **Retorno**: `static void`

### 7.152 `handle_mouse_event`
- **Protótipo**: `static void handle_mouse_event(void)`
- **Descrição**: Trata os eventos de mouse (SGR 1006): insula os modos de prompt contra cliques acidentais; trata a barra de status (incluindo `[Importar]` e `[Exportar]`); a barra de abas; os botões responsivos de filtro `[Todas] [1] [2] [3] [H] [R] [O]`; e a lista lateral, onde o clique sobre a caixa de marcação (`cols 1..5`) cicla a criticidade e anula o registro de duplo-clique para evitar abertura involuntária da nota ao clicar rapidamente.
- **Retorno**: `static void`

### 7.153 `execute_bottom_command` (Linha 7903)
- **Protótipo**: `static void execute_bottom_command(const char *cmd)`
- **Descrição**: Interpreta e executa os comandos da linha inferior (`:w`, `:q`, `:wq`, `:zm`, `:tabnew`, `:rescan`, `:life`, etc., com os alias registrados). A partir da v6.33.0, inclui suporte aos comandos `:import [dir]` e `:export [dir]` (e seus alias `:importar` e `:exportar`), permitindo especificar o caminho diretamente ou abrir o prompt interativo.
- **Retorno**: `static void`

### 7.154 `handle_prompt_line_edit` (Linha 8140)
- **Protótipo**: `static bool handle_prompt_line_edit(char *buf, size_t max_sz, int *cursor, int c)`
- **Descrição**: Editor de linha única para prompts de diálogo com suporte a setas, Home, End, Backspace e UTF-8.
- **Retorno**: `static bool`

### 7.155 `terminal_write` (Linha 8231)
- **Protótipo**: `static void terminal_write(const char *s, int len)`
- **Descrição**: Escreve diretamente uma string de controle no descritor de saída do terminal.
- **Retorno**: `static void`

### 7.156 `run_game_of_life` (Linha 8240)
- **Protótipo**: `static void run_game_of_life(void)`
- **Descrição**: Executa o screensaver/minigame Conway's Game of Life em modo tela cheia até qualquer tecla ser pressionada.
- **Retorno**: `static void`

### 7.157 `editor_process_keypress` (Linha 8428)
- **Protótipo**: `static void editor_process_keypress(void)`
- **Descrição**: O núcleo da máquina de estados do editor: despacha teclas de acordo com o foco e modo ativos.
- **Retorno**: `static void`

### 7.158 `free_editor` (Linha 9945)
- **Protótipo**: `static void free_editor(void)`
- **Descrição**: Libera toda a memória dinâmica do editor (prévia, abas, linhas, pilhas de undo/redo, catálogo de notas, caches de busca, filtro e yank) e fecha o banco por último.
- **Retorno**: `static void`

### 7.159 `init_editor` (Linha 9996)
- **Protótipo**: `static void init_editor(void)`
- **Descrição**: Zera o estado global `E`, detecta o tamanho do terminal, abre o banco (`db_open`, que pode criá-lo) e varre as notas. Com o banco VAZIO não cria nota de exemplo: apenas exibe a mensagem que aponta para `[Nova]` e `[Importar]`.
- **Retorno**: `static void`

### 7.160 `should_refresh_now` (Linha 10045)
- **Protótipo**: `static bool should_refresh_now(void)`
- **Descrição**: Gate de renderização pelo dirty-flag (v6.28.0): decide se a próxima iteração do loop deve redesenhar. Retorna `true` se o estado visual mudou (`g_needs_redraw`), se a janela foi redimensionada ou a tela pede limpeza, se o marquee de título longo avançou o passo de 300 ms, se a mensagem de status expirou (5 s) ou se venceu o batimento de manutenção de 30 s (`IDLE_HEARTBEAT_MS`). Ao criar qualquer efeito dependente de tempo, adicione a condição aqui — senão a tela congela até a próxima tecla.
- **Retorno**: `static bool`

### 7.161 `main` (Linha 10076)
- **Protótipo**: `int main(int argc, char **argv)`
- **Descrição**: Ponto de entrada: faz `chdir()` para o diretório do executável, habilita o modo raw, chama `init_editor()`, abre o arquivo passado na linha de comando (se houver) e executa o loop principal, que agora só chama `refresh_screen()` quando `should_refresh_now()` autoriza e drena toda a entrada pendente antes de voltar a bloquear no teclado.
- **Retorno**: `int`

---

## 8. Guia Prático para IAs e Desenvolvedores: Como Continuar o Desenvolvimento

Ao expandir funcionalidades sobre a base atual, observe rigorosamente as diretrizes abaixo. Todas elas foram aprendidas em ciclos reais de manutenção deste código.

### 8.1 Invariantes Estruturais

1. **Double-buffering com `ABuffer`**: nunca chame `printf()`, `puts()` ou emita sequências de escape direto para `stdout` durante rotinas de desenho. Tudo é anexado ao acumulador com `ab_append(ab, string, tamanho)`; a escrita física acontece exclusivamente no fim de `refresh_screen()`. A única exceção deliberada é a sequência OSC 52 do clipboard, emitida fora do laço de desenho.
2. **Preservação de fronteiras UTF-8**: nunca divida strings por contagem de bytes sem alinhar com `utf8_align_start()`/`utf8_align_end()`/`utf8_charlen()`. Cortar um caractere multibyte (acentos, emojis) corrompe o quadro. Ao medir colunas, use `utf8_width`/`utf8_width_len` (largura visual, não bytes).
3. **Dirty-flag**: qualquer efeito novo dependente de tempo ou assíncrono **precisa** de condição em `should_refresh_now()`; qualquer mudança de estado visual precisa ligar `g_needs_redraw` (ou `g_screen_clear_needed`, quando a tela toda muda).
4. **Sincronização de abas**: antes de alterar `E.active_tab`, chame `tab_save_active()`; depois, `tab_load(idx)`. `Tab` e `E` compartilham os mesmos `Line*` e os snapshots de undo são copiados por `memcpy`. **Somente a `Tab` libera memória**; nunca chame `free` em `E.lines`/`E.undo_stack` (double free).
5. **Identidade da nota = caminho virtual (`"db:<id>"`)**: nunca monte caminho de nota à mão nem use chamadas de arquivo direto para conteúdo. Use `note_path_from_id()`/`note_id_from_path()` para identidade e `note_read_content()`/`note_write_content()` para conteúdo — assim o modo banco e o modo arquivo real (`.md` avulso) continuam funcionando.
6. **Ordem da lista = prioridade, depois data**: qualquer caminho novo que mexa na lista deve reordenar com `notes_resort_keep(path)`, que também recoloca a seleção sobre a nota indicada. Um `qsort(..., compare_notes_priority)` cru, sem recolocar a seleção, faz o cursor "pular" de nota.
7. **Atalhos globais x estado sem abas**: em `editor_process_keypress()` existe o bloco `if (E.num_tabs == 0) { ...; return; }`, que aceita apenas `i/I/a/A/o/O/n` e `:` e descarta o resto. **Qualquer atalho global novo precisa ser tratado antes desse guard (ou liberado dentro dele)**, senão não funciona no estado sem abas — e, como a v6.29.0 tornou o banco vazio o estado inicial de primeira execução, isso deixaria o programa impossível de fechar com `:q`.

### 8.2 Receitas de Extensão

**Novo comando `:`** — em `execute_bottom_command()`, no padrão `strcmp(cmd, "x") == 0` (a função hoje despacha dezenas de nomes/aliases). Exemplo:
```c
if (strcmp(cmd, "contar") == 0 || strcmp(cmd, "wc") == 0) {
    set_status_msg("Total de linhas: %d", E.num_lines);
    return;
}
```
Ao final, adicione o novo comando à mensagem de ajuda do rodapé, se houver.

**Novo botão clicável na barra de status** — três pontos obrigatórios:
1. Campos `btn_x_start`/`btn_x_end` na struct `Editor`;
2. Desenho em `render_status_bar()`, gravando as colunas reais (`cur_col`) e **somando a largura em `btns_w`** (é ele que centraliza a barra); se o rótulo puder encolher, use o mesmo par rótulo/largura do padrão `compact_btns`;
3. Hit-test em `handle_mouse_event()`, na linha da barra de status, comparando `E.mouse_col` com os limites gravados.

**Novo modo de prompt** — seis pontos, sem exceção:
1. Entrada em `enum EditorMode`;
2. Incluir o novo valor na faixa `E.mode >= MODE_PROMPT_NEW_NOTE && E.mode <= MODE_PROMPT_<ÚLTIMO>` — a faixa aparece em **quatro lugares** (guarda de colagem, guarda de Alt no leitor de teclas, insulação do mouse e despachante de teclas);
3. Ramo de `Enter`/`Esc` no despachante de teclas;
4. Desenho do prompt em `render_command_line()`;
5. Cálculo da coluna do cursor em `refresh_screen()`;
6. O texto do rótulo precisa ser **o mesmo** nos dois últimos — defina uma macro (como `PROMPT_IMPORT_LABEL`) e use-a nos dois.

**Nova coluna no banco** — `db_open()` (schema + `PRAGMA user_version`), o CRUD `db_*` correspondente, a montagem do campo em `rescan_notes()` (linhas do banco ➔ `NoteInfo`) e, se for atributo por nota, o par `save_attention_flags()`/`db_set_attention()` como referência. Se a informação precisar sobreviver a exportação/importação, lembre que os `.md` são markdown puro.

**Novo tema** — basta uma entrada no array `themes[]` (a paleta é usada por macros que resolvem para o tema corrente); `NUM_THEMES` acompanha.

**Rotulagem compacta por largura** — quando um controle não cabe, encolha o **rótulo**, não deixe o desenho estourar o painel. Padrão já usado: `int compacto = (w < 20);` com um par rótulo/largura (`"[T]"`/3 contra `"[Todas]"`/7) e os campos `_start`/`_end` gravados a partir do rótulo **realmente usado** — senão o clique desalinha do desenho. Na barra de status o gatilho é `E.screen_cols < 108` (ou `:wrapbar`).

### 8.3 Compilar, Testar e Versionar

```sh
cd /mnt/SSDInterno/PROJETOS/Notas_Definitivas
./build.sh all             # Linux + Windows; exige -lsqlite3 e tools/sqlite3-mingw/
./tools/test/run_tests.sh  # testes de unidade (Linux e Windows via wine)
```
- **Linux**: `sqlite3.h` em `/usr/include` e `-lsqlite3` (pacote `sqlite`; no Arch/Omarchy: `sudo pacman -S sqlite`).
- **Windows/MinGW**: não existe pacote `mingw-w64-sqlite3` no Arch; a libsqlite3 para Windows é gerada uma vez a partir da *amalgamation* oficial e versionada em `tools/sqlite3-mingw/` (~1,3 MB), com `tools/build_sqlite_mingw.sh`.
- **Definição de pronto**: (1) `gcc` **e** `mingw` com **zero warnings** em `-Wall -Wextra`; (2) `run_tests.sh` verde nas duas plataformas; (3) suíte funcional de interface verde; (4) snapshot em `backup/vX.Y.Z_<slug>/` + seção nova no topo do `README.md` + commit em português descrevendo o que mudou **com as medições** que sustentam a mudança.

### 8.4 Armadilhas Conhecidas (não repetir)

- **Nunca canalize a saída do `gcc` para `head`**: o `SIGPIPE` mata o compilador e você vê "exit 2" sem binário, culpando o código.
- **O build MinGW revela avisos que o Linux nunca mostra** (código dentro de `#ifdef _WIN32`), tipicamente `-Wformat-truncation` de `%s` + prefixo em buffer do mesmo tamanho. Conserte dimensionando o buffer; ao dimensionar, confira também o buffer consumidor (`MultiByteToWideChar` **falha**, não trunca, se o destino for pequeno — alinhe os dois).
- **Cada execução cria/altera o `notas.db` ao lado do binário** (o programa faz `chdir()` para o diretório do executável; em WAL aparecem também `-wal` e `-shm`). Em teste, use um diretório de trabalho por execução, senão a segunda rodada encontra o banco populado pela primeira e "falha" um teste de banco vazio.
- **Mudar o estado inicial expõe bugs de estados raramente alcançados**: a v6.29.0 tornou "banco vazio, sem abas" o estado de primeira execução, e foi exatamente aí que surgiu o `:q` inoperante. Ao mexer no que o programa faz na partida, teste o caminho de primeira execução (diretório/banco limpos) e as saídas triviais (`:q`, `:q!`, `Esc`).
- **Extração de quadro em pty (testes)**: separe os quadros por `b'\x1b[H'`. **Não** use `b'\x1b[?25l'`: sem abas abertas o editor esconde o cursor no fim do quadro e você recebe "quadro vazio".
- **Clique do mouse** (SGR 1006, `\x1b[<0;COL;ROWM`) exige a coluna visual calculada com o modelo do próprio editor. Detalhe contraintuitivo na faixa de emojis: `➕` (U+2795) e `⬇`/`⬆` valem **1** coluna; `🔍`/`🎨`/`🧬`/`🗑️` (>= 0x1F000) valem **2**.
- **Clique x guarda de largura do painel**: quase todo o tratamento de clique do painel esquerdo vive dentro de `if (E.mouse_col <= E.left_width)`. Um controle **desenhado além da largura do painel** fica inalcançável — foi o bug dos filtros `[2]`/`[3]`. Ao criar um controle novo, ou mantenha-o dentro do painel, ou trate o clique **fora** desse guarda.
- **Marcador `●` na lista não é a seleção**: `●` = nota aberta na aba ativa; a selecionada é `▸`, e quando as duas coincidem vence o `●`. Em teste de interface, verifique a seleção por comportamento (qual nota mudou de nível no banco), não pelo marcador.
- **`data/` não é mais o armazenamento**: apagar `notas.db` zera a biblioteca. As marcações de urgência vivem só na coluna `attention` do banco e **não vão** nos `.md` exportados (limitação conhecida, documentada no README; há um pedido em aberto por um arquivo de metadados ao lado dos `.md` que torne o *round trip* sem perda).
- **Console do wine**: reporta geometria diferente (~57 linhas), então cliques na UI não são confiáveis ali — e o app está **correto** (usa `csbi.srWindow`). Para validar código Windows-only, use o binário de teste de unidade. No console Windows as linhas chegam como `\r\r\n`.

### 8.5 Estado e Backlog Conhecido

- `status_bar_lines` é forçado a 2 em `refresh_screen()`: o `:wrapbar` documentado (barra em 1 linha) hoje só alterna a compactação dos botões — meia-verdade.
- O `README.md` e este documento agora descrevem a base correta; a antiga `DESCRICAO_TECNICA_DETALHADA_V6.27.0.md` (base v6.27.0, redesenho a 150 ms, armazenamento em `data/`) foi substituída por este arquivo.
- `:import [dir]` e `:export [dir]` existem desde a v6.33.0, com fallback para prompt interativo quando o diretório é omitido. Os botões `[Importar]`/`[Exportar]` continuam sendo o caminho mais descoberto na interface.
- Sem assinatura de código: o `notas.exe` pode cair no SmartScreen (Propriedades ➔ Desbloquear).

---

## 9. Rodada de Endurecimento (pós-v6.33.0)

Esta seção documenta as mudanças feitas para tornar o editor mais resistente a falhas de memória, disco, terminal e dados corrompidos, além das otimizações de renderização e undo.

### 9.1 Persistência e Integridade

| Função / área | Antes | Agora |
| :--- | :--- | :--- |
| `write_file_atomic()` (nova) | `fopen("wb")` truncava o destino antes da escrita; falha no meio deixava arquivo parcial. | Grava em `<arquivo>.tmp.<pid>`, confere `fwrite`/`fflush`, chama `fsync`/`_commit`, fecha e só então substitui via `rename`/`MoveFileExW`. |
| `rename_utf8()` | `_wrename` falhava se o destino existisse. | `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`, necessário para a troca atômica. |
| `db_open()` | Falha de schema deixava `g_db` aberto; `PRAGMA user_version` era sempre sobrescrito. | Lê a versão antes, recusa banco mais novo, fecha o handle em qualquer falha (`sqlite3_close_v2`) e só grava a versão após criar tabela/índice. |
| `note_id_from_path()` | `atoi("db:abc")` virava 0; `db:12lixo` virava 12. | `strtol` com `ERANGE`, resto da string e faixa `1..INT_MAX`; `note_path_from_id()` rejeita id <= 0. |
| `db_set_attention()` / `db_insert_note()` / `rescan_notes()` | Níveis fora de 0..6 podiam entrar no banco/UI. | Clamp 0..6 na escrita e na leitura; valor corrompido vira 0. |
| `toggle_note_attention_at()` / comando `:atencao` | UI mudava mesmo se o `UPDATE` falhasse. | Persiste primeiro; em erro, mostra mensagem e não reordena nem altera o nível em memória. |
| `import_notes_from_dir()` | `BEGIN`/`COMMIT` não verificados; falha de conversão de caminho no Windows saía sem `ROLLBACK`; buffers fixos de 1024/2048 bytes. | Verifica `BEGIN`, faz `ROLLBACK` em todos os caminhos de erro, só confirma com `COMMIT` verificado, usa caminhos dinâmicos (`join_path_alloc`, `utf8_to_wide_alloc`) e rejeita nomes que não cabem em vez de truncar/colidir. |
| `export_notes_to_dir()` | `mkdir` e `fwrite` ignorados; retornava contagem mesmo com todas as gravações falhando. | Verifica criação da pasta, largura do caminho, escrita/fechamento e retorna `-1` se nada foi exportado; exporta com escrita atômica. |
| `execute_delete_selected_note()` | Ignorava falha de exclusão e seguia fechando abas/anunciando sucesso. | Só prossegue se `db_delete_note()`/`unlink_utf8()` retornarem sucesso. |

### 9.2 Memória, Buffers e UTF-8

- **Tabelas Markdown**: as três rotinas que criavam linha nova usavam `char new_row[512]` e calculavam `4 * colunas` sem limite. Uma linha com muitos `|` estourava a pilha. Agora `table_insert_blank_row()` monta a linha em `ABuffer`, e o número de colunas é limitado a `MAX_TABLE_COLS` (32). `align_table_lines()` **aborta sem alterar nada** quando uma linha tem mais de 32 colunas, em vez de reconstruí-la sem as colunas extras.
- **`line_replace_range()`**: detecta quando `new_str` aponta para dentro da própria linha (o `realloc` invalidaria o ponteiro), copia antes e valida `del_len`/`new_len`/overflow. Isso elimina use-after-realloc em substituições in-place.
- **`ABuffer`**: ganhou a flag `failed`; `ab_append()` checa overflow de `int` e falha de `realloc`, mantendo o conteúdo íntegro. `lines_to_content()` e a renderização usam essa flag para abortar em vez de salvar/desenhar dados truncados.
- **`table_trim_copy()`**: alocação verificada; em OOM o alinhamento é abortado sem liberar células pela metade.
- **Clipboard**: `base64_encode()` foi reescrito. O cálculo antigo de padding gerava `YQ`/`YWI` em vez de `YQ==`/`YWI=`, então o OSC 52 de textos de 1 ou 2 bytes era rejeitado por terminais. A leitura Win32 agora limita a conversão ao `GlobalSize()` do handle, sem confiar em NUL terminador.
- **Conversão UTF-8 ➔ UTF-16**: `utf8_to_wide_alloc()` substitui buffers fixos de 1024/4096 WCHAR em `fopen_utf8`, `unlink_utf8`, `rename_utf8`, `mkdir_utf8`, `stat_cross` e na abertura de URL. Caminhos com acentos e comprimento maior deixam de falhar ou passar buffers não inicializados ao `ShellExecuteW`.
- **Concatenação e Buffers Seguros**: adicionada a função `safe_strcat(dest, sz, src)` que substitui chamadas frágeis de `strncat` na barra de status (`preview_name`); substituição de `snprintf` dinâmico em `join_path_alloc()` e `rename_note()` por cópia `memcpy` com capacidade garantida; buffers de escape ANSI de posicionamento (`cpos`) expandidos de 32 para 64 bytes em `refresh_screen()`, eliminando todos os alertas de `-Wformat-truncation=2`.
- **Parsing de Comandos `:s/` e `:sub/`**: em `execute_bottom_command()`, o comando `:sub/` iniciava com 's' e caía na mesma regra de `:s/`, consumindo 2 caracteres em vez de 4 e transformando o padrão em `b/<busca>/...`. O cálculo de prefixo foi corrigido com checagem de `sub/` antes de `s/`, e os buffers de busca e substituição foram expandidos para 256 bytes com verificação de overflow, prevenindo truncamento ou apagamento silencioso de textos $\ge 128$ bytes.
- **Centralização de Prompts Modais (`prompt_label`)**: as rotinas `render_command_line()`, posicionamento de cursor em `refresh_screen()` e hit-testing do mouse em `handle_mouse_event()` continham definições literais e larguras repetidas dos 7 prompts modais. A introdução de `prompt_label(buf, sz)` e `prompt_label_color(mode)` unificou toda a lógica, reduziu ~140 linhas redundantes e corrigiu o hit-testing de mouse para `MODE_PROMPT_IMPORT_DIR` e `MODE_PROMPT_EXPORT_DIR`, que anteriormente ficavam com largura zero.

### 9.3 Desempenho

**Renderização (`RenderCursor`)** — o problema mais custoso: `render_right_panel()` chamava `find_visual_row()` uma vez por linha visível, e `find_visual_row()` percorria o documento **inteiro** para calcular o total, mesmo sem solicitá-lo. Para uma nota de 20.000 linhas e 40 linhas visíveis, eram 800 varreduras completas por quadro. O `RenderCursor` (global, resetado a cada `refresh_screen`) localiza o scroll uma vez e avança um segmento por chamada; `find_visual_row()` também passou a retornar assim que encontra a linha quando o total não é pedido.

Medição em harness isolado (Windows/MinGW, `-O2`, 20.000 linhas de 159 caracteres, 40 linhas visíveis, 20 quadros):

| Implementação | Tempo |
| :--- | ---: |
| `find_visual_row()` por linha (antes) | ~6.262 ms |
| `RenderCursor` sequencial (depois) | ~0,1 ms |

`editor_scroll()` também deixou de varrer o documento inteiro para descobrir o total: conta apenas as linhas visuais a partir do scroll atual e só limita quando a tela realmente termina antes.

**Undo** — `save_undo_for_typing()` agrupa teclas de caractere consecutivas na mesma linha/posição dentro de 800 ms. Antes, cada tecla copiava todas as linhas do documento e consumia um dos 100 níveis; agora uma frase inteira ocupa um nível. Edições estruturais continuam gerando snapshots independentes.

**Substituição (`:s/...`)** — `replace_in_current_note()` usa `strstr()` para localizar blocos e anexa trechos inteiros ao `ABuffer`, em vez de chamar `ab_append()` byte a byte. Também só cria o snapshot de undo quando encontra a primeira ocorrência, preservando o redo em buscas sem resultado.

### 9.4 Terminal, Tempo e Plataforma

- `write_stdout_all()` substitui `WriteFile`/`write` de uma tacada: trata escrita parcial, `EINTR` e buffer nulo. O quadro só é emitido se o `ABuffer` estiver completo; caso contrário, limpeza e redesenho são reagendados.
- `get_current_time_ms()` retorna `int64_t`; no Windows usa `GetTickCount64()` e não sofre mais o wrap de ~49,7 dias. `first_unsaved_time`, deadlines de undo, timers de clique/marquee e autosave acompanham o tipo.
- `get_window_size()` migrou de `atoi()` para `strtol` com validação de formato e limites de segurança (1..10000 linhas/colunas) para as variáveis de ambiente `LINES` e `COLUMNS`.
- `disable_raw_mode()` é idempotente (`g_terminal_shutdown_done`), evitando liberar o editor duas vezes por causa do `atexit`.
- `main()` configura o code page em `enable_raw_mode()` (que guarda os valores anteriores) e usa `GetModuleFileNameW`/`SetCurrentDirectoryW`; uma pasta de instalação com acentos deixa de quebrar a descoberta do executável.
- No POSIX, `open_link_target()` troca `system("xdg-open ...")` por `fork`/`exec` com `xdg-open -- <alvo>`; o conteúdo de uma nota não pode mais injetar comandos de shell. `read_entire_file()` usa `fseeko`/`ftello` e rejeita leitura curta/arquivo > `LONG_MAX`.

### 9.5 Abas, Saída e Sem Perda de Dados

- `:q`/`:qa` verificam `any_tab_modified()`, não apenas `E.modified` da aba ativa.
- `:wq`/`:x`/`:wqa` usam `save_all_modified_tabs()`: salvam todas as abas e só saem se todas gravarem.
- `tab_close()` cancela o fechamento se a aba modificada não puder ser salva.
- `close_all_tabs()` salva todas as abas antes de liberar qualquer uma e retorna `false` no primeiro erro, mantendo o workspace intacto.
- `:tabnew` cria uma nota real no SQLite (`create_new_note("Nova_Nota_N")`); antes abria um caminho em `data/` que podia não existir e falhava no primeiro `:w`.
- O fallback de renomeação de arquivo real agora renomeia **no mesmo diretório**, em vez de mover para `data/`.

### 9.6 Testes e Validação

A cobertura automatizada divide-se em três pilares:

1. **Camada de Armazenamento (`tools/test/storage_test.c`)**: 27 verificações estritas cobrindo:
   - `db_open` e banco nascendo vazio;
   - montagem, validação e rejeição de IDs virtuais (`db:0`, `db:12lixo`, `db:-1`);
   - Base64 de 1, 2 e 3 bytes com padding correto;
   - gravação atômica com conferência de integridade byte a byte;
   - importação em lote com transação atômica e exportação;
   - CRUD completo de notas e atualização de criticidade.
2. **Camada de Edição (`tools/test/editor_test.c`)**: 28 verificações sem TUI cobrindo:
   - comandos `:s/` e `:sub/` com substituição de 200 bytes, padrões sem barra final e proteção de buffer;
   - ciclos completos de `undo` e `redo`, preservação de posição de cursor e estado de dobras recolhidas;
   - coalescência temporal de digitação contínua (agrupamento de 3+ caracteres em 1 snapshot de undo);
   - consistência dos rótulos modais em `prompt_label()`.
3. **Fidelidade Visual Golden Master (`golden.c`)**:
   - Captura e comparação de 1,39 MB de sequências de escape ANSI em 198 estados de tela (`refresh_equal = True`), assegurando ausência de qualquer regressão visual para caracteres ASCII, acentuações UTF-8 e ideogramas CJK.

Validação estrita: compilação Windows/MinGW com `-Wall -Wextra -Wformat=2 -Wformat-truncation=2` com **zero avisos**; `-fanalyzer` e `cppcheck` limpos; UBSan com trap mode ativo sem falhas.

### 9.7 Rodada de Desempenho e Segurança

| Função / área | Antes | Agora |
| :--- | :--- | :--- |
| `ab_append_text()` (nova) | Texto de notas, títulos, prompts e mensagens ia cru para o terminal; um `.md` importado com `ESC` podia emitir OSC 52 (sobrescrever o clipboard), mudar o título ou reposicionar a tela. | Todo texto de origem não confiável passa por `ab_append_text()`, que descarta C0, DEL e C1 codificado em UTF-8 (U+0080..U+009F). Esses codepoints já têm largura 0, então o layout não muda. Escapes do próprio editor seguem em `ab_append()`. |
| `editor_load_content()` | `buf_insert_line()` por linha, cada uma recalculando dobras do documento inteiro: O(n²). 50 mil linhas ≈ 2,2 s. | `buf_insert_line_raw()` + um único `update_folding()` no fim: < 1 ms para 50 mil linhas. |
| `buf_reserve_lines()`, `buf_insert_line_raw()`, `buf_delete_lines()` (novas) | Crescimento sem checagem de overflow; excluir k linhas fazia k `memmove` + k recálculos. | Capacidade geométrica com checagem de overflow; exclusão de intervalos com um `memmove` e um recálculo. |
| `editor_paste_string()` | Uma inserção (memmove + recálculo) por quebra de linha; falha de memória no meio truncava a linha do cursor, perdendo o texto após ele. | Colagem em lote: todas as alocações acontecem antes de alterar o documento; em falha nada muda. LF, CRLF e CR isolado continuam aceitos. |
| `line_init()` / `line_set()` / `snapshot_lines()` | `malloc` sem checagem (NULL desreferenciado); três cópias duplicadas do snapshot de undo. | `line_init()` devolve `bool`; `line_set()` troca o conteúdo sem destruir o original em falha; `snapshot_lines()` centraliza undo/redo e libera cópias parciais. |
| `delete_visual_selection_ex()` | Se a junção das pontas falhasse, as linhas do meio e a última eram apagadas mesmo assim. | Aborta sem tocar no documento. |
| Bracketed paste | `realloc` falho interrompia a leitura e o resto do texto colado era executado como comandos do Modo Normal; sem limite de tamanho (overflow de `int`). | Lê sempre até `ESC[201~`, limite de 64 MB (`MAX_PASTE_BYTES`), bytes `ESC` do texto não viram conteúdo. |
| `get_system_clipboard()` | Win32 `CF_TEXT` usava `strdup` sem limite; POSIX devolvia clipboard truncado em falha e podia estourar `int`. | Leitura limitada a `GlobalSize()`; POSIX drena o pipe, limita a 64 MB e não devolve texto truncado. |
| `refresh_screen()` | `ABuffer` novo a cada quadro (~8 `realloc`). | Buffer de quadro persistente (`ab_reset()`), liberado após falha ou quadro > 8 MB. |
| `db_get_content()` / `db_update_content_len()` | `prepare` a cada leitura; gravação usava `strlen` (truncava em NUL). | Statement reaproveitado (finalizado em `db_close()`); gravação com o tamanho real. |
| `db_open()` | — | `SQLITE_DBCONFIG_DEFENSIVE` e `trusted_schema=OFF` contra um `notas.db` adulterado. |
| `note_update_cache()` / `rescan_notes()` | Cópia intermediária do texto via `snprintf` antes de normalizar; busca do cache antigo O(n²). | Normalização direta no destino; reaproveitamento do cache por `bsearch` sobre o id. |
| `replace_in_current_note()` | Copiava a linha montada outra vez com `line_init`. | Adota o buffer montado. |

`editor_test.c` ganhou casos para colagem multilinha (LF/CRLF/CR, cursor, undo), carga em lote de 50 mil linhas com detecção de blocos de código, exclusão de intervalos e sanitização de escapes.

---

*Documento atualizado para o estado pós-v6.33.0 (`notas.c`, 11.237 linhas). O catálogo da seção 7 preserva os números de linha da v6.30.0; a seção 9 descreve a rodada atual e as funções novas.*
