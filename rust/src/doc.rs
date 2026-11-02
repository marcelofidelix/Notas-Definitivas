//! Documento em memoria: linhas, dobras de cabecalho, historico de desfazer e
//! operacoes de edicao. Todas as posicoes de cursor sao deslocamentos em bytes
//! e ficam sempre em limites de caractere UTF-8 (nunca no meio de um caractere).

use crate::text::{ceil_boundary, floor_boundary, next_char_end, prev_char_start};
use std::time::{Duration, Instant};

pub const MAX_UNDO: usize = 100;
/// Janela para agrupar digitacao continua em um unico passo de undo.
pub const TYPING_UNDO_COALESCE: Duration = Duration::from_millis(800);

#[derive(Clone, Debug, Default)]
pub struct Line {
    pub text: String,
    /// 1..=6 se a linha comeca com `#`..`######` seguido de espaco, senao 0.
    pub heading: u8,
    pub folded: bool,
    pub hidden: bool,
    pub fence: bool,
    pub in_code: bool,
}

impl Line {
    pub fn new(s: impl Into<String>) -> Line {
        let mut l = Line {
            text: s.into(),
            ..Default::default()
        };
        l.detect_heading();
        l
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
        self.text = s;
        self.detect_heading();
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn insert_str(&mut self, at: usize, s: &str) {
        let at = floor_boundary(&self.text, at);
        self.text.insert_str(at, s);
        self.detect_heading();
    }

    pub fn delete_range(&mut self, a: usize, b: usize) {
        let a = floor_boundary(&self.text, a);
        let b = ceil_boundary(&self.text, b.max(a));
        self.text.replace_range(a..b, "");
        self.detect_heading();
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

#[derive(Clone)]
struct Snapshot {
    lines: Vec<Line>,
    cy: usize,
    cx: usize,
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
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    typing: Option<Typing>,
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
            self.undo.remove(0);
        }
        let s = self.snapshot();
        self.undo.push(s);
        self.redo.clear();
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
            self.redo.remove(0);
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
            self.undo.remove(0);
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
        let tail = self.lines[self.cy].text.split_off(self.cx);
        self.lines[self.cy].detect_heading();
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
        let tail = self.lines[cy].text.split_off(cx);
        self.lines[cy].text.push_str(segs[0]);
        self.lines[cy].detect_heading();
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
            self.lines[self.cy].text.push_str(&cur.text);
            self.lines[self.cy].detect_heading();
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
            self.lines[self.cy].text.push_str(&next.text);
            self.lines[self.cy].detect_heading();
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
        b.lines.iter().map(|l| l.text.as_str()).collect()
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
