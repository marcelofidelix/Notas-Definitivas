//! Testes de integracao: o App inteiro dirigido por teclas e mouse, com banco
//! em memoria, conferindo estado, gravacao no SQLite e os quadros desenhados.

use crate::app::{App, Focus, Mode};
use crate::db::Db;
use crate::input::{map_key, Key};
use crate::mouse::{Btn, Mouse};
use crate::notes::NoteKey;
use crate::render::draw;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn new_app() -> App {
    let mut a = App::new(Db::open_memory());
    a.cols = 120;
    a.rows = 30;
    a
}

fn app_with(notes: &[(&str, &str)]) -> App {
    let db = Db::open_memory();
    for (i, (name, content)) in notes.iter().enumerate() {
        db.insert_note(name, content, 1000 - i as i64, 0).unwrap();
    }
    let mut a = App::new(db);
    a.cols = 120;
    a.rows = 30;
    a
}

/// Digita uma sequencia: caracteres comuns mais `<Esc> <CR> <Tab> <BS> <Del>
/// <Up> <Down> <Left> <Right> <C-x> <S-Left>`.
fn keys(a: &mut App, s: &str) {
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '<' {
            let mut name = String::new();
            for n in it.by_ref() {
                if n == '>' {
                    break;
                }
                name.push(n);
            }
            let k = match name.as_str() {
                "Esc" => Key::Esc,
                "CR" => Key::Enter,
                "Tab" => Key::Tab,
                "BS" => Key::Backspace,
                "Del" => Key::Delete,
                "Up" => Key::Up,
                "Down" => Key::Down,
                "Left" => Key::Left,
                "Right" => Key::Right,
                "S-Left" => Key::ShiftLeft,
                "S-Right" => Key::ShiftRight,
                n if n.starts_with("C-") => Key::Ctrl(n.chars().nth(2).unwrap()),
                other => panic!("tecla desconhecida <{}>", other),
            };
            a.on_key(k);
        } else {
            a.on_key(Key::Char(c));
        }
    }
}

fn open(a: &mut App, title: &str) {
    let key = a.notes.iter().find(|n| n.title == title).expect("nota").key.clone();
    a.free_preview();
    a.open_note(&key, true);
    a.focus = Focus::Right;
    a.mode = Mode::Normal;
}

fn db_text(a: &App, title: &str) -> String {
    let id = a.notes.iter().find(|n| n.title == title).unwrap().key.db_id().unwrap();
    a.db.get_content(id).unwrap()
}

fn lines(a: &App) -> Vec<String> {
    a.buf().unwrap().lines.iter().map(|l| l.text.to_string()).collect()
}

