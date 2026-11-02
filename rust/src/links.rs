//! Abertura de links: URLs no navegador, pastas no explorador, arquivos no
//! aplicativo padrao e scripts/executaveis executados (atalho do editor).
//!
//! ATENCAO: links para scripts SAO executados, inclusive em notas importadas;
//! so siga links de scripts em notas de origem confiavel.

use std::path::{Path, PathBuf};

pub fn is_web(target: &str) -> bool {
    let t = target.to_ascii_lowercase();
    ["http://", "https://", "ftp://", "mailto:", "www."]
        .iter()
        .any(|p| t.starts_with(p))
}

/// Remove espacos e `<...>` do alvo de um link.
pub fn clean_target(raw: &str) -> String {
    let t = raw.trim_matches(|c: char| (c as u32) <= 32);
    let t = t
        .strip_prefix('<')
        .and_then(|r| r.strip_suffix('>'))
        .unwrap_or(t);
    t.to_string()
}

/// Decodifica `%20` etc. e remove o prefixo `file://`.
pub fn decode_local(target: &str) -> String {
    let lower = target.to_ascii_lowercase();
    let p = if lower.starts_with("file:///") {
        &target[8..]
    } else if lower.starts_with("file://") {
        &target[7..]
    } else {
        target
    };
    let b = p.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && b[i + 1].is_ascii_hexdigit() && b[i + 2].is_ascii_hexdigit() {
            let h = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00");
            out.push(u8::from_str_radix(h, 16).unwrap_or(b'%'));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn resolve(path: &str, note_dir: Option<&Path>) -> Option<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.exists().then(|| p.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir() {
        let c = cwd.join(p);
        if c.exists() {
            return Some(c);
        }
    }
    note_dir.map(|d| d.join(p)).filter(|c| c.exists())
}

/// Abre o alvo de um link e devolve a mensagem de status.
pub fn open_target(raw: &str, note_dir: Option<&Path>) -> String {
    let target = clean_target(raw);
    if target.is_empty() {
        return String::new();
    }
    if is_web(&target) {
        let url = if target.to_ascii_lowercase().starts_with("www.") {
            format!("https://{}", target)
        } else {
            target.clone()
        };
        return if platform::open_url(&url) {
            format!("Abrindo no navegador: {}", url)
        } else {
            format!("Erro ao abrir navegador para: {}", url)
        };
    }
    let local = decode_local(&target);
    let Some(path) = resolve(&local, note_dir) else {
        return format!("Arquivo ou pasta não encontrado: {}", local);
    };
    platform::open_path(&path, &local)
}

#[cfg(windows)]
mod platform {
    use std::path::Path;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn exec(verb: &str, file: &str, params: Option<&str>, dir: Option<&str>) -> isize {
        let (v, f) = (wide(verb), wide(file));
        let p = params.map(wide);
        let d = dir.map(wide);
        // SAFETY: todos os ponteiros apontam para buffers terminados em NUL que
        // vivem ate o fim da chamada; os opcionais sao nulos quando ausentes.
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                v.as_ptr(),
                f.as_ptr(),
                p.as_ref().map_or(std::ptr::null(), |x| x.as_ptr()),
                d.as_ref().map_or(std::ptr::null(), |x| x.as_ptr()),
                SW_SHOWNORMAL,
            ) as isize
        }
    }

    pub fn open_url(url: &str) -> bool {
        exec("open", url, None, None) > 32
    }

    fn has_ext(path: &Path, exts: &[&str]) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
            .unwrap_or(false)
    }

    /// Caminho sem o prefixo `\\?\` que alguns APIs devolvem.
    fn plain(path: &Path) -> String {
        let s = path.to_string_lossy().replace('/', "\\");
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    }

    pub fn open_path(path: &Path, shown: &str) -> String {
        let full = plain(path);
        if path.is_dir() {
            let mut h = exec("explore", &full, None, None);
            if h <= 32 {
                h = exec("open", &full, None, None);
            }
            return if h > 32 {
                format!("Pasta aberta no Explorer: {}", shown)
            } else {
                format!("Erro ao abrir pasta: {}", shown)
            };
        }
        let dir = path.parent().map(plain);
        let is_script = has_ext(path, &["exe", "bat", "cmd", "vbs", "js", "wsf", "lnk", "msi", "ps1"]);
        let h = if has_ext(path, &["ps1"]) {
            // O verbo "open" de um .ps1 apenas o abre no Bloco de Notas: executa
            // no PowerShell (respeitando a ExecutionPolicy) e mantem a janela.
            let params = format!("-NoProfile -NoExit -File \"{}\"", full);
            exec("open", "powershell.exe", Some(&params), dir.as_deref())
        } else {
            exec("open", &full, None, dir.as_deref())
        };
        if h > 32 {
            if is_script {
                format!("Executando: {}", shown)
            } else {
                format!("Arquivo aberto: {}", shown)
            }
        } else if h == 31 {
            // SE_ERR_NOASSOC: nenhum aplicativo associado.
            exec("openas", &full, None, None);
            format!("Abrindo diálogo 'Abrir com...' para: {}", shown)
        } else {
            format!("Erro ao abrir arquivo: {}", shown)
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    fn spawn_detached(mut cmd: Command) -> bool {
        use std::os::unix::process::CommandExt;
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        cmd.process_group(0);
        match cmd.spawn() {
            Ok(mut child) => {
                // Recolhe o processo em segundo plano para nao deixar zumbi.
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                true
            }
            Err(_) => false,
        }
    }

    pub fn open_url(url: &str) -> bool {
        let mut c = Command::new("xdg-open");
        c.arg("--").arg(url);
        spawn_detached(c)
    }

    pub fn open_path(path: &Path, shown: &str) -> String {
        let executable = path.is_file()
            && std::fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false);
        if executable {
            // Roda direto (sem shell), na pasta do proprio script.
            let mut c = Command::new(path);
            if let Some(d) = path.parent() {
                c.current_dir(d);
            }
            return if spawn_detached(c) {
                format!("Executando: {}", shown)
            } else {
                format!("Erro ao executar: {}", shown)
            };
        }
        let mut c = Command::new("xdg-open");
        c.arg("--").arg(path);
        if spawn_detached(c) {
            format!("Abrindo: {}", shown)
        } else {
            format!("Nao foi possivel abrir o link: {}", shown)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_detection_and_cleanup() {
        assert!(is_web("HTTPS://x.com"));
        assert!(is_web("www.site.com"));
        assert!(is_web("mailto:a@b.c"));
        assert!(!is_web("./pasta/x.md"));
        assert_eq!(clean_target("  <a b.pdf> \n"), "a b.pdf");
    }

    #[test]
    fn local_path_decoding() {
        assert_eq!(decode_local("file:///C:/a%20b/c.txt"), "C:/a b/c.txt");
        assert_eq!(decode_local("x%2"), "x%2");
        assert_eq!(decode_local("100%"), "100%");
        assert_eq!(decode_local("%C3%A7"), "ç");
    }

    #[test]
    fn missing_file_reports_not_found() {
        let msg = open_target("./arquivo_que_nao_existe_xyz.pdf", None);
        assert!(msg.contains("não encontrado"), "{}", msg);
    }
}
