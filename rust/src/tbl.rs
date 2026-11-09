//! Tabelas Markdown desenhadas como grade (fora do modo Insercao).
//!
//! Um bloco de 2+ linhas de tabela consecutivas vira um `TableBlock`, guardado em
//! cada `Line` do bloco (ver `doc::update_lines_syntax`). A analise das celulas e a
//! geometria (largura das colunas e quebra de linha dentro das celulas para caber
//! na janela) sao calculadas sob demanda e memorizadas por largura disponivel.
//!
//! Cada linha do documento passa a ocupar varias linhas da tela: borda superior
//! (so na primeira linha), as linhas de conteudo (uma por linha de texto da celula
//! mais alta), um filete entre linhas quando alguma celula quebra, e a borda
//! inferior (so na ultima). O texto da nota nunca e alterado.

use crate::doc::Line;
use crate::md::{find_links, find_styles, LinkSpan, StyleKind, StyleSpan};
use crate::table::{is_delimiter_cell, MAX_TABLE_COLS};
use crate::text::cp_width;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

pub const A_BOLD: u8 = 1;
pub const A_ITALIC: u8 = 2;
pub const A_LINK: u8 = 4;
pub const A_WIKI: u8 = 8;

/// Largura minima da area de texto para desenhar a grade.
const MIN_AVAIL: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Um caractere visivel de uma celula e os bytes da linha que ele representa.
#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub b: u32,
    pub e: u32,
    pub ch: char,
    /// Seguido de U+FE0F (apresentacao emoji, 2 colunas).
    pub vs16: bool,
    pub w: u8,
    pub attr: u8,
}

#[derive(Debug)]
pub struct CellP {
    pub glyphs: Vec<Glyph>,
    /// Primeiro byte do conteudo (sem espacos); usado por celulas vazias.
    pub start: u32,
    /// Byte do `|` que fecha a celula (ou fim da linha): o cursor ate aqui e dela.
    pub zone_end: u32,
    /// Bit 0: `:` a esquerda; bit 1: `:` a direita (so em linhas delimitadoras).
    colons: u8,
}

#[derive(Debug)]
pub struct RowP {
    pub cells: Vec<CellP>,
    pub delim: bool,
}

#[derive(Debug)]
pub struct Parsed {
    pub rows: Vec<RowP>,
    pub ncols: usize,
    pub aligns: Vec<Align>,
    /// A primeira linha e cabecalho (a segunda e a linha delimitadora).
    pub header: bool,
    natural: Vec<usize>,
    word: Vec<usize>,
}

#[derive(Debug)]
pub struct RowGeo {
    /// Por coluna: faixas de glifos `[ini, fim)` de cada linha de texto da celula.
    pub wraps: Vec<Vec<(u32, u32)>>,
    /// Linhas de conteudo.
    pub h: usize,
    /// Borda superior antes do conteudo (so na primeira linha).
    pub pre: usize,
    /// Filete/borda inferior depois do conteudo.
    pub post: usize,
}

#[derive(Debug)]
pub struct Geo {
    pub widths: Vec<usize>,
    /// Largura total da grade, bordas incluidas.
    pub total: usize,
    pub rows: Vec<RowGeo>,
}

pub struct TableBlock {
    rows_src: Vec<Rc<str>>,
    parsed: OnceCell<Option<Parsed>>,
    geo: RefCell<Option<(usize, Option<Rc<Geo>>)>>,
}

impl std::fmt::Debug for TableBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableBlock").field("rows", &self.rows_src.len()).finish_non_exhaustive()
    }
}

impl TableBlock {
    pub fn new(rows: Vec<Rc<str>>) -> Rc<TableBlock> {
        Rc::new(TableBlock { rows_src: rows, parsed: OnceCell::new(), geo: RefCell::new(None) })
    }

    /// O bloco ainda descreve exatamente estas linhas?
    pub fn same_rows(&self, lines: &[Line]) -> bool {
        self.rows_src.len() == lines.len() && self.rows_src.iter().zip(lines).all(|(r, l)| Rc::ptr_eq(r, &l.text))
    }