fn visible(frame: &str) -> String {
    let mut out = String::new();
    let mut it = frame.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                for n in it.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn click(a: &mut App, col: i32, row: i32) {
    a.on_mouse(Mouse { btn: Btn::Left, col, row, released: false });
    a.on_mouse(Mouse { btn: Btn::Left, col, row, released: true });
}

#[test]
fn empty_database_guides_the_user() {
    let mut a = new_app();
    assert!(a.notes.is_empty() && a.tabs.is_empty());
    let v = visible(&draw(&mut a));
    assert!(v.contains("Nova Nota") && v.contains("Nenhuma nota aberta"));
    // Sem abas: so criar nota e a linha de comando respondem.
    keys(&mut a, "wGdd");
    assert!(a.tabs.is_empty() && !a.quit, "teclas de edicao nao quebram sem abas");
    keys(&mut a, ":q<CR>");
    assert!(a.quit, ":q funciona sem nenhuma nota aberta");
}

#[test]
fn typing_saving_undo_redo_and_line_commands() {
    let mut a = app_with(&[("Alfa", "# Alfa\n\nconteudo alfa\n")]);
    open(&mut a, "Alfa");
    keys(&mut a, "Go");
    assert_eq!(a.mode, Mode::Insert);
    keys(&mut a, "Linha nova digitada<Esc>");
    assert_eq!(a.mode, Mode::Normal);
    assert_eq!(lines(&a).last().unwrap(), "Linha nova digitada");
    assert!(a.tab().unwrap().modified);
    keys(&mut a, ":w<CR>");
    assert!(!a.tab().unwrap().modified);
    assert!(db_text(&a, "Alfa").ends_with("Linha nova digitada\n"));

    keys(&mut a, "u");
    assert_ne!(lines(&a).last().unwrap(), "Linha nova digitada", "u desfaz a frase inteira");
    keys(&mut a, "<C-r>");
    assert_eq!(lines(&a).last().unwrap(), "Linha nova digitada");

    let n = lines(&a).len();
    keys(&mut a, "ggdd");
    assert_eq!(lines(&a).len(), n - 1);
    keys(&mut a, "u");
    assert_eq!(lines(&a)[0], "# Alfa");

    keys(&mut a, "ggVjd");
    assert_eq!(lines(&a).len(), n - 2);
    keys(&mut a, "u");
    assert_eq!(lines(&a).len(), n);

    keys(&mut a, ":%s/alfa/ALFA/g<CR>");
    assert!(lines(&a).iter().any(|l| l.contains("ALFA")));
    keys(&mut a, "u");
    assert!(!lines(&a).iter().any(|l| l.contains("ALFA")));
}

#[test]
fn unicode_editing_never_splits_characters() {
    let mut a = app_with(&[("U", "x\n")]);
    open(&mut a, "U");
    keys(&mut a, "A ação 😀<BS><BS>é<Esc>");
    assert_eq!(lines(&a)[0], "x açãoé");
    keys(&mut a, "0lx");
    assert_eq!(lines(&a)[0], "xaçãoé", "x apaga o espaco, sem corromper o resto");
    keys(&mut a, "$x");
    assert_eq!(lines(&a)[0], "xação", "$ vai ao ultimo caractere e x apaga o é inteiro");
    for l in lines(&a) {
        assert!(std::str::from_utf8(l.as_bytes()).is_ok());
    }
}

#[test]
fn multi_key_commands() {
    let mut a = app_with(&[("M", "# T\numa linha\noutra linha\nfim\n")]);
    open(&mut a, "M");
    keys(&mut a, "G");
    assert_eq!(a.buf().unwrap().cy, 3);
    keys(&mut a, "gg");
    assert_eq!(a.buf().unwrap().cy, 0);
    keys(&mut a, "jx");
    assert_eq!(lines(&a)[1], "ma linha");
    keys(&mut a, "rZ");
    assert_eq!(lines(&a)[1], "Za linha", "r<char> troca o caractere sob o cursor");
    keys(&mut a, "dx");
    assert_eq!(lines(&a).len(), 4, "dd so remove com duas teclas d");
    keys(&mut a, "<Esc>zM");
    assert!(a.buf().unwrap().lines[0].folded, "zM recolhe os cabecalhos");
    keys(&mut a, "zR");
    assert!(!a.buf().unwrap().lines[0].folded);
    keys(&mut a, ":3<CR>");
    assert_eq!(a.buf().unwrap().cy, 2);
}

#[test]
fn visual_mode_yank_replace_and_paste_text() {
    let mut a = app_with(&[("V", "abcdef\nsegunda\n")]);
    open(&mut a, "V");
    keys(&mut a, "vlly");
    assert_eq!(a.yank, "abc");
    assert_eq!(a.mode, Mode::Normal);
    keys(&mut a, "0vllrX");
    assert_eq!(lines(&a)[0], "XXXdef");
    keys(&mut a, "jVy");
    assert_eq!(a.yank, "segunda\n");
    assert!(a.yank_linewise);
}

#[test]
fn bracketed_paste_is_atomic_and_never_runs_as_commands() {
    let mut a = app_with(&[("P", "inicio\n")]);
    open(&mut a, "P");
    keys(&mut a, "Go");
    let before = lines(&a).len();
    a.paste_text("colada1\r\ncolada2\ncolada3");
    assert_eq!(lines(&a).len(), before + 2);
    assert_eq!(a.mode, Mode::Insert, "o texto colado nao muda de modo");
    keys(&mut a, "<Esc>u");
    assert_eq!(lines(&a).len(), before, "um unico undo remove a colagem");
    a.mode = Mode::Insert;
    a.paste_text("a dd:q<CR>b");
    assert!(!a.quit);
    assert!(lines(&a).iter().any(|l| l.contains("a dd:q")));
}

#[test]
fn line_wise_paste_goes_below_the_current_line() {
    let mut a = app_with(&[("D", "AAA\nBBB\nCCC")]);
    open(&mut a, "D");
    keys(&mut a, "y"); // copia "AAA" (linha inteira)
    keys(&mut a, "j"); // cursor em "BBB"
    keys(&mut a, "p");
    assert_eq!(lines(&a), ["AAA", "BBB", "AAA", "CCC"], "p cola abaixo da atual");
    assert_eq!(a.buf().unwrap().cy, 2, "cursor na linha colada");
    assert!(a.status_text().unwrap_or("").contains("abaixo"), "{:?}", a.status_text());

    keys(&mut a, "u");
    assert_eq!(lines(&a), ["AAA", "BBB", "CCC"]);

    // `P` continua colando acima.
    keys(&mut a, "ggj");
    keys(&mut a, "P");
    assert_eq!(lines(&a), ["AAA", "AAA", "BBB", "CCC"], "P cola acima");
    keys(&mut a, "u");

    // Sem linha abaixo: a linha nova e criada no fim.
    keys(&mut a, "G");
    assert_eq!(a.buf().unwrap().cy, 2);
    keys(&mut a, "y");
    keys(&mut a, "p");
    assert_eq!(lines(&a), ["AAA", "BBB", "CCC", "CCC"]);
    assert_eq!(a.buf().unwrap().cy, 3, "a linha nova existe abaixo da ultima");
    keys(&mut a, "u");

    // Prefixo numerico: cada copia em sua propria linha.
    keys(&mut a, "gg");
    keys(&mut a, "y");
    keys(&mut a, "2p");
    assert_eq!(lines(&a), ["AAA", "AAA", "AAA", "BBB", "CCC"]);

    // Selecao por caractere que atravessa linhas continua entrando no cursor.
    let mut a = app_with(&[("D", "um\ndois\ntres")]);
    open(&mut a, "D");
    keys(&mut a, "vjy"); // "um\nd" (nao termina em quebra: nao e registro de linha)
    assert!(!a.yank_linewise);
    keys(&mut a, "G0");
    keys(&mut a, "p");
    assert_eq!(lines(&a), ["um", "dois", "um", "dtres"], "colagem por caractere e no cursor");
}

#[test]
fn sidebar_search_filters_without_accents_and_cancels_cleanly() {
    let mut a = app_with(&[
        ("Alfa", "# Alfa\nconteudo\n"),
        ("Beta", "# Beta\nbeta: ção\n"),
        ("Gama", "# Gama\n"),
    ]);
    keys(&mut a, "<C-f>");
    assert_eq!(a.mode, Mode::SearchLeft);
    keys(&mut a, "cao");
    assert_eq!(a.filtered.len(), 1);
    assert_eq!(a.selected_note().unwrap().title, "Beta");
    assert!(a.preview.is_some(), "a previa acompanha a selecao");
    keys(&mut a, "<BS><BS><BS>");
    assert_eq!(a.filtered.len(), 3);
    keys(&mut a, "gama<Left><Left>X");
    assert_eq!(a.search_query, "gaXma");
    keys(&mut a, "<Esc>");
    assert_eq!(a.mode, Mode::Normal);
    assert!(a.search_query.is_empty() && a.filtered.len() == 3);
    assert_eq!(a.tabs.len(), 1, "Esc nao abre nenhuma nota nova");
}

#[test]
fn prompts_create_rename_and_delete_notes() {
    let mut a = app_with(&[("Antiga", "# Antiga\ncorpo\n")]);
    a.focus = Focus::Left;
    keys(&mut a, "n");
    assert_eq!(a.mode, Mode::PromptNew);
    keys(&mut a, "Nova de Teste<CR>");
    assert!(a.notes.iter().any(|n| n.title == "Nova de Teste"));
    assert_eq!(a.tab().unwrap().filename, "Nova de Teste.md");
    assert_eq!(db_text(&a, "Nova de Teste"), "# Nova de Teste\n\n");

    // Nome com caracteres invalidos vira nome seguro.
    a.focus = Focus::Left;
    keys(&mut a, "nA/B:C<CR>");
    assert!(a.notes.iter().any(|n| n.title == "A_B_C"));

    // Renomear a nota selecionada ajusta o titulo "# ..." tambem.
    a.focus = Focus::Left;
    let idx = a.filtered.iter().position(|&i| a.notes[i].title == "Antiga").unwrap();
    a.selected = idx;
    keys(&mut a, "r");
    assert_eq!(a.mode, Mode::PromptRename);
    keys(&mut a, "<C-u>Moderna<CR>");
    assert!(a.notes.iter().any(|n| n.title == "Moderna"));
    assert!(db_text(&a, "Moderna").starts_with("# Moderna\n"));
    assert!(!a.notes.iter().any(|n| n.title == "Antiga"));

    // Renomear para um nome que ja existe e recusado.
    a.focus = Focus::Left;
    a.selected = a.filtered.iter().position(|&i| a.notes[i].title == "Moderna").unwrap();
    keys(&mut a, "r<C-u>A_B_C<CR>");
    assert!(a.notes.iter().any(|n| n.title == "Moderna"), "conflito de nome nao renomeia");

    // Excluir pede confirmacao.
    a.focus = Focus::Left;
    a.selected = a.filtered.iter().position(|&i| a.notes[i].title == "Moderna").unwrap();
    keys(&mut a, "dn");
    assert!(a.notes.iter().any(|n| n.title == "Moderna"), "n cancela a exclusao");
    keys(&mut a, "ds");
    assert!(!a.notes.iter().any(|n| n.title == "Moderna"));
    assert!(a.tabs.iter().all(|t| t.title != "Moderna"), "abas da nota excluida fecham");
}

#[test]
fn urgency_marking_filters_and_ordering() {
    let mut a = app_with(&[("Um", "# Um\n"), ("Dois", "# Dois\n"), ("Tres", "# Tres\n")]);
    a.focus = Focus::Left;
    a.selected = a.filtered.iter().position(|&i| a.notes[i].title == "Tres").unwrap();
    keys(&mut a, "m");
    assert_eq!(a.notes[a.filtered[0]].title, "Tres", "marcada sobe para o topo");
    assert_eq!(a.selected_note().unwrap().title, "Tres", "selecao acompanha a nota");
    keys(&mut a, "1");
    assert_eq!(a.filtered.len(), 1);
    keys(&mut a, "1");
    assert_eq!(a.filtered.len(), 3, "mesmo filtro de novo remove");
    keys(&mut a, "mmmmmm");
    assert_eq!(a.selected_note().unwrap().attention, 0, "7 niveis, volta a sem marcacao");
    keys(&mut a, "m:atencao 6<CR>");
    // :atencao N atua sobre a nota ativa (aba aberta).
    open(&mut a, "Um");
    keys(&mut a, ":atencao h<CR>");
    assert_eq!(a.notes.iter().find(|n| n.title == "Um").unwrap().attention, 4);
}

#[test]
fn tables_align_and_navigate() {
    let mut a = app_with(&[("T", "| a | b |\n|-|-|\n| ccc | d |\n")]);
    open(&mut a, "T");
    keys(&mut a, ":table<CR>");
    assert_eq!(lines(&a)[0], "| a   | b   |");
    assert_eq!(lines(&a)[2], "| ccc | d   |");
    keys(&mut a, "Gi<Tab><Tab><Tab>");
    assert!(lines(&a).len() >= 4, "Tab na ultima celula cria nova linha");
    keys(&mut a, "<Esc>u");
}

#[test]
fn wiki_links_open_and_create_notes() {
    let mut a = app_with(&[("Origem", "ver [[Destino Novo]] aqui\n")]);
    open(&mut a, "Origem");
    keys(&mut a, "<C-n>");
    assert_eq!(a.buf().unwrap().cx, 4);
    keys(&mut a, "<CR>");
    assert_eq!(a.tab().unwrap().title, "Destino Novo");
    assert!(db_text(&a, "Destino Novo").contains("Nova nota criada via link."));
    keys(&mut a, "<BS>");
    assert_eq!(a.tab().unwrap().title, "Origem", "Backspace volta no historico");
}

#[test]
fn quit_is_protected_and_wq_saves_everything() {
    let mut a = app_with(&[("A", "x\n"), ("B", "y\n")]);
    open(&mut a, "A");
    keys(&mut a, "iprimeiro <Esc>");
    open(&mut a, "B");
    keys(&mut a, "isegundo <Esc>");
    keys(&mut a, ":q<CR>");
    assert!(!a.quit, ":q recusa com alteracoes pendentes (mesmo em outra aba)");
    keys(&mut a, ":wq<CR>");
    assert!(a.quit);
    assert!(db_text(&a, "A").starts_with("primeiro "));
    assert!(db_text(&a, "B").starts_with("segundo "));
}

#[test]
fn close_commands_never_lose_unsaved_text() {
    let mut a = app_with(&[("A", "x\n"), ("B", "y\n")]);
    open(&mut a, "A");
    keys(&mut a, "iAA<Esc>");
    open(&mut a, "B");
    keys(&mut a, "c");
    assert_eq!(a.tabs.len(), 1, "c fecha a aba ativa");
    assert!(!db_text(&a, "B").is_empty());
    keys(&mut a, "C");
    assert!(a.tabs.is_empty());
    assert!(db_text(&a, "A").starts_with("AA"), "fechar salva antes");
}

#[test]
fn themes_wrap_and_status_commands() {
    let mut a = app_with(&[("A", "x\n")]);
    open(&mut a, "A");
    keys(&mut a, ":theme dracula<CR>");
    assert_eq!(a.theme().name, "Dracula");
    keys(&mut a, ":theme nao-existe<CR>");
    assert_eq!(a.theme().name, "Dracula");
    keys(&mut a, "<C-t>");
    assert_eq!(a.theme().name, "Monokai");
    keys(&mut a, ":nowrap<CR>");
    assert!(!a.wrap);
    keys(&mut a, ":set wrap<CR>");
    assert!(a.wrap);
    keys(&mut a, ":comando_inexistente<CR>");
    assert!(a.status_text().unwrap().contains("não reconhecido"));
}

#[test]
fn import_export_commands_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Um.md"), "# Um\nconteúdo\n").unwrap();
    std::fs::write(dir.path().join("Dois.txt"), "dois\n").unwrap();
    let mut a = new_app();
    keys(&mut a, &format!(":import {}<CR>", dir.path().display()));
    assert_eq!(a.notes.len(), 2);
    let out = dir.path().join("saida");
    keys(&mut a, &format!(":export {}<CR>", out.display()));
    assert_eq!(std::fs::read_to_string(out.join("Um.md")).unwrap(), "# Um\nconteúdo\n");
    // Reimportar atualiza em vez de duplicar.
    keys(&mut a, &format!(":import {}<CR>", dir.path().display()));
    assert_eq!(a.notes.len(), 2);
}

