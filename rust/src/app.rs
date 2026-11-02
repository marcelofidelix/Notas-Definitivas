//! Estado global da aplicacao: abas, catalogo de notas, painel lateral,
//! operacoes sobre notas (abrir, salvar, criar, renomear, excluir) e filtros.

use crate::db::{now_secs, Db};
use crate::doc::{Buffer, Line};
use crate::layout::{Hide, Layout, VRowIter};
use crate::notes::{
    build_search_norm, compare_notes, matches_filter, query_words, title_from_filename, NoteInfo, NoteKey,
};
use crate::text::sanitize_note_name;
use crate::theme::{NUM_THEMES, THEMES};
use crate::{clip, fsio};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const DEFAULT_LEFT_WIDTH: i32 = 40;
pub const MIN_LEFT_WIDTH: i32 = 18;
pub const MAX_HISTORY: usize = 64;
pub const AUTOSAVE_DELAY: Duration = Duration::from_secs(60);
pub const IDLE_LIFE: Duration = Duration::from_secs(600);
pub const IDLE_HEARTBEAT: Duration = Duration::from_secs(30);
pub const STATUS_TTL: Duration = Duration::from_secs(5);
/// Tamanho maximo de campos de uma linha (busca, comando, prompts).
pub const FIELD_MAX: usize = 255;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Insert,
    VisualChar,
    VisualLine,
    Command,
    SearchLeft,
    ConfirmDelete,
    PromptNew,
    PromptRename,
    PromptFind,
    PromptReplaceFind,
    PromptReplaceWith,
    PromptImport,
    PromptExport,
}

impl Mode {
    pub fn is_prompt(self) -> bool {
        matches!(
            self,
            Mode::PromptNew
                | Mode::PromptRename
                | Mode::PromptFind
                | Mode::PromptReplaceFind
                | Mode::PromptReplaceWith
                | Mode::PromptImport
                | Mode::PromptExport
        )
    }
    pub fn is_visual(self) -> bool {
        matches!(self, Mode::VisualChar | Mode::VisualLine)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Left,
    Right,
}

/// Comando de varias teclas aguardando a proxima (gg, gt, gq, za, dd, r<char>).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pending {
    None,
    G,
    Gq,
    Z,
    D,
    ReplaceChar,
    ReplaceVisual,
}

pub struct Tab {
    pub key: NoteKey,
    pub filename: String,
    pub title: String,
    pub buf: Buffer,
    pub modified: bool,
    pub first_unsaved: Option<Instant>,
}

pub struct Preview {
    pub key: NoteKey,
    pub lines: Vec<Line>,
    pub scroll_y: usize,
}

/// Geometria dos botoes (colunas 1-based, intervalo [inicio, fim)) gravada pelo
/// desenho para o clique do mouse.
#[derive(Default, Clone)]
pub struct HitMap {
    pub btn_new: (i32, i32),
    pub btn_rename: (i32, i32),
    pub btn_delete: (i32, i32),
    pub btn_wrap: (i32, i32),
    pub btn_find: (i32, i32),
    pub btn_replace: (i32, i32),
    pub btn_theme: (i32, i32),
    pub btn_life: (i32, i32),
    pub btn_import: (i32, i32),
    pub btn_export: (i32, i32),
    /// Indexado pelo nivel de urgencia: 0 = Todas, 1..=6.
    pub filter: [(i32, i32); 7],
    /// (indice da aba, inicio, fim, coluna do "x") de cada aba desenhada.
    pub tabs: Vec<(usize, i32, i32, i32)>,
    pub tab_new: (i32, i32),
}

pub struct App {
    pub db: Db,
    pub tabs: Vec<Tab>,
    pub active: Option<usize>,
    pub notes: Vec<NoteInfo>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub sidebar_scroll: usize,
    pub filter_urgency: u8,
    pub search_query: String,
    pub search_cursor: usize,
    pub preview: Option<Preview>,

    pub mode: Mode,
    pub focus: Focus,
    pub pending: Pending,
    pub theme: usize,
    pub wrap: bool,
    pub left_width: i32,
    pub force_status_wrap: u8,
    pub rows: i32,
    pub cols: i32,

    pub prompt_buf: String,
    pub prompt_cursor: usize,
    pub rename_target: Option<NoteKey>,
    pub cmd_buf: String,
    pub cmd_cursor: usize,
    pub replace_find: String,
    pub last_search: String,