    fn parsed(&self) -> Option<&Parsed> {
        self.parsed.get_or_init(|| Parsed::new(&self.rows_src)).as_ref()
    }

    fn geo(&self, avail: usize) -> Option<Rc<Geo>> {
        if let Some((a, g)) = &*self.geo.borrow() {
            if *a == avail {
                return g.clone();
            }
        }
        let g = self.parsed().and_then(|p| compute_geo(p, avail)).map(Rc::new);
        *self.geo.borrow_mut() = Some((avail, g.clone()));
        g
    }
}

// ---- analise ----------------------------------------------------------------

fn trim_range(t: &str, mut a: usize, mut z: usize) -> (usize, usize) {
    let b = t.as_bytes();
    while a < z && matches!(b[a], b' ' | b'\t') {
        a += 1;
    }
    while z > a && matches!(b[z - 1], b' ' | b'\t') {
        z -= 1;
    }
    (a, z)
}

fn push_glyph(out: &mut Vec<Glyph>, c: char, rest: &str, b: usize, e: usize, attr: u8) -> usize {
    // Devolve quantos bytes alem de `c` foram consumidos (U+FE0F).
    let c = if c == '\t' { ' ' } else { c };
    if c.is_control() {
        return 0;
    }
    let mut vs16 = false;
    let mut extra = 0;
    let mut w = cp_width(c as u32) as u8;
    if rest.starts_with('\u{FE0F}') {
        vs16 = true;
        extra = 3;
        w = 2;
    }
    out.push(Glyph { b: b as u32, e: (e + extra) as u32, ch: c, vs16, w, attr });
    extra
}

fn style_attr(styles: &[StyleSpan], p: usize) -> u8 {
    let mut a = 0;
    for s in styles.iter().filter(|s| p >= s.content_start && p < s.content_end) {
        if matches!(s.kind, StyleKind::Bold | StyleKind::BoldItalic) {
            a |= A_BOLD;
        }
        if matches!(s.kind, StyleKind::Italic | StyleKind::BoldItalic) {
            a |= A_ITALIC;
        }
    }
    a
}

fn push_link(out: &mut Vec<Glyph>, s: &str, a: usize, l: &LinkSpan, extra_attr: u8) {
    let attr = A_LINK | if l.wiki { A_WIKI } else { 0 } | extra_attr;
    let wiki_glyph = |ch: char, from: usize| Glyph {
        b: (a + from) as u32,
        e: (a + from + 2) as u32,
        ch,
        vs16: false,
        w: 1,
        attr,
    };
    if l.wiki {
        out.push(wiki_glyph('[', l.start));
    }
    let ts = l.start + if l.wiki { 2 } else { 1 };
    let literal = s.get(ts..ts + l.text.len()).is_some_and(|x| x == l.text);
    let mut it = l.text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let (b, e) = if literal { (a + ts + i, a + ts + i + c.len_utf8()) } else { (a + l.start, a + l.end) };
        let rest = &l.text[i + c.len_utf8()..];
        let extra = push_glyph(out, c, rest, b, e, attr);
        if extra > 0 {
            it.next(); // U+FE0F
        }
    }
    if l.wiki {
        out.push(wiki_glyph(']', l.end.saturating_sub(2)));
    }
}

fn build_glyphs(t: &str, a: usize, z: usize) -> Vec<Glyph> {
    let s = &t[a..z];
    let links = if s.contains('[') { find_links(s) } else { Vec::new() };
    let styles = if s.contains('*') { find_styles(s) } else { Vec::new() };
    let mut out = Vec::with_capacity(s.len());
    let mut p = 0;
    while p < s.len() {
        if let Some(l) = links.iter().find(|l| l.start == p && l.end > p) {
            push_link(&mut out, s, a, l, style_attr(&styles, p));
            p = l.end;
            continue;
        }
        if let Some(st) = styles.iter().find(|st| {
            (p >= st.open_start && p < st.content_start) || (p >= st.close_start && p < st.close_start + st.close_len)
        }) {
            p = if p < st.content_start { st.content_start } else { st.close_start + st.close_len };
            continue;
        }
        let c = s[p..].chars().next().unwrap_or(' ');
        let clen = c.len_utf8();
        let extra = push_glyph(&mut out, c, &s[p + clen..], a + p, a + p + clen, style_attr(&styles, p));
        p += clen + extra;
    }
    out
}

