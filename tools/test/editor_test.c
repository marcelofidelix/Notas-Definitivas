/* =============================================================================
 * editor_test.c - Testes de regressao da logica de edicao (sem interface TUI)
 * -----------------------------------------------------------------------------
 * Compila o proprio notas.c como unidade de traducao (renomeando main()), como
 * o storage_test.c, e exercita a logica de edicao diretamente:
 *   - comando :s/antigo/novo/ (campos longos, sem barra final, padrao vazio);
 *   - desfazer/refazer (round trip, cursor, dobras, limite de niveis);
 *   - agrupamento de digitacao em um unico passo de undo;
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
#ifdef NOTAS_HAS_PROMPT_LABEL
    t_prompt_label();
#endif

    undo_stacks_free();
    printf("\n== %s (%d falha(s)) ==\n", falhas ? "FALHAS" : "TUDO OK", falhas);
    return falhas ? 1 : 0;
}
