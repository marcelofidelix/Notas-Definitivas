//! Operacoes de edicao sobre a nota ativa: copiar/colar/recortar, substituir,
//! tabelas, links e busca no texto.

use crate::app::{App, Focus, Mode, FIELD_MAX};
use crate::doc::Buffer;
use crate::md::{find_links, is_table_line};
use crate::notes::{contains_caseless, NoteKey};
use crate::table;
use crate::text::floor_boundary;
use crate::{clip, links};

/// Insere ate a primeira quebra de linha de `text` em `dst`, na posicao
/// `cursor`, respeitando `FIELD_MAX` e limites de caractere.
pub fn paste_into_field(dst: &mut String, cursor: &mut usize, text: &str) {
    let cur = floor_boundary(dst, (*cursor).min(dst.len()));
    let line = text.split(['\r', '\n']).next().unwrap_or("");
    let room = FIELD_MAX.saturating_sub(dst.len());
    let take = floor_boundary(line, line.len().min(room));
    if take > 0 {
        dst.insert_str(cur, &line[..take]);
        *cursor = cur + take;
    } else {
        *cursor = cur;
    }
}

/// Limites (inicio, fim) do bloco de linhas de tabela que contem `y`.
fn table_bounds(b: &Buffer, y: usize) -> (usize, usize) {
    let mut start = y;
    while start > 0 && is_table_line(&b.lines[start - 1].text) {
        start -= 1;
    }
    let mut end = y;
    while end + 1 < b.lines.len() && is_table_line(&b.lines[end + 1].text) {
        end += 1;
    }
    (start, end)
}

/// Realinha `start..=end`; `false` se a tabela nao puder ser alinhada.
fn align_range(b: &mut Buffer, start: usize, end: usize) -> bool {
    let refs: Vec<&str> = b.lines[start..=end].iter().map(|l| &*l.text).collect();
    match table::align_table(&refs) {
        Some(new) => {
            for (i, t) in new.into_iter().enumerate() {
                b.lines[start + i].set_text(t);
            }
            true
        }
        None => false,
    }
}

/// (celula, deslocamento no conteudo) do byte `cx`, para achar o mesmo ponto depois
/// de o alinhamento mudar o preenchimento das celulas.
fn cell_coords(text: &str, cx: usize) -> (usize, usize) {
    let (mut cell, mut start) = (0usize, text.find('|').map_or(0, |i| i + 1));
    loop {
        let end = text[start..].find('|').map_or(text.len(), |i| start + i);
        if cx <= end || end >= text.len() {
            let content = text[start..end].trim_start_matches([' ', '\t']);
            let cs = end - content.len();
            return (cell, cx.saturating_sub(cs).min(content.trim_end_matches([' ', '\t']).len()));
        }
        cell += 1;
        start = end + 1;
    }
}

fn cx_from_coords(text: &str, cell: usize, off: usize) -> usize {
    let mut start = text.find('|').map_or(0, |i| i + 1);
    for _ in 0..cell {
        match text[start..].find('|') {
            Some(i) => start += i + 1,
            None => break,
        }
    }
    let end = text[start..].find('|').map_or(text.len(), |i| start + i);
    let content = text[start..end].trim_start_matches([' ', '\t']);
    let cs = end - content.len();
    floor_boundary(text, (cs + off).min(end))
}

/// Realinha `start..=end` so se algo mudar, conservando o cursor na mesma celula
/// e no mesmo caractere. `Some(true)` se o texto mudou.
fn align_range_keep_cursor(b: &mut Buffer, start: usize, end: usize) -> Option<bool> {
    let refs: Vec<&str> = b.lines[start..=end].iter().map(|l| &*l.text).collect();
    let new = table::align_table(&refs)?;
    if new.iter().zip(&refs).all(|(n, o)| n == o) {
        return Some(false);
    }
    let coords = (start..=end).contains(&b.cy).then(|| cell_coords(&b.lines[b.cy].text, b.cx));
    b.save_undo();
    for (i, t) in new.into_iter().enumerate() {
        b.lines[start + i].set_text(t);
    }
    if let Some((cell, off)) = coords {
        b.cx = cx_from_coords(&b.lines[b.cy].text, cell, off);
    }
    b.update_folding();
    Some(true)
}