#[test]
fn big_note_opens_scrolls_and_draws_fast() {
    let mut body = String::from("# Grande\n");
    for i in 0..20_000 {
        body.push_str(&format!(
            "{} linha {} com texto suficiente para quebrar em telas estreitas, **negrito** e [l](http://x/{})\n",
            if i % 500 == 0 { "## Secao" } else { "-" },
            i,
            i
        ));
    }
    let mut a = app_with(&[("Grande", &body)]);
    let t0 = std::time::Instant::now();
    open(&mut a, "Grande");
    let open_ms = t0.elapsed().as_millis();
    assert_eq!(a.buf().unwrap().num_lines(), 20_001);
    keys(&mut a, "G");
    let v = visible(&draw(&mut a));
    assert!(v.contains("linha 19999 "), "G mostra a ultima linha");
    keys(&mut a, "gg");
    assert!(visible(&draw(&mut a)).contains("# Grande"));
    keys(&mut a, "zM");
    assert!(a.buf().unwrap().lines[0].folded || a.buf().unwrap().lines[1].folded);
    let t1 = std::time::Instant::now();
    for _ in 0..20 {
        a.needs_redraw = true;
        let _ = draw(&mut a);
    }
    let frame_ms = t1.elapsed().as_millis() / 20;
    eprintln!("nota de 20 mil linhas: abrir={open_ms} ms, quadro={frame_ms} ms");
    assert!(open_ms < 500, "abrir levou {open_ms} ms");
    assert!(frame_ms < 100, "quadro levou {frame_ms} ms");
}

#[test]
fn mouse_buttons_tabs_wheel_and_double_click() {
    let mut a = app_with(&[("Alfa", "# Alfa\nlinha\n"), ("Beta", "# Beta\n")]);
    draw(&mut a);
    // Botao de tema (linha penultima).
    let (s, _) = a.hit.btn_theme;
    let before = a.theme;
    let rows = a.rows;
    click(&mut a, s + 2, rows - 1);
    assert_ne!(a.theme, before);
    // Botao de wrap.
    draw(&mut a);
    let (s, _) = a.hit.btn_wrap;
    let w = a.wrap;
    a.last_theme_click = None;
    let rows = a.rows;
    click(&mut a, s + 2, rows - 1);
    assert_ne!(a.wrap, w);
    // Filtro [1] e depois [Todas].
    draw(&mut a);
    let (s1, _) = a.hit.filter[1];
    click(&mut a, s1, rows - 3);
    assert_eq!(a.filter_urgency, 1);
    let (s0, _) = a.hit.filter[0];
    click(&mut a, s0, rows - 3);
    assert_eq!(a.filter_urgency, 0);
    // Clique simples mostra previa; duplo abre em aba.
    a.free_preview();
    let n_tabs = a.tabs.len();
    click(&mut a, 10, 5);
    assert_eq!(a.tabs.len(), n_tabs);
    assert!(a.preview.is_some());
    click(&mut a, 10, 5);
    assert_eq!(a.focus, Focus::Right);
    assert_eq!(a.tabs.len(), n_tabs + 1, "duplo clique abre a nota");
    // Clique na caixinha de urgencia.
    a.focus = Focus::Left;
    click(&mut a, 3, 4);
    assert!(a.notes.iter().any(|n| n.attention == 1));
    // Roda do mouse na lista nao ultrapassa os limites.
    for _ in 0..50 {
        a.on_mouse(Mouse { btn: Btn::WheelDown, col: 5, row: 6, released: false });
    }
    assert!(a.sidebar_scroll <= a.filtered.len());
    for _ in 0..50 {
        a.on_mouse(Mouse { btn: Btn::WheelUp, col: 70, row: 6, released: false });
    }
    assert_eq!(a.buf().unwrap().scroll_y, 0);
}

#[test]
fn prompt_click_moves_the_cursor_inside_the_field() {
    let mut a = new_app();
    keys(&mut a, ":import abcdef");
    a.mode = Mode::PromptImport;
    a.prompt_buf = "abcdef".into();
    a.prompt_cursor = 6;
    let label_w = crate::text::str_width(&crate::render::prompt_label(&a).unwrap()) as i32;
    let rows = a.rows;
    click(&mut a, label_w + 3, rows);
    assert_eq!(a.prompt_cursor, 2);
}

#[test]
fn drag_selection_in_editor_enters_visual_mode() {
    let mut a = app_with(&[("D", "abcdefghij\n")]);
    open(&mut a, "D");
    draw(&mut a);
    let col0 = a.left_width + 1 + 6 + 1;
    a.on_mouse(Mouse { btn: Btn::Left, col: col0 + 1, row: 2, released: false });
    a.on_mouse(Mouse { btn: Btn::Drag, col: col0 + 5, row: 2, released: false });
    assert_eq!(a.mode, Mode::VisualChar);
    keys(&mut a, "y");
    assert_eq!(a.yank, "bcdef");
}

#[test]
fn double_click_selects_words_without_splitting_utf8() {
    for (text, offset, selected) in [
        ("Olá ação_2, fim", 6, "ação_2"),
        ("um café", 6, "café"),
        ("a 世界 z", 3, "世界"),
        ("x", 0, "x"),
        ("um, dois", 2, ","),
        ("um   dois", 3, "   "),
        ("a 😀 z", 2, "😀"),
    ] {
        let mut a = app_with(&[("D", text)]);
        open(&mut a, "D");
        draw(&mut a);
        let col = a.left_width + 8 + offset;
        click(&mut a, col, 2);
        assert_eq!(a.mode, Mode::Normal);
        click(&mut a, col, 2);
        assert_eq!(a.mode, Mode::VisualChar, "{text}");
        let b = a.buf().unwrap();
        assert_eq!(b.selection_text(false).as_deref(), Some(selected), "{text}");
        assert!(b.cur().text.is_char_boundary(b.cx));
        assert!(b.cur().text.is_char_boundary(b.vis_x));
        keys(&mut a, "<C-c>");
        assert_eq!(a.yank, selected);
        assert_eq!(lines(&a), vec![text]);
    }
}

#[test]
fn triple_click_selects_the_entire_logical_line_and_can_cut_undo() {
    let long = "palavra ".repeat(25);
    let mut a = app_with(&[("D", &format!("{long}\nseguinte"))]);
    open(&mut a, "D");
    draw(&mut a);
    let col = a.left_width + 10;
    // Segunda linha da tela, ainda dentro da primeira linha do documento.
    click(&mut a, col, 3);
    click(&mut a, col, 3);
    assert_eq!(a.mode, Mode::VisualChar);
    click(&mut a, col, 3);
    assert_eq!(a.mode, Mode::VisualLine);
    let b = a.buf().unwrap();
    assert_eq!((b.vis_y, b.cy), (0, 0));
    assert_eq!(b.selection_text(true).unwrap(), format!("{long}\n"));
    keys(&mut a, "x");
    assert_eq!(a.yank, format!("{long}\n"));
    assert_eq!(lines(&a), vec!["seguinte"]);
    keys(&mut a, "u");
    assert_eq!(lines(&a), vec![long, "seguinte".to_string()]);
}

#[test]
fn repeated_clicks_handle_empty_lines_end_of_line_and_empty_database() {
    for text in ["", "abc"] {
        let mut a = app_with(&[("D", text)]);
        open(&mut a, "D");
        draw(&mut a);
        let col = a.left_width + 8 + text.len() as i32;
        click(&mut a, col, 2);
        click(&mut a, col, 2);
        assert_eq!(a.mode, Mode::Normal, "fim da linha nao seleciona a palavra anterior");
        click(&mut a, col, 2);
        assert_eq!(a.mode, Mode::VisualLine);
        keys(&mut a, "y");
        assert_eq!(a.yank, format!("{text}\n"));
    }
    let mut a = new_app();
    let col = a.left_width + 8;
    for _ in 0..3 { click(&mut a, col, 2); }
    assert_eq!(a.mode, Mode::Normal);
    assert!(a.last_editor_click.is_none());
}

#[test]
fn editor_click_sequence_restarts_after_timeout_movement_or_other_input() {
    for interruption in 0..7 {
        let mut a = app_with(&[("D", "alpha beta")]);
        open(&mut a, "D");
        draw(&mut a);
        let mut col = a.left_width + 9;
        click(&mut a, col, 2);
        match interruption {
            0 => a.last_editor_click.as_mut().unwrap().at =
                std::time::Instant::now() - std::time::Duration::from_secs(1),
            1 => col += 6,
            2 => keys(&mut a, "l"),
            3 => a.on_mouse(Mouse { btn: Btn::WheelDown, col, row: 2, released: false }),
            4 => { let rows = a.rows; click(&mut a, col, rows); }
            5 => a.on_mouse(Mouse { btn: Btn::Drag, col: col + 1, row: 2, released: false }),
            _ => a.paste_text("z"),
        }
        click(&mut a, col, 2);
        assert_eq!(a.mode, Mode::Normal, "interrupcao {interruption}");
    }
    let mut a = app_with(&[("D", "alpha beta")]);
    open(&mut a, "D");
    draw(&mut a);
    let col = a.left_width + 9;
    for _ in 0..3 { click(&mut a, col, 2); }
    assert_eq!(a.mode, Mode::VisualLine);
    click(&mut a, col, 2);
    assert_eq!(a.mode, Mode::Normal, "quarto clique inicia nova sequencia");
}

#[test]
fn multi_click_from_insert_keeps_the_original_line_when_markdown_reflows() {
    let text = "[link](https://example.com/um/endereco/muito/longo/que/quebra) palavra\nsegunda";
    let mut a = app_with(&[("D", text)]);
    a.cols = 60;
    a.left_width = crate::app::MIN_LEFT_WIDTH;
    open(&mut a, "D");
    keys(&mut a, "i");
    let col = a.left_width + 9;
    draw(&mut a);
    click(&mut a, col, 3);
    assert_eq!(a.buf().unwrap().cy, 0);
    click(&mut a, col, 3);
    assert_eq!(a.mode, Mode::VisualChar);
    draw(&mut a); // o link agora ocupa menos espaco na tela
    click(&mut a, col, 3);
    assert_eq!(a.mode, Mode::VisualLine);
    assert_eq!(a.buf().unwrap().selection_text(true).unwrap(), format!("{}\n", text.lines().next().unwrap()));
}

