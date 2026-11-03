//! Desenho da tela: cada quadro e montado em uma unica `String` ANSI TrueColor e
//! gravado de uma vez, envolvido em atualizacao sincronizada (DEC 2026).
//!
//! Todo texto vindo de notas, nomes e prompts passa por `txt()`, que remove
//! bytes de controle: um `.md` com ESC embutido nao injeta sequencias no terminal.

use crate::app::{App, Focus, HitMap, Mode, MIN_LEFT_WIDTH};
use crate::db::now_secs;
use crate::doc::Line;
use crate::layout::{VRowIter, VisRow};
use crate::md::{bullet, find_links, find_styles, LinkSpan, StyleKind, StyleSpan};
use crate::text::{
    cp_width, format_rel_time, marquee_slice, sanitize_for_terminal, str_width, truncate_display, width_len,
};
use crate::theme::Theme;

pub const BOLD: &str = "\x1b[1m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const RESET: &str = "\x1b[0m";
/// Cursor sempre FIXO (DECSCUSR 2 = bloco, 6 = barra; ?12l desliga o piscar).
pub const CURSOR_BLOCK: &str = "\x1b[?12l\x1b[2 q";
pub const CURSOR_BAR: &str = "\x1b[?12l\x1b[6 q";
pub const SYNC_BEGIN: &str = "\x1b[?2026h";
pub const SYNC_END: &str = "\x1b[?2026l";

const PROMPT_IMPORT: &str = " ⬇ Importar de: ";
const PROMPT_EXPORT: &str = " ⬆ Exportar para: ";

/// Rotulo do prompt modal ativo (fonte unica para desenho, cursor e mouse).
pub fn prompt_label(app: &App) -> Option<String> {
    Some(match app.mode {
        Mode::PromptNew => " 📝 Nova nota: ".to_string(),
        Mode::PromptRename => " ✏️  Renomear nota: ".to_string(),
        Mode::PromptFind => " 🔍 Localizar na nota: ".to_string(),
        Mode::PromptReplaceFind => " 🔄 Localizar para substituir: ".to_string(),
        Mode::PromptReplaceWith => format!(" 🔄 Substituir '{}' por: ", app.replace_find),
        Mode::PromptImport => PROMPT_IMPORT.to_string(),
        Mode::PromptExport => PROMPT_EXPORT.to_string(),
        _ => return None,
    })
}

/// " 2 linhas, 37 caracteres selecionados" enquanto ha selecao ativa.
fn selection_summary(app: &App) -> String {
    let active = app.mode.is_visual() || (app.mode == Mode::Insert && app.buf().is_some_and(|b| b.has_selection));
    if !active || app.in_preview() {
        return String::new();
    }
    let Some(b) = app.buf() else { return String::new() };
    let linewise = app.mode == Mode::VisualLine;
    let Some((sy, _, ey, _)) = b.selection(linewise) else { return String::new() };
    let chars = b.selection_text(linewise).map_or(0, |t| t.chars().filter(|&c| c != '\n').count());
    let lines = ey - sy + 1;
    format!(
        " {} {}, {} {} selecionados ",
        lines,
        if lines == 1 { "linha" } else { "linhas" },
        chars,
        if chars == 1 { "caractere" } else { "caracteres" }
    )
}

fn prompt_color(th: &Theme, mode: Mode) -> &str {
    match mode {
        Mode::PromptNew | Mode::PromptReplaceWith | Mode::PromptImport | Mode::PromptExport => th.fg_accent,
        Mode::PromptRename => th.fg_yellow,
        Mode::PromptFind => th.fg_h1,
        Mode::PromptReplaceFind => th.fg_warn,
        _ => th.fg_white,
    }
}

pub fn attention_color(th: &Theme, level: u8) -> &str {
    let (dark, light) = match level {
        1 => ("\x1b[38;2;255;85;85m", "\x1b[38;2;207;34;46m"),
        2 => ("\x1b[38;2;255;158;100m", "\x1b[38;2;217;119;6m"),
        3 => ("\x1b[38;2;241;250;140m", "\x1b[38;2;165;110;0m"),
        4 => ("\x1b[38;2;122;162;247m", "\x1b[38;2;9;105;218m"),
        5 => ("\x1b[38;2;187;154;247m", "\x1b[38;2;130;80;223m"),
        6 => ("\x1b[38;2;80;250;123m", "\x1b[38;2;26;127;55m"),
        _ => return th.fg_main,
    };
    if th.is_dark {
        dark
    } else {
        light
    }
}

fn level_box(level: u8) -> &'static str {
    match level {
        1 => "[1] ",
        2 => "[2] ",
        3 => "[3] ",
        4 => "[H] ",
        5 => "[R] ",
        6 => "[O] ",
        _ => "[?] ",
    }
}

/// Desenha um quadro completo e atualiza a geometria de cliques do `app`.
pub fn draw(app: &mut App) -> String {
    // Largura do painel esquerdo sempre coerente com a janela.
    if app.cols < 60 && app.left_width > app.cols / 2 {
        app.left_width = app.cols / 2;
    }
    if app.left_width < MIN_LEFT_WIDTH && app.cols >= 50 {
        app.left_width = MIN_LEFT_WIDTH;
    }
    if app.left_width >= app.cols - 15 {
        app.left_width = (app.cols - 15).max(10);
    }
    app.editor_scroll();

    let (rows, cursor, hit, marquee, tick) = {
        let mut f = Frame {
            app: &*app,
            th: app.theme(),
            s: String::with_capacity(4096),
            rows: Vec::with_capacity((app.rows.max(0) as usize) + 1),
            cursor: String::new(),
            hit: HitMap::default(),
            marquee: false,
            tick: 0,
        };
        f.run();
        (f.rows, f.cursor, f.hit, f.marquee, f.tick)
    };

    let clear = app.clear_needed;
    let mut out = String::with_capacity(8 * 1024);
    out.push_str(SYNC_BEGIN);
    if app.diff_render {
        // Redesenho incremental: so as linhas que mudaram desde o quadro anterior
        // (tudo, apos limpeza, redimensionamento ou no batimento de manutencao).
        // Cada linha e autocontida e posicionada, entao pular as iguais e seguro.
        let full = clear || app.force_full || app.prev_rows.len() != rows.len();
        if clear {
            out.push_str("\x1b[2J\x1b[3J");
        }
        out.push_str("\x1b[?25l");
        for (i, row) in rows.iter().enumerate() {
            if full || app.prev_rows.get(i) != Some(row) {
                out.push_str(&format!("\x1b[{};1H\x1b[0m", i + 1));
                out.push_str(row);
            }
        }
    } else {
        // Formato de quadro completo (usado pelos testes).
        out.push_str(if clear { "\x1b[2J\x1b[3J\x1b[H" } else { "\x1b[H" });
        out.push_str("\x1b[?25l");
        out.push_str(&rows.join("\r\n"));
    }
    out.push_str(&cursor);
    out.push_str(SYNC_END);

    app.prev_rows = rows;
    app.force_full = false;
    app.hit = hit;
    app.marquee_active = marquee;
    app.last_marquee_tick = tick;
    app.clear_needed = false;
    app.needs_redraw = false;
    app.last_draw = std::time::Instant::now();
    out
}

struct Frame<'a> {
    app: &'a App,
    th: &'static Theme,
    /// Linha em construcao.
    s: String,
    /// Linhas ja prontas (uma por linha da tela).
    rows: Vec<String>,
    /// Posicionamento e formato do cursor, depois das linhas.
    cursor: String,
    hit: HitMap,
    marquee: bool,
    tick: u128,
}

impl<'a> Frame<'a> {
    fn p(&mut self, t: &str) {
        self.s.push_str(t);
    }
    /// Texto de origem nao confiavel: sem bytes de controle.
    fn txt(&mut self, t: &str) {
        self.s.push_str(&sanitize_for_terminal(t));
    }
    fn pad(&mut self, n: i32) {
        for _ in 0..n.max(0) {
            self.s.push(' ');
        }
    }