    pub yank: String,
    pub yank_linewise: bool,
    pub history: Vec<NoteKey>,
    pub history_idx: usize,

    pub status: Option<(String, Instant)>,
    pub hit: HitMap,
    pub quit: bool,

    // Controle de redesenho e inatividade.
    pub needs_redraw: bool,
    pub clear_needed: bool,
    pub marquee_active: bool,
    pub last_marquee_tick: u128,
    pub last_draw: Instant,
    pub last_activity: Instant,
    pub last_typing: Option<Instant>,
    pub last_theme_click: Option<Instant>,
    pub last_note_click: Option<(usize, Instant)>,
    pub start: Instant,
}

impl App {
    pub fn new(db: Db) -> App {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let now = Instant::now();
        let mut app = App {
            db,
            tabs: Vec::new(),
            active: None,
            notes: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            sidebar_scroll: 0,
            filter_urgency: 0,
            search_query: String::new(),
            search_cursor: 0,
            preview: None,
            mode: Mode::Normal,
            focus: Focus::Right,
            pending: Pending::None,
            theme: 0,
            wrap: true,
            left_width: DEFAULT_LEFT_WIDTH,
            force_status_wrap: 0,
            rows: i32::from(rows),
            cols: i32::from(cols),
            prompt_buf: String::new(),
            prompt_cursor: 0,
            rename_target: None,
            cmd_buf: String::new(),
            cmd_cursor: 0,
            replace_find: String::new(),
            last_search: String::new(),
            yank: String::new(),
            yank_linewise: false,
            history: Vec::new(),
            history_idx: 0,
            status: None,
            hit: HitMap::default(),
            quit: false,
            needs_redraw: true,
            clear_needed: true,
            marquee_active: false,
            last_marquee_tick: 0,
            last_draw: now,
            last_activity: now,
            last_typing: None,
            last_theme_click: None,
            last_note_click: None,
            start: now,
        };
        app.rescan_notes();
        if let Some(first) = app.notes.first().map(|n| n.key.clone()) {
            app.open_note(&first, false);
            app.selected = 0;
            app.focus = Focus::Left;
            app.load_preview(&first);
        } else {
            app.set_status("Banco de dados vazio. Use [Nova] para criar ou [Importar] para trazer seus .md.");
        }
        app
    }