#[test]
fn legacy_database_is_migrated_and_indexed_lazily() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notas.db");
    {
        // Banco no formato antigo (o mesmo que a versao em C gravava).
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE notes (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL UNIQUE,
                 content TEXT NOT NULL DEFAULT '',
                 mtime INTEGER NOT NULL DEFAULT 0,
                 attention INTEGER NOT NULL DEFAULT 0);
             INSERT INTO notes(name, content, mtime, attention)
                VALUES('Reuniao', '# Pauta\nAção de Marketing do trimestre\n', 100, 0);",
        )
        .unwrap();
    }
    let mut a = App::new(Db::open(&path).unwrap());
    a.cols = 120;
    a.rows = 30;
    assert_eq!(a.notes.len(), 1, "a coluna nova nao esconde notas antigas");
    // O indice nao e lido na abertura: e carregado na primeira busca.
    assert!(a.notes[0].search_norm.is_empty(), "abrir nao indexa nada");
    // A busca funciona com o indice vindo do banco.
    a.search_query = "ACAO pauta".into();
    a.apply_search();
    assert_eq!(a.filtered.len(), 1);
    assert!(
        a.notes[0].search_norm.contains("acao") && a.notes[0].search_norm.contains("marketing"),
        "indice reconstruido a partir do conteudo: {:?}",
        a.notes[0].search_norm
    );
    // Reconstruir uma vez basta: o indice ficou gravado junto da nota.
    assert!(
        a.db.all_content_norms()[0].1.as_deref().is_some_and(|n| n.contains("marketing")),
        "o indice fica no banco para a proxima abertura"
    );
}

#[test]
fn script_links_are_executed() {
    if !cfg!(windows) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marcador.txt");
    let script = dir.path().join("rodar.bat");
    std::fs::write(&script, "@echo off\r\necho executado> \"%~dp0marcador.txt\"\r\n").unwrap();
    let body = format!("[rodar](<{}>)\n", script.display());
    let mut a = app_with(&[("S", &body)]);
    open(&mut a, "S");
    keys(&mut a, "l<CR>");
    assert!(a.status_text().unwrap_or("").contains("Executando"), "{:?}", a.status_text());
    let mut ok = false;
    for _ in 0..60 {
        if marker.exists() {
            ok = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(ok, "o .bat nao rodou");
}

#[test]
fn rename_and_autosave_bookkeeping() {
    let mut a = app_with(&[("A", "# A\n")]);
    open(&mut a, "A");
    keys(&mut a, "ix<Esc>");
    // Autosave so dispara depois do prazo: forca a marca de tempo para o passado.
    a.tab_mut().unwrap().first_unsaved = Some(std::time::Instant::now() - std::time::Duration::from_secs(61));
    a.check_autosave();
    assert!(!a.tab().unwrap().modified);
    assert!(db_text(&a, "A").starts_with("x# A") || db_text(&a, "A").contains('x'));
}

// ---- medicoes (cargo test --release bench_ -- --ignored --nocapture) -----------

fn big_body(lines: usize) -> String {
    let mut s = String::from("# Grande\n");
    for i in 0..lines {
        s.push_str(&format!(
            "- linha {i} com texto suficiente para ocupar bastante a tela, **negrito**, *italico* e [l](http://x/{i}) ação 日本語\n"
        ));
    }
    s
}

#[test]
#[ignore]
fn bench_frame_and_typing() {
    let mut a = app_with(&[("Grande", &big_body(20_000))]);
    open(&mut a, "Grande");
    keys(&mut a, "10000G");
    let t = std::time::Instant::now();
    let mut bytes = 0;
    for _ in 0..200 {
        a.needs_redraw = true;
        bytes += draw(&mut a).len();
    }
    eprintln!("quadro 120x30: {:.0} us, {} bytes/quadro", t.elapsed().as_micros() as f64 / 200.0, bytes / 200);
    keys(&mut a, "i");
    let t = std::time::Instant::now();
    for i in 0..500 {
        a.on_key(Key::Char(if i % 2 == 0 { 'a' } else { 'b' }));
        let _ = draw(&mut a);
    }
    eprintln!("tecla+quadro (digitar, 20k linhas): {:.0} us", t.elapsed().as_micros() as f64 / 500.0);
    let t = std::time::Instant::now();
    for _ in 0..100 {
        a.on_key(Key::Enter);
    }
    eprintln!("Enter (snapshot de undo, 20k linhas): {:.0} us", t.elapsed().as_micros() as f64 / 100.0);
}

#[test]
#[ignore]
fn bench_startup_with_many_notes() {
    let db = Db::open_memory();
    let body = "# Nota\n".to_string() + &"texto de exemplo com palavras variadas para indexar. ".repeat(200);
    for i in 0..2_000 {
        db.insert_note(&format!("Nota {i}"), &body, 1_000 + i, 0).unwrap();
    }
    let t = std::time::Instant::now();
    let mut a = App::new(db);
    let abertura = t.elapsed();
    // A primeira busca carrega o indice do banco (as aberturas seguintes sao
    // gratuitas porque nada e lido ate alguem procurar).
    let t = std::time::Instant::now();
    a.search_query = "exemplo".into();
    a.apply_search();
    eprintln!(
        "App::new com {} notas de {} KB: {} ms; primeira busca (indice do banco): {} ms",
        a.notes.len(),
        body.len() / 1024,
        abertura.as_millis(),
        t.elapsed().as_millis()
    );
}

#[test]
#[ignore]
fn bench_undo_memory_growth() {
    let mut a = app_with(&[("Grande", &big_body(20_000))]);
    open(&mut a, "Grande");
    let before = crate::doc::approx_undo_bytes(a.buf().unwrap());
    keys(&mut a, "i");
    for _ in 0..150 {
        a.on_key(Key::Enter);
    }
    let after = crate::doc::approx_undo_bytes(a.buf().unwrap());
    eprintln!("memoria do historico de undo: {} KB -> {} KB", before / 1024, after / 1024);
}

// ---- fuzz deterministico -----------------------------------------------------

struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[(self.next() % v.len() as u64) as usize]
    }
}

/// Invariantes que nenhuma sequencia de entradas pode violar.
fn check_invariants(a: &App, step: usize, seed: u64, log: &[String]) {
    let recent = || log[log.len().saturating_sub(8)..].join(" | ");
    for (ti, t) in a.tabs.iter().enumerate() {
        let b = &t.buf;
        assert!(!b.lines.is_empty(), "seed {seed} passo {step}: buffer vazio (aba {ti})");
        assert!(b.cy < b.lines.len(), "seed {seed} passo {step}: cy fora do documento");
        if Some(ti) == a.active {
            let line = &b.lines[b.cy].text;
            assert!(b.cx <= line.len(), "seed {seed} passo {step}: cx > len ({}) modo {:?}; ultimas acoes: {}", b.cx, a.mode, recent());
            assert!(line.is_char_boundary(b.cx), "seed {seed} passo {step}: cursor no meio de um caractere; ultimas acoes: {}", recent());
            // O cursor sempre cai numa linha da tela que existe (tabelas incluidas).
            let lay = a.layout(false);
            let (vrow, _) = crate::layout::cursor_visual_pos(b, &lay);
            assert!(b.cur().hidden || vrow < crate::layout::total_vrows(&b.lines, &lay).max(1), "seed {seed} passo {step}: cursor fora da tela virtual; ultimas acoes: {}", recent());
        }
    }
    if let Some(i) = a.active {
        assert!(i < a.tabs.len());
    }
    assert!(a.filtered.iter().all(|&i| i < a.notes.len()));
    assert!(a.selected == 0 || a.selected < a.filtered.len());
    assert!(a.prompt_buf.is_char_boundary(a.prompt_cursor.min(a.prompt_buf.len())));
    assert!(a.cmd_buf.len() <= 255 && a.prompt_buf.len() <= 255 && a.search_query.len() <= 255);
}

