//! Pilha de estados para desfazer/refazer.
//!
//! Guardar uma copia do documento inteiro a cada passo custa O(passos x linhas)
//! (100 passos de uma nota de 20 mil linhas = ~90 MB). Aqui so o estado mais
//! recente de cada pilha fica completo; cada estado anterior e guardado como a
//! diferenca para o seguinte: quantas linhas do inicio e do fim sao as mesmas
//! (mesmo `Rc<str>`, comparado por ponteiro, sem olhar o texto) e as linhas do
//! meio que mudaram. Como as edicoes sao locais, esse meio costuma ter poucas
//! linhas. Desfazer reconstroi o estado anterior a partir do completo.

use crate::doc::Line;
use std::rc::Rc;

#[derive(Clone)]
pub struct Snapshot {
    pub lines: Vec<Line>,
    pub cy: usize,
    pub cx: usize,
}

/// Como obter um estado antigo a partir do estado seguinte: as primeiras
/// `prefix` e as ultimas `suffix` linhas deste, com `mid` entre elas.
struct Delta {
    prefix: usize,
    suffix: usize,
    mid: Vec<Line>,
    cy: usize,
    cx: usize,
}

/// Duas linhas representam o mesmo estado salvo: mesmo texto (por ponteiro) e a
/// mesma dobra. O resto (`hidden`, `fence`, bloco de tabela...) e recalculado
/// por `update_lines_syntax` ao restaurar.
fn same(a: &Line, b: &Line) -> bool {
    Rc::ptr_eq(&a.text, &b.text) && a.folded == b.folded
}

impl Delta {
    /// Diferenca de `old` para o estado `newer` (consome `old`, sem copiar linhas).
    fn between(old: Snapshot, newer: &Snapshot) -> Delta {
        let (o, n) = (&old.lines, &newer.lines);
        let max = o.len().min(n.len());
        let prefix = o.iter().zip(n).take_while(|(a, b)| same(a, b)).count();
        let suffix = o
            .iter()
            .rev()
            .zip(n.iter().rev())
            .take(max - prefix)
            .take_while(|(a, b)| same(a, b))
            .count();
        let mut lines = old.lines;
        lines.truncate(lines.len() - suffix);
        let mid = lines.split_off(prefix);
        Delta { prefix, suffix, mid, cy: old.cy, cx: old.cx }
    }

    /// Reconstroi o estado antigo a partir do seguinte.
    fn apply(&self, newer: &Snapshot) -> Snapshot {
        let n = &newer.lines;
        let mut lines = Vec::with_capacity(self.prefix + self.mid.len() + self.suffix);
        lines.extend_from_slice(&n[..self.prefix]);
        lines.extend_from_slice(&self.mid);
        lines.extend_from_slice(&n[n.len() - self.suffix..]);
        Snapshot { lines, cy: self.cy, cx: self.cx }
    }
}

/// Pilha de estados: `top` completo; `deltas[i]` descreve o estado `i` a partir
/// do `i + 1` (e o ultimo a partir de `top`).
#[derive(Default)]
pub struct Stack {
    deltas: Vec<Delta>,
    top: Option<Snapshot>,
}

impl Stack {
    pub fn len(&self) -> usize {
        self.deltas.len() + usize::from(self.top.is_some())
    }

    pub fn clear(&mut self) {
        self.deltas.clear();
        self.top = None;
    }

    pub fn push(&mut self, s: Snapshot) {
        if let Some(old) = self.top.take() {
            self.deltas.push(Delta::between(old, &s));
        }
        self.top = Some(s);
    }

    pub fn pop(&mut self) -> Option<Snapshot> {
        let top = self.top.take()?;
        if let Some(d) = self.deltas.pop() {
            self.top = Some(d.apply(&top));
        }
        Some(top)
    }

    /// Esquece o estado mais antigo (nenhum outro depende dele).
    pub fn drop_oldest(&mut self) {
        if self.deltas.is_empty() {
            self.top = None;
        } else {
            self.deltas.remove(0);
        }
    }

