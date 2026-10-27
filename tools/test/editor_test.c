/* =============================================================================
 * editor_test.c - Testes de regressao da logica de edicao (sem interface TUI)
 * -----------------------------------------------------------------------------
 * Compila o proprio notas.c como unidade de traducao (renomeando main()), como
 * o storage_test.c, e exercita a logica de edicao diretamente:
 *   - comando :s/antigo/novo/ (campos longos, sem barra final, padrao vazio);
 *   - desfazer/refazer (round trip, cursor, dobras, limite de niveis);
 *   - agrupamento de digitacao em um unico passo de undo;
 *   - colagem multilinha em lote, carga/exclusao de linhas em lote;
 *   - remocao de bytes de controle no texto enviado ao terminal;
 *   - rotulo dos prompts (fonte unica para desenho, cursor e mouse).
 * Nao abre terminal nem banco de dados.
 *
 * Uso: editor_test
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

/* ---- helpers de documento ------------------------------------------------ */

static void undo_stacks_free(void) {
    for (int i = 0; i < E.undo_count; i++) free_undo_state(&E.undo_stack[i]);
    for (int i = 0; i < E.redo_count; i++) free_undo_state(&E.redo_stack[i]);
    E.undo_count = 0;
    E.redo_count = 0;
}

static void doc_reset(void) {
    undo_stacks_free();
    invalidate_typing_undo();
    for (int i = 0; i < E.num_lines; i++) line_free(&E.lines[i]);
    free(E.lines);
    E.lines = NULL;
    E.num_lines = 0;
    E.cap_lines = 0;
    E.cursor_x = 0;
    E.cursor_y = 0;
    E.modified = false;
    E.status_msg[0] = '\0';
}

static void doc_set(int n, const char *const *ls) {
    doc_reset();
    for (int i = 0; i < n; i++) buf_insert_line(E.num_lines, ls[i], (int)strlen(ls[i]));
}

static bool doc_is(int n, const char *const *ls) {
    if (E.num_lines != n) return false;
    for (int i = 0; i < n; i++) {
        if (strcmp(E.lines[i].chars, ls[i]) != 0) return false;
    }
    return true;
}

static bool line0_is(const char *s) {
    return E.num_lines >= 1 && strcmp(E.lines[0].chars, s) == 0;
}

/* ---- :s/antigo/novo/ ----------------------------------------------------- */

static void t_substitute(void) {
    printf("\n-- comando :s/antigo/novo/ --\n");
    char cmd[600];
    char big[201];
    char pat[201];
    char expect[700];
    memset(big, 'x', 200); big[200] = '\0';
    memset(pat, 'a', 200); pat[200] = '\0';

    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    execute_bottom_command("s/foo/baz/");
    chk("s/foo/baz/ substitui todas as ocorrencias", line0_is("baz bar baz"), E.lines[0].chars);

    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    execute_bottom_command("s/foo/qux");
    chk("sem a barra final tambem substitui", line0_is("qux bar qux"), E.lines[0].chars);

    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    execute_bottom_command("%s/foo/zzz/g");
    chk("%s/foo/zzz/g substitui", line0_is("zzz bar zzz"), E.lines[0].chars);

    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    execute_bottom_command("sub/bar/BAR/");
    chk("sub/bar/BAR/ substitui", line0_is("foo BAR foo"), E.lines[0].chars);

    /* Regressao: texto de substituicao >= 128 bytes virava string vazia e a
     * ocorrencia era APAGADA em vez de substituida. */
    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    snprintf(cmd, sizeof cmd, "s/foo/%s/", big);
    execute_bottom_command(cmd);
    snprintf(expect, sizeof expect, "%s bar %s", big, big);
    chk("substituicao de 200 bytes nao apaga o texto", line0_is(expect),
        E.num_lines ? E.lines[0].chars : "(sem linhas)");

    /* Regressao: o mesmo para a substituicao sem barra final (era truncada). */
    { const char *d[] = { "foo bar foo" }; doc_set(1, d); }
    snprintf(cmd, sizeof cmd, "s/foo/%s", big);
    execute_bottom_command(cmd);
    chk("substituicao longa sem barra final nao e truncada", line0_is(expect),
        E.num_lines ? E.lines[0].chars : "(sem linhas)");

    /* Regressao: padrao >= 128 bytes era ignorado em silencio. */
    {
        char linha[300];
        snprintf(linha, sizeof linha, "%sZ", pat);
        const char *d[] = { linha };
        doc_set(1, d);
    }
    snprintf(cmd, sizeof cmd, "s/%s/Y/", pat);
    execute_bottom_command(cmd);
    chk("padrao de 200 bytes e aplicado", line0_is("YZ"), E.num_lines ? E.lines[0].chars : "(sem linhas)");

    /* Regressao: padrao vazio nao pode ser ignorado sem aviso. */
    { const char *d[] = { "foo" }; doc_set(1, d); }
    execute_bottom_command("s//x/");
    chk("padrao vazio nao altera o texto", line0_is("foo"), E.lines[0].chars);
    chk("padrao vazio informa o uso correto", strstr(E.status_msg, "Uso") != NULL, E.status_msg);

    /* Nenhuma ocorrencia: informa e nao consome undo. */
    { const char *d[] = { "foo" }; doc_set(1, d); }
    execute_bottom_command("s/nao_existe/x/");
    chk("sem ocorrencias nao consome undo", E.undo_count == 0, "undo_count != 0");
    doc_reset();
}