fn parse_row(t: &str) -> Option<RowP> {
    let b = t.as_bytes();
    let mut p = 0;
    while p < b.len() && matches!(b[p], b' ' | b'\t') {
        p += 1;
    }
    if p < b.len() && b[p] == b'|' {
        p += 1;
    }
    let mut cells = Vec::new();
    let mut all_delim = true;
    loop {
        let q = t[p..].find('|').map(|i| p + i);
        let end = q.unwrap_or(t.len());
        let (a, z) = trim_range(t, p, end);
        let raw = &t[a..z];
        let is_d = is_delimiter_cell(raw);
        all_delim &= is_d;
        let colons = if is_d { u8::from(raw.starts_with(':')) | (u8::from(raw.len() > 1 && raw.ends_with(':')) << 1) } else { 0 };
        cells.push(CellP { glyphs: build_glyphs(t, a, z), start: a as u32, zone_end: end as u32, colons });
        if cells.len() > MAX_TABLE_COLS {
            return None;
        }
        match q {
            None => break,
            Some(q) => {
                p = q + 1;
                if t[p..].trim_matches([' ', '\t']).is_empty() {
                    break;
                }
            }
        }
    }
    Some(RowP { delim: all_delim && !cells.is_empty(), cells })
}

fn glyphs_width(g: &[Glyph]) -> usize {
    g.iter().map(|g| usize::from(g.w)).sum()
}

fn looks_numeric(g: &[Glyph]) -> bool {
    !g.is_empty()
        && g.iter().any(|g| g.ch.is_ascii_digit())
        && g.iter().all(|g| g.ch.is_ascii_digit() || matches!(g.ch, '.' | ',' | '%' | '-' | '+' | '$' | ' ' | '€' | '£' | ':'))
}

impl Parsed {
    fn new(src: &[Rc<str>]) -> Option<Parsed> {
        let mut rows = Vec::with_capacity(src.len());
        for r in src {
            rows.push(parse_row(r)?);
        }
        let ncols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
        if ncols == 0 {
            return None;
        }
        let header = rows.len() > 1 && rows[1].delim && !rows[0].delim;

        // Alinhamento: declarado pela linha delimitadora; sem declaracao, colunas
        // so de numeros ficam a direita.
        let mut explicit: Vec<Option<Align>> = vec![None; ncols];
        for r in rows.iter().filter(|r| r.delim) {
            for (c, cell) in r.cells.iter().enumerate() {
                explicit[c] = match cell.colons {
                    3 => Some(Align::Center),
                    2 => Some(Align::Right),
                    1 => Some(Align::Left),
                    _ => explicit[c],
                };
            }
        }
        let body_from = if header { 2 } else { 0 };
        let aligns = (0..ncols)
            .map(|c| {
                explicit[c].unwrap_or_else(|| {
                    let cells: Vec<&CellP> = rows
                        .iter()
                        .enumerate()
                        .filter(|(i, r)| *i >= body_from && !r.delim)
                        .filter_map(|(_, r)| r.cells.get(c))
                        .filter(|cell| !cell.glyphs.is_empty())
                        .collect();
                    if !cells.is_empty() && cells.iter().all(|cell| looks_numeric(&cell.glyphs)) {
                        Align::Right
                    } else {
                        Align::Left
                    }
                })
            })
            .collect();

        let mut natural = vec![0usize; ncols];
        let mut word = vec![0usize; ncols];
        for r in rows.iter().filter(|r| !r.delim) {
            for (c, cell) in r.cells.iter().enumerate() {
                natural[c] = natural[c].max(glyphs_width(&cell.glyphs));
                let mut run = 0usize;
                for g in &cell.glyphs {
                    if g.ch == ' ' {
                        run = 0;
                    } else {
                        run += usize::from(g.w);
                        word[c] = word[c].max(run);
                    }
                }
            }
        }
        Some(Parsed { rows, ncols, aligns, header, natural, word })
    }
}

