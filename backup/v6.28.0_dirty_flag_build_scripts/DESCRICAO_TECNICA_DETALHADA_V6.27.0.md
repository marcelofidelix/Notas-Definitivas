# Arquitetura e Manual Técnico — Notas Definitivas v6.27.0
> **Documento de Engenharia de Software e Especificação Técnica Integral**  
> **Versão Alvo**: `v6.27.0_bullet_lists_coloridos_negrito_e_colar_p_windows`  
> **Arquivo Fonte Principal**: `notas.c` (em `backup/v6.27.0_bullet_lists_coloridos_negrito_e_colar_p_windows/notas.c`) — 9.336 linhas em C99 puro  
> **Escopo Restrito**: Descreve única e exclusivamente os componentes, estruturas, fluxos e funcionalidades presentes na versão 6.27.0.

---

## 1. Visão Geral e Filosofia de Design da Versão 6.27.0

O **Notas Definitivas (v6.27.0)** é um editor pessoal de notas em Markdown voltado para terminais de texto (TUI), construído em **C99 puro como uma unidade de tradução única (*Single Translation Unit*)**, sem qualquer dependência externa além da biblioteca padrão C e das APIs nativas do sistema operacional (zero dependência de `ncurses`, `glib` ou bibliotecas de terceiros).

### 1.1 Princípios Fundamentais da Arquitetura
1. **Unidade Única de Tradução (Single-File C Architecture)**: Todo o código-fonte executável reside estritamente em um único arquivo (`notas.c`), permitindo compilação direta com:
   - **Linux**: `gcc -O2 -Wall -Wextra src/notas.c -o notas -ldl`
   - **Windows**: `gcc -O2 -Wall -Wextra src/notas.c -o notas.exe -lkernel32 -luser32 -lshell32`
2. **Armazenamento Direto no Sistema de Arquivos (Flat-File Markdown)**:
   - Todas as notas residem como arquivos de texto convencionais (`.md`) dentro do diretório local `data/`.
   - O título de cada nota é derivado do primeiro cabeçalho de primeiro nível (`# Título`) ou do próprio nome do arquivo no disco.
   - A persistência dos níveis de urgência é realizada em um arquivo de metadados simples: `data/.attention`.
   - Não há bancos de dados relacionais, motores binários embutidos ou dependências de bibliotecas de armazenamento externo nesta versão.
3. **Renderização ANSI TrueColor (24 bits) sem Ncurses**:
   - Emite sequências de escape ANSI de 24 bits (`\x1b[38;2;R;G;Bm` para texto e `\x1b[48;2;R;G;Bm` para fundo).
   - Utiliza um buffer dinâmico de composição em memória (`ABuffer`) que acumula todas as operações gráficas antes de despejá-las no terminal em uma única chamada de escrita atômica, eliminando cintilação (*flickering*).
4. **Paridade Multiplataforma Total (POSIX Linux vs Windows Win32)**:
   - No Linux, opera via `termios`, sinais `SIGWINCH` e multiplexação de descritores de arquivo.
   - No Windows, consome o Virtual Terminal do Windows 10/11 (`ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT`), permitindo o uso nativo no prompt de comando, PowerShell e Windows Terminal.
   - Suporte completo a nomes de arquivos Unicode com acentos no Windows através das chamadas de API Wide (`FindFirstFileW`, `DeleteFileW`, `MoveFileExW`).
5. **Ergonomia Híbrida (Vim + Produtividade Moderna)**:
   - Modelo modal: `Modo Normal`, `Modo Inserção`, `Modo Visual Caractere`, `Modo Visual Linha`, `Modo Comando` e modos de diálogo (`Prompt`).
   - Abas dinâmicas no topo com botão de fechar (`x`), botão de adicionar aba (`[+]`) e rotação de títulos longos (*marquee*).
   - Painel esquerdo com lista de notas ordenadas pela data de modificação recente e busca difusa em tempo real (`Ctrl+F`).
   - Suporte total ao mouse (SGR 1006): alternância de abas, navegação na lista lateral, seleção de texto e rolagem independente de painéis.

---

## 2. Funcionalidades de Destaque da Versão 6.27.0

A versão 6.27.0 introduziu dois pilares fundamentais no editor:

### 2.1 Renderização de Bullet Lists (`*` e `-`) em Cores e Negrito
- **Identificação Precisa**: A função `is_bullet_list_line()` analisa a linha de texto, ignora recuos (espaços e tabulações) e detecta se o primeiro caractere visível é um hífen (`-`) ou asterisco (`*`) seguido de espaço ou início de checklist (`- [ ]`, `* [x]`).
- **Destaque Visual**: O marcador `-` é renderizado em negrito com a cor amarela do tema (`TH_FG_YELLOW`), enquanto o marcador `*` é renderizado em negrito com a cor de acento (`TH_FG_ACCENT`). O texto subsequente permanece na cor de leitura padrão (`TH_FG_MAIN`).
- **Filtro de Falsos Positivos**: O algoritmo desconsidera deliberadamente linhas horizontais divisórias (`---`, `***`, `- - -`), marcadores inline de formatação (`*texto*`, `**texto**`) e parâmetros de comando iniciados por traço.

### 2.2 Colagem do Windows (`p`, `P` e `Ctrl+V`) com Fidelidade Total a Acentos
- **Acesso Direto à Área de Transferência**: No Windows, ao pressionar `p` ou `P` no Modo Normal, ou `Ctrl+V` no Modo Inserção, o programa acessa o clipboard do sistema via `OpenClipboard(NULL)` e requisita o formato `CF_UNICODETEXT`.
- **Conversão de UTF-16 para UTF-8**: O texto capturado do Windows é convertido para UTF-8 através da função `WideCharToMultiByte(CP_UTF8)`, preservando perfeitamente caracteres acentuados da língua portuguesa (`ç`, `ã`, `é`, `ó`, `ú`, etc.).
- **Substituição Visual**: No Modo Visual, pressionar `p` apaga a seleção atual e insere o conteúdo da área de transferência no local exato.

---

## 3. Arquitetura de Dados e Estruturas Fundamentais

Todas as estruturas de dados desta versão estão declaradas no cabeçalho de `notas.c`:

### 3.1 `Theme` — Paleta TrueColor (24 bits)
Define a identidade cromática completa dos 8 temas embutidos (Tokyo Night, One Dark Pro, Dracula, Monokai, GitHub Dark, GitHub Light, VS Code Light+, Solarized Light):
```c
typedef struct {
    const char *name;            /* Nome do tema para exibição na UI */
    bool is_dark;                /* Flag: true para escuro, false para claro */
    const char *bg_main;         /* Cor de fundo do editor principal */
    const char *bg_sidebar;      /* Cor de fundo do painel lateral esquerdo */
    const char *bg_status;       /* Cor de fundo da barra de status inferior */
    const char *bg_select;       /* Cor de fundo de seleção no modo visual */
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

### 3.2 `ABuffer` — Buffer Dinâmico de Renderização
Estrutura que acumula em memória todas as sequências de escape e caracteres de texto antes da escrita no terminal:
```c
typedef struct {
    char *b;  /* Bloco de bytes alocado dinamicamente */
    int len;  /* Quantidade de bytes utilizados */
    int cap;  /* Capacidade máxima alocada */
} ABuffer;
```

### 3.3 `Line` — Linha do Documento Markdown
Representa cada linha de texto no buffer ativo do editor:
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

### 3.4 `UndoState` — Histórico de Desfazer e Refazer
Snapshots completos das linhas do documento e posição do cursor (até 100 níveis por aba):
```c
typedef struct {
    Line *lines;   /* Cópia em profundidade das linhas */
    int num_lines; /* Quantidade de linhas */
    int cursor_y;  /* Posição vertical do cursor */
    int cursor_x;  /* Posição horizontal do cursor */
} UndoState;
```

### 3.5 `Tab` — Estado Isolado por Aba
Permite a edição simultânea de até 32 arquivos (`MAX_TABS`):
```c
typedef struct {
    char cur_filename[256];        /* Nome do arquivo (ex: "Projeto.md") */
    char cur_filepath[512];        /* Caminho relativo no disco (ex: "data/Projeto.md") */
    char title[256];               /* Título extraído do cabeçalho ou nome */
    Line *lines;                   /* Array de linhas do documento */
    int num_lines;                 /* Quantidade de linhas ativas */
    int cap_lines;                 /* Capacidade alocada do array */
    bool modified;                 /* Flag de modificação não salva */
    long first_unsaved_time;       /* Timestamp da primeira alteração não salva */
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

### 3.6 `NoteInfo` — Catálogo de Notas da Pasta `data/`
Mantém o índice em memória das notas existentes no disco:
```c
typedef struct {
    char filename[512];      /* Nome do arquivo no disco */
    char filepath[512];      /* Caminho relativo (ex: "data/ideias.md") */
    char title[256];         /* Título limpo ou extraído do cabeçalho # */
    time_t mtime;            /* Data de modificação para ordenação cronológica */
    char rel_time[32];       /* Texto formatado (ex: "10m atrás", "09/10/26") */
    char *search_norm;       /* Cache textual normalizado (minúsculas, sem acentos) */
    int search_norm_len;     /* Tamanho do cache de busca */
    int attention_level;     /* Nível de urgência: 0 = nenhuma, 1 = Crítica, 2 = Média, 3 = Baixa */
    bool needs_attention;    /* true se attention_level > 0 */
} NoteInfo;
```

### 3.7 `Editor` — O Estado Global `E`
Encapsula o estado completo da aplicação durante o ciclo de execução:
- **Janela e Layout**: `screen_rows`, `screen_cols`, `left_width` (largura ajustável do painel esquerdo, mínimo 18).
- **Foco e Modos**: `focus` (`FOCUS_LEFT` ou `FOCUS_RIGHT`), `mode` (`MODE_NORMAL`, `MODE_INSERT`, `MODE_VISUAL_CHAR`, etc.).
- **Abas**: `tabs`, `num_tabs`, `active_tab`, coordenadas geométricas das abas na tela (`tab_coords`) para suporte a clique de mouse.
- **Botões e Filtros da UI**: Coordenadas de início e fim dos botões interativos inferiores:
  - `btn_new`: `[Nova]`
  - `btn_rename`: `[Renomear]`
  - `btn_delete`: `[Excluir]`
  - `btn_wrap`: `[Quebra]`
  - `btn_find`: `[Buscar]`
  - `btn_replace`: `[Subst]`
  - `btn_life`: `[Life]`
  - `btn_filter_all`: `[Todas]`
  - `btn_filter_1`: `[1]` (Crítica / Vermelho)
  - `btn_filter_2`: `[2]` (Média / Laranja)
  - `btn_filter_3`: `[3]` (Baixa / Amarelo)
- **Filtro de Urgência**: `filter_urgency` (0 = todas, 1 = nível 1, 2 = nível 2, 3 = nível 3).
- **Busca e Substituição**: `replace_find_buf`, `replace_with_buf`, `search_query`, `filtered_indices`.
- **Histórico de Navegação**: `history[MAX_HISTORY][512]` com navegação por `Backspace` e `Ctrl+Backspace`.
- **Mouse e Encerramento**: Rastreamento de botões e coordenadas do mouse, e flag `quit_requested`.

---

## 4. Máquina de Estados e Modos do Editor

### 4.1 Modos Suportados (`enum EditorMode`)
```c
enum EditorMode {
    MODE_NORMAL = 0,           /* Navegação modal Vim clássica */
    MODE_INSERT,               /* Inserção direta de texto no buffer */
    MODE_VISUAL_CHAR,          /* Seleção visual caractere a caractere */
    MODE_VISUAL_LINE,          /* Seleção visual linha a linha */
    MODE_COMMAND,              /* Linha de comando inferior iniciada por ':' */
    MODE_SEARCH_LEFT,          /* Campo de busca 'Ctrl+F' no painel lateral */
    MODE_CONFIRM_DELETE_NOTE,  /* Janela de confirmação para exclusão de nota (s/n) */
    MODE_PROMPT_NEW_NOTE,      /* Diálogo interativo para criação de nota */
    MODE_PROMPT_RENAME_NOTE,   /* Diálogo interativo para renomeação de nota */
    MODE_PROMPT_FIND,          /* Prompt para busca textual na nota ativa */
    MODE_PROMPT_REPLACE_FIND,  /* Prompt 1: Termo a localizar na substituição */
    MODE_PROMPT_REPLACE_WITH   /* Prompt 2: Novo termo a ser inserido */
};
```

### 4.2 Foco da Aplicação (`enum EditorFocus`)
- **`FOCUS_LEFT`**: Foco no painel lateral. As teclas `j`/`k` ou setas navegam na lista de notas, `Enter` ou `l` abre a nota selecionada em uma aba no painel direito, `m` cicla o nível de urgência (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[ ]`), `d` solicita confirmação de exclusão, `/` ou `Ctrl+F` foca a caixa de busca, e `1`, `2`, `3`, `0` ativam os filtros de atenção.
- **`FOCUS_RIGHT`**: Foco na área de edição de texto com suporte integral aos atalhos Vim (`hjkl`, `w`, `b`, `gg`, `G`, `x`, `d`, `y`, `p`, `u`, `Ctrl+R`, etc.).

---

## 5. Camada Multiplataforma (POSIX Linux vs Win32)

O código compila nativamente em Linux e Windows sem emuladores adicionais:

| Recurso | Linux / POSIX | Windows / Win32 |
| :--- | :--- | :--- |
| **Entrada do Terminal** | `tcsetattr` em modo raw não-bloqueante | `SetConsoleMode` com flags de entrada virtual VT |
| **Saída ANSI / Cores** | Sequências ANSI VT100 / TrueColor 24-bit | `ENABLE_VIRTUAL_TERMINAL_PROCESSING` ativo |
| **Dimensões da Tela** | `ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws)` | `GetConsoleScreenBufferInfo(hOut, &csbi)` |
| **Rastreamento de Mouse** | Sequências ANSI SGR 1006 | Mesmo padrão via processamento VT nativo |
| **Exclusão de Arquivos** | `unlink(path)` | `DeleteFileW` com conversão de caminho UTF-8 para Wide |
| **Renomeação** | `rename(old, new)` | `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)` |
| **Varredura de Diretório**| `opendir("data")`, `readdir()`, `closedir()` | `FindFirstFileW(L"data\\*")`, `FindNextFileW()` |
| **Copiar para Clipboard** | Thread para `wl-copy`/`xclip` + OSC 52 Base64 | Win32 API (`OpenClipboard`, `CF_UNICODETEXT`) assíncrona |
| **Colar do Clipboard**   | Utilitários de terminal ou buffer interno | `GetClipboardData(CF_UNICODETEXT)` com conversão UTF-8 |
| **Abertura de Links**    | `xdg-open` ou `open` via `fork`/`exec` | `ShellExecuteA(NULL, "open", target, ...)` |

---

## 6. Pipeline Gráfico de Renderização (`refresh_screen`)

A renderização na versão 6.27.0 segue uma sequência rigorosa de 8 etapas através do `ABuffer`:
1. **Posicionamento e Cursor**: Reposiciona o cursor no canto superior esquerdo (`\x1b[H`) e oculta o cursor físico (`\x1b[?25l`).
2. **Painel Esquerdo (`render_left_panel`)**: Desenha o título "NOTAS", a barra de busca rápida (`Ctrl+F`), a lista de notas com datas relativas e caixas de atenção (`[1]`, `[2]`, `[3]`), e os botões inferiores de filtro de urgência (`[Todas] [1] [2] [3]`).
3. **Divisória Vertical**: Desenha a linha vertical separadora entre o painel lateral e a área de edição com a cor de borda do tema.
4. **Barra de Abas Superior (`render_tab_bar`)**: Desenha as abas dos arquivos abertos, indicador de modificado (`*`), botão de fechar aba (`x`), botão de nova aba (`[+]`) e botão do tema atual.
5. **Painel Direito de Edição (`render_right_panel`)**:
   - Números de linha e indicadores de dobra (`+` para dobrado, `-` para cabeçalho expandido).
   - Realce de cabeçalhos Markdown (# H1, ## H2, ### H3) com suas respectivas cores de tema.
   - **Renderização de Bullet Lists**: Marcadores `-` em amarelo negrito (`TH_FG_YELLOW`) e `*` em acento negrito (`TH_FG_ACCENT`).
   - Ocultação inteligente de URLs em links Markdown (`[texto](url)`) e wikilinks (`[[nota]]`) no Modo Normal.
   - Formatação inline de negrito (`**texto**`) e itálico (`*texto*`).
   - Alinhamento de colunas em tabelas Markdown com pipes (`|`).
   - Fundo escurecido para blocos de código cercados por crases (` ``` `).
   - Realce de seleção no Modo Visual e realce de termos buscados.
6. **Barra de Status Inferior (`render_status_bar`)**: Badges de modo com fundo colorido (NORMAL, INSERT, VISUAL, COMMAND), nome do arquivo ativo, linha/coluna do cursor, percentual de navegação e botões clicáveis: `[Nova]`, `[Renomear]`, `[Excluir]`, `[Quebra]`, `[Buscar]`, `[Subst]`, `[Life]`.
7. **Linha de Comando e Prompts (`render_command_line`)**: Linha de comando `:` do Vim ou campo de digitação de diálogos.
8. **Descarga Atômica**: Reposiciona o cursor físico na posição calculada de edição (`\x1b[r;cH`), reabilita a visibilidade do cursor (`\x1b[?25h`) e emite todo o bloco de bytes acumulado no terminal através de `write()` ou `WriteConsoleA()`.

---

## 7. Catálogo Completo das 123 Funções da Versão 6.27.0

Abaixo estão catalogadas todas as 123 funções existentes no código-fonte de `notas.c` na versão 6.27.0:

### 7.1 `safe_strcpy` (Linha 60)
- **Protótipo**: `static void safe_strcpy(char *dest, size_t dest_sz, const char *src)`
- **Descrição**: Copia strings com segurança garantindo que nunca ocorra buffer overflow e assegurando terminação nula '\0'.
- **Retorno**: `static void`

### 7.2 `ab_init` (Linha 445)
- **Protótipo**: `static void ab_init(ABuffer *ab)`
- **Descrição**: Inicializa a estrutura ABuffer alocando memória inicial de 8192 bytes dinamicamente.
- **Retorno**: `static void`

### 7.3 `ab_append` (Linha 451)
- **Protótipo**: `static void ab_append(ABuffer *ab, const char *s, int len)`
- **Descrição**: Anexa uma sequência de bytes ao buffer acumulador dinâmico, expandindo a capacidade com realloc quando necessário.
- **Retorno**: `static void`

### 7.4 `ab_free` (Linha 468)
- **Protótipo**: `static void ab_free(ABuffer *ab)`
- **Descrição**: Libera a memória alocada dinamicamente pelo ABuffer e zera os ponteiros internos.
- **Retorno**: `static void`

### 7.5 `unlink_utf8` (Linha 729)
- **Protótipo**: `static int unlink_utf8(const char *path)`
- **Descrição**: Remove um arquivo Markdown do disco. No Windows converte o caminho UTF-8 para WideChar e invoca DeleteFileW.
- **Retorno**: `static int`

### 7.6 `rename_utf8` (Linha 741)
- **Protótipo**: `static int rename_utf8(const char *oldpath, const char *newpath)`
- **Descrição**: Renomeia um arquivo de nota no disco de forma atômica, garantindo suporte a caracteres acentuados no Windows e Linux.
- **Retorno**: `static int`

### 7.7 `filetime_to_timet` (Linha 755)
- **Protótipo**: `static inline time_t filetime_to_timet(const FILETIME *ft)`
- **Descrição**: Converte a estrutura FILETIME da API Win32 para o formato time_t Unix para comparação de datas.
- **Retorno**: `static inline time_t`

### 7.8 `stat_cross` (Linha 764)
- **Protótipo**: `static int stat_cross(const char *path, struct stat *st)`
- **Descrição**: Obtém metadados de arquivos (tamanho e data de modificação) de forma multiplataforma com suporte pleno a UTF-8.
- **Retorno**: `static int`

### 7.9 `disable_raw_mode` (Linha 785)
- **Protótipo**: `static void disable_raw_mode(void)`
- **Descrição**: Restaura o terminal para o modo canônico original, desativa o rastreamento do mouse e reexibe o cursor.
- **Retorno**: `static void`

### 7.10 `handle_sigwinch` (Linha 814)
- **Protótipo**: `static void handle_sigwinch(int sig)`
- **Descrição**: Trata o sinal SIGWINCH no Linux para atualizar as dimensões da janela do terminal ao redimensionar.
- **Retorno**: `static void`

### 7.11 `enable_raw_mode` (Linha 821)
- **Protótipo**: `static void enable_raw_mode(void)`
- **Descrição**: Coloca o terminal em modo Raw, desativa eco e quebra de linha canônica, e ativa o rastreamento de mouse SGR 1006.
- **Retorno**: `static void`

### 7.12 `get_cursor_position` (Linha 862)
- **Protótipo**: `static int get_cursor_position(int *rows, int *cols)`
- **Descrição**: Consulta a posição física atual do cursor emitindo a sequência DSR '\x1b[6n' e decodificando a resposta.
- **Retorno**: `static int`

### 7.13 `get_window_size` (Linha 886)
- **Protótipo**: `static void get_window_size(int *rows, int *cols)`
- **Descrição**: Obtém as dimensões de linhas e colunas da tela via ioctl (Linux) ou GetConsoleScreenBufferInfo (Windows).
- **Retorno**: `static void`

### 7.14 `utf8_charlen` (Linha 946)
- **Protótipo**: `static int utf8_charlen(unsigned char ch)`
- **Descrição**: Retorna o tamanho em bytes (1 a 4) de um caractere UTF-8 com base no seu byte inicial.
- **Retorno**: `static int`

### 7.15 `utf8_decode_codepoint` (Linha 954)
- **Protótipo**: `static int utf8_decode_codepoint(const char *s, int *bytes_read)`
- **Descrição**: Decodifica uma sequência de bytes UTF-8 em um codepoint Unicode de 32 bits.
- **Retorno**: `static int`

### 7.16 `utf8_codepoint_width` (Linha 989)
- **Protótipo**: `static int utf8_codepoint_width(int cp)`
- **Descrição**: Retorna a largura visual de um codepoint Unicode em colunas de terminal (0 para diacríticos, 2 para CJK, 1 para texto comum).
- **Retorno**: `static int`

### 7.17 `utf8_width` (Linha 1015)
- **Protótipo**: `static int utf8_width(const char *s)`
- **Descrição**: Calcula a largura total em colunas visuais que uma string UTF-8 ocupará na tela.
- **Retorno**: `static int`

### 7.18 `utf8_align_start` (Linha 1037)
- **Protótipo**: `static int utf8_align_start(const char *s, int len, int idx)`
- **Descrição**: Ajusta um índice de byte para o início do caractere UTF-8 correspondente, evitando cortar caracteres no meio.
- **Retorno**: `static int`

### 7.19 `utf8_align_end` (Linha 1046)
- **Protótipo**: `static int utf8_align_end(const char *s, int len, int idx)`
- **Descrição**: Ajusta um índice de byte para o final do caractere UTF-8 correspondente.
- **Retorno**: `static int`

### 7.20 `utf8_width_len` (Linha 1058)
- **Protótipo**: `static int utf8_width_len(const char *s, int len)`
- **Descrição**: Calcula a largura visual em colunas de um trecho de texto UTF-8 delimitado por 'len' bytes.
- **Retorno**: `static int`

### 7.21 `byte_offset_for_col` (Linha 1082)
- **Protótipo**: `static int byte_offset_for_col(const char *s, int len, int target_col)`
- **Descrição**: Mapeia uma coluna visual de tela para o deslocamento exato de byte correspondente na string UTF-8.
- **Retorno**: `static int`

### 7.22 `utf8_truncate_display` (Linha 1106)
- **Protótipo**: `static void utf8_truncate_display(const char *src, int max_cols, char *dst, size_t dst_sz)`
- **Descrição**: Trunca uma string para caber exatamente em 'max_cols' colunas sem corromper caracteres multibyte.
- **Retorno**: `static void`

### 7.23 `utf8_marquee_slice` (Linha 1133)
- **Protótipo**: `static void utf8_marquee_slice(const char *title, int step, int avail_w, char *out, size_t out_sz)`
- **Descrição**: Gera uma fatia com animação de letreiro (marquee) para títulos de notas longos na barra de abas ou painel lateral.
- **Retorno**: `static void`

### 7.24 `get_current_time_ms` (Linha 1173)
- **Protótipo**: `static long get_current_time_ms(void)`
- **Descrição**: Retorna o timestamp atual em milissegundos de alta precisão através de relógio monotônico.
- **Retorno**: `static long`

### 7.25 `win32_clipboard_thread` (Linha 1215)
- **Protótipo**: `static DWORD WINAPI win32_clipboard_thread(LPVOID param)`
- **Descrição**: Thread auxiliar do Windows que grava dados na área de transferência assincronamente sem congelar a UI.
- **Retorno**: `static DWORD WINAPI`

### 7.26 `copy_to_system_clipboard` (Linha 1302)
- **Protótipo**: `static void copy_to_system_clipboard(const char *text, int len)`
- **Descrição**: Copia texto para a área de transferência do sistema via Win32 API, sequência OSC 52 Base64 ou utilitários POSIX (wl-copy/xclip).
- **Retorno**: `static void`

### 7.27 `open_link_target` (Linha 1476)
- **Protótipo**: `static void open_link_target(const char *raw_target)`
- **Descrição**: Abre um link da web no navegador padrão ou um arquivo/pasta local no explorador de arquivos do sistema operacional.
- **Retorno**: `static void`

### 7.28 `detect_heading` (Linha 1627)
- **Protótipo**: `static void detect_heading(Line *l)`
- **Descrição**: Analisa se a linha começa com um cabeçalho Markdown ('#' a '######') e preenche o campo heading_lvl.
- **Retorno**: `static void`

### 7.29 `line_init` (Linha 1637)
- **Protótipo**: `static void line_init(Line *l, const char *s, int len)`
- **Descrição**: Aloca e inicializa uma struct Line com a string UTF-8 especificada e terminador nulo.
- **Retorno**: `static void`

### 7.30 `line_free` (Linha 1649)
- **Protótipo**: `static void line_free(Line *l)`
- **Descrição**: Desaloca a memória da string interna de uma struct Line.
- **Retorno**: `static void`

### 7.31 `mark_modified` (Linha 1655)
- **Protótipo**: `static void mark_modified(void)`
- **Descrição**: Marca a nota ativa como modificada e registra o instante para o temporizador de autosave.
- **Retorno**: `static void`

### 7.32 `buf_insert_line` (Linha 1669)
- **Protótipo**: `static void buf_insert_line(int at, const char *s, int len)`
- **Descrição**: Insere uma nova linha no buffer do editor na posição especificada 'at'.
- **Retorno**: `static void`

### 7.33 `buf_delete_line` (Linha 1682)
- **Protótipo**: `static void buf_delete_line(int at)`
- **Descrição**: Remove uma linha do buffer do editor na posição especificada 'at'.
- **Retorno**: `static void`

### 7.34 `line_insert_char` (Linha 1694)
- **Protótipo**: `static void line_insert_char(Line *l, int at, char c)`
- **Descrição**: Insere um caractere/byte na linha na posição 'at'.
- **Retorno**: `static void`

### 7.35 `line_delete_char` (Linha 1705)
- **Protótipo**: `static void line_delete_char(Line *l, int at)`
- **Descrição**: Remove um byte na linha na posição 'at'.
- **Retorno**: `static void`

### 7.36 `line_replace_range` (Linha 1714)
- **Protótipo**: `static void line_replace_range(Line *l, int at, int del_len, const char *new_str, int new_len)`
- **Descrição**: Substitui um intervalo de bytes na linha por uma nova sequência de caracteres.
- **Retorno**: `static void`

### 7.37 `line_append_string` (Linha 1734)
- **Protótipo**: `static void line_append_string(Line *l, const char *s, int len)`
- **Descrição**: Concatena uma string ao final da linha especificada.
- **Retorno**: `static void`

### 7.38 `editor_paste_string` (Linha 1745)
- **Protótipo**: `static void editor_paste_string(const char *text, int len)`
- **Descrição**: Manipulador robusto de colagem multi-linha que previne efeito escada e normaliza quebras de linha CRLF/LF.
- **Retorno**: `static void`

### 7.39 `update_lines_syntax` (Linha 1833)
- **Protótipo**: `static void update_lines_syntax(Line *lines, int num_lines)`
- **Descrição**: Verifica blocos de código cercados por crases (```) e atualiza os flags is_code_fence e in_code_block.
- **Retorno**: `static void`

### 7.40 `update_folding` (Linha 1866)
- **Protótipo**: `static void update_folding(void)`
- **Descrição**: Recalcula a visibilidade das linhas ocultas com base nos cabeçalhos atualmente dobrados (colapsados).
- **Retorno**: `static void`

### 7.41 `toggle_fold_at_cursor` (Linha 1870)
- **Protótipo**: `static void toggle_fold_at_cursor(void)`
- **Descrição**: Alterna o estado de dobra (folding) do cabeçalho sob a posição do cursor (atalho 'za').
- **Retorno**: `static void`

### 7.42 `collapse_all_headings` (Linha 1895)
- **Protótipo**: `static void collapse_all_headings(void)`
- **Descrição**: Recolhe todas as seções de cabeçalho da nota (atalho 'zM').
- **Retorno**: `static void`

### 7.43 `uncollapse_all_headings` (Linha 1916)
- **Protótipo**: `static void uncollapse_all_headings(void)`
- **Descrição**: Expande todas as seções de cabeçalho da nota (atalho 'zR').
- **Retorno**: `static void`

### 7.44 `free_undo_state` (Linha 1938)
- **Protótipo**: `static void free_undo_state(UndoState *us)`
- **Descrição**: Libera a memória alocada nas linhas de um snapshot da pilha de desfazer.
- **Retorno**: `static void`

### 7.45 `save_undo` (Linha 1948)
- **Protótipo**: `static void save_undo(void)`
- **Descrição**: Salva um snapshot completo do buffer e do cursor na pilha de desfazer antes de alterações no texto.
- **Retorno**: `static void`

### 7.46 `apply_undo` (Linha 1971)
- **Protótipo**: `static void apply_undo(void)`
- **Descrição**: Restaura o estado anterior da pilha de desfazer (atalho 'u').
- **Retorno**: `static void`

### 7.47 `apply_redo` (Linha 2008)
- **Protótipo**: `static void apply_redo(void)`
- **Descrição**: Reaplica a alteração desfeita (atalho 'Ctrl+R').
- **Retorno**: `static void`

### 7.48 `format_rel_time` (Linha 2049)
- **Protótipo**: `static void format_rel_time(time_t mtime, time_t now, char *buf, size_t sz)`
- **Descrição**: Formata a diferença de tempo de modificação de forma amigável ('agora', '5m atrás', '2h atrás', 'data').
- **Retorno**: `static void`

### 7.49 `normalize_text` (Linha 2072)
- **Protótipo**: `static void normalize_text(const char *src, char *dst, size_t dst_cap)`
- **Descrição**: Normaliza texto removendo acentos e convertendo para minúsculas para busca ultra-rápida em português.
- **Retorno**: `static void`

### 7.50 `extract_note_title` (Linha 2137)
- **Protótipo**: `static void extract_note_title(const char *filepath, const char *fname, char *out_title, size_t sz)`
- **Descrição**: Extrai o primeiro título H1 (# Título) do arquivo ou utiliza o nome do arquivo limpo.
- **Retorno**: `static void`

### 7.51 `compare_notes_mtime` (Linha 2148)
- **Protótipo**: `static int compare_notes_mtime(const void *a, const void *b)`
- **Descrição**: Função de comparação para qsort ordenando as notas da mais recentemente modificada para a mais antiga.
- **Retorno**: `static int`

### 7.52 `note_update_cache` (Linha 2172)
- **Protótipo**: `static void note_update_cache(NoteInfo *n)`
- **Descrição**: Atualiza o cache em memória normalizado de uma nota para acelerar o filtro instantâneo.
- **Retorno**: `static void`

### 7.53 `load_attention_flags` (Linha 2235)
- **Protótipo**: `static void load_attention_flags(void)`
- **Descrição**: Carrega o arquivo 'data/.attention' contendo os níveis de urgência das notas (1=Crítica, 2=Média, 3=Baixa).
- **Retorno**: `static void`

### 7.54 `save_attention_flags` (Linha 2263)
- **Protótipo**: `static void save_attention_flags(void)`
- **Descrição**: Grava os níveis de urgência de todas as notas no arquivo 'data/.attention'.
- **Retorno**: `static void`

### 7.55 `toggle_note_attention_at` (Linha 2276)
- **Protótipo**: `static void toggle_note_attention_at(int actual_idx)`
- **Descrição**: Alterna ciclicamente o nível de urgência da nota selecionada ([ ] -> [1] -> [2] -> [3] -> [ ]).
- **Retorno**: `static void`

### 7.56 `rescan_notes` (Linha 2298)
- **Protótipo**: `static void rescan_notes(void)`
- **Descrição**: Varre o diretório 'data/', cataloga todos os arquivos .md, extrai títulos e reordena por data de modificação.
- **Retorno**: `static void`

### 7.57 `str_contains_caseless` (Linha 2462)
- **Protótipo**: `static bool str_contains_caseless(const char *haystack, const char *needle)`
- **Descrição**: Verifica se uma substring existe dentro de outra sem distinguir maiúsculas e minúsculas.
- **Retorno**: `static bool`

### 7.58 `apply_note_search` (Linha 2478)
- **Protótipo**: `static void apply_note_search(const char *query)`
- **Descrição**: Aplica filtro multi-palavras em tempo real na lista de notas durante a busca (Ctrl+F).
- **Retorno**: `static void`

### 7.59 `free_preview` (Linha 2543)
- **Protótipo**: `static void free_preview(void)`
- **Descrição**: Libera as linhas do buffer de pré-visualização rápida do painel esquerdo.
- **Retorno**: `static void`

### 7.60 `load_preview` (Linha 2557)
- **Protótipo**: `static void load_preview(const char *path)`
- **Descrição**: Carrega uma nota no painel de visualização sem criar uma aba formal de edição.
- **Retorno**: `static void`

### 7.61 `tab_save_active` (Linha 2599)
- **Protótipo**: `static void tab_save_active(void)`
- **Descrição**: Sincroniza o estado atual do editor com a estrutura Tab da aba ativa.
- **Retorno**: `static void`

### 7.62 `tab_load` (Linha 2622)
- **Protótipo**: `static void tab_load(int idx)`
- **Descrição**: Carrega o estado de uma aba específica para as variáveis ativas do editor.
- **Retorno**: `static void`

### 7.63 `tab_switch` (Linha 2647)
- **Protótipo**: `static void tab_switch(int idx)`
- **Descrição**: Alterna a exibição para a aba indicada por índice.
- **Retorno**: `static void`

### 7.64 `tab_next` (Linha 2654)
- **Protótipo**: `static void tab_next(void)`
- **Descrição**: Avança para a próxima aba aberta (Ctrl+Tab ou :tabnext).
- **Retorno**: `static void`

### 7.65 `tab_prev` (Linha 2660)
- **Protótipo**: `static void tab_prev(void)`
- **Descrição**: Retorna para a aba anterior.
- **Retorno**: `static void`

### 7.66 `tab_close` (Linha 2666)
- **Protótipo**: `static void tab_close(int idx)`
- **Descrição**: Fecha a aba especificada. Se houver modificações não salvas, solicita confirmação do usuário.
- **Retorno**: `static void`

### 7.67 `close_all_tabs` (Linha 2722)
- **Protótipo**: `static void close_all_tabs(void)`
- **Descrição**: Fecha todas as abas abertas de uma só vez (atalho 'C' ou comando :closeall).
- **Retorno**: `static void`

### 7.68 `push_history` (Linha 2759)
- **Protótipo**: `static void push_history(const char *path)`
- **Descrição**: Registra uma nota visitada na pilha de histórico de navegação (até 64 notas).
- **Retorno**: `static void`

### 7.69 `navigate_history` (Linha 2773)
- **Protótipo**: `static void navigate_history(int steps)`
- **Descrição**: Navega para trás (Backspace) ou para frente (Ctrl+Backspace) no histórico de navegação entre notas.
- **Retorno**: `static void`

### 7.70 `open_note_by_path` (Linha 2785)
- **Protótipo**: `static void open_note_by_path(const char *path, bool record_history)`
- **Descrição**: Abre um arquivo de nota por caminho no disco, criando uma nova aba ou focando na aba existente.
- **Retorno**: `static void`

### 7.71 `save_current_note` (Linha 2859)
- **Protótipo**: `static bool save_current_note(void)`
- **Descrição**: Grava o conteúdo da nota ativa no disco (pasta data/) de forma segura e atualiza seu timestamp.
- **Retorno**: `static bool`

### 7.72 `save_tab_by_idx` (Linha 2912)
- **Protótipo**: `static bool save_tab_by_idx(int t_idx)`
- **Descrição**: Salva uma aba específica identificada pelo seu índice.
- **Retorno**: `static bool`

### 7.73 `check_autosave` (Linha 2950)
- **Protótipo**: `static void check_autosave(void)`
- **Descrição**: Verifica se há alterações não salvas há mais de 60 segundos e executa salvamento automático silencioso no disco.
- **Retorno**: `static void`

### 7.74 `open_or_create_wikilink_note` (Linha 2988)
- **Protótipo**: `static void open_or_create_wikilink_note(const char *note_name)`
- **Descrição**: Abre uma nota referenciada por [[wikilink]] ou cria um arquivo novo caso não exista na pasta data/.
- **Retorno**: `static void`

### 7.75 `create_new_note` (Linha 3021)
- **Protótipo**: `static void create_new_note(const char *raw_name)`
- **Descrição**: Cria uma nova nota interativamente na pasta data/ com template inicial e a abre no editor.
- **Retorno**: `static void`

### 7.76 `replace_in_current_note` (Linha 3082)
- **Protótipo**: `static void replace_in_current_note(const char *find_str, const char *rep_str)`
- **Descrição**: Executa substituição de todas as ocorrências de um termo por outro na nota ativa.
- **Retorno**: `static void`

### 7.77 `rename_note` (Linha 3128)
- **Protótipo**: `static bool rename_note(const char *old_path, const char *raw_new_name)`
- **Descrição**: Renomeia o arquivo da nota no disco (data/) e atualiza seu nome nas abas abertas e no índice em memória.
- **Retorno**: `static bool`

### 7.78 `should_hide_links` (Linha 3296)
- **Protótipo**: `static inline bool should_hide_links(bool in_preview)`
- **Descrição**: Informa se os alvos de links Markdown devem ficar ocultos na exibição no Modo Normal.
- **Retorno**: `static inline bool`

### 7.79 `should_hide_markdown_styles` (Linha 3321)
- **Protótipo**: `static inline bool should_hide_markdown_styles(bool in_preview)`
- **Descrição**: Informa se os caracteres de estilo (** ou *) devem ficar ocultos no Modo Normal.
- **Retorno**: `static inline bool`

### 7.80 `find_markdown_styles` (Linha 3326)
- **Protótipo**: `static int find_markdown_styles(const Line *l, MarkdownStyleSpan *spans, int max_spans)`
- **Descrição**: Localiza spans de negrito (**texto**) e itálico (*texto*) dentro de uma linha de texto.
- **Retorno**: `static int`

### 7.81 `find_links_in_line` (Linha 3386)
- **Protótipo**: `static int find_links_in_line(const Line *l, LinkSpan *spans, int max_spans)`
- **Descrição**: Localiza links Markdown [texto](url) e [[wikilinks]] dentro de uma linha de texto.
- **Retorno**: `static int`

### 7.82 `activate_link_at_cursor` (Linha 3473)
- **Protótipo**: `static void activate_link_at_cursor(void)`
- **Descrição**: Executa a ação correspondente ao link sob o cursor (abre página web ou carrega nota local).
- **Retorno**: `static void`

### 7.83 `yank_visual_selection` (Linha 3492)
- **Protótipo**: `static void yank_visual_selection(bool linewise)`
- **Descrição**: Copia (yank) o texto selecionado no Modo Visual para a área de transferência do editor e do sistema.
- **Retorno**: `static void`

### 7.84 `delete_visual_selection` (Linha 3557)
- **Protótipo**: `static void delete_visual_selection(void)`
- **Descrição**: Remove o texto selecionado no Modo Visual e o envia para o buffer de corte.
- **Retorno**: `static void`

### 7.85 `jump_to_next_link` (Linha 3668)
- **Protótipo**: `static void jump_to_next_link(void)`
- **Descrição**: Move o cursor do editor diretamente para o próximo link Markdown ou wikilink na nota.
- **Retorno**: `static void`

### 7.86 `is_table_line` (Linha 3754)
- **Protótipo**: `static bool is_table_line(const Line *l)`
- **Descrição**: Detecta se uma linha faz parte de uma tabela formatada com pipes ('|').
- **Retorno**: `static bool`

### 7.87 `is_delimiter_cell` (Linha 3762)
- **Protótipo**: `static bool is_delimiter_cell(const char *s)`
- **Descrição**: Detecta se uma célula de tabela é delimitadora ('---', ':---:', '---:').
- **Retorno**: `static bool`

### 7.88 `align_table_lines` (Linha 3796)
- **Protótipo**: `static void align_table_lines(Line *lines, int start_y, int end_y)`
- **Descrição**: Calcula a largura de cada coluna e reformata um bloco de tabela Markdown para alinhamento uniforme.
- **Retorno**: `static void`

### 7.89 `table_align_at` (Linha 3959)
- **Protótipo**: `static bool table_align_at(int line_idx)`
- **Descrição**: Alinha a tabela situada na linha de índice informada.
- **Retorno**: `static bool`

### 7.90 `table_align_all` (Linha 3987)
- **Protótipo**: `static void table_align_all(void)`
- **Descrição**: Alinha todas as tabelas presentes no documento aberto.
- **Retorno**: `static void`

### 7.91 `table_jump_next_cell` (Linha 4011)
- **Protótipo**: `static void table_jump_next_cell(bool create_if_at_end)`
- **Descrição**: Pula o cursor para a próxima célula da tabela (tecla Tab no modo normal/tabela).
- **Retorno**: `static void`

### 7.92 `table_insert_row_and_advance` (Linha 4111)
- **Protótipo**: `static bool table_insert_row_and_advance(void)`
- **Descrição**: Insere uma nova linha na tabela preservando a formatação das colunas ao pressionar Enter.
- **Retorno**: `static bool`

### 7.93 `get_line_segments` (Linha 4167)
- **Protótipo**: `static int get_line_segments(const Line *l, int max_cols, LineSegment *segs, int max_segs, bool hide_links, bool hide_styles)`
- **Descrição**: Divide uma linha longa em segmentos visuais para exibição no modo de quebra de linha suave (wrap mode).
- **Retorno**: `static int`

### 7.94 `get_visual_col_in_seg` (Linha 4296)
- **Protótipo**: `static int get_visual_col_in_seg(const Line *l, int seg_start, int x_in_seg, bool hide_links, bool hide_styles)`
- **Descrição**: Retorna a coluna visual de tela correspondente a um deslocamento de byte dentro de um segmento.
- **Retorno**: `static int`

### 7.95 `get_byte_offset_in_seg` (Linha 4374)
- **Protótipo**: `static int get_byte_offset_in_seg(const Line *l, int seg_start, int seg_len, int target_col, bool hide_links, bool hide_styles)`
- **Descrição**: Retorna o deslocamento de byte dentro de um segmento a partir de uma coluna visual de tela.
- **Retorno**: `static int`

### 7.96 `get_cursor_visual_pos` (Linha 4477)
- **Protótipo**: `static void get_cursor_visual_pos(int avail_w, int *out_vrow, int *out_vcol)`
- **Descrição**: Calcula a linha e coluna visual do cursor considerando o modo de quebra de linha.
- **Retorno**: `static void`

### 7.97 `move_cursor_visual_down` (Linha 4521)
- **Protótipo**: `static void move_cursor_visual_down(void)`
- **Descrição**: Move o cursor para baixo respeitando quebras de linhas visuais virtuais.
- **Retorno**: `static void`

### 7.98 `move_cursor_visual_up` (Linha 4567)
- **Protótipo**: `static void move_cursor_visual_up(void)`
- **Descrição**: Move o cursor para cima respeitando quebras de linhas visuais virtuais.
- **Retorno**: `static void`

### 7.99 `render_left_panel` (Linha 4617)
- **Protótipo**: `static void render_left_panel(ABuffer *ab, int row)`
- **Descrição**: Desenha a barra lateral esquerda: cabeçalho, barra de busca, lista de notas, badges de atenção e filtros.
- **Retorno**: `static void`

### 7.100 `render_tab_bar` (Linha 5035)
- **Protótipo**: `static void render_tab_bar(ABuffer *ab)`
- **Descrição**: Desenha a barra superior de abas de arquivos, botão de fechar, botão [+] e seletor de temas.
- **Retorno**: `static void`

### 7.101 `render_code_line` (Linha 5171)
- **Protótipo**: `static void render_code_line(ABuffer *ab, Line *l, int *cols_printed, int max_w)`
- **Descrição**: Desenha uma linha localizada dentro de um bloco de código com formatação e fundo escurecido.
- **Retorno**: `static void`

### 7.102 `is_bullet_list_line` (Linha 5361)
- **Protótipo**: `static bool is_bullet_list_line(const Line *l, int *bullet_pos, char *bullet_char)`
- **Descrição**: Detecta se uma linha é um item de lista não-ordenada (* ou -), identificando posição e caractere para coloração.
- **Retorno**: `static bool`

### 7.103 `render_right_panel` (Linha 5391)
- **Protótipo**: `static void render_right_panel(ABuffer *ab, int screen_row)`
- **Descrição**: Desenha a área principal de edição de texto, aplicando cores de tema, realces de busca e estilos Markdown.
- **Retorno**: `static void`

### 7.104 `render_status_bar` (Linha 5751)
- **Protótipo**: `static void render_status_bar(ABuffer *ab)`
- **Descrição**: Desenha a barra de status inferior com badges de modo, posição do cursor e botões de ação clicáveis.
- **Retorno**: `static void`

### 7.105 `render_command_line` (Linha 5965)
- **Protótipo**: `static void render_command_line(ABuffer *ab)`
- **Descrição**: Desenha a linha inferior de comando ':' ou prompts interativos com cursor de edição.
- **Retorno**: `static void`

### 7.106 `editor_scroll` (Linha 6066)
- **Protótipo**: `static void editor_scroll(void)`
- **Descrição**: Ajusta as variáveis de scroll vertical e horizontal para garantir que o cursor permaneça visível na janela.
- **Retorno**: `static void`

### 7.107 `refresh_screen` (Linha 6126)
- **Protótipo**: `static void refresh_screen(void)`
- **Descrição**: Monta toda a tela no ABuffer e emite atomicamente para o terminal, eliminando flickering.
- **Retorno**: `static void`

### 7.108 `set_status_msg` (Linha 6300)
- **Protótipo**: `static void set_status_msg(const char *fmt, ...)`
- **Descrição**: Define uma mensagem de notificação temporária na barra de status com temporizador de expiração.
- **Retorno**: `static void`

### 7.109 `queue_input_byte` (Linha 6318)
- **Protótipo**: `static void queue_input_byte(unsigned char b)`
- **Descrição**: Enfileira um byte no buffer interno de entrada do editor.
- **Retorno**: `static void`

### 7.110 `has_pending_input` (Linha 6324)
- **Protótipo**: `static bool has_pending_input(void)`
- **Descrição**: Verifica se há bytes pendentes aguardando leitura no buffer de entrada.
- **Retorno**: `static bool`

### 7.111 `read_raw_byte` (Linha 6356)
- **Protótipo**: `static int read_raw_byte(int timeout_ms)`
- **Descrição**: Lê um byte do terminal com controle de timeout em milissegundos sem bloqueio permanente.
- **Retorno**: `static int`

### 7.112 `convert_ansi_to_utf8` (Linha 6465)
- **Protótipo**: `static int convert_ansi_to_utf8(unsigned char c, char *out)`
- **Descrição**: Converte sequências ANSI antigas de caracteres estendidos para UTF-8 válido.
- **Retorno**: `static int`

### 7.113 `editor_read_key` (Linha 6497)
- **Protótipo**: `static int editor_read_key(void)`
- **Descrição**: Decodifica teclas pressionadas e sequências de escape ANSI/VT100 e eventos de mouse SGR 1006.
- **Retorno**: `static int`

### 7.114 `execute_delete_selected_note` (Linha 6725)
- **Protótipo**: `static void execute_delete_selected_note(void)`
- **Descrição**: Apresenta confirmação e executa a exclusão definitiva do arquivo de nota selecionado na pasta data/.
- **Retorno**: `static void`

### 7.115 `handle_mouse_event` (Linha 6766)
- **Protótipo**: `static void handle_mouse_event(void)`
- **Descrição**: Processa eventos de clique, arraste e rolagem do mouse nas diferentes regiões da interface.
- **Retorno**: `static void`

### 7.116 `execute_bottom_command` (Linha 7188)
- **Protótipo**: `static void execute_bottom_command(const char *cmd)`
- **Descrição**: Interpreta e executa comandos digitados no modo comando (:w, :q, :wq, :theme, :life, :rename, etc.).
- **Retorno**: `static void`

### 7.117 `handle_prompt_line_edit` (Linha 7425)
- **Protótipo**: `static bool handle_prompt_line_edit(char *buf, size_t max_sz, int *cursor, int c)`
- **Descrição**: Editor de linha única para prompts de diálogo com suporte a setas, Home, End, Backspace e UTF-8.
- **Retorno**: `static bool`

### 7.118 `terminal_write` (Linha 7516)
- **Protótipo**: `static void terminal_write(const char *s, int len)`
- **Descrição**: Escreve diretamente uma string de controle no descritor de saída do terminal.
- **Retorno**: `static void`

### 7.119 `run_game_of_life` (Linha 7525)
- **Protótipo**: `static void run_game_of_life(void)`
- **Descrição**: Executa o screensaver/minigame Conway's Game of Life em modo tela cheia até qualquer tecla ser pressionada.
- **Retorno**: `static void`

### 7.120 `editor_process_keypress` (Linha 7713)
- **Protótipo**: `static void editor_process_keypress(void)`
- **Descrição**: O núcleo da máquina de estados do editor: despacha teclas de acordo com o foco e modo ativos.
- **Retorno**: `static void`

### 7.121 `free_editor` (Linha 9195)
- **Protótipo**: `static void free_editor(void)`
- **Descrição**: Libera toda a memória dinâmica alocada pelo editor (abas, linhas, notas, buffers de busca e histórico).
- **Retorno**: `static void`

### 7.122 `init_editor` (Linha 9243)
- **Protótipo**: `static void init_editor(void)`
- **Descrição**: Inicializa o estado global E, detecta o terminal, carrega as notas do diretório 'data/' e abre a nota inicial.
- **Retorno**: `static void`

### 7.123 `main` (Linha 9290)
- **Protótipo**: `int main(int argc, char **argv)`
- **Descrição**: Ponto de entrada do programa: habilita modo raw, inicializa subsistemas e executa o loop principal de eventos.
- **Retorno**: `int`


---

## 8. Guia Prático para IAs e Desenvolvedores: Como Continuar o Desenvolvimento da v6.27.0

Ao expandir funcionalidades sobre a base de código da versão 6.27.0, observe rigorosamente as seguintes diretrizes:

### 8.1 Invariantes Estruturais da v6.27.0
1. **Preservação de Fronteiras UTF-8**:
   - Nunca divida strings por contagem direta de bytes sem alinhar com `utf8_align_start()` ou `utf8_charlen()`. Quebrar um caractere multibyte (como caracteres acentuados em português) causa defeitos gráficos irreversíveis no terminal.
2. **Double-Buffering com `ABuffer`**:
   - Nunca chame `printf()`, `puts()` ou emita sequências de escape diretamente para `stdout` durante rotinas de desenho.
   - Toda emissão visual deve ser anexada ao acumulador através de `ab_append(ab, string, tamanho)`. O envio físico ao terminal ocorre exclusivamente na conclusão de `refresh_screen()`.
3. **Sincronização de Abas**:
   - Antes de alterar o índice da aba ativa (`E.active_tab`), invoque sempre `tab_save_active()` para sincronizar linhas, cursores e flags de modificação. Ao ativar a nova aba, execute `tab_load(idx)`.
4. **Armazenamento em Arquivos Markdown Simples**:
   - Todo arquivo de nota deve ser salvo e lido a partir da pasta `data/`.
   - O arquivo `data/.attention` persiste o nível de urgência (`0` a `3`) das notas no formato `arquivo.md:nivel`.

### 8.2 Como Adicionar um Novo Comando na Linha de Comando Inferior (`:`)
1. Localize a função `execute_bottom_command(const char *cmd)` (próximo à linha 7188).
2. Adicione uma nova verificação de comando:
   ```c
   if (strcmp(cmd, "contar") == 0 || strcmp(cmd, "wc") == 0) {
       int palavras = 0;
       /* Percorra E.lines contando palavras */
       set_status_msg("Total de linhas: %d", E.num_lines);
       return;
   }
   ```

### 8.3 Como Adicionar um Novo Botão Clicável na Barra de Status
1. Adicione os campos de coluna inicial e final na struct `Editor` (ex: `int btn_ajuda_start, btn_ajuda_end;`).
2. Em `render_status_bar(ABuffer *ab)`:
   - Armazene o início: `E.btn_ajuda_start = cols_printed;`
   - Desenhe o botão estilizado: `ab_append(ab, "[Ajuda]", 7);`
   - Armazene o fim: `E.btn_ajuda_end = cols_printed;`
3. Em `handle_mouse_event()`:
   - Verifique se o clique ocorreu na linha da barra de status e entre `E.btn_ajuda_start` e `E.btn_ajuda_end`, disparando a rotina correspondente.

### 8.4 Comandos de Compilação
- **No Linux**:
  ```bash
  gcc -O2 -Wall -Wextra src/notas.c -o notas -ldl
  ./notas
  ```
- **No Windows (PowerShell / MinGW)**:
  ```powershell
  gcc -O2 -Wall -Wextra src/notas.c -o notas.exe -lkernel32 -luser32 -lshell32
  .\notas.exe
  ```

---
*Documento revisado e estritamente restrito à especificação técnica da versão 6.27.0.*