fn fuzz(seed: u64, steps: usize) {
    let body = "# Titulo\n\nTexto com **negrito** e *itálico* ação 😀 日本語 \u{9}tab.\n- item\n\n|a|b|\n|-|-|\n|ççç|d|\n\n|Nome|Descrição|\n|:--|--:|\n|日本語 😀|**negrito** [[Outra]] texto bem comprido que precisa quebrar dentro da celula|\n|x||\n\n```\ncodigo # c\n```\n## Sub\n[[Outra]]\nlinha comprida ".to_string()
        + &"palavra ".repeat(40)
        + "\n";
    let mut a = app_with(&[("Fuzz", &body), ("Outra", "# Outra\n\ncorpo\n"), ("Terceira", "x\n")]);
    open(&mut a, "Fuzz");
    a.diff_render = true; // caminho de producao
    let mut rng = Xs(seed | 1);
    let chars: Vec<char> = "abc xyz日本😀ç*[]()#|-:`\"'./\\0123$%&~gGdyYpPuvVrRxXwbhjklaAiIoOnNzmM!".chars().collect();
    let specials = [
        "<Esc>", "<Esc>", "<CR>", "<Tab>", "<BS>", "<Del>", "<Up>", "<Down>", "<Left>", "<Right>", "<S-Left>",
        "<S-Right>", "<C-f>", "<C-r>", "<C-t>", "<C-n>", "<C-w>", "<C-u>", "<C-a>", "<C-e>", "<C-k>", "<C-v>", "<C-x>",
        "<C-c>", "<C-s>",
    ];
    let cmds = [
        ":w<CR>", ":wrap<CR>", ":zM<CR>", ":zR<CR>", ":table<CR>", ":tableall<CR>", ":nextlink<CR>", ":5<CR>",
        ":%s/a/b/g<CR>", ":s/x/yy<CR>", ":theme<CR>", ":filter 2<CR>", ":filter<CR>", ":atencao 3<CR>", ":rename Zzz<CR>",
        ":tabnext<CR>", ":rescan<CR>", ":w!<CR>", ":e!<CR>", ":link<CR>", ":nextlink<CR>", ":/texto<CR>", ":e Fuzz2<CR>", ":nowrap<CR>", ":set wrap<CR>", ":wrapbar<CR>",
    ];
    let mut log: Vec<String> = Vec::new();
    for step in 0..steps {
        // Enter fora de insercao/prompts abriria links reais (URL, scripts): evita.
        let enter_ok = matches!(
            a.mode,
            Mode::Insert | Mode::Command | Mode::SearchLeft | Mode::ConfirmDelete
        ) || a.mode.is_prompt();
        match rng.next() % 10 {
            0..=4 => {
                let c = *rng.pick(&chars);
                log.push(format!("{c:?}"));
                a.on_key(Key::Char(c));
            }
            5 | 6 => {
                let s = *rng.pick(&specials);
                if s == "<CR>" && !enter_ok {
                    continue;
                }
                log.push(s.to_string());
                keys(&mut a, s);
            }
            7 => {
                let s = *rng.pick(&cmds);
                // So digita comandos a partir do modo normal.
                a.mode = Mode::Normal;
                a.pending = crate::app::Pending::None;
                log.push(s.to_string());
                keys(&mut a, s);
            }
            8 => {
                let btn = *rng.pick(&[Btn::Left, Btn::Left, Btn::Drag, Btn::WheelUp, Btn::WheelDown, Btn::Middle]);
                let (col, row) = ((rng.next() % 130) as i32, (rng.next() % 34) as i32);
                let released = rng.next() % 3 == 0;
                // So o botao [Life] fica de fora: ele bloquearia o teste esperando uma tecla.
                let on_life = row == a.rows - 1 && col >= a.hit.btn_life.0 && col < a.hit.btn_life.1;
                if !on_life {
                    log.push(format!("mouse {btn:?} {col},{row} rel={released}"));
                    a.on_mouse(Mouse { btn, col, row, released });
                }
            }
            _ => {
                if rng.next() % 4 == 0 {
                    a.cols = 10 + (rng.next() % 150) as i32;
                    a.rows = 4 + (rng.next() % 50) as i32;
                    a.clear_needed = true;
                }
                if rng.next() % 3 == 0 {
                    let f = draw(&mut a);
                    assert!(f.starts_with(crate::render::SYNC_BEGIN));
                }
            }
        }
        check_invariants(&a, step, seed, &log);
        if a.quit {
            a.quit = false;
        }
    }
    let _ = draw(&mut a);
    a.emergency_save();
}

#[test]
fn fuzz_random_input_never_panics_or_breaks_invariants() {
    // Campanha maior: NOTAS_FUZZ_SEEDS=200 NOTAS_FUZZ_STEPS=5000 cargo test fuzz
    let seeds: u64 = std::env::var("NOTAS_FUZZ_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let steps: usize = std::env::var("NOTAS_FUZZ_STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    for i in 0..seeds {
        fuzz(i.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i + 1), steps);
    }
}

/// Grava o texto visivel de varios estados da tela em `NOTAS_DUMP_DIR`
/// (inspecao manual): `cargo test dump_frames -- --ignored`
#[test]
#[ignore]
fn dump_frames() {
    let Some(dir) = std::env::var_os("NOTAS_DUMP_DIR") else { return };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let body = "# Boas-vindas\n\nTexto com **negrito**, *itálico* e `código`. Uma linha bem comprida para forçar a quebra visual na tela do editor, com [link](https://example.com/caminho/muito/longo) no meio.\n\n- item um\n* item dois\n\n## Tabela\n\n|Nome|Qtd|\n|:--|--:|\n|maçã|3|\n\n```bash\nsudo apt install foo --yes # c\n\techo \"$HOME\"\n```\n\nEmoji 😀 e 日本語 fim.\n";
    let mut a = app_with(&[("Boas-vindas", body), ("Outra", "# Outra\ncorpo\n")]);
    let save = |a: &mut App, name: &str| {
        let f = draw(a);
        std::fs::write(dir.join(format!("{name}.txt")), visible(&f).replace("\r\n", "\n")).unwrap();
    };
    open(&mut a, "Boas-vindas");
    save(&mut a, "01_normal");
    keys(&mut a, "jjjjj<Down><Down>i");
    save(&mut a, "02_insert");
    keys(&mut a, "<Esc>ggVjj");
    save(&mut a, "03_visual_linha");
    keys(&mut a, "<Esc>:theme git<CR>:");
    save(&mut a, "04_comando");
    keys(&mut a, "<Esc><C-f>out");
    save(&mut a, "05_busca");
    keys(&mut a, "<Esc>");
    open(&mut a, "Outra");
    a.focus = Focus::Left;
    keys(&mut a, "n");
    save(&mut a, "06_prompt_nova");
    keys(&mut a, "<Esc>");
    a.cols = 80;
    a.rows = 20;
    save(&mut a, "07_estreito");
}

/// Gera um banco de demonstracao para inspecao manual:
/// `NOTAS_SEED_DB=caminho cargo test seed_demo_db -- --ignored`
#[test]
#[ignore]
fn seed_demo_db() {
    let Some(path) = std::env::var_os("NOTAS_SEED_DB") else { return };
    let db = Db::open(std::path::Path::new(&path)).unwrap();
    let now = crate::db::now_secs();
    let demo: &[(&str, &str, u8, i64)] = &[
        ("Boas-vindas", "# Boas-vindas\n\nTexto com **negrito**, *itálico* e `código`.\n\n- item um\n* item dois\n\n## Links\n\nVeja [o site](https://example.com \"título\") e a [[Lista de Tarefas]].\n\n## Tabela\n\n|Nome|Qtd|\n|:--|--:|\n|maçã|3|\n|banana|12|\n\n## Código\n\n```bash\nsudo apt install foo --yes # comentário\necho \"$HOME\"\n```\n\nEmoji 😀 e ideogramas 日本語.\n", 1, now),
        ("Lista de Tarefas", "# Lista de Tarefas\n\n- [ ] escrever\n- [x] testar\n", 3, now - 3600),
        ("Uma nota com um titulo bem comprido para testar o letreiro da aba", "# Titulo comprido\n\ncorpo\n", 0, now - 86400 * 2),
        ("Maliciosa", "# Maliciosa\u{1b}]0;hack\u{7}\n\ntexto \u{1b}]52;c;SGFjaw==\u{7} fim\n\u{1b}[2Jrisco\n", 6, now - 86400 * 40),
    ];
    for (name, content, att, mt) in demo {
        let id = db.insert_note(name, content, *mt, *att).unwrap();
        assert!(id > 0);
    }
}

/// Numeros (1-based) das linhas da tela repintadas por um quadro incremental.
fn emitted_rows(out: &str) -> Vec<usize> {
    let mut v = Vec::new();
    for (i, _) in out.match_indices(";1H\x1b[0m") {
        let head = &out[..i];
        let digits: String = head.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
        if let Ok(n) = digits.parse::<usize>() {
            if head[..head.len() - digits.len()].ends_with("\x1b[") {
                v.push(n);
            }
        }
    }
    v
}

#[test]
fn incremental_render_repaints_exactly_the_changed_rows() {
    let body = "# Titulo\n\nTexto **negrito** e [[Outra]] ação 日本語.\n- item\n\n|a|b|\n|-|-|\n|ç|d|\n";
    let mut a = app_with(&[("Alfa", body), ("Outra", "# Outra\ncorpo\n")]);
    a.diff_render = true;
    open(&mut a, "Alfa");
    let first = draw(&mut a);
    assert_eq!(emitted_rows(&first).len() as i32, a.rows, "o primeiro quadro pinta tudo");

    let mut rng = Xs(0xC0FFEE);
    let keys_pool = ["i", "x", "ab ", "<CR>", "<BS>", "<Esc>", "j", "k", "v", "V", "<Down>", "<Up>", "u", "<C-t>", "dd", "G", "gg", "zM", "zR", ":wrap<CR>"];
    for step in 0..400 {
        let old = a.prev_rows.clone();
        let k = *rng.pick(&keys_pool);
        if k == "<CR>" && !matches!(a.mode, Mode::Insert) {
            continue; // evita abrir links reais
        }
        keys(&mut a, k);
        if step % 50 == 0 {
            a.cols = 80 + (rng.next() % 60) as i32;
            a.clear_needed = true;
        }
        let out = draw(&mut a);
        let new = a.prev_rows.clone();
        let em: std::collections::HashSet<usize> = emitted_rows(&out).into_iter().collect();
        if old.len() == new.len() {
            for i in 0..new.len() {
                if old[i] != new[i] {
                    assert!(em.contains(&(i + 1)), "passo {step} ({k:?}): linha {} mudou e nao foi repintada", i + 1);
                }
            }
            if em.len() < new.len() {
                for &r in &em {
                    assert!(old[r - 1] != new[r - 1], "passo {step}: linha {r} repintada sem mudar");
                }
            }
        }
    }
}

#[test]
fn incremental_render_sends_far_fewer_bytes_for_a_keystroke() {
    let mut a = app_with(&[("Alfa", &big_body(500))]);
    a.diff_render = true;
    open(&mut a, "Alfa");
    let full = draw(&mut a).len();
    keys(&mut a, "i");
    let _ = draw(&mut a);
    a.on_key(Key::Char('x'));
    let typed = draw(&mut a).len();
    eprintln!("quadro completo: {full} bytes; uma tecla: {typed} bytes");
    assert!(typed * 5 < full, "uma tecla deveria custar < 20% de um quadro completo ({typed} vs {full})");
    // Sem nada novo, nem as linhas sao reenviadas (so o cursor).
    let idle = draw(&mut a).len();
    assert!(idle < 300, "quadro ocioso: {idle} bytes");
}