    /// Fecha a linha em construcao e a guarda.
    fn end_row(&mut self) {
        let row = self.s.clone();
        self.s.clear();
        self.rows.push(row);
    }

    fn run(&mut self) {
        let app = self.app;
        let editor_rows = (app.rows - 3).max(0);
        let in_preview = app.in_preview();
        let lay = app.layout(in_preview);
        let (lines, scroll): (&[Line], usize) = if let (true, Some(p)) = (in_preview, app.preview.as_ref()) {
            (&p.lines, p.scroll_y)
        } else if let Some(b) = app.buf() {
            (&b.lines, b.scroll_y)
        } else {
            (&[], 0)
        };
        let mut it = VRowIter::locate(lines, lay, scroll);
        for r in 0..editor_rows {
            self.left_panel(r, editor_rows);
            if r == 0 {
                self.tab_bar();
            } else {
                let row = it.as_mut().map(|i| {
                    let c = i.current();
                    (c, i.advance())
                });
                let cur = row.map(|(c, _)| c);
                if row.is_some_and(|(_, more)| !more) {
                    it = None;
                }
                self.right_panel((r - 1) as usize, cur, lines, in_preview);
            }
            self.end_row();
        }
        self.status_bar(); // fecha as suas duas linhas
        self.command_line();
        self.end_row();
        self.place_cursor(editor_rows);
        self.cursor = std::mem::take(&mut self.s);
    }

    // ---- painel esquerdo ---------------------------------------------------

    fn divider(&mut self) {
        let th = self.th;
        self.p(th.fg_border);
        self.p("│");
        self.p(RESET);
    }

    fn left_panel(&mut self, row: i32, editor_rows: i32) {
        let th = self.th;
        let app = self.app;
        let w = app.left_width;
        let mut cols = 0;

        if row == 0 {
            self.p(th.bg_status);
            self.p(BOLD);
            self.p(th.fg_accent);
            self.p(" NotaZ");
            cols += 6;
            if w >= 41 {
                self.p("\x1b[24m");
                self.p(th.bg_status);
                self.p(th.fg_muted);
                self.p(", desenvolvido por ");
                cols += 19;
                self.p(th.fg_white);
                self.p(BOLD);
                self.p("Marcelo Fidelix");
                cols += 15;
            } else if w >= 28 {
                self.p("\x1b[24m");
                self.p(th.bg_status);
                self.p(th.fg_muted);
                self.p(" por ");
                cols += 5;
                self.p(th.fg_white);
                self.p(BOLD);
                self.p("Marcelo Fidelix");
                cols += 15;
            }
            self.p(RESET);
            self.p(th.bg_status);
            self.pad(w - cols);
            self.divider();
            return;
        }

        if row == 1 {
            if app.mode == Mode::SearchLeft {
                self.p(th.bg_select);
                self.p(th.fg_yellow);
                self.p(BOLD);
                self.p(" 🔍 ");
                cols += 4;
                self.p(th.fg_white);
                let avail = (w - cols - 2).max(2);
                let q = app.search_query.as_str();
                let qw = str_width(q) as i32;
                if qw > avail {
                    let t = truncate_display(q, avail as usize);
                    self.txt(&t);
                    cols += str_width(&t) as i32;
                } else {
                    self.txt(q);
                    cols += qw;
                }
                self.p("\x1b[7m \x1b[27m");
                cols += 1;
                self.pad(w - cols);
                self.p(RESET);
                self.p(th.fg_h2);
                self.p("┃");
                self.p(RESET);
            } else {
                self.p(th.bg_sidebar);
                self.p(" ");
                cols += 1;
                self.p(th.bg_cur_line);
                self.p(th.fg_accent);
                self.p(BOLD);
                self.p(" ⌕ ");
                cols += 3;
                self.p(RESET);
                self.p(th.bg_cur_line);
                self.p(th.fg_muted);
                let placeholder = if app.search_query.is_empty() {
                    "Buscar notas..."
                } else {
                    app.search_query.as_str()
                };
                let pw = str_width(placeholder) as i32;
                let shortcut_w = 9;
                let max_text = w - cols - shortcut_w - 2;
                if max_text > 0 {
                    if pw > max_text {
                        let t = truncate_display(placeholder, max_text as usize);
                        self.txt(&t);
                        cols += str_width(&t) as i32;
                    } else {
                        self.txt(placeholder);
                        cols += pw;
                    }
                }
                let inner = w - cols - shortcut_w - 1;
                self.pad(inner);
                cols += inner.max(0);
                self.p(th.fg_yellow);
                self.p(" [Ctrl+F]");
                cols += 9;
                self.p(" ");
                cols += 1;
                self.p(RESET);
                self.p(th.bg_sidebar);
                self.pad(w - cols);
                self.divider();
            }
            return;
        }

        if row == 2 {
            let n_f = app.filtered.len();
            let hdr: String;
            if !app.search_query.is_empty() {
                hdr = format!(" 🔍 {} {} encontrada(s)", n_f, if n_f == 1 { "nota" } else { "notas" });
                self.p(th.bg_select);
                self.p(th.fg_white);
                self.p(BOLD);
            } else if app.filter_urgency > 0 {
                self.p(th.bg_select);
                self.p(attention_color(th, app.filter_urgency));
                self.p(BOLD);
                let lbl = match app.filter_urgency {
                    1 => "🔴 NÍVEL 1",
                    2 => "🟠 NÍVEL 2",
                    3 => "🟡 NÍVEL 3",
                    4 => "🔵 MARCAÇÃO H",
                    5 => "🟣 MARCAÇÃO R",
                    _ => "🟢 MARCAÇÃO O",
                };
                hdr = format!(" {} ({}/{}) ", lbl, n_f, app.notes.len());
            } else if app.focus == Focus::Left {
                self.p(th.bg_select);
                self.p(th.fg_white);
                self.p(BOLD);
                hdr = format!(" 📂 NOTAS ({}) [ATIVO] ", app.notes.len());
            } else {
                self.p(th.bg_sidebar);
                self.p(th.fg_muted);
                self.p(BOLD);
                hdr = format!("  NOTAS ({}) ", app.notes.len());
            }
            let hw = str_width(&hdr) as i32;
            if hw > w {
                let t = truncate_display(&hdr, w as usize);
                self.txt(&t);
                cols += str_width(&t) as i32;
            } else {
                self.txt(&hdr);
                cols += hw;
            }
            self.pad(w - cols);
            self.p(RESET);
            if app.focus == Focus::Left || app.mode == Mode::SearchLeft {
                self.p(th.fg_h2);
                self.p("┃");
                self.p(RESET);
            } else {
                self.divider();
            }
            return;
        }

        if row == editor_rows - 1 {
            self.filter_buttons(w);
            return;
        }

        // Linhas da lista de notas.
        let note_pos = (row - 3) as usize + app.sidebar_scroll;
        if app.filtered.is_empty() && row == 3 {
            self.p(th.bg_sidebar);
            self.p(th.fg_muted);
            self.p("  (Nenhum resultado)");
            cols += 20;
            self.pad(w - cols);
            self.divider();
            return;
        }
        if let Some(&idx) = app.filtered.get(note_pos) {
            let n = &app.notes[idx];
            let selected = note_pos == app.selected;
            let open = app.cur_key() == Some(&n.key);
            let bg = if selected { th.bg_select } else { th.bg_sidebar };
            self.p(bg);
            if open {
                self.p(th.fg_h1);
                self.p("●");
            } else if selected {
                self.p(th.fg_yellow);
                self.p("▸");
            } else {
                self.p(" ");
            }
            cols += 1;
            if n.attention > 0 {
                self.p(attention_color(th, n.attention));
                self.p(BOLD);
                self.p(level_box(n.attention));
            } else {
                self.p(th.fg_muted);
                self.p("[ ] ");
            }
            self.p(RESET);
            self.p(bg);
            cols += 4;

            let rel = format_rel_time(n.mtime, now_secs());
            let date_w = str_width(&rel) as i32;
            let avail_title = (w - cols - 1 - date_w).max(3);
            let title_w = str_width(&n.title) as i32;
            if n.attention > 0 {
                self.p(attention_color(th, n.attention));
                self.p(BOLD);
            } else if selected {
                self.p(th.fg_white);
                self.p(BOLD);
            } else {
                self.p(th.fg_main);
            }
            if title_w > avail_title {
                let t = truncate_display(&n.title, (avail_title - 1) as usize);
                self.txt(&t);
                self.p("~");
                cols += str_width(&t) as i32 + 1;
            } else {
                self.txt(&n.title);
                cols += title_w;
            }
            let target_date_col = w - date_w - 1;
            self.pad(target_date_col - cols);
            cols = cols.max(target_date_col);
            self.p(" ");
            cols += 1;
            self.p(th.fg_date);
            self.p(&rel);
            cols += date_w;
            self.pad(w - cols);
        } else {
            self.p(th.bg_sidebar);
            self.pad(w);
        }
        self.divider();
    }