// ---- geometria --------------------------------------------------------------

/// Distribui `budget` colunas: tudo o que cabe fica na largura natural; senao as
/// colunas encolhem (a palavra mais longa, limitada, e o piso) e o que sobra e
/// repartido na proporcao do que cada uma ainda precisa.
fn fit_widths(nat: &[usize], word: &[usize], budget: usize) -> Vec<usize> {
    let n = nat.len();
    if nat.iter().sum::<usize>() <= budget {
        return nat.to_vec();
    }
    let mut w: Vec<usize> = Vec::new();
    for hi in [14usize, 6, 3, 1] {
        w = (0..n).map(|c| nat[c].min(word[c].clamp(1, hi))).collect();
        if w.iter().sum::<usize>() <= budget {
            break;
        }
    }
    let extra = budget.saturating_sub(w.iter().sum());
    let need: Vec<usize> = (0..n).map(|c| nat[c] - w[c]).collect();
    let total: usize = need.iter().sum();
    if total == 0 || extra == 0 {
        return w;
    }
    let mut given = 0;
    for c in 0..n {
        let add = extra * need[c] / total;
        w[c] += add;
        given += add;
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&c| std::cmp::Reverse(nat[c] - w[c]));
    let mut rem = extra - given;
    while rem > 0 {
        let mut moved = false;
        for &c in &order {
            if rem > 0 && w[c] < nat[c] {
                w[c] += 1;
                rem -= 1;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    w
}

/// Quebra os glifos de uma celula em linhas de ate `w` colunas: nos espacos, e no
/// meio da palavra so quando ela sozinha nao cabe.
fn wrap_glyphs(g: &[Glyph], w: usize) -> Vec<(u32, u32)> {
    let n = g.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let start = i;
        let mut cols = 0usize;
        let mut last_space: Option<usize> = None;
        let mut j = i;
        let mut next = n;
        let mut end = n;
        while j < n {
            let gw = usize::from(g[j].w);
            if g[j].ch == ' ' {
                last_space = Some(j);
            }
            if cols + gw > w && j > start {
                if g[j].ch == ' ' {
                    end = j;
                    next = j + 1;
                } else if let Some(sp) = last_space {
                    end = sp;
                    next = sp + 1;
                } else {
                    end = j;
                    next = j;
                }
                break;
            }
            cols += gw;
            j += 1;
        }
        while end > start && g[end - 1].ch == ' ' {
            end -= 1;
        }
        out.push((start as u32, end as u32));
        i = next;
        while i < n && g[i].ch == ' ' && next != n {
            i += 1;
        }
        if i >= n {
            break;
        }
    }
    out
}

fn compute_geo(p: &Parsed, avail: usize) -> Option<Geo> {
    if avail < MIN_AVAIL {
        return None;
    }
    let n = p.ncols;
    let budget = avail.checked_sub(3 * n + 1)?;
    if budget < n {
        return None;
    }
    let nat: Vec<usize> = p.natural.iter().map(|&w| w.max(1)).collect();
    let widths = fit_widths(&nat, &p.word, budget);
    let total = widths.iter().sum::<usize>() + 3 * n + 1;

    let mut rows: Vec<RowGeo> = Vec::with_capacity(p.rows.len());
    let mut any_wrap = false;
    for r in &p.rows {
        let wraps: Vec<Vec<(u32, u32)>> = (0..n)
            .map(|c| match r.cells.get(c) {
                Some(cell) if !r.delim => wrap_glyphs(&cell.glyphs, widths[c]),
                _ => Vec::new(),
            })
            .collect();
        let h = if r.delim { 1 } else { wraps.iter().map(Vec::len).max().unwrap_or(1).max(1) };
        any_wrap |= !r.delim && h > 1;
        rows.push(RowGeo { wraps, h, pre: 0, post: 0 });
    }
    let last = rows.len() - 1;
    for (i, row) in rows.iter_mut().enumerate() {
        row.pre = usize::from(i == 0);
        row.post = if i == last {
            1
        } else {
            usize::from(any_wrap && !p.rows[i].delim && !p.rows[i + 1].delim)
        };
    }
    Some(Geo { widths, total, rows })
}

// ---- visao de uma linha -----------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubKind {
    Top,
    Mid,
    Bottom,
    Delim,
    Content(usize),
}

pub struct CellLine<'a> {
    pub glyphs: &'a [Glyph],
    pub pad_l: usize,
    pub pad_r: usize,
}

