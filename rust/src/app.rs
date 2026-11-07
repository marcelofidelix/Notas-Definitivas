//! Estado global da aplicacao: abas, catalogo de notas, painel lateral,
//! operacoes sobre notas (abrir, salvar, criar, renomear, excluir) e filtros.

use crate::db::{now_secs, Db};
use crate::doc::{Buffer, Line};
use crate::layout::{Hide, Layout, VRowIter};
use crate::notes::{
    build_content_norm, build_search_norm, compare_notes, matches_filter, query_words, search_norm,
    title_from_filename, NoteInfo, NoteKey,
};
use crate::text::sanitize_note_name;
use crate::theme::{NUM_THEMES, THEMES};
use crate::{clip, fsio};
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
    PromptReplaceSelection,
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
                | Mode::PromptReplaceSelection
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

/// `3i`, `5o`: repete o que for digitado quando o modo Insercao termina (Esc).
#[derive(Clone, Copy, Debug)]
pub struct InsertRepeat {
    pub times: usize,
    pub cy: usize,
    pub cx: usize,
    /// `o`/`O`: repete a linha inteira em vez do trecho digitado.
    pub lines: bool,
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
}

struct NoteSnapshot {
    filename: String,
    content: String,
    version: Option<u64>,
}

pub struct Tab {
    pub key: NoteKey,
    pub filename: String,
    pub title: String,
    pub buf: Buffer,
    pub modified: bool,
    pub first_unsaved: Option<Instant>,
    /// Ausente apenas quando o arquivo ainda precisa ser criado.
    loaded_version: Option<u64>,
}

pub struct Preview {
    pub key: NoteKey,
    pub lines: Vec<Line>,
    pub scroll_y: usize,
}

/// Menu de contexto aberto com o botao direito sobre uma tabela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableMenu {
    /// Linha do documento em que o clique caiu (dentro do bloco da tabela).
    pub line: usize,
    /// Coluna logica da tabela (0-based).
    pub column: usize,
    /// 0 = A→Z, 1 = Z→A.
    pub selected: usize,
}

/// Geometria dos botoes (colunas 1-based, intervalo [inicio, fim)) gravada pelo
/// desenho para o clique do mouse.
#[derive(Default, Clone)]
pub struct HitMap {
    pub btn_new: (i32, i32),
    pub btn_rename: (i32, i32),
    pub btn_link: (i32, i32),
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
    /// Opcoes do menu de contexto de tabela na linha de comando.
    pub menu_asc: (i32, i32),
    pub menu_desc: (i32, i32),
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
    /// Prefixo numerico do modo Normal/Visual (`100l`); 0 = nenhum.
    pub count: usize,
    /// Segundo numero de `d3d`; 0 = nenhum.
    pub count2: usize,
    pub insert_repeat: Option<InsertRepeat>,
    pub theme: usize,
    pub wrap: bool,
    pub left_width: i32,
    pub force_status_wrap: u8,
    pub rows: i32,
    pub cols: i32,

    pub prompt_buf: String,
    pub prompt_cursor: usize,
    pub replace_selection_mode: Option<Mode>,
    pub rename_target: Option<NoteKey>,
    pub cmd_buf: String,
    pub cmd_cursor: usize,
    pub replace_find: String,
    pub last_search: String,
    /// Onde comecou a ocorrencia atual da busca: o realce pinta essa diferente
    /// das demais e o `n`/`N` partem dela.
    pub search_at: Option<(usize, usize)>,
    /// Realce das ocorrencias ligado (`:noh` desliga; uma nova busca religa).
    pub search_hl: bool,
    /// Menu de contexto da tabela (botao direito): A→Z ou Z→A.
    pub table_menu: Option<TableMenu>,

    pub yank: String,
    pub yank_linewise: bool,
    pub history: Vec<NoteKey>,
    pub history_idx: usize,

    pub status: Option<(String, Instant)>,
    pub hit: HitMap,
    /// Linhas do ultimo quadro desenhado (base do redesenho incremental).
    pub prev_rows: Vec<String>,
    /// Redesenho incremental ligado (desligado nos testes, que leem quadros completos).
    pub diff_render: bool,
    /// Forca repintar todas as linhas no proximo quadro (sem limpar a tela).
    pub force_full: bool,
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
    pub last_editor_click: Option<crate::mouse::EditorClick>,
    /// O indice de busca das notas ja foi carregado do banco (ver
    /// `load_search_index`). Comeca falso e volta a falso a cada recatalogo.
    pub norms_loaded: bool,
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
            count: 0,
            count2: 0,
            insert_repeat: None,
            theme: 0,
            wrap: true,
            left_width: DEFAULT_LEFT_WIDTH,
            force_status_wrap: 0,
            rows: i32::from(rows),
            cols: i32::from(cols),
            prompt_buf: String::new(),
            prompt_cursor: 0,
            replace_selection_mode: None,
            rename_target: None,
            cmd_buf: String::new(),
            cmd_cursor: 0,
            replace_find: String::new(),
            last_search: String::new(),
            search_at: None,
            search_hl: false,
            table_menu: None,
            yank: String::new(),
            yank_linewise: false,
            history: Vec::new(),
            history_idx: 0,
            status: None,
            hit: HitMap::default(),
            prev_rows: Vec::new(),
            diff_render: !cfg!(test),
            force_full: false,
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
            last_editor_click: None,
            norms_loaded: false,
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
        // A ocorrencia atual da busca pode ter deixado de existir: a partir daqui
        // `n`/`N` partem do cursor.
        self.search_at = None;
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

