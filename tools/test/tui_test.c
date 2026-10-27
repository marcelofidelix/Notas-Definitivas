/* =============================================================================
 * tui_test.c - Teste de integracao da interface (Windows / MinGW)
 * -----------------------------------------------------------------------------
 * Compila o proprio notas.c como unidade de traducao e dirige o editor como um
 * usuario faria: os bytes de teclado entram pela mesma fila que o leitor de
 * teclas consome (queue_input_byte) e passam por editor_process_keypress(); cada
 * quadro e gerado por refresh_screen() com a saida padrao redirecionada para um
 * arquivo, para inspecionar exatamente os bytes que o terminal receberia.
 *
 * Roda em um diretorio de trabalho descartavel (cria notas.db ali).
 * Uso: tui_test <pasta-com-fixtures-md>
 * ========================================================================== */
#define main notas_main_desativado_para_teste
#include "../../notas.c"
#undef main

static int falhas = 0;

static void chk(const char *nome, bool ok, const char *extra) {
    printf("  [%s]  %s", ok ? "OK" : "FALHA", nome);
    if (!ok && extra && *extra) printf("  -> %s", extra);
    printf("\n");
    fflush(stdout);
    if (!ok) falhas++;
}

/* ---- entrada: digita bytes e processa todas as teclas ------------------- */

static void keys_n(const char *s, int n) {
    for (int i = 0; i < n; i++) queue_input_byte((unsigned char)s[i]);
    int guard = 0;
    while (has_pending_input() && guard++ < 100000) editor_process_keypress();
}

static void keys(const char *s) { keys_n(s, (int)strlen(s)); }

/* ESC sozinho: se outra tecla vier na mesma rajada, o leitor entende Alt+tecla. */
static void esc(void) { keys("\x1b"); }

/* ---- saida: captura um quadro completo ---------------------------------- */

static char *g_frame = NULL;
static long g_frame_len = 0;

