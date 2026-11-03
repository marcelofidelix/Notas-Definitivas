//! Notas Definitivas (versao em Rust): editor TUI de notas Markdown com SQLite,
//! edicao modal estilo Vim/Neovim, abas, temas e Game of Life.
//!
//! Compartilha o `notas.db` com a versao em C (mesmo esquema).

mod app;
mod clip;
mod cmd;
mod db;
mod doc;
mod edit;
mod fsio;
mod input;
mod layout;
mod life;
mod links;
mod md;
mod mouse;
mod notes;
mod render;
mod table;
mod text;
mod theme;

#[cfg(test)]
mod tests;

use app::{App, STATUS_TTL};
use crossterm::event::{self, Event};
use crossterm::terminal;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Pedido de encerramento vindo do sistema (fechar a janela do console, logoff,
/// desligamento): o laco principal salva tudo e sai.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
/// O programa terminou de salvar; o tratador pode devolver o controle ao Windows.
static SHUTDOWN_DONE: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn console_ctrl_handler(ctrl: u32) -> i32 {
    // 0 Ctrl+C, 1 Ctrl+Break, 2 fechar janela, 5 logoff, 6 desligamento.
    if matches!(ctrl, 0 | 1 | 2 | 5 | 6) {
        SHUTDOWN.store(true, Ordering::SeqCst);
        // O Windows encerra o processo quando o tratador retorna: espera (ate ~4 s)
        // o laco principal gravar as notas pendentes.
        for _ in 0..200 {
            if SHUTDOWN_DONE.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        1
    } else {
        0
    }
}

#[cfg(windows)]
fn install_shutdown_handler() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    // SAFETY: registra uma funcao `extern "system"` valida durante todo o processo.
    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }
}

