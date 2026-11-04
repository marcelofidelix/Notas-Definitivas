//! Utilitarios de texto: largura de colunas, limites de caractere, normalizacao
//! para busca sem acentos e sanitizacao de saida para o terminal.
//!
//! As regras de largura espelham a versao em C (utf8_codepoint_width): marcas
//! combinantes e seletores de variacao valem 0, CJK/fullwidth/emoji valem 2, e
//! um U+FE0F logo apos um caractere forca largura 2.

/// Largura em colunas de um codepoint.
pub fn cp_width(cp: u32) -> usize {
    if cp == 0 {
        return 0;
    }
    if cp < 0x20 || (0x7F..0xA0).contains(&cp) {
        return 0;
    }
    if (0x0300..=0x036F).contains(&cp)
        || (0xFE00..=0xFE0F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || cp == 0x200B
        || cp == 0x200C
        || cp == 0x200D
        || cp == 0xFEFF
    {
        return 0;
    }
    if (cp >= 0x1100 && (cp <= 0x115F || cp == 0x2329 || cp == 0x232A))
        || (0x2E80..=0xA4CF).contains(&cp) && cp != 0x303F
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFE10..=0xFE19).contains(&cp)
        || (0xFE30..=0xFE6F).contains(&cp)
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
        || (0x1F000..=0x1FAFF).contains(&cp)
        || (0x20000..=0x3FFFF).contains(&cp)
    {
        return 2;
    }
    1
}

/// Largura do caractere `c`; `next` e o caractere seguinte (para U+FE0F).
/// Devolve `(largura, consumiu_fe0f)`.
#[inline]
pub fn char_width_with_next(c: char, next: Option<char>) -> (usize, bool) {
    let w = cp_width(c as u32);
    if next == Some('\u{FE0F}') {
        (2, true)
    } else {
        (w, false)
    }
}

/// Largura total de uma string em colunas.
pub fn str_width(s: &str) -> usize {
    let mut w = 0;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        let (cw, eat) = char_width_with_next(c, it.peek().copied());
        if eat {
            it.next();
        }
        w += cw;
    }
    w
}

/// Largura dos primeiros `len` bytes de `s` (alinhado ao limite de caractere).
pub fn width_len(s: &str, len: usize) -> usize {
    let end = floor_boundary(s, len.min(s.len()));
    str_width(&s[..end])
}

/// Maior indice <= `idx` que cai num limite de caractere.
pub fn floor_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Menor indice >= `idx` que cai num limite de caractere.
pub fn ceil_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Inicio do caractere imediatamente antes de `idx` (0 se `idx` == 0).
pub fn prev_char_start(s: &str, idx: usize) -> usize {
    let idx = floor_boundary(s, idx);
    if idx == 0 {
        return 0;
    }
    let mut i = idx - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Fim (exclusivo) do caractere que comeca em `idx` (ou `len` se no fim).
pub fn next_char_end(s: &str, idx: usize) -> usize {
    let idx = floor_boundary(s, idx);
    match s[idx..].chars().next() {
        Some(c) => idx + c.len_utf8(),
        None => s.len(),
    }
}

/// Hash estavel do texto (uso interno: detectar alteracao feita por outra
/// instancia quando o `mtime` empata). Nao e criptografico.
pub fn hash_text(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Todas as ocorrencias de `needle` em `haystack`, ignorando a caixa, como
/// intervalos de bytes `[inicio, fim)`.
///
/// Ocorrencias nao se sobrepoem (como no Vim): depois de um achado a busca segue
/// no fim dele. Caminho ASCII sem alocacao; fora do ASCII, dobra para minusculas
/// mantendo o mapa de volta para os bytes originais.
pub fn find_all_caseless(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    if needle.is_ascii() && haystack.is_ascii() {
        let (h, n) = (haystack.as_bytes(), needle.as_bytes());
        let mut out = Vec::new();
        let mut i = 0;
        while i + n.len() <= h.len() {
            if h[i..i + n.len()].eq_ignore_ascii_case(n) {
                out.push((i, i + n.len()));
                i += n.len();
            } else {
                i += 1;
            }
        }
        return out;
    }
    // Fora do ASCII: cada byte dobrado guarda o caractere original de origem.
    let (low, mapa) = lowered_with_map(haystack);
    let needle_low = needle.to_lowercase();
    if needle_low.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut from = 0;
    while from <= low.len() {
        let Some(rel) = low[from..].find(&needle_low) else { break };
        let a = from + rel;
        let b = a + needle_low.len();
        let (ini, _) = mapa[a];
        let (_, fim) = mapa[b - 1];
        out.push((ini, fim));
        from = b;
    }
    out
}

/// Minusculas de `s` e, para cada byte produzido, o intervalo do caractere
/// original que o gerou (alguns caracteres dobram para mais de um).
fn lowered_with_map(s: &str) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(s.len());
    let mut mapa = Vec::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        let fim = i + c.len_utf8();
        let mut buf = [0u8; 4];
        for lc in c.to_lowercase() {
            let piece = lc.encode_utf8(&mut buf);
            for _ in 0..piece.len() {
                mapa.push((i, fim));
            }
            out.push_str(piece);
        }
    }
    (out, mapa)
}