    fn filter_buttons(&mut self, w: i32) {
        let th = self.th;
        let urgency = self.app.filter_urgency;
        self.p(th.bg_sidebar);
        let mut cur_col = 1;
        let mut drawn = 0;
        let spacing = i32::from(w >= 28);
        if w >= 28 {
            self.p(" ");
            drawn += 1;
            cur_col += 1;
        }
        let (lbl_all, w_all) = if w >= 32 { ("[Todas]", 7) } else { ("[T]", 3) };
        let start = cur_col;
        if urgency == 0 {
            self.p(th.bg_select);
            self.p(th.fg_white);
            self.p(BOLD);
        } else {
            self.p(th.fg_muted);
        }
        self.p(lbl_all);
        self.p(RESET);
        self.p(th.bg_sidebar);
        drawn += w_all;
        cur_col += w_all;
        self.hit.filter[0] = (start, cur_col);

        for (level, label) in [(1u8, "[1]"), (2, "[2]"), (3, "[3]"), (4, "[H]"), (5, "[R]"), (6, "[O]")] {
            if spacing == 1 {
                self.p(" ");
                drawn += 1;
                cur_col += 1;
            }
            let start = cur_col;
            self.p(attention_color(th, level));
            self.p(BOLD);
            if urgency == level {
                self.p(th.bg_select);
            }
            self.p(label);
            self.p(RESET);
            self.p(th.bg_sidebar);
            drawn += 3;
            cur_col += 3;
            self.hit.filter[usize::from(level)] = (start, cur_col);
        }
        self.pad(w - drawn);
        self.divider();
    }

    // ---- barra de abas -----------------------------------------------------

    fn tab_bar(&mut self) {
        let th = self.th;
        let app = self.app;
        let bar_width = app.cols - app.left_width - 1;
        if bar_width <= 0 {
            return;
        }
        if app.tabs.is_empty() {
            let btn = " [➕ Nova Nota] ";
            let bw = str_width(btn) as i32;
            let start = app.left_width + 2;
            self.hit.tab_new = (start, start + bw - 1);
            self.p(th.bg_tab_active);
            self.p(th.fg_accent);
            self.p(BOLD);
            self.p(btn);
            self.p(RESET);
            self.p(th.bg_tab_inactive);
            self.p(th.fg_border);
            self.p("│");
            self.p(RESET);
            self.p(th.bg_tab_inactive);
            self.pad(bar_width - 1 - bw - 1);
            self.p("\x1b[K");
            self.p(RESET);
            return;
        }
        let n = app.tabs.len() as i32;
        let (max_w, min_w) = (24, 14);
        let tab_w = ((bar_width - 1) / n).clamp(min_w, max_w);
        let now_ms = app.start.elapsed().as_millis();
        let mut drawn = 0;
        let mut screen_col = app.left_width + 2;
        let max_visible = ((bar_width - 1) / min_w).max(1) as usize;
        let active = app.active.unwrap_or(0);
        let start_tab = if active >= max_visible { active + 1 - max_visible } else { 0 };

        for i in start_tab..app.tabs.len() {
            if drawn + min_w > bar_width - 1 {
                break;
            }
            let t = &app.tabs[i];
            let is_active = i == active;
            let mut this_w = tab_w;
            if drawn + this_w > bar_width - 1 {
                this_w = bar_width - 1 - drawn;
            }
            if this_w < min_w {
                break;
            }
            self.hit
                .tabs
                .push((i, screen_col, screen_col + this_w - 1, screen_col + this_w - 2));
            self.p(if is_active { th.bg_tab_active } else { th.bg_tab_inactive });
            if is_active {
                self.p(if t.modified { th.fg_warn } else { th.fg_h1 });
                self.p("● ");
            } else if t.modified {
                self.p(th.fg_warn);
                self.p("+ ");
            } else {
                self.p(th.fg_muted);
                self.p("  ");
            }
            let mut tab_drawn = 2;
            let avail_title = (this_w - 5).max(3);
            let title_w = str_width(&t.title) as i32;
            if is_active {
                self.p(th.fg_white);
                self.p(BOLD);
            } else {
                self.p(th.fg_muted);
            }
            if title_w <= avail_title {
                self.txt(&t.title);
                tab_drawn += title_w;
            } else {
                // Letreiro: titulo longo rola a cada 300 ms.
                let cycle = (title_w + 4) as u128;
                let step = ((now_ms / 300) % cycle) as usize;
                let slice = marquee_slice(&t.title, step, avail_title as usize);
                self.marquee = true;
                self.tick = now_ms / 300;
                self.txt(&slice);
                tab_drawn += str_width(&slice) as i32;
            }
            self.pad(this_w - 3 - tab_drawn);
            tab_drawn = tab_drawn.max(this_w - 3);
            self.p(" ");
            self.p(if is_active { th.fg_warn } else { th.fg_muted });
            self.p("×");
            tab_drawn += 2;
            self.p(th.fg_border);
            self.p("│");
            self.p(RESET);
            tab_drawn += 1;
            drawn += tab_drawn;
            screen_col += tab_drawn;
        }
        self.p(th.bg_tab_inactive);
        self.pad(bar_width - 1 - drawn);
        self.p("\x1b[K");
        self.p(RESET);
    }

    // ---- painel direito ----------------------------------------------------

    fn empty_row(&mut self, ew: i32, screen_row: usize) {
        let th = self.th;
        self.p(th.bg_main);
        match screen_row {
            2 => {
                self.p(th.fg_muted);
                let m = "  Nenhuma nota aberta no momento.";
                self.p(m);
                self.pad(ew - 1 - str_width(m) as i32);
            }
            4 => {
                self.p(th.fg_accent);
                self.p(BOLD);
                let m = "  👉 Clique na aba [➕ Nova Nota] acima para criar uma nota.";
                self.p(m);
                self.p(RESET);
                self.p(th.bg_main);
                self.pad(ew - 1 - str_width(m) as i32);
            }
            6 => {
                self.p(th.fg_muted);
                let m = "  Ou selecione uma nota no painel esquerdo e pressione Enter.";
                self.p(m);
                self.pad(ew - 1 - str_width(m) as i32);
            }
            _ => {
                self.p(th.fg_muted);
                self.p("~     ");
                self.pad(ew - 1 - 6);
            }
        }
        self.p("\x1b[K");
        self.p(RESET);
    }