    /// Quantas linhas o numero da linha precisa mostrar (minimo 4 digitos).
    /// A calha cresce quando a nota passa de 9.999 linhas; sem isso o numero
    /// ocupa uma coluna a mais e o texto (e o cursor) saem do lugar.
    fn line_no_source_lines(&self, in_preview: bool) -> usize {
        if in_preview {
            self.preview.as_ref().map_or(1, |p| p.lines.len())
        } else {
            self.buf().map_or(1, |b| b.num_lines())
        }
    }

    pub fn line_no_digits(&self, in_preview: bool) -> usize {
        let mut n = self.line_no_source_lines(in_preview).max(1);
        let mut digits = 1;
        while n >= 10 {
            n /= 10;
            digits += 1;
        }
        digits.max(4)
    }

    /// Colunas da calha do numero de linha: digitos + espaco + indicador de dobra.
    pub fn gutter_width(&self, in_preview: bool) -> usize {
        self.line_no_digits(in_preview) + 2
    }

    pub fn avail_text_w(&self, in_preview: bool) -> usize {
        let w = self.editor_width() - self.gutter_width(in_preview) as i32 - 1;
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
            avail_w: self.avail_text_w(in_preview),
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
        let mut notes = Vec::with_capacity(self.notes.len().max(8));
        for row in self.db.list_notes() {
            let filename = format!("{}.md", row.name);
            notes.push(NoteInfo {
                title: title_from_filename(&filename),
                filename,
                key: NoteKey::Db(row.id),
                mtime: if row.mtime > 0 { row.mtime } else { now },
                attention: row.attention,
                // Vazio de proposito: o indice sai do banco na primeira busca
                // (`load_search_index`). Ler e normalizar o conteudo de todas as
                // notas dominava o tempo de abertura.
                search_norm: String::new(),
            });
        }
        notes.sort_by(compare_notes);
        self.notes = notes;
        self.norms_loaded = false;
        self.apply_search();
    }

    /// Carrega (uma vez) o indice de busca de todas as notas, do banco.
    ///
    /// Notas gravadas por versoes anteriores (ou pela versao em C) nao tem o
    /// indice: o conteudo delas e lido e indexado agora, uma unica vez.
    fn load_search_index(&mut self) {
        let rows = self.db.all_content_norms();
        let mut por_id: std::collections::HashMap<i64, String> = std::collections::HashMap::with_capacity(rows.len());
        for (id, norm) in rows {
            let norm = match norm {
                Some(n) => n,
                None => {
                    let content = self.db.get_content(id).unwrap_or_default();
                    let n = build_content_norm(&content);
                    self.db.set_content_norm(id, &n);
                    n
                }
            };
            por_id.insert(id, norm);
        }
        for n in &mut self.notes {
            if let Some(id) = n.key.db_id() {
                if let Some(norm) = por_id.get(&id) {
                    let indice = search_norm(&n.filename, &n.title, norm);
                    n.search_norm = indice;
                }
            }
        }
        self.norms_loaded = true;
    }

