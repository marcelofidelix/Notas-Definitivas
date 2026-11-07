//! Prefixo numerico dos comandos do modo Normal/Visual, como no Vim: `100l`,
//! `5j`, `3dd`, `d3d`, `5G`, `3x`, `3p`, `3u`, `3gt`, `3rx`, `3ox<Esc>`...
//!
//! O numero e acumulado em `App::count`; o proximo comando o consome. Toda
//! repeticao para assim que o comando deixa de ter efeito (fim da linha, fim da
//! nota, historico esgotado), entao `1000000j` nao trava.

use crate::app::{App, InsertRepeat, Mode};
use crate::doc::{Buffer, Line};
use crate::text::{floor_boundary, next_char_end, prev_char_start};

/// Maior prefixo aceito (digitar mais digitos nao passa disso).
pub const MAX_COUNT: usize = 1_000_000;
/// Limite de bytes que uma repeticao (`3p`, `5ix`) pode produzir.
const MAX_REPEAT_BYTES: usize = 16 * 1024 * 1024;

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

impl App {
    /// Acumula um digito do prefixo (`'1'..='9'`, ou `'0'` depois de outro digito).
    pub fn push_count_digit(&mut self, c: char) {
        let d = c as usize - '0' as usize;
        self.count = (self.count * 10 + d).min(MAX_COUNT);
        self.set_status(format!("Contagem: {}", self.count));
    }

    pub fn clear_count(&mut self) {
        self.count = 0;
        self.count2 = 0;
    }

    fn cursor_pos(&self) -> Option<(usize, usize)> {
        self.buf().map(|b| (b.cy, b.cx))
    }

    /// Repete um movimento do cursor ate `n` vezes ou ate ele nao sair do lugar.
    pub fn repeat_cursor(&mut self, n: usize, f: impl Fn(&mut Buffer)) {
        for _ in 0..n {
            let before = self.cursor_pos();
            if let Some(b) = self.buf_mut() {
                f(b);
            }
            if self.cursor_pos() == before {
                break;
            }
        }
    }

    /// `h`: `n` vezes. So o `h` simples, na coluna 0, passa o foco para a lista.
    pub fn repeat_left(&mut self, n: usize) {
        for _ in 0..n {
            let before = self.cursor_pos();
            self.normal_left(n == 1);
            if self.cursor_pos() == before {
                break;
            }
        }
    }

    pub fn repeat_right(&mut self, n: usize) {
        for _ in 0..n {
            let before = self.cursor_pos();
            self.normal_right();
            if self.cursor_pos() == before {
                break;
            }
        }
    }

    /// `w`: inicio da proxima palavra (passa para a linha seguinte no fim).
    pub fn word_forward(&mut self) {
        let Some(b) = self.buf_mut() else { return };
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
    }

    /// `b`: inicio da palavra anterior.
    pub fn word_back(&mut self) {
        let Some(b) = self.buf_mut() else { return };
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
    }

    /// `5G` / `5gg`: vai para a linha `line1` (1 = primeira); alem do fim, a ultima.
    pub fn goto_line(&mut self, line1: usize) {
        self.with_buf(|b| {
            let last = b.lines.len() - 1;
            b.cy = line1.saturating_sub(1).min(last);
            while b.cy < last && b.lines[b.cy].hidden {
                b.cy += 1;
            }
            while b.cy > 0 && b.lines[b.cy].hidden {
                b.cy -= 1;
            }
            b.cx = 0;
        });
        self.editor_scroll();
        self.clear_needed = true;
        let (y, total) = self.buf().map_or((1, 1), |b| (b.cy + 1, b.lines.len()));
        if y == total {
            self.set_status(format!("Fim da nota (linha {})", y));
        } else {
            self.set_status(format!("Linha {}", y));
        }
    }