/// Uma linha do documento vista como parte de uma grade.
pub struct TView {
    blk: Rc<TableBlock>,
    pub geo: Rc<Geo>,
    pub row: usize,
}

/// A visao da linha, se ela deve ser desenhada como grade para esta largura.
pub fn view(l: &Line, avail: usize) -> Option<TView> {
    let blk = l.tb.as_ref()?;
    let row = l.trow as usize;
    if l.fence || l.in_code || !blk.rows_src.get(row).is_some_and(|r| Rc::ptr_eq(r, &l.text)) {
        return None;
    }
    let geo = blk.geo(avail)?;
    Some(TView { blk: blk.clone(), geo, row })
}

impl TView {
    pub fn parsed(&self) -> &Parsed {
        // `view` so devolve blocos cuja analise deu certo (a geometria depende dela).
        self.blk.parsed().expect("bloco analisado")
    }

    fn rg(&self) -> &RowGeo {
        &self.geo.rows[self.row]
    }

    pub fn is_last_row(&self) -> bool {
        self.row + 1 == self.geo.rows.len()
    }

    /// Linhas da tela ocupadas por esta linha do documento.
    pub fn nsub(&self) -> usize {
        let r = self.rg();
        r.pre + r.h + r.post
    }

    /// Intervalo `[ini, fim)` das linhas de conteudo (onde o cursor pode ficar).
    pub fn content_range(&self) -> (usize, usize) {
        let r = self.rg();
        (r.pre, r.pre + r.h)
    }

    pub fn is_header(&self) -> bool {
        self.parsed().header && self.row == 0
    }

    pub fn is_delim(&self) -> bool {
        self.parsed().rows[self.row].delim
    }

    pub fn sub_kind(&self, sub: usize) -> SubKind {
        let r = self.rg();
        if sub < r.pre {
            SubKind::Top
        } else if sub < r.pre + r.h {
            if self.is_delim() {
                SubKind::Delim
            } else {
                SubKind::Content(sub - r.pre)
            }
        } else if self.is_last_row() {
            SubKind::Bottom
        } else {
            SubKind::Mid
        }
    }

    fn shift(&self, c: usize, line_w: usize) -> usize {
        let free = self.geo.widths[c].saturating_sub(line_w);
        match self.parsed().aligns[c] {
            Align::Left => 0,
            Align::Right => free,
            Align::Center => free / 2,
        }
    }

    /// Texto da linha `ci` da celula `c` com o recuo de alinhamento.
    pub fn cell_line(&self, c: usize, ci: usize) -> CellLine<'_> {
        let range = self.rg().wraps.get(c).and_then(|w| w.get(ci));
        let cell = self.parsed().rows[self.row].cells.get(c);
        let glyphs: &[Glyph] = match (cell, range) {
            (Some(cell), Some(&(s, e))) => &cell.glyphs[s as usize..e as usize],
            _ => &[],
        };
        let lw = glyphs_width(glyphs);
        let free = self.geo.widths[c].saturating_sub(lw);
        let pad_l = self.shift(c, lw);
        CellLine { glyphs, pad_l, pad_r: free - pad_l }
    }

    /// Coluna (relativa ao inicio da grade) onde comeca o `|` da esquerda da celula `c`.
    fn border_x(&self, c: usize) -> usize {
        self.geo.widths[..c].iter().map(|w| w + 3).sum()
    }
}

