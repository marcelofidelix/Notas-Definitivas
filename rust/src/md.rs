//! Analise de Markdown por linha: links `[t](url)` e `[[wiki]]`, negrito/italico
//! e marcadores de lista. Todas as posicoes sao deslocamentos em bytes dentro da
//! linha, sempre em limites de caractere (os delimitadores sao ASCII).

pub const MAX_SPANS: usize = 16;
/// Linhas maiores que isto sao tratadas como texto simples (sem links/estilos):
/// analisa-las a cada quadro travaria a interface.
pub const MAX_STYLED_LINE: usize = 20_000;

#[derive(Clone, Debug, PartialEq)]
pub struct LinkSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub target: String,
    pub wiki: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleKind {
    Italic,
    Bold,
    BoldItalic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleSpan {
    pub open_start: usize,
    pub content_start: usize,
    pub content_end: usize,
    pub close_start: usize,
    pub close_len: usize,
    pub kind: StyleKind,
}

/// Procura links Markdown e wikilinks (no maximo `MAX_SPANS`).
pub fn find_links(s: &str) -> Vec<LinkSpan> {
    if s.len() > MAX_STYLED_LINE {
        return Vec::new();
    }
    let b = s.as_bytes();
    let len = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < len && out.len() < MAX_SPANS {
        // 1. WikiLink [[Nota]]
        if b[i] == b'[' && i + 1 < len && b[i + 1] == b'[' {
            let start = i;
            i += 2;
            let txt_start = i;
            while i + 1 < len && !(b[i] == b']' && b[i + 1] == b']') {
                i += 1;
            }
            if i + 1 < len && b[i] == b']' && b[i + 1] == b']' {
                let tlen = i - txt_start;
                if tlen > 0 && tlen < 500 {
                    let text = s[txt_start..i].to_string();
                    out.push(LinkSpan {
                        start,
                        end: i + 2,
                        target: text.clone(),
                        text,
                        wiki: true,
                    });
                }
                i += 2;
                continue;
            }
            break; // "[[" sem fechamento: nada mais a procurar na linha
        }

        // 2. Link Markdown [Titulo](URL)
        if b[i] == b'[' {
            let start = i;
            i += 1;
            let txt_start = i;
            while i < len && b[i] != b']' {
                i += 1;
            }
            if i < len && b[i] == b']' && i + 1 < len && b[i + 1] == b'(' {
                let txt_end = i;
                i += 2;
                let link_start = i;
                let mut depth = 1;
                while i < len && depth > 0 {
                    if b[i] == b'(' {
                        depth += 1;
                    } else if b[i] == b')' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }
                if i < len && b[i] == b')' && depth == 0 {
                    let tlen = txt_end - txt_start;
                    let raw_llen = i - link_start;
                    if tlen > 0 && tlen < 500 && raw_llen > 0 && raw_llen < 4000 {
                        let mut l_start = link_start;
                        let mut l_end = i;
                        if b[l_start] == b'<' && l_end > l_start + 1 && b[l_end - 1] == b'>' {
                            l_start += 1;
                            l_end -= 1;
                        }
                        // Remove o atributo de titulo opcional: [t](url "titulo")
                        let mut llen = l_end - l_start;
                        let mut k = 0;
                        while k < llen {
                            if b[l_start + k] == b' ' || b[l_start + k] == b'\t' {
                                let mut nk = k + 1;
                                while nk < llen && (b[l_start + nk] == b' ' || b[l_start + nk] == b'\t') {
                                    nk += 1;
                                }
                                if nk < llen && (b[l_start + nk] == b'"' || b[l_start + nk] == b'\'') {
                                    llen = k;
                                    break;
                                }
                            }
                            k += 1;
                        }
                        if llen > 0 && llen < 4000 {
                            out.push(LinkSpan {
                                start,
                                end: i + 1,
                                text: s[txt_start..txt_end].to_string(),
                                target: s[l_start..l_start + llen].to_string(),
                                wiki: false,
                            });
                        }
                    }
                    i += 1;
                    continue;
                }
                break; // parenteses nao fechados
            }
            // "[" sem "](": segue a busca logo apos o "]" encontrado
            continue;
        }
        i += 1;
    }
    out
}

/// Procura trechos `*italico*`, `**negrito**` e `***ambos***` (ignora `codigo`).
pub fn find_styles(s: &str) -> Vec<StyleSpan> {
    if s.len() > MAX_STYLED_LINE {
        return Vec::new();
    }
    let b = s.as_bytes();
    let len = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < len && out.len() < MAX_SPANS {
        if b[i] == b'`' {
            i += 1;
            while i < len && b[i] != b'`' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }
        if b[i] != b'*' {
            i += 1;
            continue;
        }
        let mut dlen = 0;
        while i + dlen < len && b[i + dlen] == b'*' {
            dlen += 1;
        }
        if (1..=3).contains(&dlen) {
            let next = i + dlen;
            if next < len && !matches!(b[next], b' ' | b'\t' | b'\r' | b'\n') {
                let mut j = next;
                let mut found = false;
                while j < len {
                    if b[j] == b'*' {
                        let mut cdlen = 0;
                        while j + cdlen < len && b[j + cdlen] == b'*' {
                            cdlen += 1;
                        }
                        if cdlen == dlen && j > 0 && b[j - 1] != b' ' && b[j - 1] != b'\t' {
                            out.push(StyleSpan {
                                open_start: i,
                                content_start: i + dlen,
                                content_end: j,
                                close_start: j,
                                close_len: dlen,
                                kind: match dlen {
                                    3 => StyleKind::BoldItalic,
                                    2 => StyleKind::Bold,
                                    _ => StyleKind::Italic,
                                },
                            });
                            i = j + cdlen;
                            found = true;
                            break;
                        }
                        j += cdlen;
                    } else {
                        j += 1;
                    }
                }
                if found {
                    continue;
                }
            }
        }
        i += dlen;
    }
    out
}

/// Marcador de lista (`-` ou `*` seguido de espaco/fim), ignorando linhas
/// horizontais (`---`, `* * *`). Devolve (posicao, caractere).
pub fn bullet(text: &str, heading: u8, fence: bool, in_code: bool) -> Option<(usize, char)> {
    if text.is_empty() || heading > 0 || fence || in_code {
        return None;
    }
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    if i >= b.len() || (b[i] != b'*' && b[i] != b'-') {
        return None;
    }
    let ch = b[i];
    if !(i + 1 == b.len() || b[i + 1] == b' ' || b[i + 1] == b'\t') {
        return None;
    }
    let mut non_sp = 0;
    for &c in &b[i..] {
        if c != b' ' && c != b'\t' {
            if c == ch {
                non_sp += 1;
            } else {
                non_sp = 0;
                break;
            }
        }
    }
    if non_sp >= 3 {
        return None;
    }
    Some((i, ch as char))
}

/// Linha de tabela Markdown: comeca com `|` e tem outro `|` depois.
pub fn is_table_line(text: &str) -> bool {
    let t = text.trim_start_matches([' ', '\t']);
    t.starts_with('|') && t[1..].contains('|')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_link_with_title_and_angle_brackets() {
        let l = find_links("ver [site](https://x.com/a_(b) \"titulo\") e [b](<c d>)");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].text, "site");
        assert_eq!(l[0].target, "https://x.com/a_(b)");
        assert_eq!(l[1].target, "c d");
        assert!(!l[0].wiki);
    }

    #[test]
    fn wikilinks_and_unclosed_brackets() {
        let l = find_links("[[Nota A]] texto [[Nota B]]");
        assert_eq!(l.iter().map(|x| x.target.as_str()).collect::<Vec<_>>(), ["Nota A", "Nota B"]);
        assert!(l[0].wiki);
        assert!(find_links("[[sem fim").is_empty());
        assert!(find_links("[a](sem fim").is_empty());
        assert!(find_links("[]() [x]").is_empty(), "texto ou alvo vazios nao sao links");
    }

    #[test]
    fn link_spans_are_char_boundaries() {
        let s = "ção [ação](ç) fim";
        for l in find_links(s) {
            assert!(s.is_char_boundary(l.start) && s.is_char_boundary(l.end));
        }
    }

    #[test]
    fn styles() {
        let st = find_styles("a *it* **b** ***bi*** `*no*` * x*");
        let kinds: Vec<_> = st.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [StyleKind::Italic, StyleKind::Bold, StyleKind::BoldItalic]);
        assert_eq!(st[0].content_start, 3);
        assert_eq!(st[0].close_start, 5);
    }

    #[test]
    fn bullets_and_rules() {
        assert_eq!(bullet("- item", 0, false, false), Some((0, '-')));
        assert_eq!(bullet("  * item", 0, false, false), Some((2, '*')));
        assert_eq!(bullet("---", 0, false, false), None);
        assert_eq!(bullet("- - -", 0, false, false), None);
        assert_eq!(bullet("-x", 0, false, false), None);
        assert_eq!(bullet("- a", 1, false, false), None);
    }

    #[test]
    fn table_line_detection() {
        assert!(is_table_line("| a | b |"));
        assert!(is_table_line("  |a|"));
        assert!(!is_table_line("| so um"));
        assert!(!is_table_line("texto | com barra"));
    }
}