static void frame(void) {
    free(g_frame);
    g_frame = NULL;
    g_frame_len = 0;
    const char *tmp = "frame.out";
    HANDLE h = CreateFileA(tmp, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (h == INVALID_HANDLE_VALUE) return;
    HANDLE old = GetStdHandle(STD_OUTPUT_HANDLE);
    SetStdHandle(STD_OUTPUT_HANDLE, h);
    g_needs_redraw = 1;
    refresh_screen();
    SetStdHandle(STD_OUTPUT_HANDLE, old);
    CloseHandle(h);
    g_frame = read_entire_file(tmp, &g_frame_len);
}

static bool frame_has(const char *s) {
    if (!g_frame) return false;
    size_t n = strlen(s);
    for (long i = 0; i + (long)n <= g_frame_len; i++) {
        if (memcmp(g_frame + i, s, n) == 0) return true;
    }
    return false;
}

/* Texto visivel do quadro: remove as sequencias CSI (ESC [ ... final) que o
 * renderizador intercala entre os caracteres para aplicar cor e estilo. */
static bool frame_text_has(const char *s) {
    if (!g_frame) return false;
    char *vis = (char *)malloc((size_t)g_frame_len + 1);
    if (!vis) return false;
    long o = 0;
    for (long i = 0; i < g_frame_len; i++) {
        if (g_frame[i] == '\x1b' && i + 1 < g_frame_len && g_frame[i + 1] == '[') {
            i += 2;
            while (i < g_frame_len && !((unsigned char)g_frame[i] >= 0x40 && (unsigned char)g_frame[i] <= 0x7E)) i++;
            continue;
        }
        vis[o++] = g_frame[i];
    }
    vis[o] = '\0';
    bool found = strstr(vis, s) != NULL;
    free(vis);
    return found;
}

/* ---- utilitarios de documento ------------------------------------------- */

static int note_index_by_title(const char *title) {
    for (int i = 0; i < E.num_notes; i++) {
        if (strcmp(E.notes[i].title, title) == 0) return i;
    }
    return -1;
}

static char *db_text_of(const char *title) {
    int idx = note_index_by_title(title);
    if (idx < 0) return NULL;
    long len = 0;
    return note_read_content(E.notes[idx].filepath, &len);
}

static bool open_by_title(const char *title) {
    int idx = note_index_by_title(title);
    if (idx < 0) return false;
    open_note_by_path(E.notes[idx].filepath, true);
    E.focus = FOCUS_RIGHT;
    E.mode = MODE_NORMAL;
    return true;
}

static bool doc_line_is(int y, const char *s) {
    return y >= 0 && y < E.num_lines && strcmp(E.lines[y].chars, s) == 0;
}

/* ========================================================================= */

static void t_startup(void) {
    printf("\n-- inicializacao com banco vazio --\n");
    init_editor();
    chk("banco criado e aberto", db_is_open(), "");
    chk("banco nasce vazio", E.num_notes == 0, "");
    chk("tamanho de tela vem de LINES/COLUMNS", E.screen_rows == 30 && E.screen_cols == 120, "");
    frame();
    chk("quadro desenhado", g_frame && g_frame_len > 1000, "quadro vazio");
    chk("estado vazio orienta a criar nota", frame_has("Nova Nota"), "");
}

static void t_import(const char *fixdir) {
    printf("\n-- importacao pela linha de comando --\n");
    char cmd[600];
    snprintf(cmd, sizeof cmd, ":import %s\r", fixdir);
    keys(cmd);
    chk("importou as notas da pasta", E.num_notes == 4, E.status_msg);
    chk("arquivo nao markdown ignorado", note_index_by_title("ignorar") < 0, "");
}

static void t_escape_injection(void) {
    printf("\n-- nota maliciosa nao injeta escapes no terminal --\n");
    chk("abre a nota com escapes", open_by_title("Malicia"), "nota nao encontrada");
    char *raw = db_text_of("Malicia");
    chk("conteudo cru preservado no banco", raw && strstr(raw, "\x1b]52;") != NULL, "");
    free(raw);
    frame();
    chk("quadro nao contem OSC 52", !frame_has("\x1b]52"), "OSC 52 vazou");
    chk("quadro nao contem BEL", !frame_has("\x07"), "BEL vazou");
    chk("quadro nao contem limpeza de tela injetada no texto", !frame_has("\x1b[2Jrisco"), "CSI vazou");
    chk("texto visivel continua aparecendo", frame_text_has("texto visivel"), "");
    chk("payload aparece como texto inerte", frame_text_has("]52;c;SGFjaw== fim"), "");
    chk("titulo com ESC nao vaza na aba", !frame_has("\x1b]0;"), "OSC 0 vazou");
}

static void t_editing(void) {
    printf("\n-- edicao por teclado e gravacao --\n");
    chk("abre Alfa", open_by_title("Alfa"), "");
    keys("G");
    keys("o");
    keys("Linha nova digitada");
    esc();
    chk("modo normal apos ESC", E.mode == MODE_NORMAL, "");
    chk("texto digitado no fim", doc_line_is(E.num_lines - 1, "Linha nova digitada"), E.lines[E.num_lines - 1].chars);
    chk("nota marcada como modificada", E.modified, "");
    keys(":w\r");
    chk("salvar limpa a modificacao", !E.modified, E.status_msg);
    char *c = db_text_of("Alfa");
    chk("conteudo gravado no SQLite", c && strstr(c, "Linha nova digitada\n") != NULL, c ? c : "(nulo)");
    free(c);

    keys("u");
    chk("u desfaz a digitacao inteira em um passo", !doc_line_is(E.num_lines - 1, "Linha nova digitada"),
        E.lines[E.num_lines - 1].chars);
    keys("\x12"); /* Ctrl+R */
    chk("Ctrl+R refaz", doc_line_is(E.num_lines - 1, "Linha nova digitada"), "");

    /* dd e p */
    int before = E.num_lines;
    keys("gg");
    keys("dd");
    chk("dd remove a linha", E.num_lines == before - 1, "");
    keys("u");
    chk("u restaura a linha removida", E.num_lines == before && doc_line_is(0, "# Alfa"), E.lines[0].chars);

    /* Modo visual linha: V j d */
    keys("gg");
    keys("Vj");
    keys("d");
    chk("V j d remove duas linhas", E.num_lines == before - 2, "");
    keys("u");
    chk("undo restaura as duas linhas", E.num_lines == before && doc_line_is(0, "# Alfa"), "");

    /* Substituicao :s */
    keys(":%s/alfa/ALFA/g\r");
    bool achou = false;
    for (int i = 0; i < E.num_lines; i++) if (strstr(E.lines[i].chars, "ALFA")) achou = true;
    chk(":%s substitui no documento", achou, E.status_msg);
    keys("u");
}

static void t_bracketed_paste(void) {
    printf("\n-- bracketed paste --\n");
    chk("abre Beta", open_by_title("Beta"), "");
    keys("G");
    keys("o");
    int before = E.num_lines;
    keys("\x1b[200~colada1\r\ncolada2\ncolada3\x1b[201~");
    chk("cola 3 linhas em um bloco", E.num_lines == before + 2, "");
    chk("conteudo colado na ordem", doc_line_is(before - 1, "colada1") && doc_line_is(before + 1, "colada3"), "");
    chk("continua no modo insercao", E.mode == MODE_INSERT, "");
    esc();
    keys("u");
    chk("um unico undo remove a colagem multilinha", E.num_lines == before, "");

    /* ESC dentro da colagem nao vira comando nem conteudo */
    keys("i");
    int nl_before = E.num_lines;
    keys("\x1b[200~a\x1b" "b dd\x1b[201~");
    chk("ESC colado e descartado", strstr(E.lines[E.cursor_y].chars, "\x1b") == NULL &&
        strstr(E.lines[E.cursor_y].chars, "ab dd") != NULL, E.lines[E.cursor_y].chars);
    chk("texto colado nao executa comandos", E.num_lines == nl_before && E.mode == MODE_INSERT, "");
    esc();
    keys(":w\r");
}

static void t_sidebar_search(void) {
    printf("\n-- busca na lista lateral --\n");
    E.focus = FOCUS_LEFT;
    E.mode = MODE_NORMAL;
    keys("\x06"); /* Ctrl+F */
    chk("Ctrl+F entra na busca", E.mode == MODE_SEARCH_LEFT, "");
    keys("cao");
    chk("busca sem acento acha 'ção' (Beta)", E.num_filtered == 1 &&
        strcmp(E.notes[E.filtered_indices[0]].title, "Beta") == 0, "");
    frame();
    chk("quadro com busca desenhado", g_frame_len > 1000, "");
    esc();
    keys("\x15"); /* Ctrl+U limpa? se nao, apaga manualmente */
    E.search_query[0] = '\0';
    E.search_cursor = 0;
    apply_note_search("");
    chk("limpar a busca mostra todas", E.num_filtered == 4, "");
}

static void t_new_note_prompt(void) {
    printf("\n-- criar nota pelo prompt --\n");
    E.focus = FOCUS_LEFT;
    E.mode = MODE_NORMAL;
    keys("n");
    chk("n abre o prompt de nova nota", E.mode == MODE_PROMPT_NEW_NOTE, "");
    keys("Nova de Teste\r");
    chk("nota criada e listada", note_index_by_title("Nova de Teste") >= 0, E.status_msg);
    chk("nota aberta em aba", strstr(E.cur_filename, "Nova de Teste") != NULL, E.cur_filename);
}

static void t_big_note(void) {
    printf("\n-- nota grande: abrir, navegar e desenhar --\n");
    ABuffer big;
    ab_init(&big);
    ab_append(&big, "# Grande\n", -1);
    for (int i = 0; i < 20000; i++) {
        char t[200];
        int n = snprintf(t, sizeof t,
            "%s linha %d com texto suficiente para quebrar em telas estreitas de terminal, **negrito** e [link](https://exemplo.com/%d)\n",
            (i % 500 == 0) ? "## Secao" : "-", i, i);
        ab_append(&big, t, n);
    }
    int id = db_insert_note("Grande", big.b, time(NULL), 0);
    ab_free(&big);
    chk("nota grande inserida", id > 0, "");
    rescan_notes();

    int64_t t0 = get_current_time_ms();
    chk("abre a nota grande", open_by_title("Grande"), "");
    int64_t t_open = get_current_time_ms() - t0;
    chk("20001 linhas carregadas", E.num_lines == 20001, "");

    t0 = get_current_time_ms();
    for (int i = 0; i < 20; i++) frame();
    int64_t t_frames = get_current_time_ms() - t0;

    t0 = get_current_time_ms();
    keys("G");
    frame();
    bool no_fim = (E.cursor_y == E.num_lines - 1);
    bool desenha_fim = frame_text_has("linha 19999 ");
    keys("gg");
    frame();
    int64_t t_nav = get_current_time_ms() - t0;
    chk("G vai a ultima linha", no_fim, "");
    chk("quadro mostra a ultima linha apos G", desenha_fim, "");
    chk("gg volta ao inicio", E.cursor_y == 0, "");

    keys("zM");
    frame();
    chk("zM recolhe secoes", E.lines[0].folded, "");
    keys("zR");

    char msg[160];
    snprintf(msg, sizeof msg, "abrir=%lld ms, 20 quadros=%lld ms, G/gg=%lld ms",
             (long long)t_open, (long long)t_frames, (long long)t_nav);
    printf("  [info] %s\n", msg);
    chk("abrir < 300 ms", t_open < 300, msg);
    chk("quadro medio < 50 ms", t_frames < 1000, msg);
}

static void t_quit(void) {
    printf("\n-- saida protegida --\n");
    chk("abre Gama", open_by_title("Gama"), "");
    keys("G");
    keys("A");
    keys(" alterado");
    esc();
    keys(":q\r");
    chk(":q com alteracao pendente NAO sai", !E.quit_requested, "saiu perdendo dados");
    keys(":wq\r");
    chk(":wq salva e sai", E.quit_requested, E.status_msg);
    char *c = db_text_of("Gama");
    chk("alteracao gravada antes de sair", c && strstr(c, "gama alterado") != NULL, c ? c : "(nulo)");
    free(c);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "uso: tui_test <pasta-fixtures>\n");
        return 2;
    }
    setlocale(LC_ALL, "");
    _putenv("LINES=30");
    _putenv("COLUMNS=120");

    printf("== teste de integracao da interface ==\n");
    t_startup();
    t_import(argv[1]);
    t_escape_injection();
    t_editing();
    t_bracketed_paste();
    t_sidebar_search();
    t_new_note_prompt();
    t_big_note();
    t_quit();

    free(g_frame);
    db_close();
    printf("\n== %s (%d falha(s)) ==\n", falhas ? "FALHAS" : "TUDO OK", falhas);
    return falhas ? 1 : 0;
}