/// (linha da tela dentro desta linha do documento, coluna) do byte `cx`.
pub fn cursor_pos(tv: &TView, cx: usize) -> (usize, usize) {
    let rg = tv.rg();
    let row = &tv.parsed().rows[tv.row];
    if row.delim || row.cells.is_empty() {
        return (rg.pre, 2);
    }
    let c = row.cells.iter().position(|cell| cx <= cell.zone_end as usize).unwrap_or(row.cells.len() - 1);
    let cell = &row.cells[c];
    let gi = cell.glyphs.iter().position(|g| g.b as usize >= cx).unwrap_or(cell.glyphs.len());
    let lines = &rg.wraps[c];
    let x0 = tv.border_x(c) + 2;
    if lines.is_empty() {
        return (rg.pre, x0);
    }
    let li = lines.iter().position(|&(_, e)| gi < e as usize).unwrap_or(lines.len() - 1);
    let (s, e) = (lines[li].0 as usize, lines[li].1 as usize);
    let col = if gi <= s { 0 } else { glyphs_width(&cell.glyphs[s..gi.min(e)]) };
    let lw = glyphs_width(&cell.glyphs[s..e]);
    (rg.pre + li, x0 + tv.shift(c, lw) + col)
}

/// Setas verticais dentro da celula: o byte, na linha de texto de cima/baixo da
/// MESMA celula, que esta na mesma coluna da tela. `None` quando a celula nao tem
/// mais linhas nessa direcao (o cursor passa para a linha da tabela vizinha).
pub fn step_in_cell(tv: &TView, cx: usize, down: bool) -> Option<usize> {
    let rg = tv.rg();
    let row = &tv.parsed().rows[tv.row];
    if row.delim || row.cells.is_empty() {
        return None;
    }
    let c = row.cells.iter().position(|cell| cx <= cell.zone_end as usize).unwrap_or(row.cells.len() - 1);
    let cell = &row.cells[c];
    let lines = &rg.wraps[c];
    let gi = cell.glyphs.iter().position(|g| g.b as usize >= cx).unwrap_or(cell.glyphs.len());
    let li = lines.iter().position(|&(_, e)| gi < e as usize).unwrap_or(lines.len().checked_sub(1)?);
    let lj = if down { li + 1 } else { li.checked_sub(1)? };
    let &(ts, te) = lines.get(lj)?;
    let (s, e) = (lines[li].0 as usize, lines[li].1 as usize);
    let lw = glyphs_width(&cell.glyphs[s..e]);
    let col_in_text = if gi <= s { 0 } else { glyphs_width(&cell.glyphs[s..gi.min(e)]) };
    let screen_x = tv.shift(c, lw) + col_in_text;
    let (ts, te) = (ts as usize, te as usize);
    let tlw = glyphs_width(&cell.glyphs[ts..te]);
    Some(glyph_byte_at(cell, ts, te, screen_x.saturating_sub(tv.shift(c, tlw))))
}

/// Byte do glifo que cobre a coluna `target` (contada do inicio do texto) numa
/// linha de texto `[s, e)` da celula; o fim da linha se passar dele.
fn glyph_byte_at(cell: &CellP, s: usize, e: usize, target: usize) -> usize {
    if s >= e {
        return cell.glyphs.get(s).map_or(cell.start as usize, |g| g.b as usize);
    }
    let mut acc = 0;
    for g in &cell.glyphs[s..e] {
        if acc + usize::from(g.w) > target {
            return g.b as usize;
        }
        acc += usize::from(g.w);
    }
    cell.glyphs[e - 1].e as usize
}