    fn right_panel(&mut self, screen_row: usize, cur: Option<VisRow>, lines: &[Line], in_preview: bool) {
        let th = self.th;
        let app = self.app;
        let ew = app.editor_width();
        if ew <= 0 {
            return;
        }
        if !in_preview && app.tabs.is_empty() {
            self.empty_row(ew, screen_row);
            return;
        }
        self.p(th.bg_main);
        let Some(vr) = cur else {
            self.p(th.fg_muted);
            self.p("~     ");
            self.pad(ew - 1 - 6);
            self.p("\x1b[K");
            self.p(RESET);
            return;
        };
        let l = &lines[vr.line];
        let buf = app.buf();
        let is_cur_line = !in_preview && buf.is_some_and(|b| vr.line == b.cy);
        let cur_bg = is_cur_line && matches!(app.mode, Mode::Normal | Mode::Command);
        let line_bg = if cur_bg { th.bg_cur_line } else { th.bg_main };
        if cur_bg {
            self.p(th.bg_cur_line);
        }

        // Numero da linha e indicador de dobra.
        let lnum = if vr.sub == 0 {
            if l.heading > 0 {
                format!("{:>4} {}", vr.line + 1, if l.folded { "▸" } else { "▾" })
            } else {
                format!("{:>4}  ", vr.line + 1)
            }
        } else {
            "      ".to_string()
        };
        if is_cur_line {
            self.p(th.fg_h1);
            self.p(BOLD);
        } else {
            self.p(th.fg_muted);
        }
        self.p(&lnum);
        self.p(RESET);
        self.p(line_bg);

        let sel = if !in_preview && (app.mode.is_visual() || (app.mode == Mode::Insert && buf.is_some_and(|b| b.has_selection))) {
            buf.and_then(|b| b.selection(app.mode == Mode::VisualLine))
        } else {
            None
        };
        let linewise = app.mode == Mode::VisualLine;
        // Trecho selecionado nesta linha: (inicio, fim, inclui a quebra de linha).
        let line_sel: Option<(usize, usize, bool)> = sel.and_then(|(sy, sx, ey, ex)| {
            let y = vr.line;
            if y < sy || y > ey {
                return None;
            }
            let a = if linewise || y > sy { 0 } else { sx };
            let b = if linewise || y < ey { l.text.len() } else { ex };
            Some((a, b, linewise || y < ey))
        });
        let last_subrow = vr.seg.off + vr.seg.len >= l.text.len();

        // Cerca de codigo.
        if l.fence && line_sel.is_some() {
            // Linha selecionada: sem colorizacao de sintaxe, para o realce ser exato.
            self.selected_row(&l.text, ew, th.fg_h3, th.bg_cur_line, line_sel.unwrap_or((0, 0, false)), linewise, last_subrow);
            return;
        }
        if l.fence {
            self.p(th.bg_cur_line);
            let mut printed = 6;
            let indent: String = l.text.chars().take_while(|c| matches!(c, ' ' | '\t')).collect();
            self.p(&indent);
            printed += indent.len() as i32;
            let rest = &l.text[indent.len()..];
            if let Some(after) = rest.strip_prefix("```") {
                self.p(th.fg_h3);
                self.p(BOLD);
                self.p("```");
                printed += 3;
                if !after.is_empty() {
                    self.p(RESET);
                    self.p(th.bg_cur_line);
                    self.p(th.fg_yellow);
                    self.p(BOLD);
                    let room = (ew - 1 - printed).max(0) as usize;
                    let t = truncate_display(after, room);
                    self.txt(&t);
                    printed += str_width(&t) as i32;
                }
            }
            self.p(RESET);
            self.p(th.bg_cur_line);
            self.pad(ew - 1 - printed);
            self.p("\x1b[K");
            self.p(RESET);
            return;
        }
        if l.in_code && line_sel.is_some() {
            self.selected_row(&l.text, ew, th.fg_main, th.bg_cur_line, line_sel.unwrap_or((0, 0, false)), linewise, last_subrow);
            return;
        }
        if l.in_code {
            self.p(th.bg_cur_line);
            self.p(th.fg_main);
            let printed = self.code_line(&l.text, ew - 1);
            self.p(RESET);
            self.p(th.bg_cur_line);
            self.pad(ew - 1 - printed);
            self.p("\x1b[K");
            self.p(RESET);
            return;
        }

        let hide = app.hide(in_preview);
        let links = if hide.links { find_links(&l.text) } else { Vec::new() };
        let styles = if l.fence { Vec::new() } else { find_styles(&l.text) };
        let line_fg = match l.heading {
            1 => th.fg_h1,
            2 => th.fg_h2,
            0 => th.fg_main,
            _ => th.fg_h3,
        };
        self.p(line_fg);
        if l.heading >= 1 {
            self.p(BOLD);
        }
        let bul = bullet(&l.text, l.heading, l.fence, l.in_code);

        let text: &str = &l.text;
        let end = vr.seg.off + vr.seg.len;
        let mut idx = vr.seg.off;
        let mut cols = 6;
        while idx < end && cols < ew - 1 {
            if let Some((pos, ch)) = bul {
                if idx == pos {
                    self.p(if ch == '*' { th.fg_accent } else { th.fg_yellow });
                    self.p(BOLD);
                    self.p(&text[idx..idx + 1]);
                    self.p("\x1b[22m");
                    self.p(line_fg);
                    cols += 1;
                    idx += 1;
                    continue;
                }
            }
            if let Some((a, z, _)) = line_sel {
                self.p(if idx >= a && idx < z { th.bg_select } else { line_bg });
            }

            // Links ocultos: so o titulo, sublinhado.
            if let Some(sp) = links.iter().find(|s| idx >= s.start && idx < s.end) {
                if idx == sp.start {
                    cols = self.render_link(sp, cols, ew);
                    self.p(line_bg);
                    self.p(line_fg);
                    if l.heading >= 1 {
                        self.p(BOLD);
                    }
                }
                idx = sp.end;
                continue;
            }

            let st = styles.iter().find(|s| idx >= s.open_start && idx < s.close_start + s.close_len);
            if let Some(s) = st {
                if hide.styles {
                    if idx >= s.open_start && idx < s.content_start {
                        if matches!(s.kind, StyleKind::Bold | StyleKind::BoldItalic) {
                            self.p(BOLD);
                        }
                        if matches!(s.kind, StyleKind::Italic | StyleKind::BoldItalic) {
                            self.p(ITALIC);
                        }
                        self.p(line_fg);
                        idx = s.content_start;
                        continue;
                    } else if idx >= s.close_start && idx < s.close_start + s.close_len {
                        self.p("\x1b[22;23m");
                        self.p(line_fg);
                        if l.heading >= 1 {
                            self.p(BOLD);
                        }
                        idx = s.close_start + s.close_len;
                        continue;
                    }
                }
            }

            let c = text[idx..].chars().next().unwrap_or(' ');
            if c == '\t' {
                let tw = 4.min((ew - 1 - cols).max(0));
                self.pad(tw);
                cols += tw;
                idx += 1;
                continue;
            }
            let mut clen = c.len_utf8();
            let mut cw = cp_width(c as u32) as i32;
            if text[idx + clen..].starts_with('\u{FE0F}') {
                cw = 2;
                clen += 3;
            }
            if cols + cw > ew - 1 {
                break;
            }
            self.emit_styled_char(&text[idx..idx + clen], idx, st, hide.styles, line_fg, l.heading);
            cols += cw;
            idx += clen;
        }
        self.p(RESET);
        self.fill_tail(cols, ew, line_bg, line_sel, linewise, last_subrow);
        self.p("\x1b[K");
        self.p(RESET);
    }

    /// Resto da linha apos o texto: no V-Line a linha inteira fica realcada; no
    /// `v` de varias linhas, a quebra de linha vira uma celula realcada.
    fn fill_tail(
        &mut self,
        cols: i32,
        ew: i32,
        base_bg: &str,
        line_sel: Option<(usize, usize, bool)>,
        linewise: bool,
        last_subrow: bool,
    ) {
        let th = self.th;
        let room = (ew - 1 - cols).max(0);
        match line_sel {
            Some(_) if linewise => {
                self.p(th.bg_select);
                self.pad(room);
            }
            Some((_, _, true)) if last_subrow && room > 0 => {
                self.p(th.bg_select);
                self.pad(1);
                self.p(base_bg);
                self.pad(room - 1);
            }
            _ => {
                self.p(base_bg);
                self.pad(room);
            }
        }
    }