/* ---- desfazer / refazer -------------------------------------------------- */

static void t_undo_redo(void) {
    printf("\n-- desfazer / refazer --\n");
    const char *base[] = { "a", "b", "c" };
    const char *edit[] = { "a", "BBB", "c" };

    doc_set(3, base);
    E.cursor_y = 1;
    E.cursor_x = 1;
    save_undo();
    line_replace_range(&E.lines[1], 0, 1, "BBB", 3);
    E.cursor_x = 3;

    apply_undo();
    chk("undo restaura o texto", doc_is(3, base), E.num_lines ? E.lines[1].chars : "");
    chk("undo restaura o cursor", E.cursor_y == 1 && E.cursor_x == 1, "cursor incorreto");
    chk("undo move o estado atual para o redo", E.redo_count == 1 && E.undo_count == 0, "contagens incorretas");

    apply_redo();
    chk("redo reaplica a edicao", doc_is(3, edit), E.num_lines ? E.lines[1].chars : "");
    chk("redo restaura o cursor", E.cursor_y == 1 && E.cursor_x == 3, "cursor incorreto");
    chk("redo devolve o estado ao undo", E.undo_count == 1 && E.redo_count == 0, "contagens incorretas");

    /* Uma nova edicao descarta o redo. */
    apply_undo();
    save_undo();
    chk("nova edicao descarta o redo", E.redo_count == 0, "redo nao foi limpo");

    /* Pilhas vazias nao fazem nada e nao quebram. */
    doc_set(3, base);
    apply_undo();
    chk("undo com pilha vazia e inofensivo", doc_is(3, base) && E.undo_count == 0, "estado alterado");
    apply_redo();
    chk("redo com pilha vazia e inofensivo", doc_is(3, base) && E.redo_count == 0, "estado alterado");

    /* O estado de dobra (folded/hidden) precisa sobreviver ao round trip. */
    {
        const char *d[] = { "# Titulo", "corpo", "## Sub", "mais" };
        doc_set(4, d);
        E.lines[0].folded = true;
        update_folding();
        bool oculto_antes = E.lines[1].hidden && E.lines[2].hidden && E.lines[3].hidden;
        save_undo();
        line_replace_range(&E.lines[1], 0, 5, "novo", 4);
        apply_undo();
        chk("dobra: cabecalho continua recolhido apos undo", E.lines[0].folded, "folded perdido");
        chk("dobra: linhas continuam ocultas apos undo", oculto_antes && E.lines[1].hidden && E.lines[3].hidden,
            "hidden perdido");
        chk("dobra: nivel do cabecalho preservado", E.lines[0].heading_lvl == 1 && E.lines[2].heading_lvl == 2,
            "heading_lvl perdido");
    }

    /* Limite de niveis: nunca ultrapassa MAX_UNDO e descarta o mais antigo. */
    doc_set(3, base);
    for (int i = 0; i < MAX_UNDO + 50; i++) {
        char t[16];
        snprintf(t, sizeof t, "v%d", i);
        line_replace_range(&E.lines[0], 0, E.lines[0].len, t, (int)strlen(t));
        save_undo();
    }
    chk("pilha de undo respeita MAX_UNDO", E.undo_count == MAX_UNDO, "undo_count fora do limite");
    int desfeitos = 0;
    while (E.undo_count > 0) { apply_undo(); desfeitos++; }
    chk("desfaz exatamente MAX_UNDO niveis", desfeitos == MAX_UNDO && E.redo_count == MAX_UNDO, "contagem incorreta");
    int refeitos = 0;
    while (E.redo_count > 0) { apply_redo(); refeitos++; }
    chk("refaz exatamente MAX_UNDO niveis", refeitos == MAX_UNDO && E.undo_count == MAX_UNDO, "contagem incorreta");
    doc_reset();
}