#[test]
fn heartbeat_and_resize_force_a_full_repaint() {
    let mut a = app_with(&[("Alfa", "x\n")]);
    a.diff_render = true;
    open(&mut a, "Alfa");
    let _ = draw(&mut a);
    a.force_full = true;
    assert_eq!(emitted_rows(&draw(&mut a)).len() as i32, a.rows);
    a.rows += 2;
    a.clear_needed = true;
    let out = draw(&mut a);
    assert!(out.contains("\x1b[2J") && emitted_rows(&out).len() as i32 == a.rows);
}

#[test]
fn saving_never_silently_overwrites_changes_made_elsewhere() {
    let mut a = app_with(&[("Nota", "original\n")]);
    open(&mut a, "Nota");
    keys(&mut a, "A minha edicao<Esc>");
    // Outra instancia (ou importacao) grava a mesma nota enquanto ela esta aberta.
    let id = a.notes[0].key.db_id().unwrap();
    assert!(a.db.update_content(id, "versao da outra instancia\n", crate::db::now_secs() + 10));

    keys(&mut a, ":w<CR>");
    assert!(a.tab().unwrap().modified, "nada foi gravado");
    assert_eq!(a.db.get_content(id).unwrap(), "versao da outra instancia\n", "o trabalho alheio foi preservado");
    assert!(a.status_text().unwrap().contains("alterada em outro lugar"), "{:?}", a.status_text());

    // :q e fechar a aba tambem recusam, mantendo o aviso do motivo.
    keys(&mut a, ":q<CR>");
    assert!(!a.quit);
    keys(&mut a, "c");
    assert_eq!(a.tabs.len(), 1, "a aba nao fecha enquanto nao for salva");
    assert!(a.status_text().unwrap().starts_with('⚠'));

    // :w! sobrescreve de proposito.
    keys(&mut a, ":w!<CR>");
    assert!(!a.tab().unwrap().modified);
    assert!(a.db.get_content(id).unwrap().contains("minha edicao"));
    keys(&mut a, "ix<Esc>:w<CR>");
    assert!(!a.tab().unwrap().modified, "depois de gravar, o proximo :w funciona normalmente");
}

#[test]
fn reload_discards_edits_and_picks_up_the_other_version() {
    let mut a = app_with(&[("Nota", "original\n")]);
    open(&mut a, "Nota");
    keys(&mut a, "Aruido<Esc>");
    let id = a.notes[0].key.db_id().unwrap();
    a.db.update_content(id, "nova versao\n", crate::db::now_secs() + 5);
    keys(&mut a, ":e!<CR>");
    assert_eq!(lines(&a), ["nova versao"]);
    assert!(!a.tab().unwrap().modified);
    keys(&mut a, "A!<Esc>:w<CR>");
    assert!(db_text(&a, "Nota").starts_with("nova versao!"));
}

#[test]
fn failed_autosave_backs_off_instead_of_retrying_every_tick() {
    let mut a = app_with(&[("Nota", "x\n")]);
    open(&mut a, "Nota");
    keys(&mut a, "iabc<Esc>");
    let id = a.notes[0].key.db_id().unwrap();
    a.db.update_content(id, "outra\n", crate::db::now_secs() + 3);
    a.tab_mut().unwrap().first_unsaved = Some(std::time::Instant::now() - std::time::Duration::from_secs(120));
    a.check_autosave();
    assert!(a.tab().unwrap().modified, "conflito: nao grava");
    let t = a.tab().unwrap().first_unsaved.unwrap();
    assert!(t.elapsed() < std::time::Duration::from_secs(5), "o relogio do autosave reinicia");
    a.status = None;
    a.check_autosave();
    assert!(a.status_text().is_none(), "sem nova tentativa (nem nova mensagem) ate o proximo minuto");
}

#[test]
fn link_button_command_and_key_copy_the_wikilink() {
    let mut a = app_with(&[("Alfa", "# Alfa\n"), ("Beta Nota", "# Beta\n")]);
    // Foco na lista: copia a nota SELECIONADA.
    a.focus = Focus::Left;
    a.selected = a.filtered.iter().position(|&i| a.notes[i].title == "Beta Nota").unwrap();
    keys(&mut a, "y");
    assert_eq!(a.yank, "[[Beta Nota]]");
    assert!(a.status_text().unwrap().contains("[[Beta Nota]]"));
    // Foco no editor: copia a nota da ABA ativa.
    open(&mut a, "Alfa");
    keys(&mut a, ":link<CR>");
    assert_eq!(a.yank, "[[Alfa]]");
    // O texto copiado funciona como link: o cursor sobre ele abre a nota.
    open(&mut a, "Beta Nota");
    keys(&mut a, "Go<Esc>");
    a.paste_normal_n(1, true); // cola o yank interno ([[Alfa]])
    keys(&mut a, "0<CR>");
    assert_eq!(a.tab().unwrap().title, "Alfa", "o link copiado abre a nota");

    // Clique no botao [🔗 Link].
    a.yank.clear();
    draw(&mut a);
    let (s, e) = a.hit.btn_link;
    assert!(s > 0 && e > s, "botao desenhado");
    let rows = a.rows;
    a.last_theme_click = None;
    click(&mut a, s + 2, rows - 1);
    assert_eq!(a.yank, "[[Alfa]]");
}

#[test]
fn link_for_standalone_file_uses_markdown_link_with_path() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("minha nota.md");
    std::fs::write(&p, "oi\n").unwrap();
    let mut a = new_app();
    a.open_path_arg(p.clone());
    keys(&mut a, ":copylink<CR>");
    assert_eq!(a.yank, format!("[minha nota](<{}>)", p.display()));
    let spans = crate::md::find_links(&a.yank);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].target, p.display().to_string(), "o alvo e o caminho completo");
}

#[test]
fn link_without_any_note_is_harmless() {
    let mut a = new_app();
    keys(&mut a, ":link<CR>");
    assert!(a.yank.is_empty());
    assert!(a.status_text().unwrap().contains("Nenhuma nota"));
}

#[test]
fn all_buttons_fit_the_window_when_possible() {
    let mut a = app_with(&[("A", "x\n")]);
    for cols in [70, 80, 100, 120, 160, 200] {
        a.cols = cols;
        a.rows = 24;
        draw(&mut a);
        let h = &a.hit;
        let last = h.btn_export.1;
        assert!(last <= cols + 1, "{cols} colunas: botao [Exportar] termina em {last}");
        assert!(h.btn_link.0 >= 1);
    }
    a.cols = 200;
    let v = visible(&draw(&mut a));
    assert!(v.contains("[🔗 Link]") && v.contains("[⬆ Exportar]"), "rotulos completos em janela larga");
}

#[test]
fn button_labels_adapt_in_three_levels() {
    let mut a = app_with(&[("A", "x\n")]);
    a.rows = 24;
    a.cols = 200;
    let wide = visible(&draw(&mut a));
    assert!(wide.contains("[🔗 Link]") && wide.contains("[⬆ Exportar]") && wide.contains("Tokyo Night"));
    a.cols = 120;
    let mid = visible(&draw(&mut a));
    assert!(mid.contains("[🔗 Link]") && mid.contains("[⬆ Exp.]") && !mid.contains("Exportar"), "{mid}");
    a.cols = 80;
    let small = visible(&draw(&mut a));
    assert!(small.contains("[🔗]") && !small.contains("Link") && small.contains("[W:ON]"));
}

#[test]
fn page_keys_move_by_screens() {
    let body: String = (0..100).map(|i| format!("linha {i}\n")).collect();
    let mut a = app_with(&[("P", &body)]);
    open(&mut a, "P");
    keys_raw(&mut a, Key::PageDown);
    assert_eq!(a.buf().unwrap().cy, a.visible_rows() - 1);
    keys_raw(&mut a, Key::PageUp);
    assert_eq!(a.buf().unwrap().cy, 0);
    a.mode = Mode::Insert;
    keys_raw(&mut a, Key::PageDown);
    assert!(a.buf().unwrap().cy > 0);
}

fn keys_raw(a: &mut App, k: Key) {
    a.on_key(k);
}

#[test]
fn map_key_handles_modifiers() {
    let e = |c, m| map_key(KeyEvent::new(c, m));
    assert_eq!(e(KeyCode::Char('f'), KeyModifiers::CONTROL), Some(Key::Ctrl('f')));
}

#[test]
fn file_argument_creates_missing_files_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("novo.md");
    let mut a = new_app();
    a.open_path_arg(p.clone());
    keys(&mut a, "ioi nota<Esc>:w<CR>");
    assert_eq!(std::fs::read_to_string(&p).unwrap().trim(), "oi nota");
    assert!(matches!(a.tab().unwrap().key, NoteKey::File(_)));
}

#[test]
fn task_list_renders_boxes_outside_insert_mode() {
    let mut a = app_with(&[("Tarefas", "# Hoje\n\n- [ ] lavar o carro\n- [x] pagar conta\n- [X] ligar\n* [ ] outro marcador\n- [x](http://x.com) link\n- [ ]colado\n")]);
    open(&mut a, "Tarefas");
    let v = visible(&draw(&mut a));
    assert!(v.contains("\u{25A1} lavar o carro"), "tarefa aberta vira caixa vazia");
    assert!(v.contains("\u{25A3} pagar conta") && v.contains("\u{25A3} ligar"), "concluida vira caixa marcada");
    assert!(v.contains("\u{25A1} outro marcador"));
    assert!(!v.contains("[ ] lavar") && !v.contains("[x] pagar"), "colchetes ocultos no modo normal");
    assert!(v.contains("- [ ]colado"), "sem espaco depois de ] nao e tarefa");
    assert!(!v.contains("\u{25A3} link") && !v.contains("[x] link"), "link nao e tarefa");

    // Insercao: texto cru.
    keys(&mut a, "jji");
    assert_eq!(a.mode, Mode::Insert);
    let v = visible(&draw(&mut a));
    assert!(v.contains("- [ ] lavar o carro") && v.contains("- [x] pagar conta"), "insercao mostra o texto cru");
    assert!(!v.contains('\u{25A1}') && !v.contains('\u{25A3}'));
    keys(&mut a, "<Esc>");
    let v = visible(&draw(&mut a));
    assert!(v.contains("\u{25A1} lavar o carro"), "volta a renderizar ao sair da insercao");
    // O documento nunca e alterado pela renderizacao.
    assert_eq!(lines(&a)[2], "- [ ] lavar o carro");
}

