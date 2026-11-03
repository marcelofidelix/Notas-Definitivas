//! Quebra de linha visual (soft wrap) e mapeamento entre posicao no texto e
//! posicao na tela. Com links/estilos ocultos, so o texto visivel ocupa colunas.

use crate::doc::{Buffer, Line, RowMemo};
use crate::md::{find_links, find_styles, is_table_line, LinkSpan, StyleSpan};
use crate::text::{cp_width, str_width, width_len};

#[derive(Clone, Copy, Debug)]
pub struct Hide {
    pub links: bool,
    pub styles: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub wrap: bool,
    pub avail_w: usize,
    pub hide: Hide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seg {
    pub off: usize,
    pub len: usize,
}

struct Spans {
    links: Vec<LinkSpan>,
    styles: Vec<StyleSpan>,
}

impl Spans {
    fn of(l: &Line, hide: Hide) -> Spans {
        let t = &l.text;
        Spans {
            links: if hide.links && t.contains('[') { find_links(t) } else { Vec::new() },
            styles: if !l.fence && hide.styles && t.contains('*') { find_styles(t) } else { Vec::new() },
        }
    }
    fn link_at(&self, p: usize) -> Option<&LinkSpan> {
        self.links.iter().find(|s| p >= s.start && p < s.end)
    }
    /// (abrindo?, trecho) se `p` cai num delimitador de estilo.
    fn style_delim_at(&self, p: usize) -> Option<(bool, &StyleSpan)> {
        for s in &self.styles {
            if p >= s.open_start && p < s.content_start {
                return Some((true, s));
            }
            if p >= s.close_start && p < s.close_start + s.close_len {
                return Some((false, s));
            }
        }
        None
    }
}

fn link_width(s: &LinkSpan) -> usize {
    str_width(&s.text) + if s.wiki { 2 } else { 0 }
}

/// Caractere em `p`: (tamanho em bytes, largura em colunas), juntando U+FE0F.
fn glyph(text: &str, p: usize) -> (usize, usize) {
    let mut it = text[p..].chars();
    let Some(c) = it.next() else { return (1, 0) };
    if c == '\t' {
        return (1, 4); // o desenho expande TAB em 4 espacos
    }
    let mut len = c.len_utf8();
    let mut w = cp_width(c as u32);
    if it.next() == Some('\u{FE0F}') {
        len += 3;
        w = 2;
    }
    (len, w)
}

fn plain_single(l: &Line, lay: &Layout) -> bool {
    // Cercas, blocos de codigo e tabelas nao quebram: sao cortados na borda.
    !lay.wrap
        || l.fence
        || l.in_code
        || (l.text.len() <= lay.avail_w && !l.text.contains('\t'))
        || is_table_line(&l.text)
}

/// Hash rapido (nao criptografico) do conteudo, para validar a memoria de layout.
fn fast_hash(b: &[u8]) -> u64 {
    const K: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut h = K ^ b.len() as u64;
    let mut chunks = b.chunks_exact(8);
    for c in &mut chunks {
        let mut w = [0u8; 8];
        w.copy_from_slice(c);
        h = (h ^ u64::from_le_bytes(w)).wrapping_mul(K).rotate_left(31);
    }
    let mut last = 0u64;
    for (i, &x) in chunks.remainder().iter().enumerate() {
        last |= u64::from(x) << (8 * i);
    }
    (h ^ last).wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ (h >> 29)
}

fn layout_key(lay: &Layout) -> u32 {
    1 | ((lay.avail_w.min(1 << 20) as u32) << 4) | (u32::from(lay.hide.links) << 3) | (u32::from(lay.hide.styles) << 2)
        | (u32::from(lay.wrap) << 1)
}

/// Numero de linhas visuais de uma linha. Linhas longas (que precisam de quebra)
/// guardam o resultado numa memoria validada pelo hash do conteudo: sem ela,
/// calcular a posicao do cursor refazia a quebra de TODAS as linhas anteriores
/// a cada quadro (centenas de ms em notas grandes).
pub fn nsegs(l: &Line, lay: &Layout) -> usize {
    if plain_single(l, lay) {
        return 1;
    }
    let (key, hash) = (layout_key(lay), fast_hash(l.text.as_bytes()));
    let m = l.memo.get();
    if m.n > 0 && m.key == key && m.hash == hash {
        return m.n as usize;
    }
    let n = segments(l, lay).len();
    l.memo.set(RowMemo { hash, key, n: n.min(u32::MAX as usize) as u32 });
    n
}

/// Segmentos (linhas visuais) de uma linha do documento.
pub fn segments(l: &Line, lay: &Layout) -> Vec<Seg> {
    let len = l.text.len();
    if len == 0 || plain_single(l, lay) {
        return vec![Seg { off: 0, len }];
    }
    let max_cols = lay.avail_w.max(5);
    let sp = Spans::of(l, lay.hide);
    let s: &str = &l.text;
    let b = s.as_bytes();
    let mut segs = Vec::new();
    let mut i = 0;
    while i < len {
        let seg_start = i;
        let mut cols = 0usize;
        let mut last_space: Option<(usize, usize)> = None;
        let mut p = i;
        while p < len {
            if let Some(link) = sp.link_at(p) {
                let w = link_width(link);
                if cols + w > max_cols && cols > 0 {
                    break;
                }
                cols += w;
                p = link.end;
                if p < len && b[p] == b' ' {
                    last_space = Some((p, p + 1));
                }
                continue;
            }
            if let Some((opening, st)) = sp.style_delim_at(p) {
                if opening {
                    p = st.content_start;
                } else {
                    p = st.close_start + st.close_len;
                    if p < len && b[p] == b' ' {
                        last_space = Some((p, p + 1));
                    }
                }
                continue;
            }
            if b[p] == b' ' && p > seg_start {
                last_space = Some((p, p + 1));
            }
            let (clen, w) = glyph(s, p);
            if cols + w > max_cols {
                break;
            }
            cols += w;
            p += clen;
        }
        if p >= len {
            segs.push(Seg { off: seg_start, len: len - seg_start });
            break;
        }
        match last_space {
            Some((sp_at, next)) if sp_at > seg_start => {
                segs.push(Seg { off: seg_start, len: sp_at - seg_start });
                i = next;
            }
            _ => {
                if p == seg_start {
                    // Nada coube (largura maior que a area): avanca um glifo.
                    p += glyph(s, p).0;
                }
                segs.push(Seg { off: seg_start, len: p - seg_start });
                i = p;
            }
        }
    }
    if segs.is_empty() {
        segs.push(Seg { off: 0, len: 0 });
    }
    segs
}

/// Colunas visiveis entre `seg_start` e `seg_start + x_in_seg`.
pub fn visual_col(l: &Line, seg_start: usize, x_in_seg: usize, hide: Hide) -> usize {
    if x_in_seg == 0 {
        return 0;
    }
    let sp = Spans::of(l, hide);
    let s: &str = &l.text;
    let target = (seg_start + x_in_seg).min(s.len());
    let mut p = seg_start;
    let mut cols = 0;
    while p < target {
        if let Some(link) = sp.link_at(p) {
            if p == link.start {
                if target >= link.end {
                    cols += link_width(link);
                } else {
                    cols += usize::from(link.wiki);
                    let inner = target.saturating_sub(link.start + if link.wiki { 2 } else { 1 });
                    cols += width_len(&link.text, inner.min(link.text.len()));
                }
            }
            p = link.end;
            continue;
        }
        if let Some((opening, st)) = sp.style_delim_at(p) {
            if opening {
                if target < st.content_start {
                    break;
                }
                p = st.content_start;
            } else {
                if target < st.close_start + st.close_len {
                    break;
                }
                p = st.close_start + st.close_len;
            }
            continue;
        }
        let (clen, w) = glyph(s, p);
        cols += w;
        p += clen;
    }
    cols
}

/// Deslocamento (relativo a `seg_start`) do caractere que cobre a coluna alvo.
pub fn byte_offset_in_seg(l: &Line, seg_start: usize, seg_len: usize, target_col: usize, hide: Hide) -> usize {
    if seg_len == 0 || target_col == 0 {
        return 0;
    }
    let sp = Spans::of(l, hide);
    let s: &str = &l.text;
    let end = (seg_start + seg_len).min(s.len());
    let mut cur = 0;
    let mut p = seg_start;
    while p < end && cur < target_col {
        if let Some(link) = sp.link_at(p) {
            let w = link_width(link);
            if cur + w > target_col {
                let rel = target_col - cur;
                let pre = usize::from(link.wiki);
                if rel <= pre {
                    return link.start - seg_start;
                }
                let b = crate::text::byte_offset_for_col(&link.text, rel - pre);
                return link.start + if link.wiki { 2 } else { 1 } + b - seg_start;
            }
            cur += w;
            p = link.end;
            continue;
        }
        if let Some((opening, st)) = sp.style_delim_at(p) {
            p = if opening { st.content_start } else { st.close_start + st.close_len };
            continue;
        }
        let (clen, w) = glyph(s, p);
        if cur + w > target_col {
            break;
        }
        cur += w;
        p += clen;
    }
    p.min(end) - seg_start
}

/// Uma linha visual da tela.
#[derive(Clone, Copy, Debug)]
pub struct VisRow {
    pub line: usize,
    pub sub: usize,
    pub seg: Seg,
}

/// Percorre linhas visuais a partir de uma linha visual alvo, sem refazer a
/// varredura do documento a cada linha da tela.
pub struct VRowIter<'a> {
    lines: &'a [Line],
    lay: Layout,
    line: usize,
    sub: usize,
    segs: Vec<Seg>,
}