/* ---- agrupamento de digitacao -------------------------------------------- */

static void t_typing_coalesce(void) {
    printf("\n-- agrupamento de digitacao --\n");
    const char *base[] = { "" };
    doc_set(1, base);
    E.cur_filepath[0] = '\0';

    save_undo_for_typing(0, 0, 1);
    save_undo_for_typing(0, 1, 1);
    save_undo_for_typing(0, 2, 1);
    chk("3 teclas seguidas geram 1 unico passo de undo", E.undo_count == 1, "mais de um snapshot");

    save_undo_for_typing(0, 10, 1);
    chk("salto de posicao abre um novo passo", E.undo_count == 2, "nao abriu novo snapshot");

    save_undo_for_typing(1, 0, 1);
    chk("troca de linha abre um novo passo", E.undo_count == 3, "nao abriu novo snapshot");
    doc_reset();
}

/* ---- colagem multilinha em lote ------------------------------------------ */

static void t_paste(void) {
    printf("\n-- colagem multilinha --\n");
    E.mode = MODE_NORMAL;

    { const char *d[] = { "hello world" }; doc_set(1, d); }
    E.cursor_y = 0; E.cursor_x = 6;
    editor_paste_string("A\r\nB\nC", 6);
    {
        const char *exp[] = { "hello A", "B", "Cworld" };
        chk("LF/CRLF dividem linhas e o resto da linha vai para o fim", doc_is(3, exp),
            E.num_lines > 2 ? E.lines[2].chars : "(linhas faltando)");
    }
    chk("cursor no fim do texto colado", E.cursor_y == 2 && E.cursor_x == 1, "cursor incorreto");
    apply_undo();
    { const char *orig[] = { "hello world" }; chk("undo desfaz a colagem inteira", doc_is(1, orig), E.lines[0].chars); }

    { const char *d[] = { "ab" }; doc_set(1, d); }
    E.cursor_y = 0; E.cursor_x = 1;
    editor_paste_string("X\n", 2);
    { const char *exp[] = { "aX", "b" }; chk("quebra final abre linha com o restante", doc_is(2, exp), "conteudo incorreto"); }
    chk("cursor no inicio da nova linha", E.cursor_y == 1 && E.cursor_x == 0, "cursor incorreto");

    { const char *d[] = { "" }; doc_set(1, d); }
    editor_paste_string("1\r2", 3);
    { const char *exp[] = { "1", "2" }; chk("CR isolado tambem quebra a linha", doc_is(2, exp), "conteudo incorreto"); }

    { const char *d[] = { "# T", "fim" }; doc_set(2, d); }
    E.cursor_y = 0; E.cursor_x = 3;
    editor_paste_string("\n```\ncodigo\n```", 15);
    chk("colagem recalcula blocos de codigo uma vez no fim",
        E.num_lines == 5 && E.lines[1].is_code_fence && E.lines[2].in_code_block && !E.lines[4].in_code_block,
        "flags de sintaxe incorretas");

    { const char *d[] = { "xy" }; doc_set(1, d); }
    E.cursor_y = 0; E.cursor_x = 1;
    editor_paste_string("123", 3);
    chk("colagem de uma linha insere no cursor", line0_is("x123y") && E.cursor_x == 4, E.lines[0].chars);
    doc_reset();
}

/* ---- carga e exclusao em lote -------------------------------------------- */

static void t_bulk_lines(void) {
    printf("\n-- carga e exclusao de linhas em lote --\n");
    const int N = 50000;
    ABuffer src;
    ab_init(&src);
    for (int i = 0; i < N; i++) {
        char t[32];
        int n = snprintf(t, sizeof t, (i % 1000 == 0) ? "```\r\n" : "linha %d\n", i);
        ab_append(&src, t, n);
    }
    doc_reset();
    int64_t t0 = get_current_time_ms();
    editor_load_content(src.b);
    int64_t dt = get_current_time_ms() - t0;
    ab_free(&src);
    chk("carrega todas as linhas", E.num_lines == N, "contagem incorreta");
    chk("CR final removido e cerca detectada", E.lines[0].len == 3 && E.lines[0].is_code_fence, E.lines[0].chars);
    chk("linha interna marcada como bloco de codigo", E.lines[1].in_code_block, "in_code_block incorreto");
    {
        char msg[64];
        snprintf(msg, sizeof msg, "%lld ms", (long long)dt);
        chk("50k linhas carregam em menos de 1 s (era O(n^2))", dt < 1000, msg);
    }

    /* Exclusao visual de varias linhas: junta prefixo e sufixo e remove o meio */
    { const char *d[] = { "abcdef", "meio1", "meio2", "xyz" }; doc_set(4, d); }
    E.mode = MODE_VISUAL_CHAR;
    E.visual_start_y = 0; E.visual_start_x = 2;
    E.cursor_y = 3; E.cursor_x = 1;
    delete_visual_selection_ex(false);
    { const char *exp[] = { "abz" }; chk("exclusao multilinha junta as pontas", doc_is(1, exp), E.lines[0].chars); }

    { const char *d[] = { "a", "b", "c", "d" }; doc_set(4, d); }
    buf_delete_lines(1, 2);
    { const char *exp[] = { "a", "d" }; chk("buf_delete_lines remove o intervalo", doc_is(2, exp), "conteudo incorreto"); }
    buf_delete_lines(0, 99);
    { const char *exp[] = { "" }; chk("apagar tudo deixa uma linha vazia", doc_is(1, exp), "documento sem linha"); }
    E.mode = MODE_NORMAL;
    doc_reset();
}