#[test]
fn task_list_cursor_and_clicks_stay_aligned() {
    let mut a = app_with(&[("T", "- [x] feito\n")]);
    open(&mut a, "T");
    // `$` vai ao ultimo caractere: nao pode quebrar nem sair do quadro.
    keys(&mut a, "$");
    assert_eq!(a.buf().unwrap().cx, "- [x] feito".len() - 1);
    keys(&mut a, "0lll");
    // Texto editado dentro da caixa continua funcionando: x troca [x] por [ ].
    keys(&mut a, "0lllrr<Esc>");
    let v = visible(&draw(&mut a));
    assert!(!v.contains('\u{25A3}'), "[r] nao e mais uma tarefa valida");
}
/// Posicao (linha, coluna) em que o quadro deixa o cursor do terminal.
fn cursor_at(frame: &str) -> (usize, usize) {
    let end = frame.rfind("\x1b[?25h").expect("cursor visivel");
    let head = &frame[..end];
    let h = head.rfind('H').expect("posicionamento");
    let start = head[..h].rfind("\x1b[").expect("posicionamento");
    let body = &head[start + 2..h];
    let (r, c) = body.split_once(';').expect("linha;coluna");
    (r.parse().unwrap(), c.parse().unwrap())
}

const TABLE_NOTE: &str = "# Tabelas\n\n|a|b|c|\n|1|2|3|\n\ndepois\n";

#[test]
#[ignore]
fn dump_tables() {
    let long = "Esta e uma descricao bem longa que precisa quebrar em varias linhas dentro da celula para caber na janela.";
    let note = format!(
        "# Tabelas\n\n|a|b|c|\n|1|2|3|\n\n| Item | Qtd | Descricao |\n|:--|--:|---|\n| Cafe | 10 | {long} |\n| **Pao** | 2 | [site](http://x.com) e [[Nota]] |\n| Leite | 1.250 | curto |\n\nfim\n"
    );
    for cols in [100, 60, 40] {
        let mut a = app_with(&[("T", &note)]);
        a.cols = cols;
        open(&mut a, "T");
        keys(&mut a, "jjj");
        let v = visible(&draw(&mut a));
        println!("===== {cols} colunas =====\n{v}");
    }
}

#[test]
fn tables_are_drawn_as_grids_outside_insert_mode() {
    let mut a = app_with(&[("T", TABLE_NOTE)]);
    open(&mut a, "T");
    let v = visible(&draw(&mut a));
    assert!(v.contains("╭───┬───┬───╮"), "borda superior:\n{v}");
    assert!(v.contains("│ a │ b │ c │") && v.contains("│ 1 │ 2 │ 3 │"), "celulas:\n{v}");
    assert!(v.contains("╰───┴───┴───╯"), "borda inferior");
    assert!(!v.contains("|a|b|c|"), "texto cru escondido");
    // Insercao: texto cru, sem grade.
    keys(&mut a, "jji");
    let v = visible(&draw(&mut a));
    assert!(v.contains("|a|b|c|") && !v.contains('╭'), "insercao mostra o texto:\n{v}");
    keys(&mut a, "<Esc>");
    assert!(visible(&draw(&mut a)).contains("╭───┬───┬───╮"));
}

#[test]
fn wide_tables_wrap_cells_to_fit_the_window() {
    let long = "palavra ".repeat(20);
    let note = format!("| Nome | Texto |\n|---|---|\n| x | {} |\n| y | curto |\n", long.trim());
    for cols in [120, 70, 50] {
        let mut a = app_with(&[("T", &note)]);
        a.cols = cols;
        a.left_width = crate::app::MIN_LEFT_WIDTH;
        open(&mut a, "T");
        let frame = draw(&mut a);
        let v = visible(&frame);
        let mut words = 0;
        for line in v.lines() {
            // Parte do editor: depois do divisor do painel lateral.
            let Some((_, editor)) = line.split_once('│') else { continue };
            if !editor.contains('│') && !editor.contains('╭') {
                continue;
            }
            let w = crate::text::str_width(editor.trim_end());
            assert!(w <= cols as usize - 19, "grade larga demais ({cols}): {editor}");
            words += editor.matches("palavra").count();
        }
        assert_eq!(words, 20, "nenhuma palavra se perde ({cols} colunas)");
    }
}

#[test]
fn cursor_moves_through_table_cells_and_clicks_land_on_text() {
    let note = "| Nome | Descricao |\n|---|---|\n| Ana | texto bem longo que quebra em varias linhas aqui |\n| Beto | b |\n";
    let mut a = app_with(&[("T", note)]);
    a.cols = 50;
    a.left_width = crate::app::MIN_LEFT_WIDTH;
    open(&mut a, "T");
    keys(&mut a, "jj");
    let mut last = (0, 0);
    // Descer pelas linhas da tela: o cursor nunca sai da area da tabela nem recua.
    for _ in 0..12 {
        let (r, c) = cursor_at(&draw(&mut a));
        assert!(r >= last.0, "linha da tela nao pode recuar: {last:?} -> {r}");
        assert!(c >= 8, "dentro da grade (apos o numero da linha): {c}");
        last = (r, c);
        keys(&mut a, "j");
    }
    assert_eq!(a.buf().unwrap().cy, 3, "chegou a ultima linha");
    // Clicar numa celula coloca o cursor no texto dela.
    let frame = draw(&mut a);
    let v = visible(&frame);
    let (row, col) = v
        .lines()
        .enumerate()
        .find_map(|(i, l)| l.find("Beto").map(|b| (i, crate::text::str_width(&l[..b]))))
        .expect("Beto na tela");
    click(&mut a, col as i32 + 1, row as i32 + 1);
    let b = a.buf().unwrap();
    assert_eq!((b.cy, &b.cur().text[b.cx..b.cx + 1]), (3, "B"));
    click(&mut a, col as i32 + 1, row as i32 + 1);
    assert_eq!(a.mode, Mode::VisualChar);
    assert_eq!(a.buf().unwrap().selection_text(false).unwrap(), "Beto");
    click(&mut a, col as i32 + 1, row as i32 + 1);
    assert_eq!(a.mode, Mode::VisualLine);
    assert_eq!(a.buf().unwrap().selection_text(true).unwrap(), "| Beto | b |\n");
}

#[test]
fn table_selection_and_line_highlight_do_not_break_the_grid() {
    let mut a = app_with(&[("T", TABLE_NOTE)]);
    open(&mut a, "T");
    keys(&mut a, "jjV");
    let v = visible(&draw(&mut a));
    assert!(v.contains("│ a │ b │ c │"));
    keys(&mut a, "<Esc>jvl");
    assert!(visible(&draw(&mut a)).contains("│ 1 │ 2 │ 3 │"));
}
#[test]
fn tables_align_automatically_when_leaving_the_line_or_insert_mode() {
    let aligned = ["| a   | b   | c   |", "| 1   | 2   | 3   |"];
    // Esc do modo Insercao.
    let mut a = app_with(&[("T", "|a|b|c|\n|1|2|3|\n")]);
    open(&mut a, "T");
    keys(&mut a, "jlll");
    keys(&mut a, "i<Esc>");
    assert_eq!(lines(&a), aligned);
    assert!(a.tab().unwrap().modified);
    let b = a.buf().unwrap();
    assert_eq!(b.cy, 1);
    assert!(b.cx > 0 && b.cx < b.cur().len(), "cursor continua dentro da linha");

    // Mudar de linha com as setas, no Insert, depois de alargar uma celula.
    let mut a = app_with(&[("T", "|a|b|c|\n|1|2|3|\n")]);
    open(&mut a, "T");
    keys(&mut a, "jli");
    keys(&mut a, "mais largo<Up>");
    assert_eq!(a.mode, Mode::Insert);
    let l = lines(&a);
    assert_eq!(l[0].len(), l[1].len(), "linhas do mesmo tamanho: {l:?}");
    assert!(l[1].starts_with("| 1mais largo"), "{l:?}");
    // O cursor permanece na linha de destino.
    assert_eq!(a.buf().unwrap().cy, 0);

    // Tabela ja alinhada: nada muda e a nota nao fica "modificada" a toa.
    let mut a = app_with(&[("T", "| a   | b   |\n| c   | d   |\n")]);
    open(&mut a, "T");
    keys(&mut a, "ji<Esc>");
    assert!(!a.tab().unwrap().modified, "sem mudanca, sem modificacao");
    assert_eq!(lines(&a), ["| a   | b   |", "| c   | d   |"]);

    // Fora de tabelas nada acontece.
    let mut a = app_with(&[("T", "texto\n|a|b|\n")]);
    open(&mut a, "T");
    keys(&mut a, "i<Esc>");
    assert_eq!(lines(&a), ["texto", "|a|b|"], "linha de tabela isolada nao e tabela");
}