    /// Linhas realmente guardadas (rede de seguranca de memoria e testes).
    pub fn stored_lines(&self) -> usize {
        self.top.as_ref().map_or(0, |s| s.lines.len()) + self.deltas.iter().map(|d| d.mid.len()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(texts: &[&str], cy: usize) -> Snapshot {
        Snapshot { lines: texts.iter().map(Line::new).collect(), cy, cx: 0 }
    }

    fn texts(s: &Snapshot) -> Vec<String> {
        s.lines.iter().map(|l| l.text.to_string()).collect()
    }

    #[test]
    fn deltas_rebuild_older_states_in_order() {
        let mut st = Stack::default();
        let a = snap(&["a", "b", "c"], 0);
        let mut b = a.clone();
        b.lines[1].set_text("B".into());
        b.cy = 1;
        let mut c = b.clone();
        c.lines.insert(0, Line::new("novo"));
        c.lines.pop();
        st.push(a);
        st.push(b);
        st.push(c);
        assert_eq!(st.len(), 3);
        // So o meio que mudou fica guardado, nao o documento inteiro.
        assert!(st.stored_lines() < 3 * 3);
        assert_eq!(texts(&st.pop().unwrap()), ["novo", "a", "B"]);
        let b = st.pop().unwrap();
        assert_eq!((texts(&b), b.cy), (vec!["a".into(), "B".into(), "c".into()], 1));
        assert_eq!(texts(&st.pop().unwrap()), ["a", "b", "c"]);
        assert!(st.pop().is_none() && st.len() == 0);
    }

    #[test]
    fn identical_states_and_fold_changes() {
        let mut st = Stack::default();
        let a = snap(&["x", "y"], 0);
        let mut b = a.clone();
        b.lines[0].folded = true; // so a dobra mudou
        st.push(a.clone());
        st.push(b);
        st.push(a.clone());
        assert!(st.pop().is_some());
        assert!(st.pop().unwrap().lines[0].folded, "a dobra e restaurada");
        assert!(!st.pop().unwrap().lines[0].folded);
        // Sem nenhuma diferenca o meio fica vazio.
        st.push(a.clone());
        st.push(a);
        assert_eq!(st.stored_lines(), 2);
    }

    #[test]
    fn drop_oldest_keeps_the_rest_consistent() {
        let mut st = Stack::default();
        let mut cur = snap(&["0"], 0);
        for i in 1..=5 {
            st.push(cur.clone());
            cur.lines[0].set_text(i.to_string());
        }
        st.drop_oldest();
        st.drop_oldest();
        assert_eq!(st.len(), 3);
        let got: Vec<String> = std::iter::from_fn(|| st.pop()).map(|s| texts(&s)[0].clone()).collect();
        assert_eq!(got, ["4", "3", "2"].map(String::from));
    }

    /// Equivalencia com a pilha ingenua (copia completa de cada passo), em edicoes
    /// aleatorias de varios tipos.
    #[test]
    fn matches_a_naive_full_copy_stack_under_random_edits() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let key = |s: &Snapshot| -> (Vec<(String, bool)>, usize, usize) {
            (s.lines.iter().map(|l| (l.text.to_string(), l.folded)).collect(), s.cy, s.cx)
        };
        let mut cur = snap(&["a", "b", "c", "d", "e"], 0);
        let (mut fast, mut naive): (Stack, Vec<Snapshot>) = (Stack::default(), Vec::new());
        for step in 0..4000 {
            match rnd(6) {
                0..=3 => {
                    fast.push(cur.clone());
                    naive.push(cur.clone());
                    for _ in 0..=rnd(3) {
                        let n = cur.lines.len();
                        match rnd(5) {
                            0 if n > 0 => cur.lines[rnd(n)].set_text(format!("t{step}")),
                            1 => cur.lines.insert(rnd(n + 1), Line::new(format!("n{step}"))),
                            2 if n > 1 => {
                                cur.lines.remove(rnd(n));
                            }
                            3 if n > 0 => {
                                let i = rnd(n);
                                cur.lines[i].folded = !cur.lines[i].folded;
                            }
                            _ => cur.lines = (0..rnd(8)).map(|i| Line::new(format!("z{i}"))).collect(),
                        }
                    }
                    cur.cy = rnd(cur.lines.len().max(1));
                    cur.cx = rnd(5);
                }
                _ => {
                    let (a, b) = (fast.pop(), naive.pop());
                    assert_eq!(a.as_ref().map(key), b.as_ref().map(key), "passo {step}");
                    if let Some(s) = a {
                        cur = s;
                    }
                }
            }
            assert_eq!(fast.len(), naive.len());
        }
    }
}