/* ---- saida de texto para o terminal -------------------------------------- */

static void t_terminal_sanitize(void) {
    printf("\n-- sanitizacao de texto para o terminal --\n");
    ABuffer ab;
    ab_init(&ab);
    /* ESC/OSC 52, BEL, CR, DEL e CSI C1 (U+009B) precisam sumir; UTF-8 fica */
    ab_append_text(&ab, "a\x1b]52;c;QUFB\x07" "b\r\x7f\xc2\x9b" "c\xc3\xa7", -1);
    chk("remove C0, DEL e C1; preserva UTF-8", ab.b && strcmp(ab.b, "a]52;c;QUFBbc\xc3\xa7") == 0,
        ab.b ? ab.b : "(vazio)");
    ab_reset(&ab);
    chk("ab_reset esvazia mantendo a capacidade", ab.len == 0 && ab.cap > 0 && ab.b[0] == '\0', "estado incorreto");
    ab_append_text(&ab, "\xc2\xa0ok", 4);
    chk("NBSP (U+00A0) nao e confundido com C1", ab.len == 4, "NBSP removido");
    ab_free(&ab);
}

/* ---- rotulos dos prompts ------------------------------------------------- */

#ifdef NOTAS_HAS_PROMPT_LABEL
static void t_prompt_label(void) {
    printf("\n-- rotulos dos prompts --\n");
    char buf[512];
    struct { enum EditorMode m; const char *trecho; } casos[] = {
        { MODE_PROMPT_NEW_NOTE,     "Nova nota: " },
        { MODE_PROMPT_RENAME_NOTE,  "Renomear nota: " },
        { MODE_PROMPT_FIND,         "Localizar na nota: " },
        { MODE_PROMPT_REPLACE_FIND, "Localizar para substituir: " },
        { MODE_PROMPT_IMPORT_DIR,   "Importar de: " },
        { MODE_PROMPT_EXPORT_DIR,   "Exportar para: " },
    };
    for (size_t i = 0; i < sizeof casos / sizeof casos[0]; i++) {
        E.mode = casos[i].m;
        bool ok = prompt_label(buf, sizeof buf) && strstr(buf, casos[i].trecho) != NULL;
        chk(casos[i].trecho, ok, buf);
    }
    E.mode = MODE_PROMPT_REPLACE_WITH;
    safe_strcpy(E.replace_find_buf, sizeof E.replace_find_buf, "alvo");
    chk("Substituir 'alvo' por: ", prompt_label(buf, sizeof buf) && strstr(buf, "Substituir 'alvo' por: ") != NULL, buf);
    E.mode = MODE_NORMAL;
    chk("fora de prompt nao ha rotulo", !prompt_label(buf, sizeof buf), "devolveu rotulo");
}
#endif

int main(void) {
    setlocale(LC_ALL, "");
    memset(&E, 0, sizeof(E));
    E.screen_cols = 120;
    E.screen_rows = 30;
    E.left_width = DEFAULT_LEFT_WIDTH;

    printf("== testes de regressao da logica de edicao ==\n");
    t_substitute();
    t_undo_redo();
    t_typing_coalesce();
    t_paste();
    t_bulk_lines();
    t_terminal_sanitize();
#ifdef NOTAS_HAS_PROMPT_LABEL
    t_prompt_label();
#endif

    undo_stacks_free();
    printf("\n== %s (%d falha(s)) ==\n", falhas ? "FALHAS" : "TUDO OK", falhas);
    return falhas ? 1 : 0;
}