/// Byte onde a coluna visual `target_col` comeca (primeiro byte cuja coluna
/// acumulada alcanca `target_col`).
pub fn byte_offset_for_col(s: &str, target_col: usize) -> usize {
    if target_col == 0 {
        return 0;
    }
    let mut col = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if col >= target_col {
            return i;
        }
        let (cw, eat) = char_width_with_next(c, it.peek().map(|&(_, n)| n));
        if eat {
            it.next();
        }
        col += cw;
    }
    s.len()
}

/// Trunca para caber em `max_cols` colunas sem cortar caracteres.
pub fn truncate_display(s: &str, max_cols: usize) -> String {
    let mut out = String::new();
    let mut cols = 0;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        let (cw, eat) = char_width_with_next(c, it.peek().copied());
        if cols + cw > max_cols {
            break;
        }
        out.push(c);
        if eat {
            out.push(it.next().unwrap_or('\u{FE0F}'));
        }
        cols += cw;
    }
    out
}

/// Fatia de letreiro (marquee) para titulos longos: o titulo duplicado com 4
/// espacos no meio, deslocado `step` caracteres, preenchido ate `avail_w`.
pub fn marquee_slice(title: &str, step: usize, avail_w: usize) -> String {
    if avail_w == 0 {
        return String::new();
    }
    let doubled: Vec<char> = title
        .chars()
        .chain("    ".chars())
        .chain(title.chars())
        .collect();
    let mut out = String::new();
    let mut drawn = 0;
    for &c in doubled.iter().skip(step) {
        if drawn >= avail_w {
            break;
        }
        out.push(c);
        drawn += 1;
    }
    while drawn < avail_w {
        out.push(' ');
        drawn += 1;
    }
    out
}

fn fold_char(c: char) -> char {
    match c as u32 {
        0xC0..=0xC5 | 0xE0..=0xE5 | 0x100..=0x105 => 'a',
        0xC7 | 0xE7 | 0x106..=0x10D => 'c',
        0x10E..=0x111 => 'd',
        0xC8..=0xCB | 0xE8..=0xEB | 0x112..=0x11B => 'e',
        0x11C..=0x123 => 'g',
        0x124..=0x127 => 'h',
        0xCC..=0xCF | 0xEC..=0xEF | 0x128..=0x131 => 'i',
        0x13F..=0x142 => 'l',
        0xD1 | 0xF1 | 0x143..=0x149 => 'n',
        0xD2..=0xD6 | 0xF2..=0xF6 | 0x14C..=0x151 => 'o',
        0x154..=0x159 => 'r',
        0x15A..=0x161 => 's',
        0x162..=0x167 => 't',
        0xD9..=0xDC | 0xF9..=0xFC | 0x168..=0x173 => 'u',
        0x179..=0x17E => 'z',
        0xDD | 0xFD | 0xFF => 'y',
        _ => c.to_ascii_lowercase(),
    }
}

/// Minusculas e sem acentos, para busca insensivel a maiusculas e diacriticos.
pub fn normalize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    normalize_into(&mut out, s);
    out
}

/// Como `normalize_text`, acrescentando a `out`. O texto ASCII (a grande maioria)
/// e convertido por bytes, sem decodificar caractere a caractere.
pub fn normalize_into(out: &mut String, s: &str) {
    if s.is_ascii() {
        out.extend(s.bytes().map(|b| b.to_ascii_lowercase() as char));
        return;
    }
    out.extend(s.chars().map(|c| if c.is_ascii() { c.to_ascii_lowercase() } else { fold_char(c) }));
}

/// Remove bytes de controle (C0, DEL e C1 U+0080..U+009F) de texto de origem
/// nao confiavel antes de envia-lo ao terminal: um `.md` com ESC embutido nao
/// pode injetar sequencias (OSC 52, titulo da janela, reposicionamento).
pub fn sanitize_for_terminal(s: &str) -> std::borrow::Cow<'_, str> {
    let bad = |c: char| (c as u32) < 0x20 || c == '\x7f' || (0x80..=0x9F).contains(&(c as u32));
    if !s.chars().any(bad) {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(s.chars().filter(|&c| !bad(c)).collect())
}

/// Extensoes tratadas como markdown/texto na importacao. Sem extensao conta.
pub fn is_markdown_ext(fname: &str) -> bool {
    match fname.rfind('.') {
        None => true,
        Some(i) => {
            let ext = fname[i..].to_ascii_lowercase();
            matches!(ext.as_str(), ".md" | ".markdown" | ".mdown" | ".txt" | ".text")
        }
    }
}

