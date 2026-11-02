//! Teclado: traducao dos eventos do terminal e despacho por modo/foco.
//! Comandos de varias teclas (gg, gt, gq, dd, za, r<c>) usam um estado
//! pendente em vez de bloquear a leitura.

use crate::app::{App, Focus, Mode, Pending, FIELD_MAX, MIN_LEFT_WIDTH};
use crate::doc::{Buffer, UndoResult};
use crate::layout;
use crate::md::{find_styles, is_table_line};
use crate::text::{floor_boundary, next_char_end, prev_char_start};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Enter,
    Tab,
    Esc,
    Backspace,
    CtrlBackspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    CtrlShiftLeft,
    CtrlShiftRight,
    CtrlLeft,
    CtrlRight,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Traduz um evento do crossterm (ignorando solturas de tecla e Alt+tecla).
pub fn map_key(ev: KeyEvent) -> Option<Key> {
    use crossterm::event::KeyEventKind;
    if ev.kind == KeyEventKind::Release {
        return None;
    }
    let m = ev.modifiers;
    let ctrl = m.contains(KeyModifiers::CONTROL);
    let alt = m.contains(KeyModifiers::ALT);
    let shift = m.contains(KeyModifiers::SHIFT);
    Some(match ev.code {
        KeyCode::Char(c) => {
            if ctrl && !alt {
                Key::Ctrl(c.to_ascii_lowercase())
            } else if alt && !ctrl {
                return None;
            } else {
                Key::Char(c)
            }
        }
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => {
            if ctrl {
                Key::CtrlBackspace
            } else {
                Key::Backspace
            }
        }
        KeyCode::Delete => Key::Delete,
        KeyCode::Up => if shift { Key::ShiftUp } else { Key::Up },
        KeyCode::Down => if shift { Key::ShiftDown } else { Key::Down },
        KeyCode::Left => match (ctrl, shift) {
            (true, true) => Key::CtrlShiftLeft,
            (true, false) => Key::CtrlLeft,
            (false, true) => Key::ShiftLeft,
            _ => Key::Left,
        },
        KeyCode::Right => match (ctrl, shift) {
            (true, true) => Key::CtrlShiftRight,
            (true, false) => Key::CtrlRight,
            (false, true) => Key::ShiftRight,
            _ => Key::Right,
        },
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        _ => return None,
    })
}

