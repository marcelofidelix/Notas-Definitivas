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
    a.buf().unwrap().lines.iter().map(|l| l.text.clone()).collect()
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
    let body = "# Titulo\n\nTexto com **negrito** e *itálico* ação 😀 日本語 \u{9}tab.\n- item\n\n|a|b|\n|-|-|\n|ççç|d|\n\n```\ncodigo # c\n```\n## Sub\n[[Outra]]\nlinha comprida ".to_string()
        + &"palavra ".repeat(40)
        + "\n";
    let mut a = app_with(&[("Fuzz", &body), ("Outra", "# Outra\n\ncorpo\n"), ("Terceira", "x\n")]);
    open(&mut a, "Fuzz");
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
        ":tabnext<CR>", ":rescan<CR>", ":/texto<CR>", ":e Fuzz2<CR>", ":nowrap<CR>", ":set wrap<CR>", ":wrapbar<CR>",
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
                // Cliques nos botoes [Importar]/[Exportar]/[Life] abririam prompts ou
                // bloqueariam o teste; o resto do mouse e exercitado normalmente.
                let on_btn_row = row == a.rows - 1;
                if !on_btn_row {
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