/// Byte da linha sob a coluna `col` da linha da tela `sub`.
pub fn offset_at(tv: &TView, sub: usize, col: usize) -> usize {
    let rg = tv.rg();
    let row = &tv.parsed().rows[tv.row];
    let Some(first) = row.cells.first() else { return 0 };
    if row.delim {
        return first.start as usize;
    }
    let ci = sub.saturating_sub(rg.pre).min(rg.h - 1);
    let n = row.cells.len();
    let c = (0..n).rev().find(|&c| tv.border_x(c) <= col).unwrap_or(0);
    let cell = &row.cells[c];
    let Some(&(s, e)) = rg.wraps[c].get(ci) else {
        return cell.glyphs.last().map_or(cell.start as usize, |g| g.e as usize);
    };
    let (s, e) = (s as usize, e as usize);
    let lw = glyphs_width(&cell.glyphs[s..e.max(s)]);
    let target = col.saturating_sub(tv.border_x(c) + 2 + tv.shift(c, lw));
    glyph_byte_at(cell, s, e, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Buffer;

    fn block(rows: &[&str]) -> Vec<Line> {
        let b = Buffer::from_content(&rows.join("\n"));
        b.lines
    }

    fn views(lines: &[Line], avail: usize) -> Vec<TView> {
        lines.iter().map(|l| view(l, avail).expect("visao")).collect()
    }

    #[test]
    fn simple_table_fits_and_has_borders() {
        let lines = block(&["|a|b|c|", "|1|2|3|"]);
        let v = views(&lines, 40);
        assert_eq!(v[0].geo.widths, [1, 1, 1]);
        assert_eq!(v[0].geo.total, 3 + 3 * 3 + 1);
        // primeira linha: borda superior + conteudo; ultima: conteudo + borda inferior
        assert_eq!(v[0].nsub(), 2);
        assert_eq!(v[1].nsub(), 2);
        assert_eq!(v[0].sub_kind(0), SubKind::Top);
        assert_eq!(v[0].sub_kind(1), SubKind::Content(0));
        assert_eq!(v[1].sub_kind(1), SubKind::Bottom);
    }

    #[test]
    fn single_line_and_code_are_not_tables() {
        assert!(block(&["| a | b |"])[0].tb.is_none());
        let lines = block(&["```", "| a | b |", "| c | d |", "```"]);
        assert!(lines.iter().all(|l| l.tb.is_none()), "tabela dentro de bloco de codigo e texto");
    }

    #[test]
    fn long_cells_wrap_to_the_available_width() {
        let long = "palavra ".repeat(12);
        let lines = block(&["| Nome | Descricao |", "|---|---|", &format!("| x | {} |", long.trim())]);
        let v = views(&lines, 40);
        assert!(v[0].geo.total <= 40, "{}", v[0].geo.total);
        assert!(v[2].rg().h > 1, "a descricao quebra em varias linhas");
        // O texto some por inteiro: nenhuma palavra se perde na quebra.
        let cell = &v[2].parsed().rows[2].cells[1];
        let words: usize = v[2].rg().wraps[1]
            .iter()
            .map(|&(s, e)| cell.glyphs[s as usize..e as usize].iter().filter(|g| g.ch == ' ').count() + 1)
            .sum();
        assert_eq!(words, 12);
    }

    #[test]
    fn too_narrow_falls_back_to_raw() {
        let lines = block(&["|a|b|c|d|e|f|g|h|i|j|", "|1|2|3|4|5|6|7|8|9|0|"]);
        assert!(view(&lines[0], 20).is_none(), "10 colunas nao cabem em 20: texto cru");
        assert!(view(&lines[0], 80).is_some());
        assert!(view(&lines[0], 5).is_none());
    }

    #[test]
    fn header_alignment_and_numbers() {
        let lines = block(&["| Item | Qtd | Obs |", "|:--|--:|:-:|", "| a | 10 | x |"]);
        let v = views(&lines, 60);
        assert!(v[0].is_header());
        assert_eq!(v[0].parsed().aligns, [Align::Left, Align::Right, Align::Center]);
        assert_eq!(v[1].sub_kind(0), SubKind::Delim);
        let lines = block(&["| Item | Valor |", "|---|---|", "| a | 1,50 |", "| b | 12 |"]);
        assert_eq!(views(&lines, 60)[0].parsed().aligns, [Align::Left, Align::Right], "numeros a direita");
    }

    #[test]
    fn markup_is_hidden_inside_cells() {
        let lines = block(&["| **negrito** | [link](http://x.com/muito/longa) |", "| b | [[Nota]] |"]);
        let v = views(&lines, 60);
        let g: String = v[0].parsed().rows[0].cells[0].glyphs.iter().map(|g| g.ch).collect();
        assert_eq!(g, "negrito");
        let l: String = v[0].parsed().rows[0].cells[1].glyphs.iter().map(|g| g.ch).collect();
        assert_eq!(l, "link");
        let w: String = v[1].parsed().rows[1].cells[1].glyphs.iter().map(|g| g.ch).collect();
        assert_eq!(w, "[Nota]");
    }

    #[test]
    fn cursor_and_clicks_round_trip() {
        let lines = block(&["| abc | de |", "| f | ghi |"]);
        let v = views(&lines, 40);
        let text = &lines[0].text;
        for (i, _) in text.char_indices() {
            let (sub, col) = cursor_pos(&v[0], i);
            let back = offset_at(&v[0], sub, col);
            let (sub2, col2) = cursor_pos(&v[0], back);
            assert_eq!((sub, col), (sub2, col2), "byte {i} -> {back}");
        }
        // Clicar no "d" de "de" devolve o byte do "d".
        let d = text.find('d').unwrap();
        let (sub, col) = cursor_pos(&v[0], d);
        assert_eq!(offset_at(&v[0], sub, col), d);
    }

    #[test]
    fn cursor_in_wrapped_cell_follows_the_text() {
        let lines = block(&["| a | um dois tres quatro cinco seis sete |", "| b | c |"]);
        let v = views(&lines, 24);
        let rg = v[0].rg();
        assert!(rg.h >= 2);
        let text = &lines[0].text;
        let (s0, _) = cursor_pos(&v[0], text.find("um").unwrap());
        let (s1, _) = cursor_pos(&v[0], text.find("sete").unwrap());
        assert!(s1 > s0, "o cursor desce junto com o texto da celula");
        // Toda posicao de caractere fica numa linha de conteudo.
        let (lo, hi) = v[0].content_range();
        for (i, _) in text.char_indices() {
            let (s, _) = cursor_pos(&v[0], i);
            assert!(s >= lo && s < hi);
        }
    }

    #[test]
    fn empty_cells_and_ragged_rows_do_not_break() {
        let lines = block(&["|||", "| a |", "| a | b | c |"]);
        let v = views(&lines, 40);
        for (i, t) in v.iter().enumerate() {
            let text = &lines[i].text;
            for (j, _) in text.char_indices() {
                let (sub, col) = cursor_pos(t, j);
                let off = offset_at(t, sub, col);
                assert!(off <= text.len());
            }
        }
    }

    #[test]
    fn wrap_prefers_spaces_and_splits_long_words() {
        let mk = |s: &str| {
            let mut out = Vec::new();
            for (i, c) in s.chars().enumerate() {
                out.push(Glyph { b: i as u32, e: i as u32 + 1, ch: c, vs16: false, w: 1, attr: 0 });
            }
            out
        };
        let g = mk("aaa bbb ccc");
        assert_eq!(wrap_glyphs(&g, 7), [(0, 7), (8, 11)]);
        assert_eq!(wrap_glyphs(&g, 3), [(0, 3), (4, 7), (8, 11)]);
        let g = mk("abcdefghij");
        assert_eq!(wrap_glyphs(&g, 4), [(0, 4), (4, 8), (8, 10)]);
        assert_eq!(wrap_glyphs(&mk(""), 4), Vec::<(u32, u32)>::new());
    }

    #[test]
    fn fit_widths_shrinks_the_wide_column_first() {
        let w = fit_widths(&[4, 60], &[4, 8], 40);
        assert_eq!(w.iter().sum::<usize>(), 40);
        assert_eq!(w[0], 4, "coluna curta mantem a largura natural");
        assert_eq!(fit_widths(&[3, 5], &[3, 5], 20), [3, 5]);
    }
}
