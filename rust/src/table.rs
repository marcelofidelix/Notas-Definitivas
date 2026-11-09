//! Alinhamento automatico de tabelas Markdown (estilo Vimwiki).

use crate::text::{normalize_text, str_width};
use std::cmp::Ordering;

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

/// Decimal exato, sem conversao para ponto flutuante nem limite de tamanho.
/// Ponto e virgula sao separadores decimais; separadores de milhar nao sao aceitos.
struct Number<'a> {
    negative: bool,
    integer: &'a str,
    fraction: &'a str,
}

impl<'a> Number<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let (negative, digits) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let mut parts = digits.split(['.', ',']);
        let integer = parts.next()?;
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some() || (integer.is_empty() && fraction.is_empty())
            || !integer.bytes().chain(fraction.bytes()).all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let integer = integer.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        Some(Self { negative: negative && !(integer.is_empty() && fraction.is_empty()), integer, fraction })
    }

    fn compare(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                let ord = self.integer.len().cmp(&other.integer.len())
                    .then_with(|| self.integer.cmp(other.integer))
                    .then_with(|| self.fraction.bytes().chain(std::iter::repeat(b'0'))
                        .zip(other.fraction.bytes().chain(std::iter::repeat(b'0')))
                        .take(self.fraction.len().max(other.fraction.len()))
                        .find_map(|(a, b)| (a != b).then(|| a.cmp(&b)))
                        .unwrap_or(Ordering::Equal));
                if self.negative { ord.reverse() } else { ord }
            }
        }
    }
}

/// Ordena as linhas de dados pela coluna `column` (0-based), mantendo no lugar
/// o cabecalho e as linhas delimitadoras, e realinha a tabela. `None` se a
/// tabela nao puder ser ordenada sem perder conteudo (ex.: coluna inexistente
/// ou mais de 32 colunas).
///
/// Colunas numericas sao ordenadas por valor; as demais, alfabeticamente sem
/// diferenciar maiusculas nem acentos. Vazias ficam no fim e empates sao estaveis.
pub fn sort_table(lines: &[&str], column: usize, descending: bool) -> Option<Vec<String>> {
    if lines.len() < 2 {
        return None;
    }
    let parsed: Vec<(usize, Row)> = lines.iter().map(|l| parse_row(l)).collect::<Option<_>>()?;
    let max_cols = parsed.iter().map(|(_, r)| r.cells.len()).max().unwrap_or(0);
    if column >= max_cols {
        return None;
    }
    // Se existe linha delimitadora, tudo ate ela (o cabecalho) fica fixo.
    // Delimitadoras em qualquer posicao nunca entram na ordenacao.
    let first_delim = parsed.iter().position(|(_, r)| r.is_delim);
    let movable: Vec<usize> = (0..lines.len())
        .filter(|&i| first_delim.map_or(true, |d| i > d) && !parsed[i].1.is_delim)
        .collect();
    if movable.is_empty() {
        return None;
    }

    let cell = |i: usize| parsed[i].1.cells.get(column).map(String::as_str).unwrap_or("");
    let numeric = movable.iter().any(|&i| !cell(i).is_empty())
        && movable.iter().all(|&i| cell(i).is_empty() || Number::parse(cell(i)).is_some());
    let mut ordered: Vec<(usize, bool, String, Option<Number<'_>>)> = movable
        .iter()
        .map(|&i| {
            let value = cell(i);
            (i, value.is_empty(), if numeric { String::new() } else { normalize_text(value) }, Number::parse(value))
        })
        .collect();
    ordered.sort_by(|a, b| {
        match (a.1, b.1) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => {
                let ord = match (&a.3, &b.3) {
                    (Some(x), Some(y)) if numeric => x.compare(y),
                    _ => a.2.cmp(&b.2),
                };
                let ord = if descending { ord.reverse() } else { ord };
                ord.then(a.0.cmp(&b.0))
            }
        }
    });

    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    for (slot, (src, _, _, _)) in movable.iter().zip(ordered) {
        out[*slot] = lines[src].to_string();
    }
    let refs: Vec<&str> = out.iter().map(String::as_str).collect();
    align_table(&refs)
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

    fn cell_values(lines: &[String], skip: usize) -> Vec<String> {
        lines
            .iter()
            .skip(skip)
            .map(|l| l.split('|').nth(1).unwrap_or("").trim().to_string())
            .collect()
    }

    #[test]
    fn sorts_data_rows_and_keeps_header_and_delimiter() {
        let table = ["| Nome | Idade |", "|:--|--:|", "| Beto | 25 |", "| ana | 30 |", "| Água | 5 |"];
        let asc = sort_table(&table, 0, false).unwrap();
        assert_eq!(cell_values(&asc, 2), ["Água", "ana", "Beto"]);
        let desc = sort_table(&table, 0, true).unwrap();
        assert_eq!(cell_values(&desc, 2), ["Beto", "ana", "Água"]);
        assert!(asc[0].contains("Nome") && asc[1].contains("--"), "{asc:?}");
    }

    #[test]
    fn sorts_by_any_column_and_puts_empty_cells_last() {
        let table = ["| a | 10 |", "| b |  |", "| c | 1 |"];
        let asc = sort_table(&table, 1, false).unwrap();
        assert_eq!(cell_values(&asc, 0), ["c", "a", "b"]);
        let desc = sort_table(&table, 1, true).unwrap();
        assert_eq!(cell_values(&desc, 0), ["a", "c", "b"], "vazias ficam no fim tambem de tras para frente");
        assert!(sort_table(&table, 2, false).is_none(), "coluna inexistente nao altera nada");
        assert!(sort_table(&["| a |"], 0, false).is_none(), "uma linha nao e uma tabela");
    }

    #[test]
    fn sorts_without_header_and_realligns() {
        let table = ["| b |", "| a |"];
        let out = sort_table(&table, 0, false).unwrap();
        assert_eq!(cell_values(&out, 0), ["a", "b"]);
        assert_eq!(out, ["| a   |", "| b   |"]);
    }

    #[test]
    fn numeric_sort_is_exact_stable_and_keeps_empty_cells_last() {
        let table = ["| Nome | Valor |", "|---|---|", "| dez | 10 |", "| vazio | |", "| dois | 2 |",
            "| negativo | -2,5 |", "| zero | -0 |", "| decimal | 2.01 |", "| igual | 02,000 |",
            "| grande | 999999999999999999999999999999 |", "| pequeno | 0,00000000000000000001 |"];
        let asc = sort_table(&table, 1, false).unwrap();
        assert_eq!(cell_values(&asc, 2), ["negativo", "zero", "pequeno", "dois", "igual", "decimal", "dez", "grande", "vazio"]);
        let desc = sort_table(&table, 1, true).unwrap();
        assert_eq!(cell_values(&desc, 2), ["grande", "dez", "decimal", "dois", "igual", "pequeno", "zero", "negativo", "vazio"]);
        assert!(asc[0].contains("Nome") && asc[1].contains("---"));
    }

    #[test]
    fn mixed_columns_keep_alphabetical_sorting() {
        let table = ["| 2 |", "| 10 |", "| texto |", "| |"];
        assert_eq!(cell_values(&sort_table(&table, 0, false).unwrap(), 0), ["10", "2", "texto", ""]);
        assert_eq!(cell_values(&sort_table(&table, 0, true).unwrap(), 0), ["texto", "2", "10", ""]);
    }
}