    /// `3x`: recorta `n` caracteres a partir do cursor (sem passar do fim da linha).
    pub fn cut_chars(&mut self, n: usize) {
        let Some(b) = self.buf() else { return };
        let t = &b.cur().text;
        let start = b.cx;
        if start >= t.len() {
            return;
        }
        let mut end = start;
        let mut taken = 0;
        while taken < n && end < t.len() {
            end = next_char_end(t, end);
            taken += 1;
        }
        let cut = t[start..end].to_string();
        self.set_yank(cut, false);
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let cy = b.cy;
            b.lines[cy].delete_range(start, end);
            let len = b.lines[cy].len();
            if b.cx >= len && b.cx > 0 {
                b.cx = prev_char_start(&b.lines[cy].text, b.cx);
            }
        });
        self.set_status(if taken == 1 {
            "Caractere recortado para a área de transferência (x)".to_string()
        } else {
            format!("{} caracteres recortados para a área de transferência (x)", taken)
        });
    }

    /// Texto de `n` linhas a partir do cursor e quantas existem de fato.
    fn lines_from_cursor(&self, n: usize) -> Option<(String, usize)> {
        let b = self.buf()?;
        let n = n.max(1).min(b.lines.len() - b.cy);
        let text = b.lines[b.cy..b.cy + n].iter().map(|l| &*l.text).collect::<Vec<_>>().join("\n");
        Some((text, n))
    }

    /// `3dd` / `d3d`: apaga `n` linhas a partir do cursor (e as copia).
    pub fn delete_lines_at_cursor(&mut self, n: usize) {
        let Some((text, n)) = self.lines_from_cursor(n) else { return };
        self.set_yank(text, true);
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let cy = b.cy;
            b.delete_lines(cy, n);
            if b.cy >= b.lines.len() {
                b.cy = b.lines.len() - 1;
            }
            b.clamp_cursor();
        });
        self.set_status(if n == 1 { "1 linha excluída".to_string() } else { format!("{} linhas excluídas", n) });
    }

    /// `3y`: copia `n` linhas a partir do cursor.
    pub fn yank_lines_at_cursor(&mut self, n: usize) {
        let Some((text, n)) = self.lines_from_cursor(n) else { return };
        self.set_yank(text, true);
        self.set_status(if n == 1 {
            "1 linha copiada para a área de transferência (y)".to_string()
        } else {
            format!("{} linhas copiadas para a área de transferência (y)", n)
        });
    }

    /// `u` / `Ctrl+R` repetidos; para quando o historico acaba.
    pub fn repeat_undo(&mut self, n: usize, redo: bool) {
        let mut done = 0;
        for _ in 0..n {
            let r = self.with_buf(|b| if redo { b.apply_redo() } else { b.apply_undo() });
            if r != Some(crate::doc::UndoResult::Done) {
                break;
            }
            done += 1;
        }
        if done > 0 {
            self.mark_modified();
            let nome = if redo { "Refazer" } else { "Desfazer" };
            self.set_status(if done == 1 { format!("{} ({})", nome, if redo { "Ctrl+R" } else { "u" }) } else { format!("{} {} alterações", nome, done) });
        } else {
            self.set_status(if redo { "Já está na alteração mais recente" } else { "Já está na alteração mais antiga" });
        }
    }

    /// Texto a colar com `3p`: `n` copias do registro (em linhas separadas se ele
    /// foi copiado por linha), limitado a um tamanho razoavel.
    pub fn repeated_register(&self, n: usize) -> String {
        let one = &self.yank;
        let sep = if self.yank_linewise { "\n" } else { "" };
        let max = (MAX_REPEAT_BYTES / (one.len() + sep.len()).max(1)).max(1);
        vec![one.as_str(); n.clamp(1, max)].join(sep)
    }

    // ---- `3i`, `5o`: repeticao do que foi digitado -----------------------------

    /// Chamada logo depois de entrar em Insercao com um prefixo.
    pub fn arm_insert_repeat(&mut self, n: usize, lines: bool) {
        self.insert_repeat = (n > 1 && self.mode == Mode::Insert)
            .then(|| self.buf().map(|b| InsertRepeat { times: n, cy: b.cy, cx: b.cx, lines }))
            .flatten();
    }

    /// Ao sair da Insercao (Esc): repete `times - 1` vezes o que foi digitado. Se a
    /// digitacao passou por mais de uma linha, nao repete (como edicao complexa).
    pub fn apply_insert_repeat(&mut self) {
        let Some(r) = self.insert_repeat.take() else { return };
        let Some(b) = self.buf() else { return };
        if b.cy != r.cy || b.cx < r.cx || r.cx > b.cur().len() || !b.cur().text.is_char_boundary(r.cx) {
            return;
        }
        let extra = r.times - 1;
        if r.lines {
            let text = b.cur().text.to_string();
            let copies = extra.min(MAX_REPEAT_BYTES / (text.len() + 1)).min(MAX_COUNT);
            self.edit(|b| {
                let at = r.cy + 1;
                b.lines.splice(at..at, (0..copies).map(|_| Line::new(&text)));
                b.cy = r.cy + copies;
                b.cx = b.cur().len();
                b.update_folding();
            });
        } else {
            let typed = b.cur().text[r.cx..b.cx].to_string();
            if typed.is_empty() {
                return;
            }
            let copies = extra.min(MAX_REPEAT_BYTES / typed.len());
            let more = typed.repeat(copies);
            self.edit(|b| b.insert_at_cursor(&more));
        }
    }

    /// `3rx`: troca `n` caracteres a partir do cursor por `c`. Se a linha nao tem
    /// `n` caracteres daqui ate o fim, nada acontece (como no Vim).
    pub fn replace_chars(&mut self, n: usize, c: char) {
        let Some(b) = self.buf() else { return };
        let t = &b.cur().text;
        let start = floor_boundary(t, b.cx);
        let mut end = start;
        for _ in 0..n {
            if end >= t.len() {
                self.set_status("-- MODO NORMAL --");
                return;
            }
            end = next_char_end(t, end);
        }
        let rep = c.to_string().repeat(n);
        self.with_buf(|b| b.save_undo());
        self.edit(|b| {
            let cy = b.cy;
            b.lines[cy].edit(|t| t.replace_range(start..end, &rep));
            b.cx = start + rep.len() - c.len_utf8();
        });
        self.set_status("-- MODO NORMAL --");
    }
}