#[cfg(not(windows))]
fn install_shutdown_handler() {}

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Terminal em modo raw, tela alternativa, mouse e colagem entre colchetes.
/// O `Drop` restaura tudo, inclusive em caso de panico ou saida antecipada.
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> std::io::Result<TerminalGuard> {
        enable_vt_output();
        terminal::enable_raw_mode()?;
        let guard = TerminalGuard;
        let mut out = std::io::stdout();
        // ?1049h tela alternativa, ?7l sem quebra automatica, mouse e colagem.
        write!(
            out,
            "\x1b[?1049h\x1b[?7l\x1b[?1000h\x1b[?1002h\x1b[?1006h\x1b[?2004h\x1b[2J\x1b[H"
        )?;
        out.flush()?;
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = write!(
            out,
            "\x1b[?2026l\x1b[?7h\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1006l\x1b[0 q\x1b[?25h\x1b[?1049l\x1b[0m"
        );
        let _ = out.flush();
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(windows)]
fn enable_vt_output() {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };
    // SAFETY: chamadas simples a APIs do console com o handle padrao de saida;
    // `mode` e um inteiro local valido para escrita.
    unsafe {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(h, &mut mode) != 0 {
            SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
fn enable_vt_output() {}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn db_path() -> PathBuf {
    match std::env::var_os("NOTAS_DB") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => exe_dir().join("notas.db"),
    }
}

fn write_frame(frame: &str) -> bool {
    let mut out = std::io::stdout().lock();
    out.write_all(frame.as_bytes()).and_then(|_| out.flush()).is_ok()
}

fn handle_event(app: &mut App, ev: Event) {
    match ev {
        Event::Key(k) => {
            if let Some(key) = input::map_key(k) {
                app.on_key(key);
            }
        }
        Event::Mouse(m) => {
            if let Some(m) = mouse::map_mouse(m) {
                app.on_mouse(m);
            }
        }
        Event::Paste(text) => {
            app.last_activity = Instant::now();
            app.needs_redraw = true;
            if text.len() > clip::MAX_PASTE_BYTES {
                app.set_status("Colagem ignorada: texto grande demais.");
            } else {
                app.paste_text(&text);
            }
        }
        Event::Resize(c, r) => {
            app.cols = i32::from(c);
            app.rows = i32::from(r);
            app.clear_needed = true;
            app.needs_redraw = true;
        }
        _ => {}
    }
}

/// Precisa redesenhar? Estado alterado, animacao do letreiro, expiracao da
/// mensagem de status ou batimento de manutencao (mantem "5m", "2h" vivos).
fn should_draw(app: &mut App) -> bool {
    if app.needs_redraw || app.clear_needed {
        return true;
    }
    if app.marquee_active && app.start.elapsed().as_millis() / 300 != app.last_marquee_tick {
        return true;
    }
    if let Some((_, t)) = &app.status {
        if t.elapsed() >= STATUS_TTL {
            app.status = None;
            return true;
        }
        return false;
    }
    if app.last_draw.elapsed() >= app::IDLE_HEARTBEAT {
        // Batimento: repinta tudo (sem limpar) para corrigir qualquer sujeira que
        // o terminal tenha deixado e manter "5m", "2h" atualizados.
        app.force_full = true;
        return true;
    }
    false
}

fn run(app: &mut App) {
    while !app.quit {
        if let Ok((c, r)) = terminal::size() {
            if i32::from(c) != app.cols || i32::from(r) != app.rows {
                app.cols = i32::from(c);
                app.rows = i32::from(r);
                app.clear_needed = true;
            }
        }
        if should_draw(app) {
            let frame = render::draw(app);
            if !write_frame(&frame) {
                // Saida indisponivel: tenta de novo no proximo ciclo.
                app.clear_needed = true;
                app.needs_redraw = true;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                while let Ok(true) = event::poll(Duration::from_millis(0)) {
                    match event::read() {
                        Ok(ev) => handle_event(app, ev),
                        Err(_) => break,
                    }
                    if app.quit {
                        return;
                    }
                }
            }
            Ok(false) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
        if SHUTDOWN.load(Ordering::SeqCst) {
            app.emergency_save();
            app.quit = true;
            return;
        }
        app.check_autosave();
        if app.last_activity.elapsed() >= app::IDLE_LIFE {
            app.run_life();
        }
    }
}

fn main() {
    let mut file_arg: Option<PathBuf> = None;
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "-V" | "--version" => {
                println!("notas {}", VERSION);
                return;
            }
            "-h" | "--help" => {
                println!(
                    "Notas Definitivas {v} (Rust)\n\nUso: notas [arquivo.md]\n\n\
                     O banco notas.db fica ao lado do executavel (ou em $NOTAS_DB).\n\
                     Dentro do programa: :q sai, :w salva, :life, :theme, :import, :export."
                , v = VERSION);
                return;
            }
            _ if file_arg.is_none() => {
                file_arg = Some(std::path::absolute(&a).unwrap_or_else(|_| PathBuf::from(&a)));
            }
            _ => {}
        }
    }

    // Como na versao em C, links relativos partem da pasta do executavel.
    let _ = std::env::set_current_dir(exe_dir());

    let db = match db::Db::open(&db_path()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Erro fatal: nao foi possivel abrir/criar o banco {}: {}", db_path().display(), e);
            std::process::exit(1);
        }
    };

    install_shutdown_handler();
    let guard = match TerminalGuard::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Erro: nao foi possivel iniciar o terminal: {}", e);
            std::process::exit(1);
        }
    };

    let mut app = App::new(db);
    if let Some(p) = file_arg {
        app.open_path_arg(p);
    }
    let total = app.db.count_notes();
    if total == 0 {
        app.set_status("Banco SQLite vazio. Use [Nova] para criar uma nota ou [⬇] para importar seus .md");
    } else {
        app.set_status(format!("Notas Definitivas | SQLite: {} nota(s) | Pressione :q para sair", total));
    }

    // Um panico nao pode custar o que ainda nao foi salvo.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut app)));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.emergency_save()));
    drop(guard);
    SHUTDOWN_DONE.store(true, Ordering::SeqCst);
    if result.is_err() {
        eprintln!("Erro interno inesperado; as alteracoes pendentes foram salvas.");
        std::process::exit(2);
    }
}
