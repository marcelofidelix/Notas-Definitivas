//! Alinhamento automatico de tabelas Markdown (estilo Vimwiki).

use crate::text::str_width;

pub const MAX_TABLE_COLS: usize = 32;

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

struct Row {
    cells: Vec<String>,
    is_delim: bool,
}

pub(crate) fn is_delimiter_cell(s: &str) -> bool {
    !s.is_empty()
        && s.contains('-')
        && s.chars().all(|c| matches!(c, '-' | ':' | ' ' | '\t'))
}

/// Parte a linha em celulas. `None` se tiver mais colunas do que o alinhador
/// representa (nada deve ser alterado, para nao perder colunas).
fn parse_row(text: &str) -> Option<(usize, Row)> {
    let mut indent = 0;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            ' ' => indent += 1,
            '\t' => indent += 4,
            _ => {
                start = i;
                break;
            }
        }
        start = i + 1;
    }
    let mut p = &text[start..];
    if let Some(r) = p.strip_prefix('|') {
        p = r;
    }
    let mut cells: Vec<String> = Vec::new();
    let mut all_delim = true;
    let mut rest = Some(p);
    while let Some(cur) = rest {
        if cur.is_empty() {
            break;
        }
        if cells.len() >= MAX_TABLE_COLS {
            if !cur.trim_matches([' ', '\t']).is_empty() {
                return None; // sobrou conteudo alem da 32a coluna
            }
            break;
        }
        let (cell, after) = match cur.find('|') {
            Some(i) => (&cur[..i], Some(&cur[i + 1..])),
            None => (cur, None),
        };
        let trimmed = cell.trim_matches([' ', '\t']).to_string();
        if !is_delimiter_cell(&trimmed) {
            all_delim = false;
        }
        cells.push(trimmed);
        rest = match after {
            Some(a) if !a.trim_matches([' ', '\t']).is_empty() => Some(a),
            _ => None,
        };
    }
    let is_delim = !cells.is_empty() && all_delim;
    Some((indent, Row { cells, is_delim }))
}

/// Realinha as linhas `lines[..]` de uma mesma tabela. Devolve `None` se a
/// tabela nao puder ser alinhada sem perder dados (mais de 32 colunas).
pub fn align_table(lines: &[&str]) -> Option<Vec<String>> {
    if lines.len() < 2 {
        return None;
    }
    let mut rows = Vec::with_capacity(lines.len());
    let mut base_indent: Option<usize> = None;
    for l in lines {
        let (indent, row) = parse_row(l)?;
        base_indent.get_or_insert(indent);
        rows.push(row);
    }
    let max_cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return None;
    }

    let mut aligns = vec![Align::Left; max_cols];
    for r in rows.iter().filter(|r| r.is_delim) {
        for (c, s) in r.cells.iter().enumerate() {
            let l_colon = s.starts_with(':');
            let r_colon = s.len() > 1 && s.ends_with(':');
            aligns[c] = if l_colon && r_colon {
                Align::Center
            } else if r_colon {
                Align::Right
            } else {
                Align::Left
            };
        }
    }

    let mut widths = vec![3usize; max_cols];
    for r in rows.iter().filter(|r| !r.is_delim) {
        for (c, s) in r.cells.iter().enumerate() {
            widths[c] = widths[c].max(str_width(s));
        }
    }

    let indent = " ".repeat(base_indent.unwrap_or(0).min(2000));
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        let mut line = indent.clone();
        for c in 0..max_cols {
            line.push_str("| ");
            let w = widths[c];
            if r.is_delim {
                match aligns[c] {
                    Align::Center => {
                        line.push(':');
                        line.push_str(&"-".repeat(w.saturating_sub(2)));
                        line.push(':');
                    }
                    Align::Right => {
                        line.push_str(&"-".repeat(w.saturating_sub(1)));
                        line.push(':');
                    }
                    Align::Left => {
                        line.push(':');
                        line.push_str(&"-".repeat(w.saturating_sub(1)));
                    }
                }
            } else {
                let cell = r.cells.get(c).map(String::as_str).unwrap_or("");
                let pad = w.saturating_sub(str_width(cell));
                match aligns[c] {
                    Align::Right => {
                        line.push_str(&" ".repeat(pad));
                        line.push_str(cell);
                    }
                    Align::Center => {
                        let left = pad / 2;
                        line.push_str(&" ".repeat(left));
                        line.push_str(cell);
                        line.push_str(&" ".repeat(pad - left));
                    }
                    Align::Left => {
                        line.push_str(cell);
                        line.push_str(&" ".repeat(pad));
                    }
                }
            }
            line.push(' ');
        }
        line.push('|');
        out.push(line);
    }
    Some(out)
}

/// Numero de colunas de uma linha de tabela (minimo 2 se vazia).
pub fn count_columns(text: &str) -> usize {
    if text.is_empty() {
        return 2;
    }
    let t = text.trim_matches([' ', '\t']);
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.matches('|').count() + 1
}

/// Linha vazia de tabela com `cols` colunas.
pub fn blank_row(cols: usize) -> Option<String> {
    let cols = if cols == 0 { 2 } else { cols };
    if cols > MAX_TABLE_COLS {
        return None;
    }
    Some(format!("|{}", "   |".repeat(cols)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_columns_and_delimiter_row() {
        let out = align_table(&["|a|bb|", "|-|:-:|", "|ccc|d|"]).unwrap();
        assert_eq!(out[0], "| a   | bb  |");
        assert_eq!(out[1], "| :-- | :-: |");
        assert_eq!(out[2], "| ccc |  d  |", "coluna centralizada pelo delimitador :-:");
    }

    #[test]
    fn right_and_center_alignment() {
        let out = align_table(&["| x | y |", "| --: | :--: |", "| 1 | 22 |"]).unwrap();
        assert_eq!(out[2], "|   1 | 22  |");
    }

    #[test]
    fn wide_characters_pad_by_display_width() {
        let out = align_table(&["| 日本 | a |", "| x | b |"]).unwrap();
        assert_eq!(out[0], "| 日本 | a   |");
        assert_eq!(out[1], "| x    | b   |");
    }

    #[test]
    fn indentation_of_first_row_is_kept() {
        let out = align_table(&["  | a | b |", "  | c | d |"]).unwrap();
        assert!(out.iter().all(|l| l.starts_with("  | ")));
    }

    #[test]
    fn more_than_32_columns_aborts_instead_of_losing_data() {
        let wide = format!("{}|", "| x ".repeat(33));
        assert!(align_table(&[wide.as_str(), "| a | b |"]).is_none());
    }

    #[test]
    fn single_line_is_not_a_table() {
        assert!(align_table(&["| a | b |"]).is_none());
    }

    #[test]
    fn column_counting_and_blank_rows() {
        assert_eq!(count_columns("| a | b | c |"), 3);
        assert_eq!(count_columns(""), 2);
        assert_eq!(blank_row(2).unwrap(), "|   |   |");
        assert!(blank_row(33).is_none());
    }
}