impl<'a> VRowIter<'a> {
    pub fn locate(lines: &'a [Line], lay: Layout, target_vrow: usize) -> Option<VRowIter<'a>> {
        let mut vrow = 0;
        for (i, l) in lines.iter().enumerate() {
            if l.hidden {
                continue;
            }
            let n = nsegs(l, &lay);
            if target_vrow < vrow + n {
                let segs = segments(l, &lay);
                return Some(VRowIter {
                    lines,
                    lay,
                    line: i,
                    sub: target_vrow - vrow,
                    segs,
                });
            }
            vrow += n;
        }
        None
    }

    pub fn current(&self) -> VisRow {
        VisRow {
            line: self.line,
            sub: self.sub,
            seg: self.segs[self.sub.min(self.segs.len() - 1)],
        }
    }

    /// Avanca uma linha visual; `false` ao chegar ao fim do documento.
    pub fn advance(&mut self) -> bool {
        if self.sub + 1 < self.segs.len() {
            self.sub += 1;
            return true;
        }
        let mut next = self.line + 1;
        while next < self.lines.len() && self.lines[next].hidden {
            next += 1;
        }
        if next >= self.lines.len() {
            return false;
        }
        self.line = next;
        self.sub = 0;
        self.segs = segments(&self.lines[next], &self.lay);
        true
    }
}

/// Total de linhas visuais do documento.
pub fn total_vrows(lines: &[Line], lay: &Layout) -> usize {
    lines
        .iter()
        .filter(|l| !l.hidden)
        .map(|l| nsegs(l, lay))
        .sum()
}

pub fn find_visual_row(lines: &[Line], lay: Layout, target_vrow: usize) -> Option<VisRow> {
    VRowIter::locate(lines, lay, target_vrow).map(|it| it.current())
}

fn sub_for_cursor(segs: &[Seg], cx: usize) -> usize {
    let mut cur_sub = 0;
    for (s, seg) in segs.iter().enumerate() {
        let seg_end = seg.off + seg.len;
        if cx >= seg.off && (cx <= seg_end || s == segs.len() - 1) {
            cur_sub = s;
            if s + 1 < segs.len() && cx >= segs[s + 1].off {
                continue;
            }
            break;
        }
    }
    cur_sub
}

/// (linha visual, coluna visual) do cursor em relacao ao inicio do documento.
pub fn cursor_visual_pos(buf: &Buffer, lay: &Layout) -> (usize, usize) {
    let mut vrow = 0;
    for (i, l) in buf.lines.iter().enumerate() {
        if l.hidden {
            continue;
        }
        if i == buf.cy {
            let segs = segments(l, lay);
            let cur_sub = sub_for_cursor(&segs, buf.cx);
            let seg = segs[cur_sub];
            let x_in = buf.cx.saturating_sub(seg.off).min(seg.len);
            return (vrow + cur_sub, visual_col(l, seg.off, x_in, lay.hide));
        }
        vrow += nsegs(l, lay);
    }
    (vrow, 0)
}

/// Seta para baixo seguindo as linhas visuais.
pub fn move_down(buf: &mut Buffer, lay: &Layout) {
    buf.clamp_cursor();
    let l = &buf.lines[buf.cy];
    let segs = segments(l, lay);
    let cur_sub = sub_for_cursor(&segs, buf.cx);
    if cur_sub + 1 < segs.len() {
        let col = visual_col(l, segs[cur_sub].off, buf.cx.saturating_sub(segs[cur_sub].off), lay.hide);
        let next = segs[cur_sub + 1];
        buf.cx = next.off + byte_offset_in_seg(l, next.off, next.len, col, lay.hide);
    } else if buf.cy + 1 < buf.lines.len() {
        buf.cy += 1;
        while buf.cy + 1 < buf.lines.len() && buf.lines[buf.cy].hidden {
            buf.cy += 1;
        }
        buf.cx = buf.cx.min(buf.lines[buf.cy].len());
    }
    buf.clamp_cursor();
}

/// Seta para cima seguindo as linhas visuais.
pub fn move_up(buf: &mut Buffer, lay: &Layout) {
    buf.clamp_cursor();
    let l = &buf.lines[buf.cy];
    let segs = segments(l, lay);
    let cur_sub = sub_for_cursor(&segs, buf.cx);
    if cur_sub > 0 {
        let col = visual_col(l, segs[cur_sub].off, buf.cx.saturating_sub(segs[cur_sub].off), lay.hide);
        let prev = segs[cur_sub - 1];
        buf.cx = prev.off + byte_offset_in_seg(l, prev.off, prev.len, col, lay.hide);
    } else if buf.cy > 0 {
        buf.cy -= 1;
        while buf.cy > 0 && buf.lines[buf.cy].hidden {
            buf.cy -= 1;
        }
        buf.cx = buf.cx.min(buf.lines[buf.cy].len());
    }
    buf.clamp_cursor();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay(avail: usize, hide: bool) -> Layout {
        Layout {
            wrap: true,
            avail_w: avail,
            hide: Hide { links: hide, styles: hide },
        }
    }

    #[test]
    fn short_lines_are_one_segment() {
        let l = Line::new("curta");
        assert_eq!(segments(&l, &lay(20, false)), [Seg { off: 0, len: 5 }]);
        assert_eq!(segments(&Line::new(""), &lay(20, false)), [Seg { off: 0, len: 0 }]);
    }

    #[test]
    fn wraps_at_spaces_without_splitting_words() {
        let l = Line::new("aaaa bbbb cccc dddd");
        let segs = segments(&l, &lay(9, false));
        let parts: Vec<&str> = segs.iter().map(|s| &l.text[s.off..s.off + s.len]).collect();
        assert_eq!(parts, ["aaaa bbbb", "cccc dddd"]);
    }

    #[test]
    fn long_word_is_split_hard() {
        let l = Line::new("abcdefghijklmnop");
        let segs = segments(&l, &lay(5, false));
        assert_eq!(segs.len(), 4);
        assert_eq!(segs.iter().map(|s| s.len).sum::<usize>(), 16);
    }

    #[test]
    fn wide_chars_count_two_columns() {
        let l = Line::new("日本語日本語日本語");
        let segs = segments(&l, &lay(6, false));
        for s in &segs {
            assert!(str_width(&l.text[s.off..s.off + s.len]) <= 6);
        }
        assert_eq!(segs.len(), 3, "9 ideogramas de 2 colunas em linhas de 6 colunas");
    }

    #[test]
    fn hidden_link_urls_take_no_columns() {
        let l = Line::new("[abc](https://example.com/muito/longa/url) fim");
        assert_eq!(segments(&l, &lay(10, true)).len(), 1);
        assert!(segments(&l, &lay(10, false)).len() > 1);
    }

    #[test]
    fn visual_column_skips_hidden_delimiters() {
        let l = Line::new("a **bb** c");
        let hide = Hide { links: true, styles: true };
        // depois de "a **bb**" (8 bytes): visivel "a bb" = 4 colunas
        assert_eq!(visual_col(&l, 0, 8, hide), 4);
        assert_eq!(visual_col(&l, 0, 8, Hide { links: false, styles: false }), 8);
    }

    #[test]
    fn click_column_maps_back_to_byte_offsets() {
        let l = Line::new("a **bb** c");
        let hide = Hide { links: true, styles: true };
        assert_eq!(byte_offset_in_seg(&l, 0, l.len(), 3, hide), 5);
        let w = Line::new("日本x");
        let nohide = Hide { links: false, styles: false };
        assert_eq!(byte_offset_in_seg(&w, 0, w.len(), 2, nohide), 3);
        assert_eq!(byte_offset_in_seg(&w, 0, w.len(), 1, nohide), 0, "meio do glifo largo fica antes");
    }

    #[test]
    fn row_iteration_skips_hidden_lines_and_counts_wraps() {
        let mut b = Buffer::from_content("# T\nx\n## S\ny\nz\n");
        b.lines[0].folded = true;
        b.update_folding();
        let l = lay(40, false);
        assert_eq!(total_vrows(&b.lines, &l), 1);
        let b2 = Buffer::from_content("aaaa bbbb cccc\nz\n");
        let l2 = lay(9, false);
        assert_eq!(total_vrows(&b2.lines, &l2), 3);
        let mut it = VRowIter::locate(&b2.lines, l2, 1).unwrap();
        assert_eq!((it.current().line, it.current().sub), (0, 1));
        assert!(it.advance());
        assert_eq!(it.current().line, 1);
        assert!(!it.advance());
        assert!(VRowIter::locate(&b2.lines, l2, 3).is_none());
    }

    #[test]
    fn vertical_motion_follows_wrapped_rows() {
        let mut b = Buffer::from_content("aaaa bbbb cccc dddd\nfim\n");
        let l = lay(9, false);
        b.cx = 2;
        move_down(&mut b, &l);
        assert_eq!((b.cy, b.cx), (0, 12), "mesma coluna visual na linha de baixo");
        move_down(&mut b, &l);
        assert_eq!(b.cy, 1);
        move_up(&mut b, &l);
        assert_eq!(b.cy, 0);
        // Como no editor em C, a coluna nao e memorizada: "fim" so tem 3 colunas.
        let (vrow, vcol) = cursor_visual_pos(&b, &l);
        assert_eq!((vrow, vcol), (0, 3));
    }
}
