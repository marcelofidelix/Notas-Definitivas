//! Documento em memoria: linhas, dobras de cabecalho, historico de desfazer e
//! operacoes de edicao. Todas as posicoes de cursor sao deslocamentos em bytes
//! e ficam sempre em limites de caractere UTF-8 (nunca no meio de um caractere).

use crate::history::{Snapshot, Stack};
use crate::md::is_table_line;
use crate::tbl::TableBlock;
use crate::text::{ceil_boundary, floor_boundary, next_char_end, prev_char_start};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const MAX_UNDO: usize = 100;
/// Janela para agrupar digitacao continua em um unico passo de undo.
pub const TYPING_UNDO_COALESCE: Duration = Duration::from_millis(800);

/// Memoria do numero de linhas visuais de uma linha, valida enquanto a
/// configuracao de layout (chave) for a mesma. E descartada sempre que o texto da
/// linha ou o seu bloco de tabela mudam — ver `set_text`, `edit` e
/// `assign_table_blocks`. Antes era validada por um hash do texto, recalculado a
/// cada consulta (custo por quadro proporcional ao documento).
#[derive(Clone, Copy, Debug, Default)]
pub struct RowMemo {
    pub key: u32,
    pub n: u32,
}

/// Uma linha do documento. O texto e compartilhado (`Rc<str>`): copiar o
/// documento para o historico de desfazer custa um contador por linha, nao uma
/// copia do texto. Para editar, use `edit`/`set_text`/`insert_str`/`delete_range`.
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub text: Rc<str>,
    /// 1..=6 se a linha comeca com `#`..`######` seguido de espaco, senao 0.
    pub heading: u8,
    pub folded: bool,
    pub hidden: bool,
    pub fence: bool,
    pub in_code: bool,
    pub memo: Cell<RowMemo>,
    /// Bloco de tabela (2+ linhas seguidas) a que a linha pertence, e seu indice nele.
    pub tb: Option<Rc<TableBlock>>,
    pub trow: u16,
}

impl Line {
    pub fn new(s: impl AsRef<str>) -> Line {
        let mut l = Line {
            text: Rc::from(s.as_ref()),
            ..Default::default()
        };
        l.detect_heading();
        l
    }

    /// Edita o texto como `String`. Custa uma copia da linha (nao do documento).
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut String) -> R) -> R {
        let mut s = String::from(&*self.text);
        let r = f(&mut s);
        self.text = Rc::from(s);
        self.memo.set(RowMemo::default());
        self.detect_heading();
        r
    }

    pub fn detect_heading(&mut self) {
        let b = self.text.as_bytes();
        let n = b.iter().take_while(|&&c| c == b'#').count();
        self.heading = if (1..=6).contains(&n) && n < b.len() && b[n] == b' ' {
            n as u8
        } else {
            0
        };
    }

    pub fn set_text(&mut self, s: String) {
        self.text = Rc::from(s);
        self.memo.set(RowMemo::default());
        self.detect_heading();
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn insert_str(&mut self, at: usize, s: &str) {
        let at = floor_boundary(&self.text, at);
        self.edit(|t| t.insert_str(at, s));
    }

    pub fn delete_range(&mut self, a: usize, b: usize) {
        let a = floor_boundary(&self.text, a);
        let b = ceil_boundary(&self.text, b.max(a));
        self.edit(|t| t.replace_range(a..b, ""));
    }
}

/// Marca cercas de codigo (```), blocos de codigo e linhas ocultas por dobras.
pub fn update_lines_syntax(lines: &mut [Line]) {
    let mut hide_lvl = 0u8;
    let mut in_code = false;
    for l in lines.iter_mut() {
        if l.text.trim_start_matches([' ', '\t']).starts_with("```") {
            l.fence = true;
            l.in_code = false;
            in_code = !in_code;
        } else {
            l.fence = false;
            l.in_code = in_code;
        }
        if l.heading > 0 && !l.in_code {
            if hide_lvl > 0 && l.heading <= hide_lvl {
                hide_lvl = 0;
            }
            if hide_lvl > 0 {
                l.hidden = true;
            } else {
                l.hidden = false;
                if l.folded {
                    hide_lvl = l.heading;
                }
            }
        } else {
            l.hidden = hide_lvl > 0;
        }
    }
    assign_table_blocks(lines);
}