    // ---- utilidades de estado ---------------------------------------------

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
        self.needs_redraw = true;
    }

    pub fn status_text(&self) -> Option<&str> {
        match &self.status {
            Some((m, t)) if t.elapsed() < STATUS_TTL && !m.is_empty() => Some(m.as_str()),
            _ => None,
        }
    }

    pub fn theme(&self) -> &'static crate::theme::Theme {
        &THEMES[self.theme % NUM_THEMES]
    }

    pub fn cycle_theme(&mut self) {
        self.theme = (self.theme + 1) % NUM_THEMES;
        self.clear_needed = true;
        self.set_status(format!("Tema alterado para: {}", THEMES[self.theme].name));
    }

    pub fn tab(&self) -> Option<&Tab> {
        self.active.and_then(|i| self.tabs.get(i))
    }
    pub fn tab_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }
    pub fn buf(&self) -> Option<&Buffer> {
        self.tab().map(|t| &t.buf)
    }
    pub fn buf_mut(&mut self) -> Option<&mut Buffer> {
        self.tab_mut().map(|t| &mut t.buf)
    }

    pub fn mark_modified(&mut self) {
        if let Some(t) = self.tab_mut() {
            // Toda edicao termina aqui: garante cursor valido mesmo se a linha encurtou.
            t.buf.clamp_cursor();
            t.modified = true;
            if t.first_unsaved.is_none() {
                t.first_unsaved = Some(Instant::now());
            }
        }
    }

    /// Executa uma edicao no buffer ativo e marca a aba como modificada.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Buffer) -> R) -> Option<R> {
        let r = f(self.buf_mut()?);
        self.mark_modified();
        Some(r)
    }

    pub fn cur_key(&self) -> Option<&NoteKey> {
        self.tab().map(|t| &t.key)
    }

    pub fn in_preview(&self) -> bool {
        self.focus == Focus::Left && self.preview.is_some()
    }

    // ---- geometria ---------------------------------------------------------

    pub fn editor_width(&self) -> i32 {
        self.cols - self.left_width - 1
    }

    pub fn avail_text_w(&self) -> usize {
        let w = self.editor_width() - 7;
        if w <= 5 {
            5
        } else {
            w as usize
        }
    }

    /// Linhas da area de conteudo do editor (sem barra de abas nem rodape).
    pub fn visible_rows(&self) -> usize {
        ((self.rows - 4).max(1)) as usize
    }

    /// Linhas visiveis da lista de notas.
    pub fn sidebar_visible(&self) -> usize {
        ((self.rows - 7).max(1)) as usize
    }

    pub fn hide(&self, in_preview: bool) -> Hide {
        let h = in_preview || self.mode != Mode::Insert;
        Hide { links: h, styles: h }
    }

    pub fn layout(&self, in_preview: bool) -> Layout {
        Layout {
            wrap: self.wrap,
            avail_w: self.avail_text_w(),
            hide: self.hide(in_preview),
        }
    }

    /// Ajusta rolagens para manter o cursor visivel e dentro dos limites.
    pub fn editor_scroll(&mut self) {
        let lay = self.layout(false);
        let visible = self.visible_rows();
        if let Some(i) = self.active {
            let b = &mut self.tabs[i].buf;
            b.clamp_cursor();
            let (cur_vrow, _) = crate::layout::cursor_visual_pos(b, &lay);
            b.scroll_y = clamp_scroll(&b.lines, lay, b.scroll_y, visible);
            if cur_vrow < b.scroll_y {
                b.scroll_y = cur_vrow;
            }
            if cur_vrow >= b.scroll_y + visible {
                b.scroll_y = cur_vrow + 1 - visible;
            }
        }
        if self.focus == Focus::Left {
            let play = self.layout(true);
            if let Some(p) = &mut self.preview {
                p.scroll_y = clamp_scroll(&p.lines, play, p.scroll_y, visible);
            }
        }
        let vis = self.sidebar_visible();
        if self.selected < self.sidebar_scroll {
            self.sidebar_scroll = self.selected;
        }
        if self.selected >= self.sidebar_scroll + vis {
            self.sidebar_scroll = self.selected + 1 - vis;
        }
        let max_scroll = self.filtered.len().saturating_sub(vis);
        self.sidebar_scroll = self.sidebar_scroll.min(max_scroll);
    }

    // ---- catalogo ----------------------------------------------------------

    pub fn rescan_notes(&mut self) {
        let now = now_secs();
        // Reaproveita o indice de busca das notas que nao mudaram.
        let mut cache: HashMap<NoteKey, (i64, String, String)> = HashMap::new();
        for n in self.notes.drain(..) {
            cache.insert(n.key.clone(), (n.mtime, n.filename, n.search_norm));
        }
        let mut notes = Vec::new();
        for row in self.db.list_notes() {
            let key = NoteKey::Db(row.id);
            let filename = format!("{}.md", row.name);
            let mtime = if row.mtime > 0 { row.mtime } else { now };
            let search_norm = match cache.remove(&key) {
                Some((m, f, s)) if m == mtime && f == filename => s,
                _ => {
                    let content = self.db.get_content(row.id).unwrap_or_default();
                    build_search_norm(&filename, &title_from_filename(&filename), &content)
                }
            };
            notes.push(NoteInfo {
                title: title_from_filename(&filename),
                filename,
                key,
                mtime,
                attention: row.attention,
                search_norm,
            });
        }
        notes.sort_by(compare_notes);
        self.notes = notes;
        self.apply_search();
    }

    pub fn apply_search(&mut self) {
        let words = query_words(&self.search_query);
        let urgency = self.filter_urgency;
        self.filtered = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| matches_filter(n, &words, urgency))
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = 0;
            self.sidebar_scroll = 0;
        }
        if self.focus == Focus::Left {
            self.sync_preview_to_selection();
        }
    }

    pub fn selected_note(&self) -> Option<&NoteInfo> {
        self.filtered.get(self.selected).and_then(|&i| self.notes.get(i))
    }

    pub fn selected_key(&self) -> Option<NoteKey> {
        self.selected_note().map(|n| n.key.clone())
    }

    /// Mostra a previa da nota selecionada (ou limpa se a lista esta vazia).
    pub fn sync_preview_to_selection(&mut self) {
        match self.selected_key() {
            Some(k) => self.load_preview(&k),
            None => self.free_preview(),
        }
    }

    pub fn free_preview(&mut self) {
        self.preview = None;
    }

    pub fn load_preview(&mut self, key: &NoteKey) {
        if self.preview.as_ref().is_some_and(|p| &p.key == key) {
            return;
        }
        self.preview = None;
        let Some(content) = self.read_note(key) else { return };
        let mut lines = Buffer::from_content(&content).lines;
        crate::doc::update_lines_syntax(&mut lines);
        self.preview = Some(Preview {
            key: key.clone(),
            lines,
            scroll_y: 0,
        });
    }

    /// Reordena a lista mantendo a selecao sobre `keep`.
    pub fn resort_keep(&mut self, keep: Option<&NoteKey>) {
        self.notes.sort_by(compare_notes);
        self.apply_search();
        if let Some(k) = keep {
            if let Some(pos) = self.filtered.iter().position(|&i| &self.notes[i].key == k) {
                self.selected = pos;
            }
            if self.focus == Focus::Left {
                self.sync_preview_to_selection();
            }
        }
    }

    fn select_in_sidebar(&mut self, key: &NoteKey) {
        if let Some(pos) = self.filtered.iter().position(|&i| &self.notes[i].key == key) {
            self.selected = pos;
        }
    }

    pub fn toggle_attention_at(&mut self, idx: usize) {
        let Some(n) = self.notes.get(idx) else { return };
        let key = n.key.clone();
        let title = n.title.clone();
        let level = (n.attention + 1) % 7;
        // Persiste primeiro: se falhar, a lista nao mente sobre o que esta salvo.
        if let Some(id) = key.db_id() {
            if !self.db.set_attention(id, level) {
                self.set_status("Falha ao gravar a marcacao de urgencia no banco.");
                return;
            }
        }
        self.notes[idx].attention = level;
        self.resort_keep(Some(&key));
        self.clear_needed = true;
        self.set_status(attention_message(level, &title));
    }

    /// Define o nivel de urgencia da nota ativa (`:atencao N`).
    pub fn set_attention_of_current(&mut self, level: u8) {
        let Some(key) = self.cur_key().cloned() else { return };
        let Some(idx) = self.notes.iter().position(|n| n.key == key) else { return };
        if let Some(id) = key.db_id() {
            if !self.db.set_attention(id, level) {
                self.set_status("Falha ao gravar a marcacao de urgencia no banco.");
                return;
            }
        }
        let title = self.notes[idx].title.clone();
        self.notes[idx].attention = level;
        self.resort_keep(Some(&key));
        self.clear_needed = true;
        self.set_status(attention_message(level, &title));
    }

    // ---- leitura/escrita de notas -----------------------------------------

    pub fn read_note(&self, key: &NoteKey) -> Option<String> {
        match key {
            NoteKey::Db(id) => self.db.get_content(*id),
            NoteKey::File(p) => fsio::read_text_file(p).ok(),
        }
    }

    fn write_note(&self, key: &NoteKey, content: &str) -> bool {
        match key {
            NoteKey::Db(id) => self.db.update_content(*id, content, now_secs()),
            NoteKey::File(p) => fsio::write_file_atomic(p, content.as_bytes()).is_ok(),
        }
    }

    // ---- abas --------------------------------------------------------------

    pub fn open_note(&mut self, key: &NoteKey, record_history: bool) {
        if let Some(i) = self.tabs.iter().position(|t| &t.key == key) {
            self.tab_switch(i);
            if record_history {
                self.push_history(key);
            }
            return;
        }
        let Some(content) = self.read_note(key) else {
            self.set_status("Nota não encontrada (foi excluída ou o arquivo sumiu).");
            return;
        };
        let filename = match key {
            NoteKey::Db(id) => self
                .db
                .get_name(*id)
                .map(|n| format!("{}.md", n))
                .unwrap_or_else(|| key.to_string()),
            NoteKey::File(p) => p
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.to_string()),
        };
        let title = title_from_filename(&filename);
        self.tabs.push(Tab {
            key: key.clone(),
            filename,
            title,
            buf: Buffer::from_content(&content),
            modified: false,
            first_unsaved: None,
        });
        self.active = Some(self.tabs.len() - 1);
        if record_history {
            self.push_history(key);
        }
        self.select_in_sidebar(key);
        self.clear_needed = true;
    }

    /// Abre o arquivo passado na linha de comando. Se ainda nao existe, abre uma
    /// aba vazia que cria o arquivo ao salvar.
    pub fn open_path_arg(&mut self, path: PathBuf) {
        let key = NoteKey::File(path.clone());
        if path.exists() {
            self.open_note(&key, true);
        } else {
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.to_string());
            let title = title_from_filename(&filename);
            self.tabs.push(Tab {
                key: key.clone(),
                filename,
                title,
                buf: Buffer::new_empty(),
                modified: false,
                first_unsaved: None,
            });
            self.active = Some(self.tabs.len() - 1);
            self.push_history(&key);
            self.clear_needed = true;
        }
        self.focus = Focus::Right;
    }

    pub fn tab_switch(&mut self, idx: usize) {
        if idx >= self.tabs.len() || Some(idx) == self.active {
            return;
        }
        self.active = Some(idx);
        if let Some(b) = self.buf_mut() {
            b.invalidate_typing();
        }
        let key = self.tabs[idx].key.clone();
        self.select_in_sidebar(&key);
        let name = self.tabs[idx].filename.clone();
        self.clear_needed = true;
        self.set_status(format!("Aba: {}", name));
    }

    pub fn tab_next(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        let next = (self.active.unwrap_or(0) + 1) % self.tabs.len();
        self.tab_switch(next);
    }

    pub fn tab_prev(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        let n = self.tabs.len();
        let prev = (self.active.unwrap_or(0) + n - 1) % n;
        self.tab_switch(prev);
    }

    pub fn tab_close(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        if self.tabs[idx].modified && !self.save_tab(idx) {
            self.set_status("Aba nao fechada: falha ao salvar as alteracoes");
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active = None;
            self.set_status("Última aba fechada. Clique em [➕ Nova Nota] para criar uma nova nota.");
        } else {
            let act = self.active.unwrap_or(0);
            if idx == act {
                self.active = Some(idx.saturating_sub(1).min(self.tabs.len() - 1));
                self.focus = Focus::Right;
            } else if idx < act {
                self.active = Some(act - 1);
            }
            self.set_status("Aba fechada");
        }
        self.clear_needed = true;
    }

    /// Salva todas as abas antes de fechar qualquer uma: se uma gravacao falhar,
    /// nada e fechado e o usuario nao perde o que ainda nao foi salvo.
    pub fn close_all_tabs(&mut self) -> bool {
        for i in 0..self.tabs.len() {
            if self.tabs[i].modified && !self.save_tab(i) {
                let name = self.tabs[i].filename.clone();
                self.set_status(format!("Nao foi possivel salvar '{}'; abas mantidas abertas", name));
                return false;
            }
        }
        self.tabs.clear();
        self.active = None;
        self.clear_needed = true;
        self.set_status("Todas as abas foram fechadas. Clique na aba [➕ Nova Nota] para começar.");
        true
    }

    // ---- salvar ------------------------------------------------------------

    /// Grava a aba `idx`. Atualiza o catalogo em memoria; nao reordena a lista.
    pub fn save_tab(&mut self, idx: usize) -> bool {
        let Some(t) = self.tabs.get(idx) else { return false };
        let content = t.buf.to_content();
        let key = t.key.clone();
        if !self.write_note(&key, &content) {
            return false;
        }
        let now = now_secs();
        if let Some(n) = self.notes.iter_mut().find(|n| n.key == key) {
            n.mtime = now;
            n.search_norm = build_search_norm(&n.filename, &n.title, &content);
        }
        let t = &mut self.tabs[idx];
        t.modified = false;
        t.first_unsaved = None;
        true
    }

    pub fn save_current(&mut self) -> bool {
        let Some(idx) = self.active else {
            self.set_status("Nenhuma nota aberta para salvar.");
            return false;
        };
        let name = self.tabs[idx].filename.clone();
        if !self.save_tab(idx) {
            self.set_status(format!("Erro salvando nota: {}", name));
            return false;
        }
        let key = self.tabs[idx].key.clone();
        if self.notes.iter().any(|n| n.key == key) {
            self.resort_keep(Some(&key));
        } else {
            self.rescan_notes();
        }
        let lines = self.tabs[idx].buf.num_lines();
        self.set_status(format!("\"{}\" {}L gravado", name, lines));
        true
    }

    pub fn any_tab_modified(&self) -> bool {
        self.tabs.iter().any(|t| t.modified)
    }

    pub fn save_all_modified(&mut self) -> bool {
        for i in 0..self.tabs.len() {
            if self.tabs[i].modified && !self.save_tab(i) {
                let name = self.tabs[i].filename.clone();
                self.set_status(format!("Erro salvando nota: {}", name));
                return false;
            }
        }
        true
    }

    /// Salva em silencio as abas com alteracoes pendentes ha mais de 1 minuto.
    pub fn check_autosave(&mut self) {
        let mut saved = 0;
        let mut last = String::new();
        for i in 0..self.tabs.len() {
            let due = self.tabs[i].modified
                && self.tabs[i].first_unsaved.is_some_and(|t| t.elapsed() >= AUTOSAVE_DELAY);
            if due && self.save_tab(i) {
                saved += 1;
                last = self.tabs[i].filename.clone();
            }
        }
        if saved > 0 {
            let keep = if self.focus == Focus::Left {
                self.selected_key()
            } else {
                self.cur_key().cloned()
            };
            self.resort_keep(keep.as_ref());
            if saved == 1 {
                self.set_status(format!("💾 Salvo automaticamente: {}", last));
            } else {
                self.set_status(format!("💾 {} notas salvas automaticamente", saved));
            }
        }
    }

    // ---- historico ---------------------------------------------------------

    pub fn push_history(&mut self, key: &NoteKey) {
        if self.history.last() == Some(key) {
            return;
        }
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(key.clone());
        self.history_idx = self.history.len() - 1;
    }

    pub fn navigate_history(&mut self, steps: usize) {
        if self.history.is_empty() {
            return;
        }
        self.history_idx = self.history_idx.saturating_sub(steps).min(self.history.len() - 1);
        let key = self.history[self.history_idx].clone();
        self.set_status(format!("Histórico: {}", key));
        self.open_note(&key, false);
    }

    // ---- criar / renomear / excluir ---------------------------------------

    pub fn create_new_note(&mut self, raw: &str) {
        let name = raw.trim_matches([' ', '\t']);
        if name.is_empty() {
            self.set_status("Nome da nota não pode ser vazio.");
            return;
        }
        let (filename, title) = split_md_ext(name);
        let mut safe = sanitize_note_name(&title);
        safe.truncate(crate::text::floor_boundary(&safe, 255));
        if safe.is_empty() {
            self.set_status("Nome de nota invalido.");
            return;
        }
        if let Some(id) = self.db.find_id_by_name(&safe) {
            self.set_status(format!("Nota '{}' já existe! Abrindo...", filename));
            self.open_note(&NoteKey::Db(id), true);
            self.focus = Focus::Right;
            return;
        }
        let Some(id) = self.db.insert_note(&safe, &format!("# {}\n\n", title), now_secs(), 0) else {
            self.set_status(format!("Erro ao criar nota: {}", safe));
            return;
        };
        let key = NoteKey::Db(id);
        self.rescan_notes();
        self.free_preview();
        self.open_note(&key, true);
        self.focus = Focus::Right;
        if let Some(b) = self.buf_mut() {
            if b.num_lines() > 1 {
                b.cy = b.num_lines() - 1;
                b.cx = 0;
            }
        }
        self.set_status(format!("Nova nota criada: {}", filename));
    }

    pub fn open_or_create_wikilink(&mut self, name: &str) {
        let clean: String = sanitize_note_name(name).chars().take(250).collect();
        let id = match self.db.find_id_by_name(&clean) {
            Some(id) => id,
            None => {
                let body = format!("# {}\n\nNova nota criada via link.\n", name);
                let Some(id) = self.db.insert_note(&clean, &body, now_secs(), 0) else {
                    self.set_status(format!("Erro ao criar nota via link: {}", clean));
                    return;
                };
                self.rescan_notes();
                id
            }
        };
        self.open_note(&NoteKey::Db(id), true);
        self.set_status(format!("Nota aberta: [[{}]]", name));
    }

    pub fn rename_note(&mut self, old: &NoteKey, raw: &str) -> bool {
        let name = raw.trim_matches([' ', '\t']);
        if name.is_empty() {
            self.set_status("Novo nome não pode ser vazio.");
            return false;
        }
        let (new_filename, new_title) = split_md_ext(name);
        let mut safe_new = sanitize_note_name(&new_title);
        safe_new.truncate(crate::text::floor_boundary(&safe_new, 255));
        if safe_new.is_empty() {
            self.set_status("Novo nome invalido.");
            return false;
        }

        let new_key = match old {
            NoteKey::Db(id) => {
                let current = self.notes.iter().find(|n| &n.key == old).map(|n| n.title.clone());
                if current.as_deref() == Some(safe_new.as_str()) {
                    self.set_status("O nome da nota permanece o mesmo.");
                    return true;
                }
                if self.db.find_id_by_name(&safe_new).is_some_and(|other| other != *id) {
                    self.set_status(format!("Já existe uma nota com o nome: {}", new_filename));
                    return false;
                }
                if !self.db.update_name(*id, &safe_new) {
                    self.set_status("Erro ao renomear a nota no banco.");
                    return false;
                }
                old.clone()
            }
            NoteKey::File(p) => {
                let dir = p.parent().map(PathBuf::from).unwrap_or_default();
                let np = dir.join(&new_filename);
                if &np == p {
                    self.set_status("O nome da nota permanece o mesmo.");
                    return true;
                }
                if np.exists() {
                    self.set_status(format!("Já existe uma nota com o nome: {}", new_filename));
                    return false;
                }
                if std::fs::rename(p, &np).is_err() {
                    self.set_status("Erro ao renomear arquivo no disco.");
                    return false;
                }
                NoteKey::File(np)
            }
        };

        // Atualiza as abas abertas e o titulo "# ..." da primeira linha.
        let heading = format!("# {}", new_title);
        let mut open_idx = Vec::new();
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if &t.key == old {
                t.key = new_key.clone();
                t.filename = new_filename.clone();
                t.title = new_title.clone();
                if t.buf.lines.first().is_some_and(|l| l.text.starts_with("# ")) {
                    t.buf.lines[0].set_text(heading.clone());
                    t.buf.update_folding();
                    t.buf.clamp_cursor(); // a linha pode ter encurtado sob o cursor
                    t.modified = true;
                }
                open_idx.push(i);
            }
        }
        if open_idx.is_empty() {
            // Nota fechada: ajusta o cabecalho direto no armazenamento.
            if let Some(content) = self.read_note(&new_key) {
                let body = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
                if body.starts_with("# ") {
                    let rest = body.find('\n').map(|i| &body[i..]).unwrap_or("");
                    let _ = self.write_note(&new_key, &format!("{}{}", heading, rest));
                }
            }
        } else {
            for i in open_idx {
                self.save_tab(i);
            }
        }
        if self.preview.as_ref().is_some_and(|p| &p.key == old) {
            self.free_preview();
            self.load_preview(&new_key);
        }
        self.rescan_notes();
        self.select_in_sidebar(&new_key);
        self.set_status(format!("Nota renomeada com sucesso: {}", new_filename));
        true
    }

    pub fn delete_selected_note(&mut self) {
        let Some(key) = self.selected_key() else { return };
        // Se a exclusao falhar, mantem tudo como esta.
        match &key {
            NoteKey::Db(id) => {
                if !self.db.delete_note(*id) {
                    self.set_status("Falha ao excluir a nota do banco.");
                    return;
                }
            }
            NoteKey::File(p) => {
                if std::fs::remove_file(p).is_err() {
                    self.set_status(format!("Falha ao excluir o arquivo: {}", p.display()));
                    return;
                }
            }
        }
        for i in (0..self.tabs.len()).rev() {
            if self.tabs[i].key == key {
                self.tabs[i].modified = false;
                self.tab_close(i);
            }
        }
        self.history.retain(|k| k != &key);
        self.history_idx = self.history.len().saturating_sub(1);
        if self.preview.as_ref().is_some_and(|p| p.key == key) {
            self.free_preview();
        }
        self.rescan_notes();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.sync_preview_to_selection();
        self.set_status("Nota excluída com sucesso.");
    }

    // ---- importar / exportar ----------------------------------------------

    pub fn ui_import(&mut self, raw: &str) {
        let dir = fsio::clean_dir_input(raw);
        if dir.is_empty() {
            self.set_status("Importacao cancelada: nenhuma pasta informada.");
            return;
        }
        match fsio::import_notes_from_dir(&self.db, &dir) {
            Err(e) => self.set_status(format!("Importacao falhou: {}", e)),
            Ok(r) => {
                self.rescan_notes();
                self.clear_needed = true;
                let total = r.added + r.updated;
                if total == 0 {
                    self.set_status(format!("Nenhum arquivo markdown encontrado em: {}", dir));
                } else {
                    self.set_status(format!(
                        "Importadas {} nota(s): {} nova(s), {} atualizada(s) de {}",
                        total, r.added, r.updated, dir
                    ));
                }
            }
        }
    }

    pub fn ui_export(&mut self, raw: &str) {
        let dir = fsio::clean_dir_input(raw);
        if dir.is_empty() {
            self.set_status("Exportacao cancelada: nenhuma pasta informada.");
            return;
        }
        if self.db.count_notes() == 0 {
            self.set_status("Nada para exportar: o banco esta vazio.");
            return;
        }
        match fsio::export_notes_to_dir(&self.db, &dir) {
            Err(e) => self.set_status(format!("Exportacao falhou: {}", e)),
            Ok(n) => self.set_status(format!("Exportadas {} nota(s) como arquivos .md em {}", n, dir)),
        }
    }

    /// Salvamento de emergencia (encerramento ou falha inesperada).
    pub fn emergency_save(&mut self) {
        for i in 0..self.tabs.len() {
            if self.tabs[i].modified {
                self.save_tab(i);
            }
        }
    }

    /// Copia para o clipboard do sistema e guarda no buffer interno.
    pub fn set_yank(&mut self, text: String, linewise: bool) {
        clip::copy(&text);
        self.yank = text;
        self.yank_linewise = linewise;
    }

    /// Busca da proxima linha (caso indiferente) a partir de `from`, com volta.
    pub fn find_line(&self, pat: &str, from: usize, wrap_to: usize) -> Option<(usize, bool)> {
        let b = self.buf()?;
        let n = b.lines.len();
        (from..n)
            .find(|&i| crate::notes::contains_caseless(&b.lines[i].text, pat))
            .map(|i| (i, false))
            .or_else(|| {
                (0..wrap_to.min(n))
                    .find(|&i| crate::notes::contains_caseless(&b.lines[i].text, pat))
                    .map(|i| (i, true))
            })
    }
}