    /// Linha de codigo/cerca com selecao: texto simples com realce exato.
    #[allow(clippy::too_many_arguments)]
    fn selected_row(
        &mut self,
        text: &str,
        ew: i32,
        fg: &str,
        base_bg: &str,
        line_sel: (usize, usize, bool),
        linewise: bool,
        last_subrow: bool,
    ) {
        let th = self.th;
        let (a, z, _) = line_sel;
        let mut cols = 6;
        self.p(fg);
        for (i, c) in text.char_indices() {
            let (cw, piece): (i32, String) = if c == '\t' {
                let w = 4.min((ew - 1 - cols).max(0));
                (w, " ".repeat(w as usize))
            } else {
                (cp_width(c as u32) as i32, c.to_string())
            };
            if cols + cw > ew - 1 {
                break;
            }
            self.p(if i >= a && i < z { th.bg_select } else { base_bg });
            self.txt(&piece);
            cols += cw;
        }
        self.p(RESET);
        self.fill_tail(cols, ew, base_bg, Some(line_sel), linewise, last_subrow);
        self.p("\x1b[K");
        self.p(RESET);
    }

    /// Escreve um caractere aplicando estilo de negrito/italico quando houver.
    fn emit_styled_char(
        &mut self,
        ch: &str,
        idx: usize,
        st: Option<&StyleSpan>,
        hide_styles: bool,
        line_fg: &str,
        heading: u8,
    ) {
        let th = self.th;
        let bold_it = |kind: StyleKind| (
            matches!(kind, StyleKind::Bold | StyleKind::BoldItalic),
            matches!(kind, StyleKind::Italic | StyleKind::BoldItalic),
        );
        let Some(s) = st else {
            self.p(line_fg);
            if heading >= 1 {
                self.p(BOLD);
            }
            self.txt(ch);
            return;
        };
        let (b, i) = bold_it(s.kind);
        let clen = ch.len();
        if !hide_styles && idx >= s.open_start && idx < s.content_start {
            // Delimitador de abertura, discreto no modo insercao.
            self.p(th.fg_muted);
            self.txt(ch);
            if idx + clen >= s.content_start {
                if b {
                    self.p(BOLD);
                }
                if i {
                    self.p(ITALIC);
                }
                self.p(line_fg);
            }
        } else if idx >= s.content_start && idx < s.content_end {
            if b {
                self.p(BOLD);
            }
            if i {
                self.p(ITALIC);
            }
            self.p(line_fg);
            self.txt(ch);
            if idx + clen >= s.content_end {
                self.p("\x1b[22;23m");
                self.p(line_fg);
                if heading >= 1 {
                    self.p(BOLD);
                }
                if !hide_styles {
                    self.p(th.fg_muted);
                }
            }
        } else if !hide_styles && idx >= s.close_start && idx < s.close_start + s.close_len {
            self.p(th.fg_muted);
            self.txt(ch);
            if idx + clen >= s.close_start + s.close_len {
                self.p("\x1b[22;23m");
                self.p(line_fg);
                if heading >= 1 {
                    self.p(BOLD);
                }
            }
        }
    }

    fn render_link(&mut self, sp: &LinkSpan, cols: i32, ew: i32) -> i32 {
        let th = self.th;
        let wiki = i32::from(sp.wiki) * 2;
        let full_w = str_width(&sp.text) as i32 + wiki;
        let (text, w) = if cols + full_w > ew - 1 {
            let avail = ew - 1 - cols - wiki;
            if avail <= 0 {
                return ew - 1;
            }
            let t = truncate_display(&sp.text, avail as usize);
            let w = str_width(&t) as i32 + wiki;
            (t, w)
        } else {
            (sp.text.clone(), full_w)
        };
        if sp.wiki {
            self.p(th.fg_wiki);
            self.p(BOLD);
            self.p("[");
            self.txt(&text);
            self.p("]");
            self.p("\x1b[22m");
        } else {
            self.p(th.fg_link);
            self.p(UNDERLINE);
            self.txt(&text);
            self.p(RESET);
        }
        if cols + full_w > ew - 1 {
            ew - 1
        } else {
            cols + w
        }
    }