/// Agrupa linhas de tabela consecutivas (fora de blocos de codigo) em blocos de
/// 2+ linhas. Um bloco igual ao anterior e reaproveitado, com a geometria ja
/// calculada; digitar fora de tabelas nao custa mais que uma varredura.
fn assign_table_blocks(lines: &mut [Line]) {
    let n = lines.len();
    let mut i = 0;
    while i < n {
        let is_tbl = |l: &Line| !l.fence && !l.in_code && is_table_line(&l.text);
        if !is_tbl(&lines[i]) {
            if lines[i].tb.is_some() {
                lines[i].tb = None;
                lines[i].memo.set(RowMemo::default());
            }
            i += 1;
            continue;
        }
        let start = i;
        while i < n && is_tbl(&lines[i]) {
            i += 1;
        }
        let run = &mut lines[start..i];
        if run.len() < 2 || run.len() > usize::from(u16::MAX) {
            for l in run.iter_mut() {
                if l.tb.is_some() {
                    l.tb = None;
                    l.memo.set(RowMemo::default());
                }
            }
            continue;
        }
        let reuse = run[0].tb.clone().filter(|b| b.same_rows(run));
        let blk = reuse.unwrap_or_else(|| TableBlock::new(run.iter().map(|l| l.text.clone()).collect()));
        for (k, l) in run.iter_mut().enumerate() {
            l.trow = k as u16;
            if !l.tb.as_ref().is_some_and(|b| Rc::ptr_eq(b, &blk)) {
                l.tb = Some(blk.clone());
                // Entrar/sair de uma tabela muda as linhas de tela da linha sem
                // tocar no texto: a memoria de layout precisa ser refeita.
                l.memo.set(RowMemo::default());
            }
        }
    }
}

/// Divide texto em linhas aceitando LF, CRLF e CR isolado.
pub fn split_any_newline(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\n' => {
                out.push(&text[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                out.push(&text[start..i]);
                i += if i + 1 < b.len() && b[i + 1] == b'\n' { 2 } else { 1 };
                start = i;
            }
            _ => i += 1,
        }
    }
    out.push(&text[start..]);
    out
}

struct Typing {
    deadline: Instant,
    line: usize,
    expect_x: usize,
}

#[derive(Default)]
pub struct Buffer {
    pub lines: Vec<Line>,
    pub cy: usize,
    pub cx: usize,
    pub scroll_y: usize,
    pub vis_y: usize,
    pub vis_x: usize,
    pub has_selection: bool,
    undo: Stack,
    redo: Stack,
    typing: Option<Typing>,
}

/// Teto de linhas guardadas no historico de desfazer (rede de seguranca de memoria).
const MAX_UNDO_LINES: usize = 2_000_000;

/// Memoria aproximada ocupada pelos historicos de desfazer/refazer.
#[cfg(test)]
pub fn approx_undo_bytes(b: &Buffer) -> usize {
    // O texto e compartilhado entre estados: o custo proprio e a estrutura das linhas.
    (b.undo.stored_lines() + b.redo.stored_lines()) * std::mem::size_of::<Line>()
}

#[derive(PartialEq, Eq, Debug)]
pub enum UndoResult {
    Done,
    Nothing,
}

impl Buffer {
    pub fn new_empty() -> Buffer {
        Buffer {
            lines: vec![Line::new("")],
            ..Default::default()
        }
    }