fn clamp_scroll(lines: &[Line], lay: Layout, scroll: usize, visible: usize) -> usize {
    match VRowIter::locate(lines, lay, scroll) {
        None => scroll.min(crate::layout::total_vrows(lines, &lay).saturating_sub(visible)),
        Some(mut it) => {
            let mut counted = 1;
            while counted < visible && it.advance() {
                counted += 1;
            }
            if counted < visible {
                // O documento termina antes de encher a tela.
                (scroll + counted).saturating_sub(visible).min(scroll)
            } else {
                scroll
            }
        }
    }
}

pub fn attention_message(level: u8, title: &str) -> String {
    match level {
        1 => format!("🔴 Urgência Nível 1 [Crítica - Vermelho]: {}", title),
        2 => format!("🟠 Urgência Nível 2 [Média - Laranja]: {}", title),
        3 => format!("🟡 Urgência Nível 3 [Baixa - Amarelo]: {}", title),
        4 => format!("🔵 Marcação [H - Azul]: {}", title),
        5 => format!("🟣 Marcação [R - Roxo]: {}", title),
        6 => format!("🟢 Marcação [O - Verde]: {}", title),
        _ => format!("Marcação de urgência removida: {}", title),
    }
}

/// (nome do arquivo com .md, titulo sem .md). Compara bytes ASCII: seguro com
/// nomes que terminam em caractere multibyte.
pub fn split_md_ext(name: &str) -> (String, String) {
    let b = name.as_bytes();
    if b.len() > 3 && b[b.len() - 3..].eq_ignore_ascii_case(b".md") {
        (name.to_string(), name[..name.len() - 3].to_string())
    } else {
        (format!("{}.md", name), name.to_string())
    }
}

#[cfg(test)]
mod name_tests {
    use super::split_md_ext;

    #[test]
    fn md_extension_is_split_safely() {
        assert_eq!(split_md_ext("Nota.MD"), ("Nota.MD".to_string(), "Nota".to_string()));
        assert_eq!(split_md_ext("Nota"), ("Nota.md".to_string(), "Nota".to_string()));
        assert_eq!(split_md_ext("日本語").1, "日本語");
        assert_eq!(split_md_ext("x.md").1, "x");
        assert_eq!(split_md_ext(".md").1, ".md");
    }
}