/// Inicio do texto da celula que comeca em `pos` (logo depois de um `|`): pula o
/// recuo; numa celula vazia fica depois do primeiro espaco, pronto para digitar.
fn cell_text_start(text: &str, pos: usize) -> usize {
    let b = text.as_bytes();
    let mut p = pos;
    while p < b.len() && b[p] == b' ' {
        p += 1;
    }
    if p < b.len() && b[p] == b'|' {
        (pos + 1).min(p)
    } else {
        p
    }
}

/// Posicao do texto da primeira celula de uma linha.
fn after_first_bar(text: &str) -> usize {
    text.find('|').map_or(0, |i| cell_text_start(text, i + 1))
}

impl App {
    // ---- copiar / recortar / colar ----------------------------------------

    pub fn yank_visual(&mut self, linewise: bool) {
        let Some(text) = self.buf().and_then(|b| b.selection_text(linewise)) else { return };
        self.set_yank(text, linewise);
    }

    /// Apaga a selecao (copiando antes se `do_yank`).
    pub fn delete_visual_ex(&mut self, do_yank: bool) {
        if self.buf().is_none() {
            return;
        }
        let linewise = self.mode == Mode::VisualLine;
        if do_yank {
            self.yank_visual(linewise);
        }
        let insert = self.mode == Mode::Insert;
        if let Some(b) = self.buf_mut() {
            b.save_undo();
        }
        let removed = self.edit(|b| b.delete_selection(linewise, insert)).flatten();
        match removed {
            Some(n) => self.set_status(format!("{} linha(s) excluída(s)", n)),
            None => self.set_status("Texto selecionado excluído"),
        }
        self.clear_needed = true;
        if self.mode != Mode::Insert {
            self.mode = Mode::Normal;
        }
    }

    /// `p` com selecao ativa: troca a selecao pelo conteudo do clipboard.
    pub fn paste_replace_selection(&mut self) {
        let text = match clip::paste() {
            Some(t) => t,
            None if !self.yank.is_empty() => self.yank.clone(),
            None => {
                self.set_status("Área de transferência vazia para colar.");
                return;
            }
        };
        if self.yank != text {
            self.yank_linewise = text.contains('\n');
            self.yank = text.clone();
        }
        self.delete_visual_ex(false);
        self.mode = Mode::Normal;
        if let Some(b) = self.buf_mut() {
            b.has_selection = false;
        }
        self.paste_text(&text);
        self.editor_scroll();
        self.set_status("Texto colado sobre a seleção com 'p'.");
    }

