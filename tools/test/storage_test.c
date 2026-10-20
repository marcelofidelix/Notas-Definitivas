/* =============================================================================
 * storage_test.c - Teste de unidade da camada de armazenamento (banco SQLite)
 * -----------------------------------------------------------------------------
 * Compila o proprio notas.c como unidade de traducao (renomeando main()) e
 * exercita a API de armazenamento sem interface. Roda igual no Linux e no
 * Windows, o que permite verificar o ramo especifico de cada plataforma
 * (opendir/readdir no Linux, FindFirstFileW no Windows).
 *
 * Uso: storage_test <pasta_com_md> [pasta_de_saida]
 *   - usa um notas.db no diretorio atual
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

int main(int argc, char **argv) {
    setlocale(LC_ALL, "");
    char err[256] = {0};
    char buf[512];

    if (argc < 2) {
        printf("uso: storage_test <pasta_com_md> [pasta_de_saida]\n");
        return 2;
    }

    printf("== teste da camada de armazenamento SQLite ==\n");
    printf("   pasta de origem : %s\n", argv[1]);
    if (argc > 2) printf("   pasta de destino: %s\n", argv[2]);
    printf("\n");

    chk("db_open cria/abre o banco", db_open(), "notas.db");
    chk("banco nasce VAZIO", db_count_notes() == 0, "");
    chk("caminho virtual montado corretamente", ({
        char p[32]; note_path_from_id(42, p, sizeof(p)); strcmp(p, "db:42") == 0;
    }), "");
    chk("id extraido do caminho virtual", note_id_from_path("db:42") == 42, "");
    chk("caminho real nao e confundido com virtual", note_id_from_path("data/x.md") < 0, "");
    chk("id 0/invalido e rejeitado", note_id_from_path("db:0") < 0, "");
    chk("id com lixo e rejeitado", note_id_from_path("db:12lixo") < 0, "");
    chk("id negativo e rejeitado", note_id_from_path("db:-1") < 0, "");

    {
        char *b64a = base64_encode((const unsigned char *)"a", 1);
        char *b64ab = base64_encode((const unsigned char *)"ab", 2);
        char *b64abc = base64_encode((const unsigned char *)"abc", 3);
        chk("base64 de 1 byte tem padding correto", b64a && strcmp(b64a, "YQ==") == 0, b64a ? b64a : "(nulo)");
        chk("base64 de 2 bytes tem padding correto", b64ab && strcmp(b64ab, "YWI=") == 0, b64ab ? b64ab : "(nulo)");
        chk("base64 de 3 bytes nao tem padding", b64abc && strcmp(b64abc, "YWJj") == 0, b64abc ? b64abc : "(nulo)");
        free(b64a); free(b64ab); free(b64abc);
    }

    {
        const char *tmpname = "atomic_write_test.txt";
        const char *payload = "linha 1\nlinha 2\n";
        chk("escrita atomica de arquivo", write_file_atomic(tmpname, payload, (long)strlen(payload)), "");
        long rlen = 0;
        char *rd = read_entire_file(tmpname, &rlen);
        chk("conteudo gravado de forma atomica", rd && rlen == (long)strlen(payload) &&
            strcmp(rd, payload) == 0, rd ? rd : "(nulo)");
        if (rd) free(rd);
        unlink_utf8(tmpname);
    }

    /* ---------------- importacao ---------------- */
    int added = 0, updated = 0;
    int n = import_notes_from_dir(argv[1], &added, &updated, err, sizeof(err));
    snprintf(buf, sizeof(buf), "n=%d novas=%d atualizadas=%d erro=%s", n, added, updated, err);
    chk("importacao da pasta de markdown", n > 0, buf);
    chk("3 notas importadas (.pdf ignorado)", n == 3, buf);

    int id_alfa = db_find_id_by_name("Alfa");
    int id_beta = db_find_id_by_name("Beta");
    chk("nota 'Alfa' presente", id_alfa > 0, "");
    chk("nota 'Beta' presente", id_beta > 0, "");

    long clen = 0;
    char *c = db_get_content(id_alfa, &clen);
    chk("conteudo da Alfa preservado", c && strstr(c, "conteudo alfa"), c ? c : "(nulo)");
    if (c) free(c);
    c = db_get_content(id_beta, &clen);
    chk("acentos UTF-8 preservados ('ção')", c && strstr(c, "ção"), c ? c : "(nulo)");
    if (c) free(c);

    /* ---------------- CRUD ---------------- */
    int id_novo = db_insert_note("TesteNovo", "# TesteNovo\n\ncorpo\n", time(NULL), 0);
    chk("db_insert_note devolve id", id_novo > 0, "");
    chk("nome duplicado e rejeitado pelo UNIQUE",
        db_insert_note("TesteNovo", "outro\n", time(NULL), 0) < 0, "");
    chk("db_update_content", db_update_content(id_novo, "# TesteNovo\n\neditado\n", time(NULL)), "");
    c = db_get_content(id_novo, &clen);
    chk("conteudo atualizado lido de volta", c && strstr(c, "editado"), c ? c : "(nulo)");
    if (c) free(c);
    chk("db_set_attention", db_set_attention(id_novo, 2), "");
    chk("db_update_name", db_update_name(id_novo, "TesteRenomeado"), "");
    chk("nome antigo nao existe mais", db_find_id_by_name("TesteNovo") < 0, "");
    chk("id preservado apos renomear", db_find_id_by_name("TesteRenomeado") == id_novo, "");
    chk("db_delete_note", db_delete_note(id_novo), "");
    chk("nota removida", db_find_id_by_name("TesteRenomeado") < 0, "");

    /* ---------------- reimportacao atualiza, nao duplica ---------------- */
    added = updated = 0;
    n = import_notes_from_dir(argv[1], &added, &updated, err, sizeof(err));
    snprintf(buf, sizeof(buf), "n=%d novas=%d atualizadas=%d", n, added, updated);
    chk("reimportar nao duplica (atualiza)", added == 0 && updated == 3, buf);
    chk("total permanece 3", db_count_notes() == 3, "");

    /* ---------------- exportacao ---------------- */
    if (argc > 2) {
        int w = export_notes_to_dir(argv[2], err, sizeof(err));
        snprintf(buf, sizeof(buf), "escritas=%d erro=%s", w, err);
        chk("exportacao da pasta .md", w == 3, buf);
    }

    db_close();
    printf("\n== %s (%d falha(s)) ==\n", falhas ? "COM FALHAS" : "TUDO OK", falhas);
    return falhas ? 1 : 0;
}