#[test]
fn enter_in_a_table_inserts_an_aligned_row_below() {
    let mut a = app_with(&[("T", "|a|b|c|\n|1|2|3|\n")]);
    open(&mut a, "T");
    keys(&mut a, "ji<CR>");
    let l = lines(&a);
    assert_eq!(l.len(), 3, "{l:?}");
    assert_eq!(l[0], "| a   | b   | c   |");
    assert_eq!(l[1], "| 1   | 2   | 3   |");
    assert_eq!(l[2], "|     |     |     |", "linha nova ja alinhada");
    let b = a.buf().unwrap();
    assert_eq!((b.cy, b.cx), (2, 2), "cursor na primeira celula da linha nova");
    assert_eq!(a.mode, Mode::Insert);
    // Digitar a primeira celula e Enter de novo mantem tudo alinhado.
    keys(&mut a, "novo<CR>");
    let l = lines(&a);
    assert_eq!(l.len(), 4);
    assert!(l.iter().all(|x| x.len() == l[0].len()), "{l:?}");

    // Tabela de uma unica linha tambem ganha a linha alinhada.
    let mut a = app_with(&[("T", "|x|yy|\n")]);
    open(&mut a, "T");
    keys(&mut a, "i<CR>");
    assert_eq!(lines(&a), ["| x   | yy  |", "|     |     |"]);
}


// ---- prefixo numerico (como no Vim) ------------------------------------------------

fn digits_note() -> String {
    format!("{}\nsegunda linha\nterceira\nquarta\nquinta\nsexta\n", "0123456789".repeat(20))
}

fn pos(a: &App) -> (usize, usize) {
    let b = a.buf().unwrap();
    (b.cy, b.cx)
}

#[test]
fn count_repeats_horizontal_and_vertical_motions() {
    let mut a = app_with(&[("N", &digits_note())]);
    open(&mut a, "N");
    keys(&mut a, "100l");
    assert_eq!(pos(&a), (0, 100), "100l desloca 100 casas");
    keys(&mut a, "0");
    assert_eq!(pos(&a), (0, 0), "0 sozinho continua indo ao inicio");
    keys(&mut a, "10l");
    assert_eq!(pos(&a), (0, 10), "o 0 de 10 faz parte do numero");
    keys(&mut a, "3h");
    assert_eq!(pos(&a), (0, 7));

    // `j`/`k` andam por linhas da tela; aqui cada linha cabe em uma.
    let mut a = app_with(&[("V", "a\nb\nc\nd\ne\nf\n")]);
    open(&mut a, "V");
    keys(&mut a, "3j");
    assert_eq!(pos(&a).0, 3);
    keys(&mut a, "2k");
    assert_eq!(pos(&a).0, 1);
    keys(&mut a, "999j");
    assert_eq!(pos(&a).0, 5, "para na ultima linha");
    keys(&mut a, "999k");
    assert_eq!(pos(&a).0, 0, "e na primeira");
}

#[test]
fn count_stops_at_line_edges_and_never_changes_the_pane() {
    let mut a = app_with(&[("N", &digits_note())]);
    open(&mut a, "N");
    keys(&mut a, "5l");
    keys(&mut a, "100h");
    assert_eq!(pos(&a), (0, 0));
    assert_eq!(a.focus, Focus::Right, "100h nao passa o foco para a lista de notas");
    keys(&mut a, "99999999999l");
    assert_eq!(pos(&a).0, 0, "numero enorme: para no fim da linha, sem travar");
    assert!(pos(&a).1 >= 199);
    keys(&mut a, "h");
    keys(&mut a, "0h");
    assert_eq!(a.focus, Focus::Left, "h simples na coluna 0 continua levando a lista");
}

#[test]
fn count_with_words_lines_and_goto() {
    let mut a = app_with(&[("N", "um dois tres quatro cinco\nsegunda\nterceira\nquarta\nquinta\n")]);
    open(&mut a, "N");
    keys(&mut a, "3w");
    assert_eq!(&a.buf().unwrap().cur().text[pos(&a).1..], "quatro cinco");
    keys(&mut a, "2b");
    assert_eq!(&a.buf().unwrap().cur().text[pos(&a).1..], "dois tres quatro cinco");
    keys(&mut a, "4G");
    assert_eq!(pos(&a), (3, 0), "4G vai a linha 4");
    keys(&mut a, "2gg");
    assert_eq!(pos(&a), (1, 0), "2gg vai a linha 2");
    keys(&mut a, "1000G");
    assert_eq!(pos(&a).0, 4, "alem do fim: ultima linha");
    keys(&mut a, "G");
    assert_eq!(pos(&a).0, 4);
    keys(&mut a, "gg");
    assert_eq!(pos(&a), (0, 0));
    keys(&mut a, "2$");
    assert_eq!(pos(&a).0, 1, "2$ desce uma linha e vai ao fim");
}

#[test]
fn count_with_delete_yank_and_cut() {
    let mut a = app_with(&[("N", "abcdefgh\nl2\nl3\nl4\nl5\n")]);
    open(&mut a, "N");
    keys(&mut a, "3x");
    assert_eq!(lines(&a)[0], "defgh");
    assert_eq!(a.yank, "abc");
    keys(&mut a, "u");
    assert_eq!(lines(&a)[0], "abcdefgh", "3x e um unico passo de desfazer");

    keys(&mut a, "2dd");
    assert_eq!(lines(&a), ["l3", "l4", "l5"]);
    assert_eq!(a.yank, "abcdefgh\nl2");
    keys(&mut a, "u");
    assert_eq!(lines(&a).len(), 5, "2dd e um unico passo de desfazer");
    keys(&mut a, "d2d");
    assert_eq!(lines(&a), ["l3", "l4", "l5"], "d2d == 2dd");
    keys(&mut a, "u");
    keys(&mut a, "2d2d");
    assert_eq!(lines(&a), ["l5"], "2d2d apaga 4 linhas (os numeros se multiplicam)");
    keys(&mut a, "u");
    keys(&mut a, "gg9dd");
    assert_eq!(lines(&a), [""], "alem do fim: apaga ate o fim, sem restar nada invalido");

    let mut a = app_with(&[("N", "a\nb\nc\nd\n")]);
    open(&mut a, "N");
    keys(&mut a, "3y");
    assert_eq!(a.yank, "a\nb\nc");
    assert!(a.yank_linewise);
    assert_eq!(lines(&a).len(), 4, "y nao altera a nota");
}

#[test]
fn count_with_undo_redo_replace_and_paste() {
    let mut a = app_with(&[("N", "abcdef\n")]);
    open(&mut a, "N");
    for c in ["x", "x", "x"] {
        keys(&mut a, c);
    }
    assert_eq!(lines(&a)[0], "def");
    keys(&mut a, "2u");
    assert_eq!(lines(&a)[0], "bcdef", "2u desfaz dois passos");
    keys(&mut a, "9u");
    assert_eq!(lines(&a)[0], "abcdef", "para quando o historico acaba");
    keys(&mut a, "2<C-r>");
    assert_eq!(lines(&a)[0], "cdef");

    keys(&mut a, "u");
    keys(&mut a, "u");
    keys(&mut a, "3rz");
    assert_eq!(lines(&a)[0], "zzzdef");
    assert_eq!(pos(&a), (0, 2), "cursor no ultimo caractere trocado");
    keys(&mut a, "$");
    keys(&mut a, "5rq");
    assert_eq!(lines(&a)[0], "zzzdef", "linha curta demais: nada acontece");

    keys(&mut a, "0");
    a.set_yank("ab".into(), false);
    keys(&mut a, "3p");
    assert_eq!(lines(&a)[0], "abababzzzdef");
    keys(&mut a, "u");
    assert_eq!(lines(&a)[0], "zzzdef", "3p e um unico passo de desfazer");
}

#[test]
fn count_repeats_what_was_typed_on_leaving_insert() {
    let mut a = app_with(&[("N", "fim\n")]);
    open(&mut a, "N");
    keys(&mut a, "3ia<Esc>");
    assert_eq!(lines(&a)[0], "aaafim");
    assert_eq!(a.mode, Mode::Normal);
    keys(&mut a, "u");
    assert_eq!(lines(&a)[0], "fim", "o texto repetido volta junto com a digitacao");

    keys(&mut a, "2Axy<Esc>");
    assert_eq!(lines(&a)[0], "fimxyxy");

    keys(&mut a, "3ol<Esc>");
    assert_eq!(lines(&a), ["fimxyxy", "l", "l", "l"]);
    keys(&mut a, "2Ob<Esc>");
    assert_eq!(lines(&a).iter().filter(|l| *l == "b").count(), 2);

    // Digitou mais de uma linha: nao repete (edicao complexa).
    let mut a = app_with(&[("N", "z\n")]);
    open(&mut a, "N");
    keys(&mut a, "3ia<CR>b<Esc>");
    assert_eq!(lines(&a), ["a", "bz"]);
    // Prefixo 1 e sem prefixo se comportam igual.
    keys(&mut a, "1ic<Esc>");
    assert_eq!(a.buf().unwrap().cur().text.matches('c').count(), 1);
}

#[test]
fn count_is_cancelled_and_works_in_visual_mode_and_tabs() {
    let mut a = app_with(&[("N", &digits_note()), ("M", "um\n"), ("P", "dois\n")]);
    open(&mut a, "N");
    keys(&mut a, "5<Esc>l");
    assert_eq!(pos(&a), (0, 1), "Esc cancela o prefixo");
    keys(&mut a, "0v3l");
    assert_eq!(a.mode, Mode::VisualChar);
    assert_eq!(pos(&a), (0, 3), "contagem tambem vale no Visual");
    keys(&mut a, "<Esc>");
    // `:` e outros comandos descartam o prefixo em vez de deixa-lo para depois.
    keys(&mut a, "7:<Esc>l");
    assert_eq!(pos(&a).1, 4, "o prefixo nao sobrevive a um comando que nao o usa (l anda so 1)");

    open(&mut a, "M");
    open(&mut a, "P");
    keys(&mut a, "1gt");
    assert_eq!(a.active, Some(0), "1gt vai para a primeira aba");
    keys(&mut a, "2gt");
    assert_eq!(a.active, Some(1));
    keys(&mut a, "9gt");
    assert_eq!(a.active, Some(1), "aba inexistente: continua onde estava");
}