    /// Colorizacao leve de blocos de codigo. Devolve as colunas usadas.
    fn code_line(&mut self, text: &str, max_w: i32) -> i32 {
        let th = self.th;
        let mut cols = 6;
        let t = text;
        let b = t.as_bytes();
        let mut i = 0;
        // Recuo preservado (TAB = 4 colunas).
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t') && cols < max_w {
            let w = if b[i] == b'\t' { 4.min(max_w - cols) } else { 1 };
            self.pad(w);
            cols += w;
            i += 1;
        }
        let rest = &t[i..];
        // [secao] de arquivos ini/conf.
        if rest.starts_with('[') && rest.contains(']') {
            self.p(th.fg_h2);
            self.p(BOLD);
            let close = rest.find(']').unwrap_or(0);
            let sect = truncate_display(&rest[..=close], (max_w - cols).max(0) as usize);
            self.txt(&sect);
            cols += str_width(&sect) as i32;
            i += close + 1;
            self.p(RESET);
            self.p(th.bg_cur_line);
            self.p(th.fg_main);
        }
        let rest = &t[i..];
        if rest.starts_with('#') || rest.starts_with("//") || rest.starts_with(';') {
            self.p(th.fg_muted);
            let tr = truncate_display(rest, (max_w - cols).max(0) as usize);
            self.txt(&tr);
            cols += str_width(&tr) as i32;
            self.p(RESET);
            self.p(th.bg_cur_line);
            self.p(th.fg_main);
            return cols;
        }
        const KEYWORDS: &[&str] = &[
            "sudo", "def", "return", "if", "else", "elif", "for", "while", "in", "import", "from", "class", "int",
            "char", "void", "const", "static", "struct", "bool", "true", "false", "null", "echo", "cat", "grep",
            "mkdir", "cd", "ls", "chmod", "chown", "systemctl", "apt", "yum", "export", "function", "case", "esac",
            "fi", "then", "do", "done", "set", "alias", "restart", "path", "writeable", "create", "directory",
            "mask", "public", "yes", "no", "read", "write",
        ];
        while i < t.len() && cols < max_w {
            let c = t[i..].chars().next().unwrap_or(' ');
            let room = |cols: i32| (max_w - cols).max(0) as usize;
            if c == '#' || (t[i..].starts_with("//")) || c == ';' {
                self.p(th.fg_muted);
                let tr = truncate_display(&t[i..], room(cols));
                self.txt(&tr);
                cols += str_width(&tr) as i32;
                break;
            }
            if c == '"' || c == '\'' {
                // String entre aspas.
                let mut j = i + 1;
                while j < t.len() {
                    let cj = t[j..].chars().next().unwrap_or(' ');
                    if cj == '\\' && j + 1 < t.len() {
                        j += 1 + t[j + 1..].chars().next().map_or(0, char::len_utf8);
                        continue;
                    }
                    j += cj.len_utf8();
                    if cj == c {
                        break;
                    }
                }
                self.p(th.fg_date);
                let tr = truncate_display(&t[i..j.min(t.len())], room(cols));
                self.txt(&tr);
                cols += str_width(&tr) as i32;
                self.p(th.fg_main);
                i = j.min(t.len());
                continue;
            }
            if c == '$' {
                let mut j = i + 1;
                if t[j..].starts_with('{') {
                    j += t[j..].find('}').map_or(t.len() - j, |k| k + 1);
                } else {
                    while j < t.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                        j += 1;
                    }
                }
                self.p(th.fg_warn);
                let tr = truncate_display(&t[i..j], room(cols));
                self.txt(&tr);
                cols += str_width(&tr) as i32;
                self.p(th.fg_main);
                i = j;
                continue;
            }
            let prev_space = i == 0 || b[i - 1].is_ascii_whitespace();
            if c == '-' && prev_space && i + 1 < t.len() && (b[i + 1].is_ascii_alphabetic() || b[i + 1] == b'-') {
                let mut j = i;
                while j < t.len() && !b[j].is_ascii_whitespace() && b[j] != b'=' {
                    j += 1;
                }
                self.p(th.fg_yellow);
                let tr = truncate_display(&t[i..j], room(cols));
                self.txt(&tr);
                cols += str_width(&tr) as i32;
                self.p(th.fg_main);
                i = j;
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' {
                let mut j = i;
                while j < t.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                let word = &t[i..j];
                let kw = KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(word));
                if kw {
                    self.p(th.fg_h1);
                    self.p(BOLD);
                }
                let tr = truncate_display(word, room(cols));
                self.txt(&tr);
                cols += str_width(&tr) as i32;
                if kw {
                    self.p(RESET);
                    self.p(th.bg_cur_line);
                    self.p(th.fg_main);
                }
                i = j;
                continue;
            }
            if matches!(c, '=' | ':' | '|' | '&') {
                self.p(th.fg_accent);
                self.p(&t[i..i + 1]);
                self.p(th.fg_main);
                cols += 1;
                i += 1;
                continue;
            }
            if c == '\t' {
                let w = 4.min(max_w - cols);
                self.pad(w);
                cols += w;
                i += 1;
                continue;
            }
            let mut clen = c.len_utf8();
            let mut cw = cp_width(c as u32) as i32;
            if t[i + clen..].starts_with('\u{FE0F}') {
                cw = 2;
                clen += 3;
            }
            if cols + cw > max_w {
                break;
            }
            self.txt(&t[i..i + clen]);
            cols += cw;
            i += clen;
        }
        cols
    }

    // ---- barra de status ---------------------------------------------------

    fn status_bar(&mut self) {
        let th = self.th;
        let app = self.app;
        let (badge, badge_bg) = match app.mode {
            Mode::Insert => (
                if app.buf().is_some_and(|b| b.has_selection) { " INSERT [SEL] " } else { " INSERT " },
                th.bg_badge_ins,
            ),
            Mode::VisualChar => (" VISUAL ", th.bg_badge_vis),
            Mode::VisualLine => (" V-LINE ", th.bg_badge_vis),
            Mode::Command => (" COMMAND ", th.bg_badge_cmd),
            Mode::SearchLeft => (" SEARCH ", th.bg_badge_cmd),
            Mode::ConfirmDelete => (" DELETE? ", th.bg_badge_cmd),
            Mode::PromptNew => (" NEW NOTE ", th.bg_badge_ins),
            Mode::PromptRename => (" RENAME ", th.bg_badge_cmd),
            _ => (" NORMAL ", th.bg_badge_nor),
        };

        let preview_name = if app.focus == Focus::Left {
            app.preview.as_ref().map(|p| match p.key.db_id().and_then(|id| app.db.get_name(id)) {
                Some(n) if n.contains('.') => n,
                Some(n) => format!("{}.md", n),
                None => match &p.key {
                    crate::notes::NoteKey::File(path) => path
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    k => k.to_string(),
                },
            })
        } else {
            None
        };
        let modified = app.tab().is_some_and(|t| t.modified);
        let cur_name = app.tab().map_or_else(|| "[Sem Arquivo]".to_string(), |t| t.filename.clone());
        // Resumo do que sera copiado/recortado, para o usuario saber o alcance.
        let sel_info = selection_summary(app);
        let note_desc_w = if let Some(pn) = &preview_name {
            str_width(&format!("[Prévia] {} (Enter ou duplo clique p/ abrir)", pn))
        } else {
            str_width(&format!(
                "{}{}{}",
                if app.tabs.is_empty() { "[Sem Abas Abertas]" } else { cur_name.as_str() },
                if modified { " [+]" } else { "" },
                sel_info
            ))
        };

        let rinfo = if app.focus == Focus::Left && app.preview.is_some() {
            format!(" {}L (prévia)", app.preview.as_ref().map_or(0, |p| p.lines.len()))
        } else if let Some(b) = app.buf() {
            format!(" {}:{} {}L", b.cy + 1, b.cx + 1, b.num_lines())
        } else {
            " 1:1 0L".to_string()
        };
        let rlen = str_width(&rinfo) as i32;

        // Tres niveis de rotulo: completo, curto (texto abreviado) e so icones. No
        // modo automatico vale o primeiro que cabe na largura da janela.
        let labels = |level: u8| -> [String; 11] {
            let c = |full: &str, mid: &str, short: &str| match level {
                0 => full.to_string(),
                1 => mid.to_string(),
                _ => short.to_string(),
            };
            [
                c(" [➕ Nova] ", " [➕ Nova] ", " [➕] "),
                c(" [✏️ Renomear] ", " [✏️ Ren.] ", " [✏️] "),
                c(" [🔗 Link] ", " [🔗 Link] ", " [🔗] "),
                c(" [🗑️ Excluir] ", " [🗑️ Excl.] ", " [🗑️] "),
                match level {
                    0 | 1 => format!(" [{}] ", if app.wrap { "Wrap:ON" } else { "Wrap:OFF" }),
                    _ => format!(" [W:{}] ", if app.wrap { "ON" } else { "OFF" }),
                },
                c(" [🔍 Buscar] ", " [🔍 Busca] ", " [🔍] "),
                c(" [🔄 Subst] ", " [🔄 Subst] ", " [🔄] "),
                match level {
                    0 => format!(" [🎨 {}] ", th.name),
                    _ => " [🎨] ".to_string(),
                },
                c(" [🧬 Life] ", " [🧬 Life] ", " [🧬] "),
                c(" [⬇ Importar] ", " [⬇ Imp.] ", " [⬇] "),
                c(" [⬆ Exportar] ", " [⬆ Exp.] ", " [⬆] "),
            ]
        };
        let total_w = |l: &[String; 11]| l.iter().map(|b| str_width(b) as i32).sum::<i32>();
        let level = match app.force_status_wrap {
            1 => 2,
            2 => 0,
            _ => [0u8, 1, 2]
                .into_iter()
                .find(|&lv| total_w(&labels(lv)) <= app.cols)
                .unwrap_or(2),
        };
        let btns = labels(level);
        let widths: Vec<i32> = btns.iter().map(|b| str_width(b) as i32).collect();
        let btns_w: i32 = widths.iter().sum();

        // Linha 1: badge, nome da nota e posicao do cursor.
        self.p(th.bg_status);
        self.p(badge_bg);
        self.p(th.fg_dark);
        self.p(BOLD);
        self.p(badge);
        self.p(RESET);
        self.p(th.bg_status);
        self.p(" ");
        let used1 = badge.len() as i32 + 1;
        if let Some(pn) = &preview_name {
            self.p(th.fg_yellow);
            self.p("[Prévia] ");
            self.p(th.fg_white);
            self.txt(pn);
            self.p(th.fg_muted);
            self.p(" (Enter ou duplo clique p/ abrir)");
        } else {
            self.p(th.fg_white);
            self.p(BOLD);
            self.txt(&cur_name);
            self.p(RESET);
            self.p(th.bg_status);
            if modified {
                self.p(th.fg_warn);
                self.p(" [+]");
            }
            if !sel_info.is_empty() {
                self.p(th.bg_select);
                self.p(th.fg_white);
                self.p(&sel_info);
                self.p(RESET);
                self.p(th.bg_status);
            }
        }
        self.pad((app.cols - used1 - note_desc_w as i32 - rlen).max(1));
        self.p(th.fg_muted);
        self.p(&rinfo);
        // Sem "\x1b[K": a linha ja preenche a largura e o apagamento comeria a
        // ultima coluna (a quebra automatica esta desligada).
        self.p(RESET);
        self.end_row();

        // Linha 2: botoes centralizados.
        self.p(th.bg_select);
        let mut cur_col = 1;
        let margin = ((app.cols - btns_w) / 2).max(1);
        for _ in 1..margin {
            self.p(" ");
            cur_col += 1;
        }
        let mut place = |me: &mut Frame, text: &str, style: &[&str], slot: usize| {
            let start = cur_col;
            me.p(th.bg_select);
            for s in style {
                me.p(s);
            }
            me.p(text);
            cur_col += widths[slot];
            (start, cur_col)
        };
        self.hit.btn_new = place(self, &btns[0], &[th.fg_accent, BOLD], 0);
        self.hit.btn_rename = place(self, &btns[1], &[th.fg_white, BOLD], 1);
        self.hit.btn_link = place(self, &btns[2], &[th.fg_link, BOLD], 2);
        self.hit.btn_delete = place(self, &btns[3], &[th.fg_warn, BOLD], 3);
        self.hit.btn_wrap = place(self, &btns[4], &[th.fg_white, BOLD], 4);
        self.hit.btn_find = place(self, &btns[5], &[], 5);
        self.hit.btn_replace = place(self, &btns[6], &[], 6);
        self.hit.btn_theme = place(self, &btns[7], &[], 7);
        self.hit.btn_life = place(self, &btns[8], &[th.fg_h1, BOLD], 8);
        self.hit.btn_import = place(self, &btns[9], &[th.fg_h2, BOLD], 9);
        self.hit.btn_export = place(self, &btns[10], &[th.fg_h2, BOLD], 10);
        self.pad(app.cols - cur_col + 1);
        self.p("\x1b[K");
        self.p(RESET);
        self.end_row();
    }

    fn command_line(&mut self) {
        let th = self.th;
        let app = self.app;
        let cols = app.cols;
        self.p(th.bg_status);
        if app.mode == Mode::Command {
            self.p(th.fg_h1);
            self.p(BOLD);
            self.p(":");
            self.p(th.fg_white);
            self.txt(&app.cmd_buf);
            self.pad(cols - 1 - str_width(&app.cmd_buf) as i32);
        } else if app.mode == Mode::ConfirmDelete {
            self.p(th.fg_warn);
            self.p(BOLD);
            let t = app.selected_note().map_or("esta nota".to_string(), |n| n.title.clone());
            let prompt = format!(" Deseja realmente excluir a nota '{}'? (s/n): ", t);
            self.txt(&prompt);
            self.p(RESET);
            self.pad(cols - str_width(&prompt) as i32);
        } else if let Some(label) = prompt_label(app) {
            self.p(prompt_color(th, app.mode));
            self.p(BOLD);
            self.txt(&label);
            self.p(RESET);
            self.p(th.fg_white);
            self.txt(&app.prompt_buf);
            self.pad(cols - (str_width(&label) + str_width(&app.prompt_buf)) as i32);
        } else if let Some(msg) = app.status_text() {
            self.p(th.fg_yellow);
            self.p(" ");
            self.txt(msg);
            self.pad(cols - 1 - str_width(msg) as i32);
        } else {
            let hint = if app.focus == Focus::Left {
                " [j/k]: Navegar | [Enter]: Abrir | [n]: Nova | [r]: Renomear | [d]: Excluir | [/]: Buscar | [Tab]: Editor"
            } else {
                " [gt/gT]: Abas | [Ctrl+T]: Tema | [Roda do mouse na aba]: Fechar aba | [:w/:q]: Comandos"
            };
            self.p(th.fg_muted);
            let shown = truncate_display(hint, cols.max(0) as usize);
            self.p(&shown);
            self.pad(cols - 1 - str_width(&shown) as i32);
        }
        self.p(RESET);
    }

    // ---- cursor ------------------------------------------------------------

    fn place_cursor(&mut self, editor_rows: i32) {
        let app = self.app;
        let rows = app.rows;
        match app.mode {
            Mode::Command => {
                let col = 1 + width_len(&app.cmd_buf, app.cmd_cursor) as i32 + 1;
                self.p(&format!("\x1b[{};{}H{}\x1b[?25h", rows, col, CURSOR_BAR));
            }
            Mode::ConfirmDelete => {
                let t = app.selected_note().map_or("esta nota".to_string(), |n| n.title.clone());
                let col = str_width(&format!(" Deseja realmente excluir a nota '{}'? (s/n): ", t)) as i32 + 1;
                self.p(&format!("\x1b[{};{}H{}\x1b[?25h", rows, col, CURSOR_BLOCK));
            }
            m if m.is_prompt() => {
                let label_w = prompt_label(app).map_or(0, |l| str_width(&l)) as i32;
                let col = label_w + width_len(&app.prompt_buf, app.prompt_cursor) as i32 + 1;
                self.p(&format!("\x1b[{};{}H{}\x1b[?25h", rows, col, CURSOR_BAR));
            }
            Mode::SearchLeft => {
                let avail = (app.left_width - 6).max(2);
                let qw = (str_width(&app.search_query) as i32).min(avail);
                self.p(&format!("\x1b[2;{}H{}\x1b[?25h", 5 + qw, CURSOR_BAR));
            }
            _ if app.focus == Focus::Left => {
                let crow = ((app.selected as i32 - app.sidebar_scroll as i32) + 4).clamp(4, editor_rows.max(4));
                self.p(&format!("\x1b[{};3H{}\x1b[?25h", crow, CURSOR_BLOCK));
            }
            _ if app.tabs.is_empty() => self.p("\x1b[?25l"),
            _ => {
                let lay = app.layout(false);
                let Some(b) = app.buf() else { return };
                let (vrow, vcol) = crate::layout::cursor_visual_pos(b, &lay);
                let cy = (vrow as i32 - b.scroll_y as i32 + 2).clamp(2, editor_rows.max(2));
                let lo = app.left_width + 2;
                let cx = (app.left_width + 1 + 6 + vcol as i32 + 1).clamp(lo, app.cols.max(lo));
                let shape = if app.mode == Mode::Insert { CURSOR_BAR } else { CURSOR_BLOCK };
                self.p(&format!("\x1b[{};{}H{}\x1b[?25h", cy, cx, shape));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn app_with_notes() -> App {
        let db = Db::open_memory();
        db.insert_note("Alfa", "# Alfa\n\nconteudo **negrito** e [link](http://x.com)\n", 100, 1);
        db.insert_note("Beta", "# Beta\n\n```\nlet x = 1; # c\n```\n", 50, 0);
        let mut a = App::new(db);
        a.cols = 120;
        a.rows = 30;
        a
    }

    /// Texto visivel do quadro: remove sequencias CSI.
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

    #[test]
    fn frame_is_synchronized_and_cursor_is_steady() {
        let mut a = app_with_notes();
        let f = draw(&mut a);
        assert!(f.starts_with(SYNC_BEGIN) && f.ends_with(SYNC_END));
        for bad in ["\x1b[5 q", "\x1b[1 q", "\x1b[3 q", "\x1b[0 q"] {
            assert!(!f.contains(bad), "cursor piscante {:?}", bad);
        }
        assert!(f.contains("\x1b[?12l"));
    }

    #[test]
    fn every_mode_requests_a_steady_shape() {
        let mut a = app_with_notes();
        for (mode, focus, shape) in [
            (Mode::Normal, Focus::Right, CURSOR_BLOCK),
            (Mode::Insert, Focus::Right, CURSOR_BAR),
            (Mode::Command, Focus::Right, CURSOR_BAR),
            (Mode::Normal, Focus::Left, CURSOR_BLOCK),
            (Mode::SearchLeft, Focus::Left, CURSOR_BAR),
            (Mode::PromptNew, Focus::Right, CURSOR_BAR),
            (Mode::ConfirmDelete, Focus::Left, CURSOR_BLOCK),
        ] {
            a.mode = mode;
            a.focus = focus;
            let f = draw(&mut a);
            assert!(f.contains(shape), "{:?}/{:?} deveria usar {:?}", mode, focus, shape);
            assert!(!f.contains("\x1b[5 q"));
        }
    }

    #[test]
    fn frame_has_exact_row_count_and_visible_content() {
        let mut a = app_with_notes();
        a.focus = Focus::Right;
        let f = draw(&mut a);
        let v = visible(&f);
        assert_eq!(v.matches("\r\n").count() as i32, a.rows - 1);
        assert!(v.contains("NotaZ"));
        assert!(v.contains("Alfa") && v.contains("Beta"));
        assert!(v.contains("negrito"), "texto estilizado aparece sem os asteriscos");
        assert!(!v.contains("**negrito**"));
        assert!(v.contains("link") && !v.contains("http://x.com"), "URL oculta no modo normal");
    }

    #[test]
    fn insert_mode_shows_markdown_delimiters_and_urls() {
        let mut a = app_with_notes();
        a.focus = Focus::Right;
        a.mode = Mode::Insert;
        let v = visible(&draw(&mut a));
        assert!(v.contains("**negrito**"));
        assert!(v.contains("http://x.com"));
    }

    #[test]
    fn malicious_note_content_cannot_inject_escapes() {
        let db = Db::open_memory();
        db.insert_note("Mal", "# Mal\x1b]0;hack\x07\n\ntexto \x1b]52;c;QQ==\x07 fim\n\x1b[2Jrisco\n", 10, 0);
        let mut a = App::new(db);
        a.cols = 120;
        a.rows = 30;
        a.focus = Focus::Right;
        let f = draw(&mut a);
        assert!(!f.contains("\x1b]52"));
        assert!(!f.contains("\x1b]0;"));
        assert!(!f.contains('\x07'));
        assert!(!f.contains("\x1b[2Jrisco"));
        assert!(visible(&f).contains("]52;c;QQ== fim"), "o payload vira texto inerte");
    }

    #[test]
    fn button_hit_boxes_are_recorded_and_ordered() {
        let mut a = app_with_notes();
        draw(&mut a);
        let h = &a.hit;
        let order = [h.btn_new, h.btn_rename, h.btn_link, h.btn_delete, h.btn_wrap, h.btn_find, h.btn_replace, h.btn_theme, h.btn_life, h.btn_import, h.btn_export];
        for w in order.windows(2) {
            assert_eq!(w[0].1, w[1].0, "botoes contiguos");
            assert!(w[0].0 < w[0].1);
        }
        assert!(h.filter[0].0 < h.filter[1].0 && h.filter[5].0 < h.filter[6].0);
        assert_eq!(h.tabs.len(), 1);
    }

    #[test]
    fn narrow_terminals_use_compact_labels() {
        let mut a = app_with_notes();
        a.cols = 90;
        let v = visible(&draw(&mut a));
        assert!(v.contains("[W:ON]") && !v.contains("Wrap:ON"));
    }

    #[test]
    fn tiny_windows_do_not_panic() {
        let mut a = app_with_notes();
        for (c, r) in [(20, 6), (10, 4), (1, 1), (50, 5), (200, 80)] {
            a.cols = c;
            a.rows = r;
            let _ = draw(&mut a);
        }
    }

    /// Parte direita (editor) de cada linha do quadro, sem a lista lateral.
    fn editor_rows(frame: &str) -> Vec<String> {
        // Linha 0 = barra de abas; so as linhas do editor (o rodape usa a mesma cor).
        frame
            .split("\r\n")
            .take(21)
            .map(|r| r.split_once("│\x1b[0m").map_or(r.to_string(), |(_, right)| right.to_string()))
            .collect()
    }

    fn selected_app(content: &str) -> App {
        let db = Db::open_memory();
        db.insert_note("S", content, 10, 0);
        let mut a = App::new(db);
        a.cols = 100;
        a.rows = 24;
        a.focus = Focus::Right;
        a
    }

    #[test]
    fn visual_line_highlights_entire_rows_including_empty_ones_and_code() {
        let mut a = selected_app("primeira\n\n```\ncodigo aqui\n```\nultima\n");
        let key = a.notes[0].key.clone();
        a.open_note(&key, true);
        a.focus = Focus::Right;
        a.mode = Mode::VisualLine;
        let b = a.buf_mut().unwrap();
        b.vis_y = 0;
        b.cy = 4; // da 1a linha ate a cerca final
        let bg = a.theme().bg_select;
        let f = draw(&mut a);
        let rows = editor_rows(&f);
        // linhas de conteudo do editor: 1..=5 (a 0 e a barra de abas)
        for (i, name) in ["primeira", "(vazia)", "```", "codigo", "```"].iter().enumerate() {
            let row = &rows[i + 1];
            assert!(row.contains(bg), "linha {name} sem realce: {row:?}");
            // O realce chega ate o fim da area: o ultimo bg e o de selecao.
            let last_bg = row.rfind("\x1b[48;2;").unwrap();
            assert!(row[last_bg..].starts_with(bg), "preenchimento nao realcado em {name}");
        }
        assert!(!rows[6].contains(bg), "linha fora da selecao nao deve ser realcada");
        let v = visible(&f);
        assert!(v.contains("5 linhas") && v.contains("selecionados"), "contador na barra de status");
    }

    /// Caracteres impressos enquanto o fundo era o de selecao (decodifica o quadro).
    fn highlighted(row: &str, bg: &str) -> String {
        let (mut cur_sel, mut out) = (false, String::new());
        let mut rest = row;
        while let Some(c) = rest.chars().next() {
            if rest.starts_with("\x1b[") {
                let end = rest.find(|ch: char| ('@'..='~').contains(&ch) && ch != '[').map_or(rest.len(), |i| i + 1);
                let seq = &rest[..end];
                if seq == bg {
                    cur_sel = true;
                } else if seq.starts_with("\x1b[48;2;") || seq == "\x1b[0m" {
                    cur_sel = false;
                }
                rest = &rest[end..];
                continue;
            }
            if cur_sel {
                out.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
        out
    }

    #[test]
    fn charwise_selection_shows_line_break_and_exact_range() {
        let mut a = selected_app("abcdef\nxyz\n");
        let key = a.notes[0].key.clone();
        a.open_note(&key, true);
        a.focus = Focus::Right;
        a.mode = Mode::VisualChar;
        let b = a.buf_mut().unwrap();
        b.vis_y = 0;
        b.vis_x = 4; // "ef" + quebra de linha + "xy"
        b.cy = 1;
        b.cx = 1;
        let bg = a.theme().bg_select;
        let f = draw(&mut a);
        let rows = editor_rows(&f);
        // O espaco extra e a celula da quebra de linha, realcada.
        assert_eq!(highlighted(&rows[1], bg).trim_end_matches(' '), "ef");
        assert!(highlighted(&rows[1], bg).len() > 2, "a quebra de linha tambem e realcada");
        assert_eq!(highlighted(&rows[2], bg), "xy");
        let v = visible(&f);
        assert!(v.contains("2 linhas") && v.contains("4 caracteres"), "{}", v);
    }

    #[test]
    fn code_block_charwise_selection_is_exact() {
        let mut a = selected_app("```\nabcdef\n```\n");
        let key = a.notes[0].key.clone();
        a.open_note(&key, true);
        a.focus = Focus::Right;
        a.mode = Mode::VisualChar;
        let b = a.buf_mut().unwrap();
        b.vis_y = 1;
        b.vis_x = 1;
        b.cy = 1;
        b.cx = 3; // "bcd"
        let bg = a.theme().bg_select;
        let rows = editor_rows(&draw(&mut a));
        assert_eq!(highlighted(&rows[2], bg), "bcd");
    }
    #[test]
    fn no_highlight_or_counter_outside_visual_modes() {
        let mut a = selected_app("abc\n");
        let key = a.notes[0].key.clone();
        a.open_note(&key, true);
        a.focus = Focus::Right;
        let bg = a.theme().bg_select;
        let f = draw(&mut a);
        assert!(!editor_rows(&f).iter().skip(1).any(|r| r.contains(bg)), "sem selecao nao ha realce no editor");
        assert!(!visible(&f).contains("selecionados"));
    }

    #[test]
    fn code_blocks_do_not_wrap_or_duplicate() {
        let mut a = app_with_notes();
        a.cols = 40;
        let n = a.notes.iter().position(|n| n.title == "Beta").unwrap();
        let key = a.notes[n].key.clone();
        a.open_note(&key, true);
        a.focus = Focus::Right;
        let v = visible(&draw(&mut a));
        assert_eq!(v.matches("let x").count(), 1);
    }
}
