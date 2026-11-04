//! Mouse: cliques nos botoes, abas, lista de notas, selecao por cliques/arrasto e
//! rolagem. As coordenadas seguem a convencao do terminal (1-based).

use crate::app::{App, Focus, Mode};
use crate::layout::{self, click_offset, find_visual_row, table_view};
use crate::render::prompt_label;
use crate::tbl::{self, SubKind};
use crate::text::{prev_char_start, str_width, width_len};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

/// Botao no vocabulario do protocolo SGR do terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Btn {
    Left,
    Middle,
    Right,
    Drag,
    WheelUp,
    WheelDown,
}

#[derive(Clone, Copy, Debug)]
pub struct Mouse {
    pub btn: Btn,
    pub col: i32,
    pub row: i32,
    pub released: bool,
}

/// Sequencia de cliques no texto; conserva o ponto original mesmo se a selecao
/// mudar a exibicao do Markdown ao sair do modo Insercao.
pub struct EditorClick {
    pub at: Instant,
    col: i32,
    row: i32,
    cursor: (usize, usize),
    count: u8,
}

/// Letras Unicode, numeros e sublinhados formam palavras. Espacos e sinais
/// formam grupos separados, sem incluir a palavra vizinha na selecao.
fn word_range(text: &str, at: usize) -> Option<(usize, usize)> {
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let kind = class(text.get(at..)?.chars().next()?);
    let start = text[..at].char_indices().rev()
        .take_while(|&(_, c)| class(c) == kind)
        .last().map_or(at, |(i, _)| i);
    let end = text[at..].char_indices()
        .take_while(|&(_, c)| class(c) == kind)
        .last().map(|(i, c)| at + i + c.len_utf8())?;
    Some((start, prev_char_start(text, end)))
}

pub fn map_mouse(ev: MouseEvent) -> Option<Mouse> {
    let (btn, released) = match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => (Btn::Left, false),
        MouseEventKind::Up(MouseButton::Left) => (Btn::Left, true),
        MouseEventKind::Down(MouseButton::Right) => (Btn::Right, false),
        MouseEventKind::Drag(MouseButton::Left) => (Btn::Drag, false),
        MouseEventKind::Down(MouseButton::Middle) => (Btn::Middle, false),
        MouseEventKind::ScrollUp => (Btn::WheelUp, false),
        MouseEventKind::ScrollDown => (Btn::WheelDown, false),
        _ => return None,
    };
    Some(Mouse {
        btn,
        col: i32::from(ev.column) + 1,
        row: i32::from(ev.row) + 1,
        released,
    })
}

fn within(col: i32, range: (i32, i32)) -> bool {
    col >= range.0 && col < range.1
}

impl App {
    pub fn on_mouse(&mut self, m: Mouse) {
        self.count = 0;
        self.count2 = 0;
        self.last_activity = Instant::now();
        self.needs_redraw = true;
        let now = Instant::now();
        if m.btn != Btn::Left {
            self.last_editor_click = None;
        }

        // 1. Prompts modais: so o clique na propria linha do prompt vale.
        if self.mode.is_prompt() {
            self.last_editor_click = None;
            if m.btn == Btn::Left && !m.released && m.row == self.rows {
                let label_w = prompt_label(self).map_or(0, |l| str_width(&l)) as i32;
                let clicked = (m.col - label_w - 1).max(0) as usize;
                let buf = &self.prompt_buf;
                let mut byte = 0;
                let mut w = 0;
                for (i, c) in buf.char_indices() {
                    let cw = width_len(buf, i + c.len_utf8()) - width_len(buf, i);
                    if w + cw > clicked {
                        break;
                    }
                    w += cw;
                    byte = i + c.len_utf8();
                }
                self.prompt_cursor = byte;
            }
            return;
        }

        // 2. Menu de contexto da tabela: as opcoes ficam na linha de comando.
        if self.table_menu.is_some() && !m.released {
            match m.btn {
                Btn::Left => {
                    let asc = self.hit.menu_asc;
                    let desc = self.hit.menu_desc;
                    if m.row == self.rows && within(m.col, asc) {
                        self.apply_table_menu();
                        return;
                    }
                    if m.row == self.rows && within(m.col, desc) {
                        if let Some(menu) = self.table_menu.as_mut() {
                            menu.selected = 1;
                        }
                        self.apply_table_menu();
                        return;
                    }
                    // Clique fora do menu: fecha e deixa o clique seguir seu curso.
                    self.table_menu = None;
                }
                Btn::WheelUp | Btn::WheelDown => {
                    self.table_menu = None;
                }
                Btn::Middle | Btn::Drag => {
                    self.table_menu = None;
                }
                _ => {}
            }
        }

        // 3. Rejeicao de toque acidental no touchpad durante a digitacao.
        if self.mode == Mode::Insert && m.btn == Btn::Left {
            if let Some(t) = self.last_typing {
                if now.duration_since(t) < Duration::from_millis(150) {
                    self.last_editor_click = None;
                    return;
                }
            }
        }

        match m.btn {
            Btn::WheelUp | Btn::WheelDown => self.on_wheel(m),
            Btn::Right => self.on_right_click(m),
            Btn::Middle => {
                if !m.released && m.row == 1 && m.col > self.left_width + 1 {
                    if let Some(&(i, ..)) = self.hit.tabs.iter().find(|t| m.col >= t.1 && m.col <= t.2) {
                        self.tab_close(i);
                    }
                }
            }
            Btn::Left if !m.released => self.on_left_click(m, now),
            Btn::Drag => self.on_drag(m),
            Btn::Left => self.on_release(),
        }
    }

