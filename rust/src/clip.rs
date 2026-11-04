//! Area de transferencia do sistema: copia assincrona (sem travar a interface),
//! colagem com tentativas e a sequencia OSC 52 para terminais remotos.

use std::cell::RefCell;
use std::io::Write;

/// Teto para texto vindo do clipboard ou de colagem do terminal.
///
/// Era 64 MB: uma colagem desse tamanho virava `String` de entrada + uma `Rc<str>`
/// por linha + o vetor de linhas, com pico de ~200 MB. 16 MB cobre qualquer
/// colagem realista e evita um OOM em maquina pequena.
pub const MAX_PASTE_BYTES: usize = 16 * 1024 * 1024;
/// Terminais costumam recusar OSC 52 maiores que isto.
const MAX_OSC52_BYTES: usize = 75_000;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

fn want_osc52() -> bool {
    if cfg!(windows) {
        // Localmente o texto ja vai por CF_UNICODETEXT; OSC 52 so serve via SSH.
        ["SSH_CLIENT", "SSH_TTY", "SSH_CONNECTION"]
            .iter()
            .any(|v| std::env::var_os(v).is_some())
    } else {
        true
    }
}

/// Copia `text` para o clipboard do sistema (e via OSC 52 quando util).
/// Nunca bloqueia: a gravacao nativa roda numa thread.
pub fn copy(text: &str) {
    // Testes nunca tocam no clipboard real do usuario.
    if text.is_empty() || cfg!(test) {
        return;
    }
    if want_osc52() && text.len() <= MAX_OSC52_BYTES {
        let mut out = std::io::stdout().lock();
        let _ = write!(out, "\x1b]52;c;{}\x07", base64_encode(text.as_bytes()));
        let _ = out.flush();
    }
    let owned = text.to_string();
    let _ = std::thread::Builder::new().name("clipboard".into()).spawn(move || {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            set_text(&mut cb, owned);
        }
    });
}

#[cfg(target_os = "linux")]
fn set_text(cb: &mut arboard::Clipboard, text: String) {
    use arboard::SetExtLinux;
    // Em X11/Wayland o dono precisa continuar vivo para servir o conteudo.
    let _ = cb.set().wait().text(text);
}

#[cfg(not(target_os = "linux"))]
fn set_text(cb: &mut arboard::Clipboard, text: String) {
    let _ = cb.set_text(text);
}

thread_local! {
    /// Handle reaproveitado para leitura: abrir um `arboard::Clipboard` por
    /// colagem custava uma inicializacao de COM (Windows) ou de conexao X11
    /// (Linux) a cada `p` / `Ctrl+V`. Descartado se uma leitura falhar.
    static READER: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
}

/// Le o texto do clipboard (vazio ou indisponivel => `None`).
///
/// A leitura e sincrona: até 3 tentativas de 5 ms (antes eram 10, ou seja, ate
/// 50 ms de congelamento da interface com o clipboard disputado).
pub fn paste() -> Option<String> {
    if cfg!(test) {
        return None;
    }
    for _ in 0..3 {
        let lido = READER.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                *slot = arboard::Clipboard::new().ok();
            }
            slot.as_mut().map(|cb| cb.get_text())
        });
        match lido {
            Some(Ok(t)) if t.len() <= MAX_PASTE_BYTES => {
                return if t.is_empty() { None } else { Some(t) };
            }
            Some(Ok(_)) => return None,
            Some(Err(arboard::Error::ContentNotAvailable)) => return None,
            _ => {
                // Handle quebrado ou clipboard ocupado: descarta e tenta de novo.
                READER.with(|slot| *slot.borrow_mut() = None);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_padding_is_correct() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
        assert_eq!(base64_encode("ação".as_bytes()), "YcOnw6Nv");
        assert_eq!(base64_encode(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }
}
