//! Conway's Game of Life em tela cheia (protetor de tela e botao [Life]).

use crate::app::App;
use crate::render::{BOLD, RESET};
use crossterm::event::{self, Event};
use std::io::Write;
use std::time::{Duration, Instant};

/// Gerador xorshift simples (nao precisa ser criptografico).
struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn percent(&mut self, p: u64) -> bool {
        self.next() % 100 < p
    }
}

/// Um passo da simulacao (toroidal). Devolve o numero de celulas vivas.
pub fn step(grid: &[u8], next: &mut [u8], age: &mut [u8], rows: usize, cols: usize) -> usize {
    let mut live = 0;
    for r in 0..rows {
        let up = if r == 0 { rows - 1 } else { r - 1 };
        let down = if r == rows - 1 { 0 } else { r + 1 };
        for c in 0..cols {
            let left = if c == 0 { cols - 1 } else { c - 1 };
            let right = if c == cols - 1 { 0 } else { c + 1 };
            let n = grid[up * cols + left]
                + grid[up * cols + c]
                + grid[up * cols + right]
                + grid[r * cols + left]
                + grid[r * cols + right]
                + grid[down * cols + left]
                + grid[down * cols + c]
                + grid[down * cols + right];
            let i = r * cols + c;
            if grid[i] == 1 && (n == 2 || n == 3) {
                next[i] = 1;
                age[i] = age[i].saturating_add(1).min(250);
                live += 1;
            } else if grid[i] == 0 && n == 3 {
                next[i] = 1;
                age[i] = 1;
                live += 1;
            } else {
                next[i] = 0;
                age[i] = 0;
            }
        }
    }
    live
}

impl App {
    pub fn run_life(&mut self) {
        let (cols, rows) = crossterm::terminal::size().map_or((self.cols, self.rows), |(c, r)| {
            (i32::from(c), i32::from(r))
        });
        if rows < 6 || cols < 20 {
            return;
        }
        let (rows, cols) = (rows as usize, cols as usize);
        let active = rows - 1;
        let total = active * cols;
        let mut rng = Rng::new();
        let mut grid = vec![0u8; total];
        let mut next = vec![0u8; total];
        let mut age = vec![0u8; total];
        for i in 0..total {
            if rng.percent(22) {
                grid[i] = 1;
                age[i] = 1;
            }
        }
        let th = self.theme();
        let mut out = std::io::stdout();
        let _ = write!(out, "\x1b[?25l\x1b[2J\x1b[3J\x1b[H");
        let (mut generation, mut stagnant, mut last_live) = (0u64, 0, 0usize);
        let started = Instant::now();

        loop {
            let live = step(&grid, &mut next, &mut age, active, cols);
            stagnant = if live == last_live { stagnant + 1 } else { 0 };
            last_live = live;
            if live < 10 || stagnant > 30 {
                for i in 0..total {
                    if rng.percent(18) {
                        next[i] = 1;
                        age[i] = 1;
                    }
                }
                stagnant = 0;
            }

            let mut f = String::with_capacity(total * 8);
            f.push_str("\x1b[H");
            f.push_str(th.bg_main);
            for r in 0..active {
                for c in 0..cols {
                    let i = r * cols + c;
                    if next[i] == 1 {
                        match age[i] {
                            0..=1 => {
                                f.push_str(th.fg_accent);
                                f.push_str(BOLD);
                                f.push('●');
                            }
                            2..=4 => {
                                f.push_str(th.fg_h1);
                                f.push_str(BOLD);
                                f.push('o');
                            }
                            5..=10 => {
                                f.push_str(th.fg_h2);
                                f.push('•');
                            }
                            _ => {
                                f.push_str(th.fg_muted);
                                f.push('·');
                            }
                        }
                    } else {
                        f.push(' ');
                    }
                }
                f.push_str("\r\n");
            }
            generation += 1;
            let bar = format!(
                " 🧬 Conway's Game of Life [{}] │ Geração: {} │ Células Vivas: {}",
                th.name, generation, live
            );
            let hint = "[Pressione qualquer tecla ou clique para sair] ";
            f.push_str(th.bg_status);
            f.push_str(th.fg_muted);
            f.push_str(&bar);
            let pad = cols as i32 - crate::text::str_width(&bar) as i32 - crate::text::str_width(hint) as i32;
            for _ in 0..pad.max(1) {
                f.push(' ');
            }
            f.push_str(th.fg_h1);
            f.push_str(BOLD);
            f.push_str(hint);
            f.push_str("\x1b[K");
            f.push_str(RESET);
            let _ = out.write_all(f.as_bytes());
            let _ = out.flush();
            std::mem::swap(&mut grid, &mut next);

            self.check_autosave();

            // Qualquer tecla ou clique encerra (ignora o "soltar" do clique que
            // abriu o Life, nos primeiros 350 ms).
            if event::poll(Duration::from_millis(120)).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Mouse(m)) => {
                        let release = matches!(m.kind, event::MouseEventKind::Up(_));
                        if !(release && started.elapsed() < Duration::from_millis(350))
                            && !matches!(m.kind, event::MouseEventKind::Moved)
                        {
                            break;
                        }
                    }
                    Ok(Event::Key(k)) if k.kind == event::KeyEventKind::Release => {}
                    Ok(_) | Err(_) => break,
                }
            }
        }
        // Descarta eventos restantes para nao acionar nada ao voltar.
        while event::poll(Duration::from_millis(0)).unwrap_or(false) {
            let _ = event::read();
        }
        self.clear_needed = true;
        self.needs_redraw = true;
        self.last_activity = Instant::now();
        let _ = write!(out, "\x1b[2J\x1b[3J\x1b[H\x1b[?25h");
        let _ = out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blinker_oscillates() {
        let (rows, cols) = (5, 5);
        let mut g = vec![0u8; 25];
        for c in 1..4 {
            g[2 * cols + c] = 1;
        }
        let (mut n, mut a) = (vec![0u8; 25], vec![0u8; 25]);
        let live = step(&g, &mut n, &mut a, rows, cols);
        assert_eq!(live, 3);
        for r in 1..4 {
            assert_eq!(n[r * cols + 2], 1, "linha horizontal vira vertical");
        }
        assert_eq!(n[2 * cols + 1], 0);
    }

    #[test]
    fn ages_cap_and_reset() {
        let (rows, cols) = (4, 4);
        let mut g = vec![0u8; 16];
        // bloco 2x2 e estavel
        for (r, c) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
            g[r * cols + c] = 1;
        }
        let (mut n, mut a) = (vec![0u8; 16], vec![250u8; 16]);
        let live = step(&g, &mut n, &mut a, rows, cols);
        assert_eq!(live, 4);
        assert_eq!(a[cols + 1], 250);
        assert_eq!(a[0], 0, "celula morta zera a idade");
    }

    #[test]
    fn rng_is_not_stuck() {
        let mut r = Rng(12345);
        let a = r.next();
        assert_ne!(a, r.next());
    }
}