    fn on_wheel(&mut self, m: Mouse) {
        let up = m.btn == Btn::WheelUp;
        if m.col <= self.left_width {
            if up {
                self.sidebar_scroll = self.sidebar_scroll.saturating_sub(1);
            } else if self.sidebar_scroll + self.sidebar_visible() < self.filtered.len() {
                self.sidebar_scroll += 1;
            }
            return;
        }
        if self.in_preview() {
            if let Some(p) = &mut self.preview {
                if up {
                    p.scroll_y = p.scroll_y.saturating_sub(1);
                } else if p.scroll_y + 1 < p.lines.len() {
                    p.scroll_y += 1;
                }
            }
        } else if self.focus == Focus::Right {
            // A roda anda com o cursor, como `j`/`k`, em vez de rolar a tela
            // apenas ate o cursor encostar no topo. Assim a nota inteira pode
            // ser percorrida e a visualizacao acompanha o cursor.
            let lay = self.layout(false);
            self.with_buf(|b| {
                if up {
                    layout::move_up(b, &lay);
                } else {
                    layout::move_down(b, &lay);
                }
            });
            self.editor_scroll();
        }
    }

    /// Botao direito sobre uma celula de tabela: abre o menu A→Z / Z→A daquela
    /// coluna. Fora de uma tabela, fecha o menu que estiver aberto.
    fn on_right_click(&mut self, m: Mouse) {
        match self.table_target_at(m) {
            Some((line, column)) => self.open_table_menu(line, column),
            None => {
                if self.table_menu.take().is_some() {
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// (linha do documento, coluna logica da tabela) sob o ponteiro, se o
    /// clique caiu numa celula desenhada como grade.
    fn table_target_at(&self, m: Mouse) -> Option<(usize, usize)> {
        if self.focus != Focus::Right
            || !matches!(self.mode, Mode::Normal | Mode::VisualChar | Mode::VisualLine)
            || m.row < 2
            || m.row > self.rows - 3
        {
            return None;
        }
        let content_start = self.left_width + 1 + self.gutter_width(false) as i32 + 1;
        if m.col < content_start {
            return None;
        }
        let lay = self.layout(false);
        let b = self.buf()?;
        let screen_row = (m.row - 2) as usize;
        let target = screen_row + b.scroll_y;
        let vr = find_visual_row(&b.lines, lay, target)?;
        let line = &b.lines[vr.line];
        let tv = table_view(line, &lay)?;
        if !matches!(tv.sub_kind(vr.sub), SubKind::Content(_)) {
            return None;
        }
        let col = (m.col - content_start) as usize;
        if col >= tv.geo.total {
            return None;
        }
        let byte = tbl::offset_at(&tv, vr.sub, col).min(line.len());
        let row = &tv.parsed().rows[tv.row];
        if row.delim || row.cells.is_empty() {
            return None;
        }
        let column = row
            .cells
            .iter()
            .position(|cell| byte <= cell.zone_end as usize)
            .unwrap_or(row.cells.len() - 1);
        Some((vr.line, column))
    }

    fn on_left_click(&mut self, m: Mouse, now: Instant) {
        // Qualquer clique fora do texto interrompe a sequencia do editor.
        let previous = self.last_editor_click.take();
        // Confirmacao de exclusao: clique em "s" ou "n" no prompt.
        if self.mode == Mode::ConfirmDelete {
            if m.row == self.rows {
                let title = self.selected_note().map_or("esta nota".to_string(), |n| n.title.clone());
                let plen = str_width(&format!(" Deseja realmente excluir a nota '{}'? (s/n): ", title)) as i32;
                if m.col >= plen - 7 && m.col <= plen - 5 {
                    self.mode = Mode::Normal;
                    self.delete_selected_note();
                    return;
                }
                if m.col >= plen - 4 && m.col <= plen - 2 {
                    self.mode = Mode::Normal;
                    self.set_status("Exclusão cancelada.");
                    return;
                }
            }
            self.mode = Mode::Normal;
            self.set_status("Exclusão cancelada.");
            return;
        }

        // Barra de botoes (penultima linha).
        if m.row == self.rows - 1 {
            let debounced = self
                .last_theme_click
                .map_or(true, |t| now.duration_since(t) > Duration::from_millis(200));
            if debounced {
                self.last_theme_click = Some(now);
                self.click_bottom_button(m.col);
            }
            return;
        }
        // Linhas de status e de comando: sem acoes de clique.
        if m.row > self.rows - 3 {
            return;
        }

        // Barra de abas.
        if m.row == 1 && m.col > self.left_width + 1 {
            if self.tabs.is_empty() {
                if m.col >= self.hit.tab_new.0 && m.col <= self.hit.tab_new.1 {
                    self.prompt_buf.clear();
                    self.prompt_cursor = 0;
                    self.mode = Mode::PromptNew;
                    self.set_status("Criar nova nota:");
                }
            } else if let Some(&(i, _, _, close)) =
                self.hit.tabs.iter().find(|t| m.col >= t.1 && m.col <= t.2)
            {
                if m.col >= close {
                    self.tab_close(i);
                } else {
                    self.tab_switch(i);
                    self.free_preview();
                    if self.mode == Mode::SearchLeft {
                        self.mode = Mode::Normal;
                        self.set_status("-- MODO NORMAL --");
                    }
                    self.focus = Focus::Right;
                }
            }
            return;
        }

        // Botoes de filtro de urgencia (ultima linha do painel esquerdo).
        if m.row == self.rows - 3 {
            for level in [1u8, 2, 3, 4, 5, 6, 0] {
                if within(m.col, self.hit.filter[usize::from(level)]) {
                    self.click_filter(level);
                    return;
                }
            }
            return;
        }

        if m.col <= self.left_width {
            self.click_left_panel(m, now);
        } else {
            self.click_editor(m, now, previous);
        }
    }

    /// Botoes da barra inferior. `true` se algum foi acionado.
    fn click_bottom_button(&mut self, col: i32) -> bool {
        let h = self.hit.clone();
        if within(col, h.btn_new) {
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.mode = Mode::PromptNew;
        } else if within(col, h.btn_rename) {
            let target = if self.focus == Focus::Left {
                self.selected_key().or_else(|| self.cur_key().cloned())
            } else {
                self.cur_key().cloned()
            };
            match target {
                Some(k) => {
                    let name = self
                        .notes
                        .iter()
                        .find(|n| n.key == k)
                        .map(|n| n.title.clone())
                        .or_else(|| self.tab().map(|t| t.title.clone()))
                        .unwrap_or_default();
                    self.rename_target = Some(k);
                    self.prompt_buf = name;
                    self.prompt_cursor = self.prompt_buf.len();
                    self.mode = Mode::PromptRename;
                }
                None => self.set_status("Nenhuma nota aberta para renomear."),
            }
        } else if within(col, h.btn_link) {
            self.copy_note_link();
        } else if within(col, h.btn_delete) {
            if self.selected_note().is_some() {
                self.mode = Mode::ConfirmDelete;
            } else {
                self.set_status("Nenhuma nota selecionada para excluir.");
            }
        } else if within(col, h.btn_wrap) {
            self.wrap = !self.wrap;
            self.clear_needed = true;
            let on = self.wrap;
            self.set_status(format!("Quebra de linha (wrap): {}", if on { "ATIVADO" } else { "DESATIVADO" }));
        } else if within(col, h.btn_find) {
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.mode = Mode::PromptFind;
            self.set_status("Localizar na nota (digite o termo e pressione Enter):");
        } else if within(col, h.btn_replace) {
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.mode = Mode::PromptReplaceFind;
            self.set_status("Substituir: digite o termo a ser localizado e pressione Enter");
        } else if within(col, h.btn_theme) {
            self.cycle_theme();
        } else if within(col, h.btn_life) {
            self.run_life();
            self.set_status("Simulação do Game of Life encerrada.");
        } else if within(col, h.btn_import) {
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.mode = Mode::PromptImport;
            self.set_status(" ⬇ Importar de: informe a pasta com os arquivos .md");
        } else if within(col, h.btn_export) {
            self.prompt_buf.clear();
            self.prompt_cursor = 0;
            self.mode = Mode::PromptExport;
            self.set_status(" ⬆ Exportar para: informe a pasta de destino");
        } else {
            return false;
        }
        true
    }

    fn click_filter(&mut self, level: u8) {
        let target = if level != 0 && self.filter_urgency == level { 0 } else { level };
        self.filter_urgency = target;
        self.apply_search();
        let n = self.filtered.len();
        let msg = match (target, level) {
            (0, 0) => format!("Filtro removido: exibindo todas as {} notas", n),
            (0, _) => format!("Filtro de urgência removido (Exibindo todas as {} notas)", n),
            (1, _) => format!("🔴 Filtrando notas com Urgência Nível 1 (Crítico - Vermelho): {} encontrada(s)", n),
            (2, _) => format!("🟠 Filtrando notas com Urgência Nível 2 (Médio - Laranja): {} encontrada(s)", n),
            (3, _) => format!("🟡 Filtrando notas com Urgência Nível 3 (Baixo - Amarelo): {} encontrada(s)", n),
            (4, _) => format!("🔵 Filtrando notas com Marcação [H] (Azul): {} encontrada(s)", n),
            (5, _) => format!("🟣 Filtrando notas com Marcação [R] (Roxo): {} encontrada(s)", n),
            _ => format!("🟢 Filtrando notas com Marcação [O] (Verde): {} encontrada(s)", n),
        };
        self.set_status(msg);
    }

    fn click_left_panel(&mut self, m: Mouse, now: Instant) {
        self.with_buf_clear_selection();
        if self.mode == Mode::Insert {
            self.mode = Mode::Normal;
            self.set_status("-- MODO NORMAL --");
        }
        self.focus = Focus::Left;
        if m.row <= 3 {
            if m.row == 2 {
                self.mode = Mode::SearchLeft;
                self.search_query.clear();
                self.search_cursor = 0;
                self.apply_search();
                self.sync_preview_to_selection();
                self.set_status("Busca de notas (digite o termo, Enter para abrir)");
            }
            return;
        }
        let clicked = (m.row - 4).max(0) as usize + self.sidebar_scroll;
        let Some(&actual) = self.filtered.get(clicked) else { return };
        self.selected = clicked;
        if (1..=5).contains(&m.col) {
            self.last_note_click = None;
            self.toggle_attention_at(actual);
            return;
        }
        let double = self
            .last_note_click
            .is_some_and(|(i, t)| i == clicked && now.duration_since(t) < Duration::from_millis(500));
        let key = self.notes[actual].key.clone();
        if double {
            self.last_note_click = None;
            self.free_preview();
            self.open_note(&key, true);
            if self.mode == Mode::SearchLeft {
                self.mode = Mode::Normal;
            }
            self.focus = Focus::Right;
            let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
            self.set_status(format!("Aberta em aba: {}", name));
        } else {
            self.last_note_click = Some((clicked, now));
            self.load_preview(&key);
            let title = self.notes[actual].title.clone();
            self.set_status(format!("Prévia: {} (Enter ou duplo clique para abrir)", title));
        }
    }

    fn with_buf_clear_selection(&mut self) {
        if let Some(b) = self.buf_mut() {
            b.has_selection = false;
        }
    }

    /// Se a previa esta na tela, abre a nota em aba mantendo a rolagem.
    fn promote_preview(&mut self) {
        let key = self.preview.as_ref().map(|p| p.key.clone()).or_else(|| {
            if self.focus == Focus::Left {
                self.selected_key()
            } else {
                None
            }
        });
        if let Some(key) = key {
            if self.focus == Focus::Left || self.mode == Mode::SearchLeft {
                let prev_scroll = self.preview.as_ref().map_or(0, |p| p.scroll_y);
                self.free_preview();
                self.open_note(&key, true);
                if let Some(b) = self.buf_mut() {
                    b.scroll_y = prev_scroll;
                }
                if self.mode == Mode::SearchLeft {
                    self.mode = Mode::Normal;
                }
                let name = self.tab().map(|t| t.filename.clone()).unwrap_or_default();
                self.set_status(format!("Aberta em aba: {}", name));
            }
        }
    }

    /// Posiciona o cursor do texto na celula clicada.
    fn place_cursor_at(&mut self, m: Mouse) -> bool {
        let lay = self.layout(false);
        let col = (m.col - (self.left_width + 1 + self.gutter_width(false) as i32 + 1)).max(0) as usize;
        let screen_row = (m.row - 2).max(0) as usize;
        let Some(b) = self.buf_mut() else { return false };
        let target = screen_row + b.scroll_y;
        if let Some(vr) = find_visual_row(&b.lines, lay, target) {
            b.cy = vr.line;
            b.cx = click_offset(&b.lines[vr.line], vr.sub, vr.seg, col, &lay).min(b.lines[vr.line].len());
            b.clamp_cursor();
            return true;
        }
        // Clique abaixo da ultima linha: vai para o ultimo caractere da ultima
        // linha visivel, em vez de nao mover o cursor.
        let last = b.lines.iter().rposition(|l| !l.hidden).unwrap_or(0);
        b.cy = last;
        let len = b.lines[last].len();
        b.cx = if len == 0 { 0 } else { prev_char_start(&b.lines[last].text, len) };
        b.clamp_cursor();
        true
    }

    fn click_editor(&mut self, m: Mouse, now: Instant, previous: Option<EditorClick>) {
        self.promote_preview();
        self.focus = Focus::Right;
        if m.row > self.rows - 3 {
            return;
        }
        let previous = previous.filter(|p| {
            p.col == m.col && p.row == m.row && p.count < 3
                && now.duration_since(p.at) < Duration::from_millis(500)
        });
        let count = previous.as_ref().map_or(1, |p| p.count + 1);
        if let Some(p) = previous {
            if let Some(b) = self.buf_mut() {
                (b.cy, b.cx) = p.cursor;
                b.clamp_cursor();
            }
        } else if !self.place_cursor_at(m) {
            return;
        }
        let Some(b) = self.buf() else { return };
        self.last_editor_click = Some(EditorClick {
            at: now, col: m.col, row: m.row, cursor: (b.cy, b.cx), count,
        });
        if let Some(b) = self.buf_mut() {
            b.vis_y = b.cy;
            b.vis_x = b.cx;
            b.has_selection = false;
        }
        if self.mode.is_visual() {
            self.mode = Mode::Normal;
            self.set_status("-- MODO NORMAL --");
        }
        if count == 2 {
            let range = self.buf().and_then(|b| word_range(&b.cur().text, b.cx));
            if let Some((start, end)) = range {
                if let Some(b) = self.buf_mut() {
                    b.vis_x = start;
                    b.cx = end;
                }
                self.mode = Mode::VisualChar;
                self.set_status("-- VISUAL (Palavra selecionada) --");
            }
        } else if count == 3 {
            if let Some(b) = self.buf_mut() {
                b.vis_x = 0;
                b.cx = prev_char_start(&b.cur().text, b.cur().len());
            }
            self.mode = Mode::VisualLine;
            self.set_status("-- VISUAL LINHA (Linha selecionada) --");
        }
    }

    fn on_drag(&mut self, m: Mouse) {
        if !matches!(self.mode, Mode::Normal | Mode::VisualChar | Mode::VisualLine | Mode::Insert) {
            return;
        }
        if m.col > self.left_width + 1 && m.row >= 2 && m.row <= self.rows - 3 {
            self.promote_preview();
            self.focus = Focus::Right;
            self.place_cursor_at(m);
            if self.mode == Mode::Insert {
                if let Some(b) = self.buf_mut() {
                    if (b.vis_y, b.vis_x) != (b.cy, b.cx) {
                        b.has_selection = true;
                    }
                }
                self.set_status("-- INSERÇÃO (Texto selecionado com mouse) --");
            } else {
                if self.mode != Mode::VisualChar {
                    self.mode = Mode::VisualChar;
                }
                self.set_status("-- VISUAL (Seleção com Mouse) --");
            }
        }
    }

    fn on_release(&mut self) {
        let Some(b) = self.buf() else { return };
        let moved = (b.vis_y, b.vis_x) != (b.cy, b.cx);
        if self.mode == Mode::VisualChar && moved {
            self.set_status("Texto selecionado! (y: copiar | p: colar/substituir | x: recortar | Esc: cancelar)");
        } else if self.mode == Mode::Insert && b.has_selection && moved {
            self.set_status("Texto selecionado! (Ctrl+V: colar e substituir | Ctrl+C: copiar | Esc: cancelar)");
        }
    }
}