    /// Carrega texto: ignora BOM UTF-8, aceita CRLF e nao cria linha extra para
    /// o `\n` final. Nunca devolve um buffer sem linhas.
    pub fn from_content(content: &str) -> Buffer {
        let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);
        let mut lines: Vec<Line> = Vec::new();
        if !content.is_empty() {
            let body = content.strip_suffix('\n').unwrap_or(content);
            lines.extend(
                body.split('\n')
                    .map(|l| Line::new(l.trim_end_matches('\r'))),
            );
        }
        if lines.is_empty() {
            lines.push(Line::new(""));
        }
        let mut b = Buffer {
            lines,
            ..Default::default()
        };
        b.update_folding();
        b
    }

    pub fn to_content(&self) -> String {
        let cap: usize = self.lines.iter().map(|l| l.text.len() + 1).sum();
        let mut s = String::with_capacity(cap);
        for l in &self.lines {
            s.push_str(&l.text);
            s.push('\n');
        }
        s
    }

    pub fn update_folding(&mut self) {
        update_lines_syntax(&mut self.lines);
    }

    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }

    /// Garante `cy`/`cx` validos (e em limite de caractere).
    pub fn clamp_cursor(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(Line::new(""));
        }
        self.cy = self.cy.min(self.lines.len() - 1);
        let t = &self.lines[self.cy].text;
        self.cx = floor_boundary(t, self.cx.min(t.len()));
    }

    pub fn cur(&self) -> &Line {
        &self.lines[self.cy.min(self.lines.len() - 1)]
    }

    // ---- desfazer / refazer ------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cy: self.cy,
            cx: self.cx,
        }
    }

    pub fn invalidate_typing(&mut self) {
        self.typing = None;
    }

    pub fn save_undo(&mut self) {
        self.typing = None;
        if self.undo.len() >= MAX_UNDO {
            self.undo.drop_oldest();
        }
        let s = self.snapshot();
        self.undo.push(s);
        self.redo.clear();
        // Documentos enormes: descarta os niveis mais antigos alem do teto.
        while self.undo.stored_lines() > MAX_UNDO_LINES && self.undo.len() > 1 {
            self.undo.drop_oldest();
        }
    }

    /// Antes de inserir um caractere: teclas seguidas na mesma linha/posicao,
    /// dentro da janela, reaproveitam o snapshot (um "burst" vira um passo).
    pub fn save_undo_for_typing(&mut self, line: usize, at: usize, bytes: usize) {
        let now = Instant::now();
        if let Some(t) = &mut self.typing {
            if now <= t.deadline && t.line == line && t.expect_x == at {
                t.expect_x = at + bytes;
                t.deadline = now + TYPING_UNDO_COALESCE;
                return;
            }
        }
        self.save_undo();
        self.typing = Some(Typing {
            deadline: now + TYPING_UNDO_COALESCE,
            line,
            expect_x: at + bytes,
        });
    }

    #[cfg(test)]
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }
    #[cfg(test)]
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    fn restore(&mut self, s: Snapshot) {
        self.lines = s.lines;
        if self.lines.is_empty() {
            self.lines.push(Line::new(""));
        }
        self.cy = s.cy;
        self.cx = s.cx;
        self.update_folding();
        self.clamp_cursor();
    }

    pub fn apply_undo(&mut self) -> UndoResult {
        self.typing = None;
        let Some(prev) = self.undo.pop() else {
            return UndoResult::Nothing;
        };
        if self.redo.len() >= MAX_UNDO {
            self.redo.drop_oldest();
        }
        let cur = self.snapshot();
        self.redo.push(cur);
        self.restore(prev);
        UndoResult::Done
    }

    pub fn apply_redo(&mut self) -> UndoResult {
        self.typing = None;
        let Some(next) = self.redo.pop() else {
            return UndoResult::Nothing;
        };
        if self.undo.len() >= MAX_UNDO {
            self.undo.drop_oldest();
        }
        let cur = self.snapshot();
        self.undo.push(cur);
        self.restore(next);
        UndoResult::Done
    }

    // ---- dobras ------------------------------------------------------------

    /// Indice do cabecalho que governa a linha do cursor (ele proprio ou o
    /// anterior mais proximo).
    pub fn heading_for_cursor(&self) -> Option<usize> {
        if self.cy >= self.lines.len() {
            return None;
        }
        (0..=self.cy).rev().find(|&i| self.lines[i].heading > 0)
    }

    pub fn toggle_fold(&mut self) -> Option<bool> {
        let t = self.heading_for_cursor()?;
        self.lines[t].folded = !self.lines[t].folded;
        self.update_folding();
        let folded = self.lines[t].folded;
        if folded {
            self.cy = t;
        }
        self.clamp_cursor();
        Some(folded)
    }

    pub fn set_fold_at_cursor(&mut self, folded: bool) -> bool {
        let Some(t) = self.heading_for_cursor() else {
            return false;
        };
        self.lines[t].folded = folded;
        self.update_folding();
        if folded {
            self.cy = t;
        }
        self.clamp_cursor();
        true
    }

    /// Recolhe/expande todos os cabecalhos; devolve quantos existem.
    pub fn set_all_folds(&mut self, folded: bool) -> usize {
        let mut n = 0;
        for l in self.lines.iter_mut().filter(|l| l.heading > 0) {
            l.folded = folded;
            n += 1;
        }
        self.update_folding();
        if folded {
            while self.cy > 0 && self.lines[self.cy].hidden {
                self.cy -= 1;
            }
        }
        self.clamp_cursor();
        n
    }

    /// Salta para a linha `n` (1-based), desdobrando o que a esconder.
    pub fn goto_line(&mut self, n: usize) {
        if self.lines.is_empty() || n == 0 {
            return;
        }
        self.cy = n.min(self.lines.len()) - 1;
        self.cx = 0;
        if self.lines[self.cy].hidden {
            for i in (0..=self.cy).rev() {
                if self.lines[i].heading > 0 && self.lines[i].folded {
                    self.lines[i].folded = false;
                }
            }
            self.update_folding();
        }
    }

    // ---- edicao ------------------------------------------------------------

    /// Insere texto SEM quebras de linha no cursor e avanca o cursor.
    pub fn insert_at_cursor(&mut self, s: &str) {
        self.clamp_cursor();
        let (cy, cx) = (self.cy, self.cx);
        self.lines[cy].insert_str(cx, s);
        self.cx = cx + s.len();
    }

    /// Enter: divide a linha no cursor.
    pub fn split_line(&mut self) {
        self.clamp_cursor();
        let cx = self.cx;
        let tail = self.lines[self.cy].edit(|t| t.split_off(cx));
        self.lines.insert(self.cy + 1, Line::new(tail));
        self.cy += 1;
        self.cx = 0;
        self.update_folding();
    }

    /// Insere texto multilinha (LF/CRLF/CR) no cursor, de uma vez.
    pub fn paste(&mut self, text: &str) {
        self.clamp_cursor();
        let segs = split_any_newline(text);
        if segs.len() == 1 {
            self.insert_at_cursor(segs[0]);
            return;
        }
        let (cy, cx) = (self.cy, self.cx);
        let tail = self.lines[cy].edit(|t| {
            let tail = t.split_off(cx);
            t.push_str(segs[0]);
            tail
        });
        let last = segs.len() - 1;
        let mut new_lines: Vec<Line> = segs[1..last].iter().map(|s| Line::new(*s)).collect();
        let last_len = segs[last].len();
        new_lines.push(Line::new(format!("{}{}", segs[last], tail)));
        let added = new_lines.len();
        self.lines.splice(cy + 1..cy + 1, new_lines);
        self.cy = cy + added;
        self.cx = last_len;
        self.update_folding();
    }

    /// Cola um registro copiado por linha inteira: cada linha do texto entra como
    /// linha completa, abaixo (`below`) ou acima da linha do cursor — como o `p` e
    /// o `P` do Vim. Sem linha abaixo, a nova e criada no fim do texto. O cursor
    /// fica no primeiro caractere da primeira linha colada. Devolve quantas
    /// linhas entraram.
    pub fn paste_lines(&mut self, text: &str, below: bool) -> usize {
        self.clamp_cursor();
        let mut segs = split_any_newline(text);
        if segs.last().is_some_and(|s| s.is_empty()) {
            // A quebra final apenas fecha a ultima linha; nao cria outra vazia.
            segs.pop();
        }
        if segs.is_empty() {
            segs.push("");
        }
        let at = if below { (self.cy + 1).min(self.lines.len()) } else { self.cy };
        let count = segs.len();
        self.lines.splice(at..at, segs.into_iter().map(Line::new));
        self.cy = at;
        let first = &self.lines[at].text;
        self.cx = first.len() - first.trim_start_matches([' ', '\t']).len();
        self.update_folding();
        count
    }

    /// Backspace no modo insercao.
    pub fn backspace(&mut self) {
        self.clamp_cursor();
        if self.cx > 0 {
            let prev = prev_char_start(&self.lines[self.cy].text, self.cx);
            let cx = self.cx;
            self.lines[self.cy].delete_range(prev, cx);
            self.cx = prev;
        } else if self.cy > 0 {
            let cur = self.lines.remove(self.cy);
            self.cy -= 1;
            self.cx = self.lines[self.cy].len();
            let cy = self.cy;
            self.lines[cy].edit(|t| t.push_str(&cur.text));
            self.update_folding();
        }
    }

    /// Delete no modo insercao.
    pub fn delete_forward(&mut self) {
        self.clamp_cursor();
        let len = self.lines[self.cy].len();
        if self.cx < len {
            let end = next_char_end(&self.lines[self.cy].text, self.cx);
            let cx = self.cx;
            self.lines[self.cy].delete_range(cx, end);
        } else if self.cy + 1 < self.lines.len() {
            let next = self.lines.remove(self.cy + 1);
            let cy = self.cy;
            self.lines[cy].edit(|t| t.push_str(&next.text));
            self.update_folding();
        }
    }

    /// Remove `count` linhas a partir de `at`; nunca deixa o documento vazio.
    pub fn delete_lines(&mut self, at: usize, count: usize) {
        if at >= self.lines.len() || count == 0 {
            return;
        }
        let end = (at + count).min(self.lines.len());
        self.lines.drain(at..end);
        if self.lines.is_empty() {
            self.lines.push(Line::new(""));
        }
        self.update_folding();
    }

    pub fn insert_line(&mut self, at: usize, text: &str) {
        let at = at.min(self.lines.len());
        self.lines.insert(at, Line::new(text));
        self.update_folding();
    }

    // ---- selecao -----------------------------------------------------------

    /// Selecao normalizada: `(y0, x0, y1, x1)` com `x1` EXCLUSIVO (a selecao e
    /// inclusiva ate o caractere sob o cursor, como no Vim).
    pub fn selection(&self, linewise: bool) -> Option<(usize, usize, usize, usize)> {
        if self.lines.is_empty() {
            return None;
        }
        let (mut sy, mut sx, mut ey, mut ex) = (self.vis_y, self.vis_x, self.cy, self.cx);
        if sy > ey || (sy == ey && sx > ex) {
            std::mem::swap(&mut sy, &mut ey);
            std::mem::swap(&mut sx, &mut ex);
        }
        let ey = ey.min(self.lines.len() - 1);
        let sy = sy.min(ey);
        if linewise {
            return Some((sy, 0, ey, self.lines[ey].len()));
        }
        let st = &self.lines[sy].text;
        let sx = floor_boundary(st, sx.min(st.len()));
        let et = &self.lines[ey].text;
        let ex = if ex >= et.len() {
            et.len()
        } else {
            next_char_end(et, ex)
        };
        Some((sy, sx, ey, ex))
    }

    /// Texto selecionado (para copiar). Linhas inteiras terminam em `\n`.
    pub fn selection_text(&self, linewise: bool) -> Option<String> {
        let (sy, sx, ey, ex) = self.selection(linewise)?;
        let mut out = String::new();
        if linewise {
            for l in &self.lines[sy..=ey] {
                out.push_str(&l.text);
                out.push('\n');
            }
        } else if sy == ey {
            out.push_str(&self.lines[sy].text[sx..ex.max(sx)]);
        } else {
            out.push_str(&self.lines[sy].text[sx..]);
            out.push('\n');
            for l in &self.lines[sy + 1..ey] {
                out.push_str(&l.text);
                out.push('\n');
            }
            out.push_str(&self.lines[ey].text[..ex]);
        }
        Some(out)
    }

    /// Apaga a selecao. Devolve o numero de linhas removidas no modo linha.
    pub fn delete_selection(&mut self, linewise: bool, insert_mode: bool) -> Option<usize> {
        let (sy, sx, ey, ex) = self.selection(linewise)?;
        let mut removed = None;
        if linewise {
            self.delete_lines(sy, ey - sy + 1);
            self.cy = sy.min(self.lines.len() - 1);
            self.cx = 0;
            removed = Some(ey - sy + 1);
        } else if sy == ey {
            if sx < ex {
                self.lines[sy].delete_range(sx, ex);
            }
            self.cy = sy;
            self.cx = sx;
        } else {
            let keep = self.lines[sy].text[..sx].to_string();
            let suffix = self.lines[ey].text[ex..].to_string();
            self.lines[sy].set_text(keep + &suffix);
            self.lines.drain(sy + 1..=ey);
            self.cy = sy;
            self.cx = sx;
            self.update_folding();
        }
        self.update_folding();
        self.clamp_cursor();
        if !linewise {
            // Fora do modo insercao o cursor nao fica depois do ultimo caractere.
            let len = self.lines[self.cy].len();
            if !insert_mode && self.cx >= len && len > 0 {
                self.cx = prev_char_start(&self.lines[self.cy].text, len);
            }
        }
        removed
    }

    // ---- movimento por caractere / palavra --------------------------------

    pub fn move_left_char(&mut self) {
        self.cx = prev_char_start(&self.lines[self.cy].text, self.cx);
    }

    pub fn move_right_char(&mut self) {
        let t = &self.lines[self.cy].text;
        if self.cx < t.len() {
            self.cx = next_char_end(t, self.cx);
        }
    }

    /// Ctrl+Seta esquerda (palavras separadas por espaco).
    pub fn word_left(&mut self) {
        let t = &self.lines[self.cy].text;
        let mut x = self.cx;
        while x > 0 && t.as_bytes()[x - 1] == b' ' {
            x -= 1;
        }
        while x > 0 && t.as_bytes()[x - 1] != b' ' {
            x = prev_char_start(t, x);
        }
        self.cx = x;
    }

    /// Ctrl+Seta direita.
    pub fn word_right(&mut self) {
        let t = &self.lines[self.cy].text;
        let len = t.len();
        let mut x = self.cx;
        while x < len && t.as_bytes()[x] != b' ' {
            x = next_char_end(t, x);
        }
        while x < len && t.as_bytes()[x] == b' ' {
            x += 1;
        }
        self.cx = x;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_content(s)
    }
    fn texts(b: &Buffer) -> Vec<&str> {
        b.lines.iter().map(|l| &*l.text).collect()
    }

    #[test]
    fn load_and_save_roundtrip_rules() {
        assert_eq!(texts(&buf("a\r\nb\n")), ["a", "b"]);
        assert_eq!(texts(&buf("")), [""]);
        assert_eq!(texts(&buf("\n")), [""]);
        assert_eq!(texts(&buf("\u{FEFF}# T\n\nx")), ["# T", "", "x"]);
        assert_eq!(buf("a\nb").to_content(), "a\nb\n");
    }

    #[test]
    fn heading_detection() {
        assert_eq!(Line::new("## x").heading, 2);
        assert_eq!(Line::new("#x").heading, 0);
        assert_eq!(Line::new("####### x").heading, 0);
        assert_eq!(Line::new("# ").heading, 1);
        assert_eq!(Line::new("#").heading, 0);
    }

    #[test]
    fn folding_hides_until_same_or_higher_level() {
        let mut b = buf("# A\nx\n## B\ny\n# C\nz\n");
        b.lines[0].folded = true;
        b.update_folding();
        let hidden: Vec<bool> = b.lines.iter().map(|l| l.hidden).collect();
        assert_eq!(hidden, [false, true, true, true, false, false]);
        b.cy = 3;
        b.goto_line(4);
        assert!(!b.lines[0].folded, "ir para linha oculta desdobra");
    }

    #[test]
    fn headings_inside_code_blocks_do_not_fold() {
        let mut b = buf("# A\n```\n# nao\n```\nfim\n");
        assert!(b.lines[1].fence && b.lines[2].in_code && b.lines[3].fence);
        b.lines[0].folded = true;
        b.update_folding();
        assert!(b.lines[4].hidden);
    }

    #[test]
    fn undo_redo_roundtrip_and_limits() {
        let mut b = buf("a\nb\n");
        b.save_undo();
        b.lines[0].set_text("X".into());
        assert_eq!(b.apply_undo(), UndoResult::Done);
        assert_eq!(texts(&b), ["a", "b"]);
        assert_eq!(b.redo_depth(), 1);
        assert_eq!(b.apply_redo(), UndoResult::Done);
        assert_eq!(texts(&b), ["X", "b"]);
        assert_eq!(b.apply_redo(), UndoResult::Nothing);
        for _ in 0..(MAX_UNDO + 20) {
            b.save_undo();
        }
        assert_eq!(b.undo_depth(), MAX_UNDO);
        assert_eq!(b.redo_depth(), 0, "nova edicao descarta o redo");
    }

    /// O historico por diferencas se comporta como copias completas: edicoes
    /// aleatorias com desfazer/refazer, comparadas a um modelo ingenuo.
    #[test]
    fn undo_redo_matches_full_copies_under_random_edits() {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let mut b = buf("# T\n\n|a|b|\n|1|2|\n\nfim\n");
        let model = |b: &Buffer| -> Vec<String> { b.lines.iter().map(|l| l.text.to_string()).collect() };
        let (mut undo, mut redo): (Vec<Vec<String>>, Vec<Vec<String>>) = (Vec::new(), Vec::new());
        for step in 0..3000 {
            match rnd(8) {
                0..=3 => {
                    undo.push(model(&b));
                    redo.clear();
                    if undo.len() > MAX_UNDO {
                        undo.remove(0);
                    }
                    b.save_undo();
                    let n = b.lines.len();
                    match rnd(4) {
                        0 => b.lines[rnd(n)].set_text(format!("edit {step}")),
                        1 => b.insert_line(rnd(n + 1), &format!("nova {step}")),
                        2 if n > 1 => {
                            b.cy = rnd(n);
                            b.delete_lines(b.cy, 1);
                        }
                        _ => {
                            b.lines[rnd(n)].set_text(format!("| x{step} | y |"));
                        }
                    }
                    b.update_folding();
                    b.clamp_cursor();
                }
                4 | 5 => {
                    let before = model(&b);
                    let got = b.apply_undo();
                    match undo.pop() {
                        Some(prev) => {
                            assert_eq!(got, UndoResult::Done);
                            redo.push(before);
                            assert_eq!(model(&b), prev, "undo no passo {step}");
                        }
                        None => assert_eq!(got, UndoResult::Nothing),
                    }
                }
                _ => {
                    let before = model(&b);
                    let got = b.apply_redo();
                    match redo.pop() {
                        Some(next) => {
                            assert_eq!(got, UndoResult::Done);
                            assert_eq!(model(&b), next, "redo no passo {step}");
                            undo.push(before);
                        }
                        None => assert_eq!(got, UndoResult::Nothing),
                    }
                }
            }
            assert_eq!(b.undo_depth(), undo.len(), "passo {step}");
            assert_eq!(b.redo_depth(), redo.len(), "passo {step}");
        }
    }

    #[test]
    fn typing_burst_is_one_undo_step() {
        let mut b = buf("");
        for (i, c) in "abc".chars().enumerate() {
            b.save_undo_for_typing(0, i, 1);
            b.insert_at_cursor(&c.to_string());
        }
        assert_eq!(b.undo_depth(), 1);
        b.save_undo_for_typing(0, 99, 1);
        assert_eq!(b.undo_depth(), 2, "salto de posicao abre novo passo");
        b.save_undo_for_typing(1, 0, 1);
        assert_eq!(b.undo_depth(), 3, "troca de linha abre novo passo");
    }

    #[test]
    fn paste_lines_goes_below_or_above_and_creates_the_last_line() {
        // `p`: abaixo da linha do cursor.
        let mut b = buf("a\nb\nc");
        b.cy = 1;
        assert_eq!(b.paste_lines("X\n", true), 1);
        assert_eq!(texts(&b), ["a", "b", "X", "c"]);
        assert_eq!((b.cy, b.cx), (2, 0), "cursor na linha colada");

        // `P`: acima da linha do cursor.
        let mut b = buf("a\nb\nc");
        b.cy = 1;
        assert_eq!(b.paste_lines("X\n", false), 1);
        assert_eq!(texts(&b), ["a", "X", "b", "c"]);
        assert_eq!((b.cy, b.cx), (1, 0));

        // Varias linhas entram na ordem, como um bloco.
        let mut b = buf("a\nb");
        b.cy = 0;
        assert_eq!(b.paste_lines("X\nY\n", true), 2);
        assert_eq!(texts(&b), ["a", "X", "Y", "b"]);
        assert_eq!((b.cy, b.cx), (1, 0));

        // Sem linha abaixo, a linha nova e criada no fim.
        let mut b = buf("a\nb");
        b.cy = 1;
        assert_eq!(b.paste_lines("X\n", true), 1);
        assert_eq!(texts(&b), ["a", "b", "X"]);
        assert_eq!(b.cy, 2);
        let mut b = buf("unica");
        assert_eq!(b.paste_lines("X\nY\n", true), 2);
        assert_eq!(texts(&b), ["unica", "X", "Y"]);

        // O cursor fica no primeiro caractere nao branco da linha colada.
        let mut b = buf("a\nb");
        b.cy = 0;
        b.paste_lines("\t  - item\n", true);
        assert_eq!(texts(&b), ["a", "\t  - item", "b"]);
        assert_eq!(b.cx, 3);

        // Registro com linha vazia continua sendo uma linha.
        let mut b = buf("a\nb");
        b.cy = 0;
        assert_eq!(b.paste_lines("\n", true), 1);
        assert_eq!(texts(&b), ["a", "", "b"]);
    }

    #[test]
    fn multiline_paste_in_the_middle_of_a_line() {
        let mut b = buf("hello world");
        b.cx = 6;
        b.paste("A\r\nB\nC");
        assert_eq!(texts(&b), ["hello A", "B", "Cworld"]);
        assert_eq!((b.cy, b.cx), (2, 1));
        let mut b = buf("ab");
        b.cx = 1;
        b.paste("X\n");
        assert_eq!(texts(&b), ["aX", "b"]);
        assert_eq!((b.cy, b.cx), (1, 0));
        let mut b = buf("");
        b.paste("1\r2");
        assert_eq!(texts(&b), ["1", "2"]);
    }

    #[test]
    fn backspace_and_delete_respect_utf8_and_join_lines() {
        let mut b = buf("aç\nb");
        b.cx = 3; // depois do ç
        b.backspace();
        assert_eq!(texts(&b), ["a", "b"]);
        b.cy = 1;
        b.cx = 0;
        b.backspace();
        assert_eq!(texts(&b), ["ab"]);
        assert_eq!((b.cy, b.cx), (0, 1));
        let mut b = buf("😀x");
        b.delete_forward();
        assert_eq!(texts(&b), ["x"]);
    }

    #[test]
    fn selection_is_inclusive_of_the_cursor_character() {
        let mut b = buf("abcdef\nmeio\nxyz");
        b.vis_y = 0;
        b.vis_x = 2;
        b.cy = 2;
        b.cx = 1;
        assert_eq!(b.selection_text(false).unwrap(), "cdef\nmeio\nxy");
        b.delete_selection(false, false);
        assert_eq!(texts(&b), ["abz"]);
        assert_eq!((b.cy, b.cx), (0, 2), "cursor fica no ponto de juncao");
    }

    #[test]
    fn linewise_selection_and_delete() {
        let mut b = buf("a\nb\nc\nd");
        b.vis_y = 1;
        b.cy = 2;
        assert_eq!(b.selection_text(true).unwrap(), "b\nc\n");
        assert_eq!(b.delete_selection(true, false), Some(2));
        assert_eq!(texts(&b), ["a", "d"]);
        let mut b = buf("só");
        b.vis_y = 0;
        b.cy = 0;
        b.delete_selection(true, false);
        assert_eq!(texts(&b), [""], "apagar tudo deixa uma linha vazia");
    }

    #[test]
    fn selection_on_wide_chars_never_splits_them() {
        let mut b = buf("a😀b");
        b.vis_x = 2; // no meio do emoji
        b.cx = 2;
        let t = b.selection_text(false).unwrap();
        assert_eq!(t, "😀");
    }

    #[test]
    fn word_motion_in_insert_mode() {
        let mut b = buf("um dois três");
        b.cx = b.lines[0].len();
        b.word_left();
        assert_eq!(&b.lines[0].text[b.cx..], "três");
        b.word_left();
        assert_eq!(&b.lines[0].text[b.cx..], "dois três");
        b.word_right();
        assert_eq!(&b.lines[0].text[b.cx..], "três");
    }

    #[test]
    fn delete_lines_never_leaves_an_empty_buffer() {
        let mut b = buf("a\nb");
        b.delete_lines(0, 99);
        assert_eq!(texts(&b), [""]);
    }
}