/// Troca por `_` os caracteres invalidos em nomes de arquivo.
pub fn sanitize_note_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Tempo relativo curto para a lista lateral ("now", "5m", "2h", "3d", ...).
pub fn format_rel_time(mtime: i64, now: i64) -> String {
    let mtime = if mtime <= 0 { now } else { mtime };
    let diff = (now - mtime).max(0);
    if diff < 60 {
        "now".to_string()
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else if diff < 604800 {
        format!("{}d", diff / 86400)
    } else if diff < 2592000 {
        format!("{}w", diff / 604800)
    } else if diff < 31536000 {
        format!("{}mo", diff / 2592000)
    } else {
        format!("{}y", diff / 31536000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_follow_the_c_rules() {
        assert_eq!(str_width("abc"), 3);
        assert_eq!(str_width("日本"), 4);
        assert_eq!(str_width("e\u{0301}"), 1);
        assert_eq!(str_width("\u{2764}\u{FE0F}"), 2);
        assert_eq!(str_width("😀"), 2);
    }

    #[test]
    fn boundaries_never_split_chars() {
        let s = "aç😀b";
        assert_eq!(floor_boundary(s, 2), 1);
        assert_eq!(ceil_boundary(s, 2), 3);
        assert_eq!(prev_char_start(s, 3), 1);
        assert_eq!(next_char_end(s, 1), 3);
        assert_eq!(next_char_end(s, s.len()), s.len());
    }

    #[test]
    fn column_to_byte_offset() {
        assert_eq!(byte_offset_for_col("日本語", 2), 3);
        assert_eq!(byte_offset_for_col("abc", 10), 3);
        assert_eq!(byte_offset_for_col("abc", 0), 0);
    }

    #[test]
    fn find_all_finds_every_occurrence_case_insensitively() {
        // ASCII: todas as ocorrencias, sem sobrepor e sem alocar.
        assert_eq!(find_all_caseless("alvo e ALVO e alvo", "alvo"), [(0, 4), (7, 11), (14, 18)]);
        assert_eq!(find_all_caseless("aaaa", "aa"), [(0, 2), (2, 4)], "sem sobrepor");
        assert_eq!(find_all_caseless("abc", "x"), []);
        assert_eq!(find_all_caseless("abc", ""), []);
        assert_eq!(find_all_caseless("", "a"), []);
        assert_eq!(find_all_caseless("abc", "abcd"), []);
        // Acentos e ideogramas: o intervalo volta para os bytes originais.
        assert_eq!(find_all_caseless("Ação e ação", "ação"), [(0, 6), (9, 15)]);
        assert_eq!(find_all_caseless("日本語です", "です"), [(9, 15)]);
        assert_eq!(find_all_caseless("İİ", "i̇"), [(0, 2), (2, 4)], "caractere que dobra ao baixar");
        assert_eq!(find_all_caseless("ÜBER ÜBER", "über"), [(0, 5), (6, 11)]);
    }

    #[test]
    fn truncation_keeps_wide_chars_whole() {
        assert_eq!(truncate_display("日本語", 5), "日本");
        assert_eq!(truncate_display("abc", 2), "ab");
    }

    #[test]
    fn marquee_scrolls_and_pads() {
        assert_eq!(marquee_slice("ab", 0, 5), "ab   ");
        assert_eq!(marquee_slice("ab", 1, 6), "b    a");
        assert_eq!(marquee_slice("ab", 0, 0), "");
    }

    #[test]
    fn search_folding_matches_the_c_table() {
        assert_eq!(normalize_text("Ação Çá É Ñ Ü"), "acao ca e n u");
        assert_eq!(normalize_text("Łódź"), "lodz");
        assert_eq!(normalize_text("ŷ ÿ ý"), "ŷ y y");
    }

    #[test]
    fn sanitize_strips_controls_but_keeps_text() {
        assert_eq!(sanitize_for_terminal("a\x1b]52;c;QQ==\x07b"), "a]52;c;QQ==b");
        assert_eq!(sanitize_for_terminal("x\u{9b}y\u{a0}z"), "xy\u{a0}z");
        assert!(matches!(sanitize_for_terminal("limpo"), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn relative_time_buckets() {
        assert_eq!(format_rel_time(100, 100), "now");
        assert_eq!(format_rel_time(0, 100), "now");
        assert_eq!(format_rel_time(1, 62), "1m");
        assert_eq!(format_rel_time(1, 1 + 7200), "2h");
        assert_eq!(format_rel_time(1, 1 + 3 * 86400), "3d");
        assert_eq!(format_rel_time(1, 1 + 2 * 604800), "2w");
        assert_eq!(format_rel_time(1, 1 + 3 * 2592000), "3mo");
        assert_eq!(format_rel_time(1, 1 + 2 * 31536000), "2y");
    }

    #[test]
    fn markdown_extensions() {
        assert!(is_markdown_ext("a.MD"));
        assert!(is_markdown_ext("semext"));
        assert!(!is_markdown_ext("a.pdf"));
        assert_eq!(sanitize_note_name("a/b:c*d"), "a_b_c_d");
    }
}