    /// Insere texto colado onde estiver o foco: campo de busca/comando/prompt
    /// ou o cursor da nota ativa.
    pub fn paste_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.mode {
            Mode::SearchLeft => {
                paste_into_field(&mut self.search_query, &mut self.search_cursor, text);
                self.apply_search();
                self.sync_preview_to_selection();
            }
            Mode::Command => paste_into_field(&mut self.cmd_buf, &mut self.cmd_cursor, text),
            m if m.is_prompt() => paste_into_field(&mut self.prompt_buf, &mut self.prompt_cursor, text),
            _ => {
                if self.active.is_none() {
                    return;
                }
                if let Some(b) = self.buf_mut() {
                    b.save_undo();
                }
                self.edit(|b| b.paste(text));
                self.clear_needed = true;
            }
        }
    }

    /// `p` / `P` sem selecao: cola o clipboard do sistema (ou o ultimo yank).
    pub fn paste_normal_n(&mut self, n: usize) {
        if let Some(t) = clip::paste() {
            self.yank_linewise = t.contains('\n');
            self.yank = t;
        }
        if self.yank.is_empty() {
            self.set_status("Área de transferência vazia.");
            return;
        }
        let t = self.repeated_register(n);
        self.paste_text(&t);
        self.set_status("Colado da área de transferência");
    }

    // ---- substituir / buscar ----------------------------------------------

    pub fn replace_in_current(&mut self, find: &str, rep: &str) {
        if find.is_empty() {
            return;
        }
        let Some(b) = self.buf_mut() else { return };
        let mut count = 0;
        let mut undo_saved = false;
        for i in 0..b.lines.len() {
            let hits = b.lines[i].text.matches(find).count();
            if hits == 0 {
                continue;
            }
            if !undo_saved {
                // Snapshot so quando ha o que trocar: comando sem ocorrencias
                // nao consome undo nem apaga o redo.
                b.save_undo();
                undo_saved = true;
            }
            let new = b.lines[i].text.replace(find, rep);
            b.lines[i].set_text(new);
            count += hits;
        }
        if count > 0 {
            b.clamp_cursor();
            b.update_folding();
            self.mark_modified();
            self.clear_needed = true;
            self.set_status(format!("Substituição: {} ocorrência(s) de '{}' substituída(s)", count, find));
        } else {
            self.set_status(format!("Nenhuma ocorrencia de '{}' encontrada", find));
        }
    }

    /// `:/padrao`: primeira linha do documento que contem o padrao.
    pub fn search_from_top(&mut self, pat: &str) {
        self.last_search = pat.chars().take(FIELD_MAX).collect();
        let found = self
            .buf()
            .and_then(|b| b.lines.iter().position(|l| contains_caseless(&l.text, pat)));
        match found {
            Some(i) => {
                if let Some(b) = self.buf_mut() {
                    b.cy = i;
                    b.cx = 0;
                }
                self.set_status(format!("Padrão encontrado na linha {}", i + 1));
            }
            None => self.set_status(format!("Padrão não encontrado: {}", pat)),
        }
    }

    /// Prompt de localizar: do cursor para baixo e depois do topo.
    pub fn search_from_cursor(&mut self, pat: &str) {
        self.last_search = pat.chars().take(FIELD_MAX).collect();
        let cy = self.buf().map_or(0, |b| b.cy);
        match self.find_line(pat, cy, cy) {
            Some((i, wrapped)) => {
                if let Some(b) = self.buf_mut() {
                    b.cy = i;
                    b.cx = 0;
                }
                if wrapped {
                    self.set_status(format!("Padrão encontrado na linha {} (reinício)", i + 1));
                } else {
                    self.set_status(format!("Padrão encontrado na linha {} (use n para próximo)", i + 1));
                }
            }
            None => self.set_status(format!("Padrão não encontrado: {}", pat)),
        }
    }

    /// `n`: proxima ocorrencia do ultimo padrao.
    pub fn search_next(&mut self) {
        if self.last_search.is_empty() {
            return;
        }
        let pat = self.last_search.clone();
        let cy = self.buf().map_or(0, |b| b.cy);
        match self.find_line(&pat, cy + 1, cy + 1) {
            Some((i, wrapped)) => {
                if let Some(b) = self.buf_mut() {
                    b.cy = i;
                    b.cx = 0;
                }
                if wrapped {
                    self.set_status(format!("Ocorrência na linha {} (reinício)", i + 1));
                } else {
                    self.set_status(format!("Ocorrência na linha {}", i + 1));
                }
            }
            None => self.set_status(format!("Padrão não encontrado: {}", pat)),
        }
    }

    // ---- tabelas -----------------------------------------------------------

    pub fn table_align_at(&mut self, idx: usize) -> bool {
        let Some(b) = self.buf_mut() else { return false };
        if idx >= b.lines.len() {
            return false;
        }
        let mut target = idx;
        if !is_table_line(&b.lines[target].text) {
            if target > 0 && is_table_line(&b.lines[target - 1].text) {
                target -= 1;
            } else if target + 1 < b.lines.len() && is_table_line(&b.lines[target + 1].text) {
                target += 1;
            } else {
                return false;
            }
        }
        let (start, end) = table_bounds(b, target);
        if end <= start {
            return false;
        }
        b.save_undo();
        if align_range(b, start, end) {
            b.clamp_cursor();
            self.mark_modified();
            self.set_status(format!("Tabela alinhada ({} linhas)", end - start + 1));
        } else {
            self.set_status("Tabela com mais de 32 colunas: nada foi alterado");
        }
        true
    }

    /// Alinhamento silencioso da tabela que contem a linha `y` (ao deixa-la no modo
    /// Insercao). Nao faz nada se a tabela ja esta alinhada ou nao pode ser alinhada.
    pub fn table_autoalign(&mut self, y: usize) {
        let Some(b) = self.buf_mut() else { return };
        let y = y.min(b.lines.len().saturating_sub(1));
        let Some(y) = [y, y.saturating_sub(1)].into_iter().find(|&i| is_table_line(&b.lines[i].text)) else {
            return;
        };
        let (start, end) = table_bounds(b, y);
        if end > start && align_range_keep_cursor(b, start, end) == Some(true) {
            self.mark_modified();
        }
    }

    pub fn table_align_all(&mut self) {
        let Some(b) = self.buf_mut() else { return };
        let has_table = (0..b.lines.len()).any(|y| {
            is_table_line(&b.lines[y].text) && y + 1 < b.lines.len() && is_table_line(&b.lines[y + 1].text)
        });
        if !has_table {
            self.set_status("Nenhuma tabela com 2+ linhas encontrada nesta nota");
            return;
        }
        b.save_undo();
        let mut aligned = 0;
        let mut y = 0;
        while y < b.lines.len() {
            if is_table_line(&b.lines[y].text) {
                let start = y;
                while y + 1 < b.lines.len() && is_table_line(&b.lines[y + 1].text) {
                    y += 1;
                }
                if y > start && align_range(b, start, y) {
                    aligned += 1;
                }
            }
            y += 1;
        }
        b.clamp_cursor();
        if aligned > 0 {
            self.mark_modified();
            self.set_status(format!("{} tabela(s) alinhada(s)", aligned));
        } else {
            self.set_status("Nenhuma tabela encontrada nesta nota");
        }
    }

    /// Tab numa tabela: alinha e vai para a proxima celula (criando linha se
    /// `create_if_at_end`).
    pub fn table_jump_next_cell(&mut self, create_if_at_end: bool) {
        let Some(b) = self.buf_mut() else { return };
        b.clamp_cursor();
        if !is_table_line(&b.lines[b.cy].text) {
            return;
        }
        let (start, mut end) = table_bounds(b, b.cy);
        b.save_undo();
        align_range(b, start, end);
        b.clamp_cursor(); // o realinhamento pode ter mudado o texto sob o cursor
        let cols = table::count_columns(&b.lines[b.cy].text);
        let cur_text = b.lines[b.cy].text.to_string();

        // Proxima celula na mesma linha.
        if b.cx < cur_text.len() {
            if let Some(rel) = cur_text[b.cx..].find('|') {
                let p = b.cx + rel;
                if p + 1 < cur_text.len() && cur_text[p + 1..].contains('|') {
                    b.cx = cell_text_start(&cur_text, p + 1);
                    self.mark_modified();
                    return;
                }
            }
        }

        if b.cy < end {
            b.cy += 1;
            let np = b.lines[b.cy].text.trim_start_matches([' ', '\t']);
            let np = np.strip_prefix('|').unwrap_or(np).trim_start_matches([' ', '\t']);
            if np.starts_with('-') || np.starts_with(':') {
                // Linha delimitadora: pula para a seguinte (ou cria uma).
                if b.cy < end {
                    b.cy += 1;
                } else if create_if_at_end {
                    let Some(row) = table::blank_row(cols) else {
                        self.mark_modified();
                        return;
                    };
                    b.insert_line(b.cy + 1, &row);
                    end += 1;
                    align_range(b, start, end);
                    b.cy += 1;
                }
            }
            b.cx = after_first_bar(&b.lines[b.cy].text);
            self.mark_modified();
            return;
        }

        if create_if_at_end {
            if let Some(row) = table::blank_row(cols) {
                b.insert_line(b.cy + 1, &row);
                end += 1;
                align_range(b, start, end);
                b.cy += 1;
                b.cx = after_first_bar(&b.lines[b.cy].text);
            }
        }
        self.mark_modified();
    }

    /// Enter numa tabela (modo insercao): nova linha de tabela abaixo.
    pub fn table_insert_row_and_advance(&mut self) -> bool {
        let Some(b) = self.buf_mut() else { return false };
        b.clamp_cursor();
        if !is_table_line(&b.lines[b.cy].text) {
            return false;
        }
        let (start, end) = table_bounds(b, b.cy);
        let cols = table::count_columns(&b.lines[b.cy].text);
        let Some(row) = table::blank_row(cols) else { return false };
        b.save_undo();
        align_range(b, start, end);
        b.insert_line(b.cy + 1, &row);
        align_range(b, start, end + 1);
        b.cy += 1;
        b.cx = after_first_bar(&b.lines[b.cy].text);
        self.mark_modified();
        self.set_status("Nova linha de tabela inserida");
        true
    }

    // ---- links -------------------------------------------------------------

    pub fn activate_link(&mut self) {
        let Some(b) = self.buf() else { return };
        let (cx, text) = (b.cx, b.cur().text.to_string());
        let found = find_links(&text).into_iter().find(|s| cx >= s.start && cx <= s.end);
        let Some(span) = found else {
            self.set_status("Nenhum link sob o cursor (Enter em [título](url) ou [[nota]])");
            return;
        };
        if span.wiki {
            self.open_or_create_wikilink(&span.target);
        } else {
            let note_dir = match self.cur_key() {
                Some(NoteKey::File(p)) => p.parent().map(|d| d.to_path_buf()),
                _ => None,
            };
            let msg = links::open_target(&span.target, note_dir.as_deref());
            if !msg.is_empty() {
                self.set_status(msg);
            }
        }
    }

    pub fn jump_to_next_link(&mut self) {
        if self.buf().is_none() {
            return;
        }
        self.focus = Focus::Right;
        self.mode = Mode::Normal;
        let Some(b) = self.buf() else { return };
        let (cy, cx) = (b.cy, b.cx);
        let n = b.lines.len();
        // 1. resto da linha atual; 2. linhas seguintes; 3. volta ao topo;
        // 4. inicio da linha atual.
        let mut target = find_links(&b.lines[cy].text)
            .into_iter()
            .find(|s| s.start > cx)
            .map(|s| (cy, s));
        if target.is_none() {
            target = (cy + 1..n)
                .chain(0..cy)
                .find_map(|y| find_links(&b.lines[y].text).into_iter().next().map(|s| (y, s)));
        }
        if target.is_none() {
            target = find_links(&b.lines[cy].text)
                .into_iter()
                .next()
                .filter(|s| s.start != cx)
                .map(|s| (cy, s));
        }
        match target {
            Some((y, s)) => {
                if let Some(b) = self.buf_mut() {
                    b.cy = y;
                    b.cx = s.start;
                }
                self.editor_scroll();
                if s.wiki {
                    self.set_status(format!("WikiLink: [[{}]] (Enter para abrir)", s.target));
                } else {
                    self.set_status(format!("Link: [{}]({}) (Enter para abrir)", s.text, s.target));
                }
            }
            None => self.set_status("Nenhum link encontrado nesta nota"),
        }
    }
}
