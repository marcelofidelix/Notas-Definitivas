//! Catalogo de notas: identidade, ordenacao por prioridade e busca sem acentos.

use crate::text::normalize_text;
use std::cmp::Ordering;
use std::path::PathBuf;

/// Identidade de uma nota: linha do banco ou (fallback) arquivo avulso aberto
/// pela linha de comando.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NoteKey {
    Db(i64),
    File(PathBuf),
}

impl NoteKey {
    pub fn db_id(&self) -> Option<i64> {
        match self {
            NoteKey::Db(id) => Some(*id),
            NoteKey::File(_) => None,
        }
    }
}

impl std::fmt::Display for NoteKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteKey::Db(id) => write!(f, "db:{}", id),
            NoteKey::File(p) => write!(f, "{}", p.display()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NoteInfo {
    pub key: NoteKey,
    /// Nome de exibicao: "<nome no banco>.md".
    pub filename: String,
    /// Titulo limpo (sem extensao).
    pub title: String,
    pub mtime: i64,
    /// 0 = nenhuma, 1..=3 urgencia, 4 = [H], 5 = [R], 6 = [O].
    pub attention: u8,
    /// Texto normalizado (minusculas, sem acentos) para a busca.
    pub search_norm: String,
}

/// Remove a extensao de markdown/texto do nome.
pub fn title_from_filename(fname: &str) -> String {
    if let Some(i) = fname.rfind('.') {
        let ext = fname[i..].to_ascii_lowercase();
        if matches!(ext.as_str(), ".md" | ".markdown" | ".mdown" | ".txt" | ".text") {
            return fname[..i].to_string();
        }
    }
    fname.to_string()
}

/// Normaliza apenas o conteudo da nota. Fica gravado no banco (`content_norm`)
/// para que abrir o programa nao precise reler e normalizar todas as notas.
pub fn build_content_norm(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    crate::text::normalize_into(&mut out, content);
    out
}

/// Indice de busca da nota: nome, titulo e conteudo ja normalizado. Nome e titulo
/// entram aqui porque mudam em `:rename` sem tocar no conteudo.
pub fn search_norm(filename: &str, title: &str, content_norm: &str) -> String {
    let mut out = String::with_capacity(filename.len() + title.len() + content_norm.len() + 2);
    crate::text::normalize_into(&mut out, filename);
    out.push(' ');
    crate::text::normalize_into(&mut out, title);
    out.push(' ');
    out.push_str(content_norm);
    out
}

/// Indice de busca montado a partir do conteudo cru (testes e reconstrucao).
pub fn build_search_norm(filename: &str, title: &str, content: &str) -> String {
    search_norm(filename, title, &build_content_norm(content))
}

/// Ordem da lista: grupo de urgencia (1..6, depois sem marcacao), mais
/// recente primeiro, e por nome para desempate estavel.
pub fn compare_notes(a: &NoteInfo, b: &NoteInfo) -> Ordering {
    let rank = |n: &NoteInfo| if (1..=6).contains(&n.attention) { n.attention - 1 } else { 6 };
    rank(a)
        .cmp(&rank(b))
        .then(b.mtime.cmp(&a.mtime))
        .then_with(|| a.filename.as_bytes().cmp(b.filename.as_bytes()))
}

/// Palavras da busca (ate 32), normalizadas.
pub fn query_words(query: &str) -> Vec<String> {
    normalize_text(query)
        .split(|c: char| " \t\r\n,;\"'()[]{}!?".contains(c))
        .filter(|w| !w.is_empty())
        .take(32)
        .map(str::to_string)
        .collect()
}

pub fn matches_filter(n: &NoteInfo, words: &[String], urgency: u8) -> bool {
    if urgency > 0 && n.attention != urgency {
        return false;
    }
    words.iter().all(|w| n.search_norm.contains(w.as_str()))
}

/// Comparacao de substring ignorando maiusculas/minusculas.
///
/// O caminho ASCII (praticamente todas as buscas) nao aloca: antes cada linha
/// comparada virava dois `to_lowercase()`, e uma busca percorre todas as linhas.
pub fn contains_caseless(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        let (h, n) = (haystack.as_bytes(), needle.as_bytes());
        if n.len() > h.len() {
            return false;
        }
        return h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n));
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(name: &str, mtime: i64, att: u8) -> NoteInfo {
        let filename = format!("{}.md", name);
        NoteInfo {
            key: NoteKey::Db(1),
            title: title_from_filename(&filename),
            filename,
            mtime,
            attention: att,
            search_norm: String::new(),
        }
    }

    #[test]
    fn titles_drop_only_known_extensions() {
        assert_eq!(title_from_filename("Nota.md"), "Nota");
        assert_eq!(title_from_filename("v1.2.md"), "v1.2");
        assert_eq!(title_from_filename("x.pdf"), "x.pdf");
        assert_eq!(title_from_filename("A.TXT"), "A");
    }

    #[test]
    fn ordering_priority_then_recency_then_name() {
        let mut v = [
            note("sem", 100, 0),
            note("azul", 50, 4),
            note("crit_velha", 10, 1),
            note("crit_nova", 90, 1),
            note("b", 100, 0),
            note("a", 100, 0),
        ];
        v.sort_by(compare_notes);
        let names: Vec<_> = v.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(names, ["crit_nova", "crit_velha", "azul", "a", "b", "sem"]);
    }

    #[test]
    fn search_ignores_accents_case_and_requires_all_words() {
        let mut n = note("Reunião", 1, 0);
        n.search_norm = build_search_norm(&n.filename, &n.title, "# Pauta\nAção de Marketing\n");
        assert!(matches_filter(&n, &query_words("ACAO marketing"), 0));
        assert!(matches_filter(&n, &query_words("reuniao, pauta"), 0));
        assert!(!matches_filter(&n, &query_words("acao vendas"), 0));
        assert!(matches_filter(&n, &[], 0));
        assert!(!matches_filter(&n, &[], 3), "filtro de urgencia");
    }

    #[test]
    fn query_tokenizer_limits_and_splits() {
        assert_eq!(query_words("a,b;c (d)"), ["a", "b", "c", "d"]);
        assert_eq!(query_words(&"x ".repeat(100)).len(), 32);
    }

    #[test]
    fn caseless_contains() {
        assert!(contains_caseless("Olá MUNDO", "mundo"));
        assert!(contains_caseless("x", ""));
        assert!(!contains_caseless("abc", "d"));
    }
}