/// Edicao de uma linha (comando, busca, prompts). Devolve `true` se tratou a tecla.
pub fn edit_field(buf: &mut String, cursor: &mut usize, key: &Key) -> bool {
    *cursor = floor_boundary(buf, (*cursor).min(buf.len()));
    match key {
        Key::Left => *cursor = prev_char_start(buf, *cursor),
        Key::Right => *cursor = next_char_end(buf, *cursor),
        Key::CtrlLeft => {
            while *cursor > 0 && buf.as_bytes()[*cursor - 1] == b' ' {
                *cursor -= 1;
            }
            while *cursor > 0 && buf.as_bytes()[*cursor - 1] != b' ' {
                *cursor = prev_char_start(buf, *cursor);
            }
        }
        Key::CtrlRight => {
            while *cursor < buf.len() && buf.as_bytes()[*cursor] != b' ' {
                *cursor = next_char_end(buf, *cursor);
            }
            while *cursor < buf.len() && buf.as_bytes()[*cursor] == b' ' {
                *cursor += 1;
            }
        }
        Key::Home | Key::Ctrl('a') => *cursor = 0,
        Key::End | Key::Ctrl('e') => *cursor = buf.len(),
        Key::Backspace => {
            if *cursor > 0 {
                let prev = prev_char_start(buf, *cursor);
                buf.replace_range(prev..*cursor, "");
                *cursor = prev;
            }
        }
        Key::Delete => {
            if *cursor < buf.len() {
                let end = next_char_end(buf, *cursor);
                buf.replace_range(*cursor..end, "");
            }
        }
        Key::Ctrl('u') => {
            buf.clear();
            *cursor = 0;
        }
        Key::Ctrl('k') => buf.truncate(*cursor),
        Key::Char(c) if !c.is_control() => {
            if buf.len() + c.len_utf8() <= FIELD_MAX {
                buf.insert(*cursor, *c);
                *cursor += c.len_utf8();
            }
        }
        _ => return false,
    }
    true
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

impl App {
    fn with_buf<R>(&mut self, f: impl FnOnce(&mut Buffer) -> R) -> Option<R> {
        Some(f(self.buf_mut()?))
    }

    fn enter_mode_normal(&mut self, msg: &str) {
        self.mode = Mode::Normal;
        self.set_status(msg);
    }

    /// Ponto de entrada: processa uma tecla.
    pub fn on_key(&mut self, key: Key) {
        self.last_activity = Instant::now();
        self.needs_redraw = true;

        if self.pending != Pending::None {
            self.handle_pending(key);
            return;
        }
        if self.mode.is_prompt() {
            self.handle_prompt(key);
            return;
        }
        if self.mode == Mode::ConfirmDelete {
            match key {
                Key::Char('s' | 'S' | 'y' | 'Y') => {
                    self.mode = Mode::Normal;
                    self.delete_selected_note();
                }
                Key::Char('n' | 'N') | Key::Esc | Key::Ctrl('c') | Key::Enter => {
                    self.mode = Mode::Normal;
                    self.set_status("Exclusão cancelada.");
                }
                _ => {}
            }
            return;
        }
        if key == Key::Esc {
            self.generic_escape();
            return;
        }

        // Atalhos globais do modo Normal.
        if self.mode == Mode::Normal {
            match key {
                Key::Ctrl('s') => {
                    self.save_current();
                    return;
                }
                Key::Ctrl('t') => {
                    self.cycle_theme();
                    return;
                }
                Key::CtrlLeft => {
                    if self.left_width > MIN_LEFT_WIDTH {
                        self.left_width -= 2;
                    }
                    self.set_status(format!("Largura do painel: {}", self.left_width));
                    return;
                }
                Key::CtrlRight => {
                    if self.left_width < self.cols - 25 {
                        self.left_width += 2;
                    }
                    self.set_status(format!("Largura do painel: {}", self.left_width));
                    return;
                }
                Key::Ctrl('n') => {
                    self.jump_to_next_link();
                    return;
                }
                _ => {}
            }
        }
        if key == Key::Ctrl('f') && matches!(self.mode, Mode::Normal | Mode::SearchLeft) {
            self.start_sidebar_search("Busca de notas (digite o termo, Enter para abrir)");
            return;
        }
        if matches!(key, Key::Tab | Key::Ctrl('w')) && matches!(self.mode, Mode::Normal | Mode::SearchLeft) {
            let on_table = self
                .buf()
                .is_some_and(|b| is_table_line(&b.cur().text));
            if key == Key::Tab && self.focus == Focus::Right && on_table {
                self.table_jump_next_cell(false);
                return;
            }
            if self.focus == Focus::Left {
                self.free_preview();
                self.focus = Focus::Right;
            } else {
                self.focus = Focus::Left;
                self.sync_preview_to_selection();
            }
            let msg = if self.focus == Focus::Left {
                "Notas (use j/k e Enter)"
            } else {
                "Editor de Texto"
            };
            self.set_status(format!("Painel ativo: {}", msg));
            return;
        }

        match self.mode {
            Mode::SearchLeft => self.handle_search_left(key),
            Mode::Command => self.handle_command_mode(key),
            _ if self.focus == Focus::Left => self.handle_left_focus(key),
            Mode::Insert => self.handle_insert(key),
            _ => self.handle_normal(key),
        }
    }

    /// Esc: sempre volta ao modo Normal com o foco no editor.
    fn generic_escape(&mut self) {
        match self.mode {
            Mode::SearchLeft => {
                self.search_query.clear();
                self.search_cursor = 0;
                self.apply_search();
                self.free_preview();
            }
            Mode::Command => {
                self.cmd_buf.clear();
                self.cmd_cursor = 0;
            }
            Mode::Insert => {
                self.with_buf(|b| {
                    b.cx = prev_char_start(&b.cur().text, b.cx);
                });
            }
            _ => {}
        }
        self.with_buf(|b| b.has_selection = false);
        self.mode = Mode::Normal;
        self.focus = Focus::Right;
        self.pending = Pending::None;
        self.set_status("-- MODO NORMAL --");
    }

    fn start_sidebar_search(&mut self, msg: &str) {
        self.mode = Mode::SearchLeft;
        self.focus = Focus::Left;
        self.search_query.clear();
        self.search_cursor = 0;
        self.apply_search();
        self.sync_preview_to_selection();
        self.set_status(msg);
    }

    fn open_prompt(&mut self, mode: Mode, initial: &str) {
        self.prompt_buf = initial.to_string();
        self.prompt_cursor = self.prompt_buf.len();
        self.mode = mode;
    }

    // ---- teclas de varias etapas ------------------------------------------

    fn handle_pending(&mut self, key: Key) {
        let pending = std::mem::replace(&mut self.pending, Pending::None);
        if key == Key::Esc {
            if matches!(pending, Pending::ReplaceChar | Pending::ReplaceVisual) {
                self.set_status(if self.mode == Mode::VisualLine {
                    "-- VISUAL LINHA --"
                } else if self.mode == Mode::VisualChar {
                    "-- VISUAL --"
                } else {
                    "-- MODO NORMAL --"
                });
            }
            return;
        }
        match pending {
            Pending::G => match key {
                Key::Char('g') if self.focus == Focus::Left => {
                    if !self.filtered.is_empty() {
                        self.selected = 0;
                        self.sidebar_scroll = 0;
                        self.sync_preview_to_selection();
                        self.set_status(format!("Primeira nota (1 de {})", self.filtered.len()));
                    }
                }
                Key::Char('g') => {
                    self.with_buf(|b| {
                        b.cy = 0;
                        while b.cy + 1 < b.lines.len() && b.lines[b.cy].hidden {
                            b.cy += 1;
                        }
                        b.cx = 0;
                        b.scroll_y = 0;
                    });
                    self.editor_scroll();
                    self.clear_needed = true;
                    let y = self.buf().map_or(1, |b| b.cy + 1);
                    self.set_status(format!("Topo da nota (linha {})", y));
                }
                Key::Char('q') => self.pending = Pending::Gq,
                Key::Char('t') => self.tab_next(),
                Key::Char('T') => self.tab_prev(),
                Key::Char('l') | Key::Tab => self.jump_to_next_link(),
                other => self.on_key(other),
            },
            Pending::Gq => {
                if matches!(key, Key::Char('q' | ' ') | Key::Enter) {
                    let y = self.buf().map_or(0, |b| b.cy);
                    self.table_align_at(y);
                }
            }
            Pending::Z => match key {
                Key::Char('a') => match self.with_buf(|b| b.toggle_fold()).flatten() {
                    Some(folded) => {
                        self.editor_scroll();
                        self.clear_needed = true;
                        self.set_status(format!(
                            "Dobra de seção {}",
                            if folded { "recolhida (za)" } else { "expandida (za)" }
                        ));
                    }
                    None => self.set_status("Nenhum cabeçalho (#) encontrado para recolher/expandir"),
                },
                Key::Char('M' | 'm') => self.fold_all(true),
                Key::Char('R' | 'r') => self.fold_all(false),
                Key::Char('c') => {
                    if self.with_buf(|b| b.set_fold_at_cursor(true)) == Some(true) {
                        self.editor_scroll();
                        self.clear_needed = true;
                        self.set_status("Seção recolhida (zc)");
                    }
                }
                Key::Char('o') => {
                    if self.with_buf(|b| b.set_fold_at_cursor(false)) == Some(true) {
                        self.editor_scroll();
                        self.clear_needed = true;
                        self.set_status("Seção expandida (zo)");
                    }
                }
                other => self.on_key(other),
            },
            Pending::D => {
                if key == Key::Char('d') {
                    self.delete_current_line();
                }
            }
            Pending::ReplaceChar => self.replace_under_cursor(key),
            Pending::ReplaceVisual => self.replace_selection_chars(key),
            Pending::None => {}
        }
    }

    pub fn fold_all(&mut self, folded: bool) {
        let n = self.with_buf(|b| b.set_all_folds(folded)).unwrap_or(0);
        self.editor_scroll();
        self.clear_needed = true;
        if n == 0 {
            self.set_status("Nenhum cabeçalho (#) encontrado nesta nota");
        } else if folded {
            self.set_status(format!("Todas as {} seções recolhidas (zM)", n));
        } else {
            self.set_status(format!("Todas as {} seções expandidas (zR)", n));
        }
    }

    fn delete_current_line(&mut self) {
        let Some(b) = self.buf() else { return };
        let text = b.cur().text.clone();
        self.set_yank(text, true);
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let cy = b.cy;
            b.delete_lines(cy, 1);
            if b.cy >= b.lines.len() {
                b.cy = b.lines.len() - 1;
            }
            b.clamp_cursor();
        });
        self.set_status("1 linha excluída");
    }

    fn replace_under_cursor(&mut self, key: Key) {
        let Some(b) = self.buf() else { return };
        if b.cx >= b.cur().len() {
            return;
        }
        match key {
            Key::Ctrl('c') => self.set_status("-- MODO NORMAL --"),
            Key::Enter => {
                self.with_buf(|b| b.save_undo());
                self.edit(|b| {
                    let (cy, cx) = (b.cy, b.cx);
                    let end = next_char_end(&b.lines[cy].text, cx);
                    let rest = b.lines[cy].text.split_off(end);
                    b.lines[cy].text.truncate(cx);
                    b.lines[cy].detect_heading();
                    b.insert_line(cy + 1, &rest);
                    b.cy += 1;
                    b.cx = 0;
                });
                self.set_status("-- MODO NORMAL --");
            }
            Key::Char(c) if !c.is_control() => {
                self.with_buf(|b| b.save_undo());
                self.edit(|b| {
                    let (cy, cx) = (b.cy, b.cx);
                    let end = next_char_end(&b.lines[cy].text, cx);
                    let mut s = [0u8; 4];
                    b.lines[cy].text.replace_range(cx..end, c.encode_utf8(&mut s));
                    b.lines[cy].detect_heading();
                });
                self.set_status("-- MODO NORMAL --");
            }
            _ => self.set_status("-- MODO NORMAL --"),
        }
    }

    fn replace_selection_chars(&mut self, key: Key) {
        let visual_msg = if self.mode == Mode::VisualLine { "-- VISUAL LINHA --" } else { "-- VISUAL --" };
        let c = match key {
            Key::Char(c) if !c.is_control() => c,
            Key::Ctrl('c') => {
                self.set_status(visual_msg);
                return;
            }
            _ => return,
        };
        let linewise = self.mode == Mode::VisualLine;
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let Some((sy, sx, ey, ex)) = b.selection(linewise) else { return };
            for y in sy..=ey {
                let t = &b.lines[y].text;
                let a = if y == sy { sx } else { 0 };
                let z = if y == ey { ex.min(t.len()) } else { t.len() };
                let a = floor_boundary(t, a.min(z));
                let n = t[a..z].chars().count();
                let rep: String = std::iter::repeat(c).take(n).collect();
                let mut new = String::with_capacity(t.len());
                new.push_str(&t[..a]);
                new.push_str(&rep);
                new.push_str(&t[z..]);
                b.lines[y].set_text(new);
            }
            b.clamp_cursor();
        });
        self.mode = Mode::Normal;
        self.set_status("-- MODO NORMAL --");
    }

    // ---- prompts -----------------------------------------------------------

    fn handle_prompt(&mut self, key: Key) {
        if matches!(key, Key::Esc | Key::Ctrl('c')) {
            let msg = match self.mode {
                Mode::PromptNew => "Criação de nota cancelada.",
                Mode::PromptRename => "Renomeação cancelada.",
                Mode::PromptFind => "Busca cancelada.",
                Mode::PromptReplaceFind | Mode::PromptReplaceWith => "Substituição cancelada.",
                Mode::PromptImport => "Importação cancelada.",
                _ => "Exportação cancelada.",
            };
            self.set_status(msg);
            self.mode = Mode::Normal;
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.rename_target = None;
            return;
        }
        if key == Key::Enter {
            let text = std::mem::take(&mut self.prompt_buf);
            self.prompt_cursor = 0;
            let mode = self.mode;
            match mode {
                Mode::PromptImport => {
                    self.mode = Mode::Normal;
                    self.ui_import(&text);
                    self.focus = Focus::Left;
                    self.clear_needed = true;
                }
                Mode::PromptExport => {
                    self.mode = Mode::Normal;
                    self.ui_export(&text);
                }
                Mode::PromptNew => {
                    self.mode = Mode::Normal;
                    self.create_new_note(&text);
                }
                Mode::PromptRename => {
                    self.mode = Mode::Normal;
                    if let Some(target) = self.rename_target.take() {
                        self.rename_note(&target, &text);
                    }
                }
                Mode::PromptFind => {
                    self.mode = Mode::Normal;
                    if !text.is_empty() {
                        self.search_from_cursor(&text);
                    }
                }
                Mode::PromptReplaceFind => {
                    if text.is_empty() {
                        self.mode = Mode::Normal;
                        self.set_status("Substituição cancelada (termo vazio).");
                    } else {
                        self.replace_find = text;
                        self.mode = Mode::PromptReplaceWith;
                        let f = self.replace_find.clone();
                        self.set_status(format!(
                            "Substituir '{}' por (digite o novo texto e pressione Enter):",
                            f
                        ));
                    }
                }
                _ => {
                    self.mode = Mode::Normal;
                    let find = self.replace_find.clone();
                    self.replace_in_current(&find, &text);
                }
            }
            return;
        }
        let (mut buf, mut cur) = (std::mem::take(&mut self.prompt_buf), self.prompt_cursor);
        edit_field(&mut buf, &mut cur, &key);
        self.prompt_buf = buf;
        self.prompt_cursor = cur;
    }

    // ---- busca lateral e linha de comando ---------------------------------

    fn handle_search_left(&mut self, key: Key) {
        match key {
            Key::Enter => {
                self.mode = Mode::Normal;
                if let Some(k) = self.selected_key() {
                    self.free_preview();
                    self.open_note(&k, true);
                    self.focus = Focus::Right;
                    let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
                    self.set_status(format!("Aberta em aba: {}", name));
                }
            }
            Key::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.sync_preview_to_selection();
                }
            }
            Key::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                    self.sync_preview_to_selection();
                }
            }
            other => {
                let (mut q, mut cur) = (std::mem::take(&mut self.search_query), self.search_cursor);
                let changed = edit_field(&mut q, &mut cur, &other);
                let edits = matches!(
                    other,
                    Key::Backspace | Key::Delete | Key::Ctrl('u') | Key::Ctrl('k') | Key::Char(_)
                );
                self.search_query = q;
                self.search_cursor = cur;
                if changed && edits {
                    self.apply_search();
                    self.sync_preview_to_selection();
                }
            }
        }
    }

    fn handle_command_mode(&mut self, key: Key) {
        match key {
            Key::Enter => {
                self.mode = Mode::Normal;
                let cmd = std::mem::take(&mut self.cmd_buf);
                self.cmd_cursor = 0;
                self.execute_command(&cmd);
            }
            Key::Backspace if self.cmd_buf.is_empty() => self.mode = Mode::Normal,
            other => {
                let (mut b, mut c) = (std::mem::take(&mut self.cmd_buf), self.cmd_cursor);
                edit_field(&mut b, &mut c, &other);
                self.cmd_buf = b;
                self.cmd_cursor = c;
            }
        }
    }

    // ---- foco no painel esquerdo ------------------------------------------

    fn open_selected(&mut self, record: bool) -> bool {
        let Some(k) = self.selected_key() else { return false };
        self.free_preview();
        self.open_note(&k, record);
        true
    }

    fn toggle_filter(&mut self, level: u8, label: &str) {
        let new = if self.filter_urgency == level { 0 } else { level };
        self.filter_urgency = new;
        self.apply_search();
        let n = self.filtered.len();
        if new == level {
            self.set_status(format!("{} {} encontrada(s)", label, n));
        } else {
            self.set_status(format!("Filtro removido: exibindo todas as {} notas", n));
        }
    }

    fn handle_left_focus(&mut self, key: Key) {
        match key {
            Key::Char('j') | Key::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                    self.sync_preview_to_selection();
                }
            }
            Key::Char('k') | Key::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.sync_preview_to_selection();
                }
            }
            Key::Enter => {
                if self.open_selected(true) {
                    self.focus = Focus::Right;
                    let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
                    self.set_status(format!("Aberta em aba: {}", name));
                }
            }
            Key::Char('l') | Key::Right => {
                let prev_scroll = self.preview.as_ref().map_or(0, |p| p.scroll_y);
                if self.open_selected(true) {
                    self.with_buf(|b| b.scroll_y = prev_scroll);
                    let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
                    self.set_status(format!("Aberta em aba: {}", name));
                } else {
                    self.free_preview();
                    self.set_status("Foco no Editor (Tab para voltar às Notas)");
                }
                self.focus = Focus::Right;
            }
            Key::Char('i' | 'a') => {
                if self.open_selected(true) {
                    self.focus = Focus::Right;
                    self.mode = Mode::Insert;
                    let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
                    self.set_status(format!("Modo Inserção: {}", name));
                }
            }
            Key::Char('h') | Key::Left => {
                self.free_preview();
                self.focus = Focus::Right;
                self.set_status("Foco no Editor (Tab para voltar às Notas)");
            }
            Key::Char('c') => match self.active {
                Some(i) => self.tab_close(i),
                None => self.set_status("Nenhuma aba aberta."),
            },
            Key::Char('C') => {
                self.close_all_tabs();
            }
            Key::Char('y' | 'Y') => match self.selected_note().map(|n| n.title.clone()) {
                Some(title) => {
                    crate::clip::copy(&title);
                    self.set_status(format!("Nome copiado p/ clipboard: {} (use [[{}]])", title, title));
                }
                None => self.set_status("Nenhuma nota selecionada para copiar."),
            },
            Key::Char('d') => {
                if self.selected_note().is_some() {
                    self.mode = Mode::ConfirmDelete;
                } else {
                    self.set_status("Nenhuma nota selecionada para excluir.");
                }
            }
            Key::Char('n') => self.open_prompt(Mode::PromptNew, ""),
            Key::Char('r') => match self.selected_note().map(|n| (n.key.clone(), n.title.clone())) {
                Some((k, title)) => {
                    self.rename_target = Some(k);
                    self.open_prompt(Mode::PromptRename, &title);
                }
                None => self.set_status("Nenhuma nota selecionada para renomear."),
            },
            Key::Char('m' | '!') => {
                if let Some(&idx) = self.filtered.get(self.selected) {
                    self.toggle_attention_at(idx);
                }
            }
            Key::Char('1') => self.toggle_filter(1, "🔴 Filtrando notas com Urgência Nível 1 (Crítico - Vermelho):"),
            Key::Char('2') => self.toggle_filter(2, "🟠 Filtrando notas com Urgência Nível 2 (Médio - Laranja):"),
            Key::Char('3') => self.toggle_filter(3, "🟡 Filtrando notas com Urgência Nível 3 (Baixo - Amarelo):"),
            Key::Char('4' | 'H') => self.toggle_filter(4, "🔵 Filtrando notas com Marcação [H] (Azul):"),
            Key::Char('5') => self.toggle_filter(5, "🟣 Filtrando notas com Marcação [R] (Roxo):"),
            Key::Char('6' | 'O') => self.toggle_filter(6, "🟢 Filtrando notas com Marcação [O] (Verde):"),
            Key::Char('0') => {
                self.filter_urgency = 0;
                self.apply_search();
                let n = self.filtered.len();
                self.set_status(format!("Filtro removido: exibindo todas as {} notas", n));
            }
            Key::Char('R') => {
                self.rescan_notes();
                let n = self.notes.len();
                self.set_status(format!("Notas atualizadas ({} encontradas)", n));
            }
            Key::Char('/') => self.start_sidebar_search("Buscar notas: digite termos de pesquisa"),
            Key::Char('g') => self.pending = Pending::G,
            Key::Char('G') => {
                if !self.filtered.is_empty() {
                    self.selected = self.filtered.len() - 1;
                    self.sync_preview_to_selection();
                    let n = self.filtered.len();
                    self.set_status(format!("Última nota ({} de {})", n, n));
                }
            }
            Key::Char(':') => {
                self.cmd_buf.clear();
                self.cmd_cursor = 0;
                self.mode = Mode::Command;
            }
            _ => {}
        }
    }

    // ---- modo insercao -----------------------------------------------------

    fn shift_select(&mut self, f: impl FnOnce(&mut Buffer)) {
        self.with_buf(|b| {
            b.clamp_cursor();
            if !b.has_selection {
                b.vis_y = b.cy;
                b.vis_x = b.cx;
            }
            f(b);
            b.has_selection = (b.vis_y, b.vis_x) != (b.cy, b.cx);
        });
        self.set_status("Texto selecionado | Ctrl+C para copiar | Shift+Setas para expandir");
        self.editor_scroll();
    }

    fn drop_selection_if_any(&mut self) {
        if self.buf().is_some_and(|b| b.has_selection) {
            self.delete_visual_ex(true);
            self.with_buf(|b| b.has_selection = false);
        }
    }

    fn handle_insert(&mut self, key: Key) {
        if self.buf().is_none() {
            self.mode = Mode::Normal;
            return;
        }
        let lay = self.layout(false);
        match key {
            Key::Ctrl('c') => {
                if self.buf().is_some_and(|b| b.has_selection) {
                    self.yank_visual(false);
                    self.set_status("Texto selecionado copiado para a área de transferência! (Ctrl+C)");
                } else {
                    self.set_status("Nenhum texto selecionado para copiar (use Shift+Setas ou Mouse)");
                }
            }
            Key::Ctrl('v') => {
                if let Some(t) = crate::clip::paste() {
                    self.yank_linewise = t.contains('\n');
                    self.yank = t;
                }
                if self.buf().is_some_and(|b| b.has_selection) {
                    self.delete_visual_ex(false);
                    self.with_buf(|b| b.has_selection = false);
                }
                if self.yank.is_empty() {
                    self.set_status("Área de transferência vazia.");
                } else {
                    let t = self.yank.clone();
                    self.paste_text(&t);
                    self.set_status("Colado da área de transferência (Ctrl+V)");
                }
            }
            Key::Ctrl('x') => {
                if self.buf().is_some_and(|b| b.has_selection) {
                    self.delete_visual_ex(true);
                    self.with_buf(|b| b.has_selection = false);
                    self.set_status("Texto recortado para a área de transferência!");
                }
            }
            Key::Ctrl('a') => {
                self.with_buf(|b| {
                    b.vis_y = 0;
                    b.vis_x = 0;
                    b.cy = b.lines.len() - 1;
                    b.cx = b.lines[b.cy].len();
                    b.has_selection = true;
                });
                self.set_status("Todo o texto selecionado! (Ctrl+C para copiar)");
                self.editor_scroll();
            }
            Key::ShiftLeft => self.shift_select(|b| {
                if b.cx > 0 {
                    b.move_left_char();
                } else if b.cy > 0 {
                    b.cy -= 1;
                    b.cx = b.lines[b.cy].len();
                }
            }),
            Key::ShiftRight => self.shift_select(|b| {
                if b.cx < b.cur().len() {
                    b.move_right_char();
                } else if b.cy + 1 < b.lines.len() {
                    b.cy += 1;
                    b.cx = 0;
                }
            }),
            Key::CtrlShiftLeft => self.shift_select(|b| b.word_left()),
            Key::CtrlShiftRight => self.shift_select(|b| b.word_right()),
            Key::ShiftUp => self.shift_select(|b| layout::move_up(b, &lay)),
            Key::ShiftDown => self.shift_select(|b| layout::move_down(b, &lay)),
            Key::Enter => {
                self.last_typing = Some(Instant::now());
                self.drop_selection_if_any();
                let on_table = self.buf().is_some_and(|b| is_table_line(&b.cur().text));
                if on_table && self.table_insert_row_and_advance() {
                    return;
                }
                self.with_buf(|b| b.save_undo());
                self.edit(|b| b.split_line());
            }
            Key::Tab => {
                self.last_typing = Some(Instant::now());
                self.drop_selection_if_any();
                if self.buf().is_some_and(|b| is_table_line(&b.cur().text)) {
                    self.table_jump_next_cell(true);
                    return;
                }
                self.with_buf(|b| b.save_undo());
                self.edit(|b| b.insert_at_cursor("    "));
            }
            Key::Backspace => {
                self.last_typing = Some(Instant::now());
                if self.buf().is_some_and(|b| b.has_selection) {
                    self.drop_selection_if_any();
                    return;
                }
                let can = self.buf().is_some_and(|b| b.cx > 0 || b.cy > 0);
                if can {
                    self.with_buf(|b| b.save_undo());
                    self.edit(|b| b.backspace());
                }
            }
            Key::Delete => {
                self.last_typing = Some(Instant::now());
                if self.buf().is_some_and(|b| b.has_selection) {
                    self.drop_selection_if_any();
                    return;
                }
                let can = self
                    .buf()
                    .is_some_and(|b| b.cx < b.cur().len() || b.cy + 1 < b.lines.len());
                if can {
                    self.with_buf(|b| b.save_undo());
                    self.edit(|b| b.delete_forward());
                }
            }
            Key::Up => self.nav_insert(|b| layout::move_up(b, &lay)),
            Key::Down => self.nav_insert(|b| layout::move_down(b, &lay)),
            Key::Left => self.nav_insert(|b| {
                if b.cx > 0 {
                    b.move_left_char();
                } else if b.cy > 0 {
                    b.cy -= 1;
                    b.cx = b.lines[b.cy].len();
                }
            }),
            Key::Right => self.nav_insert(|b| {
                if b.cx < b.cur().len() {
                    b.move_right_char();
                } else if b.cy + 1 < b.lines.len() {
                    b.cy += 1;
                    b.cx = 0;
                }
            }),
            Key::Home => self.nav_insert(|b| b.cx = 0),
            Key::End => self.nav_insert(|b| b.cx = b.cur().len()),
            Key::PageUp | Key::PageDown => {
                let (n, down) = (self.visible_rows().saturating_sub(1).max(1), key == Key::PageDown);
                self.nav_insert(|b| {
                    for _ in 0..n {
                        if down {
                            layout::move_down(b, &lay);
                        } else {
                            layout::move_up(b, &lay);
                        }
                    }
                });
            }
            Key::CtrlLeft => self.nav_insert(|b| b.word_left()),
            Key::CtrlRight => self.nav_insert(|b| b.word_right()),
            Key::Char(c) if !c.is_control() => {
                self.last_typing = Some(Instant::now());
                self.drop_selection_if_any();
                let mut tmp = [0u8; 4];
                let s: &str = c.encode_utf8(&mut tmp);
                self.edit(|b| {
                    b.clamp_cursor();
                    b.save_undo_for_typing(b.cy, b.cx, s.len());
                    b.insert_at_cursor(s);
                });
            }
            _ => {}
        }
    }

    fn nav_insert(&mut self, f: impl FnOnce(&mut Buffer)) {
        self.with_buf(|b| {
            b.has_selection = false;
            b.clamp_cursor();
            f(b);
            b.clamp_cursor();
        });
    }

    // ---- modo normal e visual ---------------------------------------------

    fn handle_normal(&mut self, key: Key) {
        // Sem nenhuma aba aberta so ha criacao de nota e a linha de comando.
        if self.active.is_none() {
            match key {
                Key::Char('i' | 'I' | 'a' | 'A' | 'o' | 'O' | 'n') => {
                    self.open_prompt(Mode::PromptNew, "");
                    self.set_status("Criar nova nota:");
                }
                Key::Char(':') => {
                    self.cmd_buf.clear();
                    self.cmd_cursor = 0;
                    self.mode = Mode::Command;
                }
                Key::Char('c') => self.set_status("Nenhuma aba aberta."),
                _ => {}
            }
            return;
        }
        let lay = self.layout(false);

        // Shift+Setas no modo Normal iniciam a selecao visual.
        if self.mode == Mode::Normal
            && matches!(key, Key::ShiftLeft | Key::ShiftRight | Key::ShiftUp | Key::ShiftDown)
        {
            self.with_buf(|b| {
                b.clamp_cursor();
                b.vis_y = b.cy;
                b.vis_x = b.cx;
                match key {
                    Key::ShiftLeft => b.move_left_char(),
                    Key::ShiftRight => b.move_right_char(),
                    Key::ShiftUp => layout::move_up(b, &lay),
                    _ => layout::move_down(b, &lay),
                }
            });
            self.mode = Mode::VisualChar;
            self.set_status("-- VISUAL (Shift+Setas) --");
            return;
        }
        if key == Key::Ctrl('c') && self.mode.is_visual() {
            self.yank_visual(self.mode == Mode::VisualLine);
            self.mode = Mode::Normal;
            self.set_status("Seleção copiada para a área de transferência! (Ctrl+C)");
            return;
        }
        if key == Key::Ctrl('n') {
            self.jump_to_next_link();
            return;
        }

        let has_sel = self.mode.is_visual() || self.buf().is_some_and(|b| b.has_selection);
        match key {
            Key::Char('g') => self.pending = Pending::G,
            Key::Char('G') => {
                self.with_buf(|b| {
                    b.cy = b.lines.len() - 1;
                    while b.cy > 0 && b.lines[b.cy].hidden {
                        b.cy -= 1;
                    }
                    b.cx = 0;
                });
                self.editor_scroll();
                self.clear_needed = true;
                let y = self.buf().map_or(1, |b| b.cy + 1);
                self.set_status(format!("Fim da nota (linha {})", y));
            }
            Key::Backspace => self.navigate_history(1),
            Key::CtrlBackspace => self.navigate_history(3),
            Key::Char('h') | Key::Left => self.normal_left(),
            Key::Char('l') | Key::Right => self.normal_right(),
            Key::Char('j') | Key::Down => {
                self.with_buf(|b| layout::move_down(b, &lay));
            }
            Key::Char('k') | Key::Up => {
                self.with_buf(|b| layout::move_up(b, &lay));
            }
            Key::PageUp | Key::PageDown => {
                let (n, down) = (self.visible_rows().saturating_sub(1).max(1), key == Key::PageDown);
                self.with_buf(|b| {
                    for _ in 0..n {
                        if down {
                            layout::move_down(b, &lay);
                        } else {
                            layout::move_up(b, &lay);
                        }
                    }
                });
            }
            Key::Char('c') if self.mode == Mode::Normal => {
                if let Some(i) = self.active {
                    self.tab_close(i);
                }
            }
            Key::Char('C') if self.mode == Mode::Normal => {
                self.close_all_tabs();
            }
            Key::Char(' ') if self.mode == Mode::Normal => {
                self.with_buf(|b| b.save_undo());
                self.edit(|b| b.insert_at_cursor(" "));
                self.set_status("Espaço inserido");
            }
            Key::Char('w') => {
                self.with_buf(|b| {
                    b.clamp_cursor();
                    let t = b.cur().text.as_bytes();
                    let mut x = b.cx;
                    while x < t.len() && !is_space(t[x]) {
                        x += 1;
                    }
                    while x < t.len() && is_space(t[x]) {
                        x += 1;
                    }
                    b.cx = x;
                    if b.cx >= b.cur().len() && b.cy + 1 < b.lines.len() {
                        b.cy += 1;
                        b.cx = 0;
                    }
                });
            }
            Key::Char('b') => {
                self.with_buf(|b| {
                    b.clamp_cursor();
                    if b.cx > 0 {
                        let t = b.cur().text.as_bytes();
                        let mut x = b.cx;
                        while x > 0 && is_space(t[x - 1]) {
                            x -= 1;
                        }
                        while x > 0 && !is_space(t[x - 1]) {
                            x -= 1;
                        }
                        b.cx = x;
                    } else if b.cy > 0 {
                        b.cy -= 1;
                        b.cx = b.cur().len();
                    }
                    b.clamp_cursor();
                });
            }
            Key::Char('0') | Key::Home => {
                self.with_buf(|b| b.cx = 0);
            }
            Key::Char('$') | Key::End => {
                // Como no Vim: ultimo caractere da linha (e nao depois dele).
                self.with_buf(|b| {
                    let t = &b.cur().text;
                    b.cx = if t.is_empty() { 0 } else { prev_char_start(t, t.len()) };
                });
            }
            Key::Char('i') => self.begin_insert(|_| {}),
            Key::Char('I') => self.begin_insert(|b| {
                let t = b.cur().text.as_bytes();
                let mut x = 0;
                while x < t.len() && is_space(t[x]) {
                    x += 1;
                }
                b.cx = x;
            }),
            Key::Char('a') => self.begin_insert(|b| {
                if b.cx < b.cur().len() {
                    b.move_right_char();
                }
            }),
            Key::Char('A') => self.begin_insert(|b| b.cx = b.cur().len()),
            Key::Char('o') => {
                self.with_buf(|b| b.save_undo());
                self.edit(|b| {
                    b.insert_line(b.cy + 1, "");
                    b.cy += 1;
                    b.cx = 0;
                });
                self.mode = Mode::Insert;
                self.set_status("-- MODO INSERÇÃO --");
            }
            Key::Char('O') => {
                self.with_buf(|b| b.save_undo());
                self.edit(|b| {
                    b.insert_line(b.cy, "");
                    b.cx = 0;
                });
                self.mode = Mode::Insert;
                self.set_status("-- MODO INSERÇÃO --");
            }
            Key::Char('v') => match self.mode {
                Mode::VisualChar => self.enter_mode_normal("-- MODO NORMAL --"),
                Mode::VisualLine => {
                    self.mode = Mode::VisualChar;
                    self.set_status("-- VISUAL --");
                }
                _ => {
                    self.with_buf(|b| {
                        b.vis_y = b.cy;
                        b.vis_x = b.cx;
                    });
                    self.mode = Mode::VisualChar;
                    self.set_status("-- VISUAL --");
                }
            },
            Key::Char('V') => match self.mode {
                Mode::VisualLine => self.enter_mode_normal("-- MODO NORMAL --"),
                Mode::VisualChar => {
                    self.mode = Mode::VisualLine;
                    self.set_status("-- VISUAL LINHA --");
                }
                _ => {
                    self.with_buf(|b| {
                        b.vis_y = b.cy;
                        b.vis_x = 0;
                    });
                    self.mode = Mode::VisualLine;
                    self.set_status("-- VISUAL LINHA --");
                }
            },
            Key::Char('r') => {
                if self.mode.is_visual() {
                    self.pending = Pending::ReplaceVisual;
                    self.set_status("-- REPLACE --");
                } else if self.buf().is_some_and(|b| b.cx < b.cur().len()) {
                    self.pending = Pending::ReplaceChar;
                    self.set_status("-- REPLACE --");
                }
            }
            Key::Char('m' | '!') => {
                if let Some(k) = self.cur_key().cloned() {
                    if let Some(idx) = self.notes.iter().position(|n| n.key == k) {
                        self.toggle_attention_at(idx);
                    }
                }
            }
            Key::Char('x') => {
                if has_sel {
                    self.delete_visual_ex(true);
                    self.mode = Mode::Normal;
                    self.with_buf(|b| b.has_selection = false);
                    self.set_status("Texto selecionado recortado para a área de transferência! (x)");
                } else {
                    self.cut_char();
                }
            }
            Key::Char('d') => {
                if self.mode.is_visual() {
                    self.delete_visual_ex(true);
                } else {
                    self.pending = Pending::D;
                }
            }
            Key::Char('y' | 'Y') => {
                if has_sel {
                    self.yank_visual(self.mode == Mode::VisualLine);
                    self.mode = Mode::Normal;
                    self.with_buf(|b| b.has_selection = false);
                    self.set_status("Texto selecionado copiado para a área de transferência! (y)");
                } else if let Some(t) = self.buf().map(|b| b.cur().text.clone()) {
                    self.set_yank(t, true);
                    self.set_status("1 linha copiada para a área de transferência (y)");
                }
            }
            Key::Char('p' | 'P') => {
                if has_sel {
                    self.paste_replace_selection();
                } else {
                    self.paste_normal();
                }
            }
            Key::Char('u') => match self.with_buf(|b| b.apply_undo()) {
                Some(UndoResult::Done) => {
                    self.mark_modified();
                    self.set_status("Desfazer (u)");
                }
                _ => self.set_status("Já está na alteração mais antiga"),
            },
            Key::Ctrl('r') => match self.with_buf(|b| b.apply_redo()) {
                Some(UndoResult::Done) => {
                    self.mark_modified();
                    self.set_status("Refazer (Ctrl+R)");
                }
                _ => self.set_status("Já está na alteração mais recente"),
            },
            Key::Char('z') => self.pending = Pending::Z,
            Key::Char('n') => self.search_next(),
            Key::Enter => self.activate_link(),
            Key::Char(':') => {
                self.cmd_buf.clear();
                self.cmd_cursor = 0;
                self.mode = Mode::Command;
            }
            Key::Char('/') => {
                self.cmd_buf = "/".to_string();
                self.cmd_cursor = 1;
                self.mode = Mode::Command;
            }
            _ => {}
        }
    }

    fn begin_insert(&mut self, f: impl FnOnce(&mut Buffer)) {
        self.with_buf(|b| {
            b.clamp_cursor();
            f(b);
        });
        self.mode = Mode::Insert;
        self.set_status("-- MODO INSERÇÃO --");
    }

    fn cut_char(&mut self) {
        let Some(b) = self.buf() else { return };
        let t = &b.cur().text;
        if b.cx >= t.len() {
            return;
        }
        let end = next_char_end(t, b.cx);
        let ch = t[b.cx..end].to_string();
        self.set_yank(ch, false);
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let (cy, cx) = (b.cy, b.cx);
            let end = next_char_end(&b.lines[cy].text, cx);
            b.lines[cy].delete_range(cx, end);
            let len = b.lines[cy].len();
            if b.cx >= len && b.cx > 0 {
                b.cx = prev_char_start(&b.lines[cy].text, b.cx);
            }
        });
        self.set_status("Caractere recortado para a área de transferência (x)");
    }

    /// `h`: esquerda; pula os delimitadores ocultos de negrito/italico.
    fn normal_left(&mut self) {
        let in_normal = self.mode == Mode::Normal;
        let Some(b) = self.buf_mut() else { return };
        b.clamp_cursor();
        if b.cx > 0 {
            b.cx = prev_char_start(&b.cur().text, b.cx);
            if !b.cur().fence {
                for s in find_styles(&b.cur().text) {
                    if b.cx >= s.open_start && b.cx < s.content_start {
                        b.cx = if s.open_start > 0 {
                            prev_char_start(&b.cur().text, s.open_start)
                        } else {
                            0
                        };
                        break;
                    }
                    if b.cx >= s.close_start && b.cx < s.close_start + s.close_len {
                        b.cx = if s.content_end > s.content_start {
                            prev_char_start(&b.cur().text, s.content_end)
                        } else {
                            s.open_start
                        };
                        break;
                    }
                }
            }
        } else if in_normal {
            self.focus = Focus::Left;
            self.sync_preview_to_selection();
            self.set_status("Foco na lista de notas (use j/k para navegar)");
        }
    }

    /// `l`: direita; pula os delimitadores ocultos de negrito/italico.
    fn normal_right(&mut self) {
        let Some(b) = self.buf_mut() else { return };
        b.clamp_cursor();
        if b.cx < b.cur().len() {
            b.cx = next_char_end(&b.cur().text, b.cx);
            if !b.cur().fence {
                for s in find_styles(&b.cur().text) {
                    if b.cx >= s.open_start && b.cx < s.content_start {
                        b.cx = s.content_start;
                        break;
                    }
                    if b.cx >= s.close_start && b.cx < s.close_start + s.close_len {
                        b.cx = s.close_start + s.close_len;
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_editing_is_utf8_safe() {
        let (mut s, mut c) = (String::new(), 0usize);
        for ch in "aç😀b".chars() {
            edit_field(&mut s, &mut c, &Key::Char(ch));
        }
        assert_eq!(s, "aç😀b");
        edit_field(&mut s, &mut c, &Key::Left);
        edit_field(&mut s, &mut c, &Key::Backspace);
        assert_eq!(s, "açb");
        edit_field(&mut s, &mut c, &Key::Home);
        edit_field(&mut s, &mut c, &Key::Delete);
        assert_eq!(s, "çb");
        edit_field(&mut s, &mut c, &Key::Ctrl('k'));
        assert_eq!(s, "");
    }

    #[test]
    fn field_respects_max_length() {
        let (mut s, mut c) = ("x".repeat(FIELD_MAX), FIELD_MAX);
        edit_field(&mut s, &mut c, &Key::Char('y'));
        assert_eq!(s.len(), FIELD_MAX);
    }

    #[test]
    fn word_jumps_in_fields() {
        let (mut s, mut c) = ("um dois  tres".to_string(), 13usize);
        edit_field(&mut s, &mut c, &Key::CtrlLeft);
        assert_eq!(c, 9);
        edit_field(&mut s, &mut c, &Key::CtrlLeft);
        assert_eq!(c, 3);
        edit_field(&mut s, &mut c, &Key::CtrlRight);
        assert_eq!(c, 9);
    }

    #[test]
    fn key_mapping() {
        let k = |code, m| map_key(KeyEvent::new(code, m));
        assert_eq!(k(KeyCode::Char('s'), KeyModifiers::CONTROL), Some(Key::Ctrl('s')));
        assert_eq!(k(KeyCode::Char('A'), KeyModifiers::SHIFT), Some(Key::Char('A')));
        assert_eq!(k(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::SHIFT), Some(Key::CtrlShiftLeft));
        assert_eq!(k(KeyCode::Up, KeyModifiers::SHIFT), Some(Key::ShiftUp));
        assert_eq!(k(KeyCode::Char('x'), KeyModifiers::ALT), None);
        // AltGr (Ctrl+Alt) produz caracteres imprimiveis em varios teclados.
        assert_eq!(
            k(KeyCode::Char('@'), KeyModifiers::CONTROL | KeyModifiers::ALT),
            Some(Key::Char('@'))
        );
        assert_eq!(k(KeyCode::Backspace, KeyModifiers::CONTROL), Some(Key::CtrlBackspace));
    }
}