    pub fn apply_search(&mut self) {
        let words = query_words(&self.search_query);
        if !words.is_empty() && !self.norms_loaded {
            self.load_search_index();
        }
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

    fn snapshot(&self, key: &NoteKey, allow_new: bool) -> Result<NoteSnapshot, String> {
        match key {
            NoteKey::Db(id) => {
                let (name, content, version) = self.db.snapshot(*id).map_err(|e| e.to_string())?;
                Ok(NoteSnapshot { filename: format!("{name}.md"), content, version: Some(version) })
            }
            NoteKey::File(path) => {
                let (content, version) = match fsio::file_snapshot(path) {
                    Ok((content, version)) => (content, Some(version)),
                    Err(e) if allow_new && e.kind() == std::io::ErrorKind::NotFound => (String::new(), None),
                    Err(e) => return Err(format!("{}: {e}", path.display())),
                };
                let filename = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().into_owned();
                Ok(NoteSnapshot { filename, content, version })
            }
        }
    }

    // ---- abas --------------------------------------------------------------

    pub fn open_note(&mut self, key: &NoteKey, record_history: bool) {
        self.open_tab(key, record_history, false);
    }

    fn open_tab(&mut self, key: &NoteKey, record_history: bool, allow_new: bool) {
        if let Some(i) = self.tabs.iter().position(|t| &t.key == key) {
            self.tab_switch(i);
            if record_history {
                self.push_history(key);
            }
            return;
        }
        let NoteSnapshot { filename, content, version } = match self.snapshot(key, allow_new) {
            Ok(note) => note,
            Err(e) => {
                self.set_status(format!("Erro ao abrir nota: {e}"));
                return;
            }
        };
        let title = title_from_filename(&filename);
        self.tabs.push(Tab {
            key: key.clone(),
            filename,
            title,
            buf: Buffer::from_content(&content),
            modified: false,
            first_unsaved: None,
            loaded_version: version,
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
        self.open_tab(&NoteKey::File(path), true, true);
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
        self.save_tab_ex(idx, false)
    }

    /// Como `save_tab`; com `force` grava mesmo que a nota tenha sido alterada
    /// por outra instancia desde que foi carregada (`:w!`).
    pub fn save_tab_ex(&mut self, idx: usize, force: bool) -> bool {
        let Some(t) = self.tabs.get(idx) else { return false };
        let content = t.buf.to_content();
        let key = t.key.clone();
        let result = match &key {
            NoteKey::Db(id) => {
                let expected = if force { None } else { t.loaded_version };
                self.db.save_note(*id, expected, None, &content)
            }
            NoteKey::File(path) => {
                let mode = if force {
                    fsio::FileWrite::Overwrite
                } else {
                    t.loaded_version.map_or(fsio::FileWrite::Create, fsio::FileWrite::IfUnchanged)
                };
                fsio::write_file(path, content.as_bytes(), mode)
                    .map(|_| fsio::file_version(content.as_bytes()))
                    .map_err(|e| e.to_string())
            }
        };
        match result {
            Ok(version) => self.tabs[idx].loaded_version = Some(version),
            Err(e) => {
                self.set_status(format!("⚠ Falha ao salvar {}: {e}", t.filename));
                return false;
            }
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
        self.save_current_ex(false)
    }

    pub fn save_current_ex(&mut self, force: bool) -> bool {
        let Some(idx) = self.active else {
            self.set_status("Nenhuma nota aberta para salvar.");
            return false;
        };
        let name = self.tabs[idx].filename.clone();
        if !self.save_tab_ex(idx, force) {
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

    /// `:e!` — descarta as alteracoes da aba ativa e recarrega a nota do banco.
    pub fn reload_current(&mut self) {
        let Some(idx) = self.active else { return };
        let key = self.tabs[idx].key.clone();
        let NoteSnapshot { filename, content, version } = match self.snapshot(&key, false) {
            Ok(note) => note,
            Err(e) => {
                self.set_status(format!("Falha ao recarregar a nota: {e}"));
                return;
            }
        };
        let t = &mut self.tabs[idx];
        t.title = title_from_filename(&filename);
        t.filename = filename;
        let (cy, cx) = (t.buf.cy, t.buf.cx);
        t.buf = Buffer::from_content(&content);
        t.buf.cy = cy;
        t.buf.cx = cx;
        t.buf.clamp_cursor();
        t.modified = false;
        t.first_unsaved = None;
        t.loaded_version = version;
        self.clear_needed = true;
        let name = self.tabs[idx].filename.clone();
        self.set_status(format!("\"{}\" recarregada", name));
    }

    pub fn any_tab_modified(&self) -> bool {
        self.tabs.iter().any(|t| t.modified)
    }

    pub fn save_all_modified(&mut self) -> bool {
        for i in 0..self.tabs.len() {
            if self.tabs[i].modified && !self.save_tab(i) {
                return false;
            }
        }
        true
    }

    /// Salva em silencio as abas com alteracoes pendentes ha mais de 1 minuto.
    pub fn check_autosave(&mut self) {
        let mut saved = 0;
        let mut failed = false;
        let mut last = String::new();
        for i in 0..self.tabs.len() {
            let due = self.tabs[i].modified
                && self.tabs[i].first_unsaved.is_some_and(|t| t.elapsed() >= AUTOSAVE_DELAY);
            if due {
                if self.save_tab(i) {
                    saved += 1;
                    last = self.tabs[i].filename.clone();
                } else {
                    // Falhou (conflito, disco...): tenta de novo so em 1 minuto, em vez
                    // de martelar o banco e a mensagem de status a cada 100 ms.
                    failed = true;
                    self.tabs[i].first_unsaved = Some(Instant::now());
                }
            }
        }
        if saved > 0 {
            let keep = if self.focus == Focus::Left {
                self.selected_key()
            } else {
                self.cur_key().cloned()
            };
            self.resort_keep(keep.as_ref());
            if failed {
                return;
            }
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

        let idx = self.tabs.iter().position(|t| &t.key == old);
        let NoteSnapshot { filename, mut content, version } = match idx {
            Some(i) => {
                let t = &self.tabs[i];
                NoteSnapshot { filename: t.filename.clone(), content: t.buf.to_content(), version: t.loaded_version }
            }
            None => match self.snapshot(old, false) {
                Ok(note) => note,
                Err(e) => {
                    self.set_status(format!("Falha ao ler a nota para renomear: {e}"));
                    return false;
                }
            },
        };

        let new_filename = match old {
            NoteKey::Db(_) => format!("{safe_new}.md"),
            NoteKey::File(_) => sanitize_note_name(&new_filename),
        };
        if filename == new_filename {
            self.set_status("O nome da nota permanece o mesmo.");
            return true;
        }
        let heading = format!("# {new_title}");
        let body = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
        let update_heading = body.starts_with("# ");
        if update_heading {
            let end = content.find('\n').unwrap_or(content.len());
            content.replace_range(..end, &heading);
        }
        let result = match old {
            NoteKey::Db(id) => self.db.save_note(*id, version, Some(&safe_new), &content)
                .map(|v| (old.clone(), v)),
            NoteKey::File(path) => {
                let dest = path.with_file_name(&new_filename);
                fsio::rename_note_file(path, &dest, version, content.as_bytes())
                    .map(|v| (NoteKey::File(dest), v))
            }
        };
        let (new_key, version) = match result {
            Ok(saved) => saved,
            Err(e) => {
                self.set_status(format!("⚠ Falha ao renomear {filename}: {e}"));
                return false;
            }
        };

        if let Some(i) = idx {
            let t = &mut self.tabs[i];
            t.key = new_key.clone();
            t.title = title_from_filename(&new_filename);
            t.filename = new_filename.clone();
            if update_heading {
                t.buf.lines[0].set_text(heading);
                t.buf.update_folding();
                t.buf.clamp_cursor();
            }
            t.loaded_version = Some(version);
            t.modified = false;
            t.first_unsaved = None;
        }
        for key in &mut self.history {
            if key == old {
                *key = new_key.clone();
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

    // ---- link da nota ------------------------------------------------------

    /// Nota-alvo das acoes sobre "a nota atual": a selecionada na lista (se o
    /// foco esta nela) ou a da aba ativa.
    pub fn target_note_key(&self) -> Option<NoteKey> {
        if self.focus == Focus::Left {
            self.selected_key().or_else(|| self.cur_key().cloned())
        } else {
            self.cur_key().cloned().or_else(|| self.selected_key())
        }
    }

    /// Texto de link para a nota: `[[Titulo]]` (wikilink) para notas do banco e
    /// `[nome](<caminho>)` para arquivos avulsos.
    pub fn link_text_for(&self, key: &NoteKey) -> Option<String> {
        match key {
            NoteKey::Db(_) => {
                let title = self
                    .notes
                    .iter()
                    .find(|n| &n.key == key)
                    .map(|n| n.title.clone())
                    .or_else(|| self.tabs.iter().find(|t| &t.key == key).map(|t| t.title.clone()))?;
                Some(format!("[[{}]]", title))
            }
            NoteKey::File(p) => {
                let name = p.file_name()?.to_string_lossy().into_owned();
                Some(format!("[{}](<{}>)", title_from_filename(&name), p.display()))
            }
        }
    }

    /// Copia o link da nota para a area de transferencia (e para o buffer de
    /// yank, entao `p` tambem cola). Facilita criar links entre notas.
    pub fn copy_note_link(&mut self) {
        let Some(key) = self.target_note_key() else {
            self.set_status("Nenhuma nota para copiar o link.");
            return;
        };
        match self.link_text_for(&key) {
            Some(link) => {
                self.set_yank(link.clone(), false);
                self.set_status(format!("🔗 Link copiado: {} (cole com p ou Ctrl+V)", link));
            }
            None => self.set_status("Nao foi possivel montar o link desta nota."),
        }
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
    pub fn emergency_save(&mut self) -> bool {
        let mut saved = true;
        for i in 0..self.tabs.len() {
            if self.tabs[i].modified && !self.save_tab(i) {
                saved = false;
            }
        }
        saved
    }

    /// Copia para o clipboard do sistema e guarda no buffer interno.
    pub fn set_yank(&mut self, text: String, linewise: bool) {
        clip::copy(&text);
        self.yank = text;
        self.yank_linewise = linewise;
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
