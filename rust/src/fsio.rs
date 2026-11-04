//! Arquivos: leitura limitada, gravacao atomica, importacao e exportacao.

use crate::db::{now_secs, Db};
use crate::text::{is_markdown_ext, sanitize_note_name};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Teto para ler um arquivo inteiro (importacao e previa de arquivo avulso).
pub const MAX_NOTE_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Le um arquivo UTF-8 (bytes invalidos viram U+FFFD) respeitando o teto.
pub fn read_text_file(path: &Path) -> std::io::Result<String> {
    let f = fs::File::open(path)?;
    let len = f.metadata()?.len();
    if len > MAX_NOTE_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "arquivo grande demais",
        ));
    }
    let mut buf = Vec::with_capacity(len as usize);
    f.take(MAX_NOTE_FILE_BYTES + 1).read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Grava em `<arquivo>.tmp.<pid>`, confere cada etapa, faz `fsync` e so entao
/// substitui o destino com `rename`: uma falha no meio nunca deixa o destino
/// truncado.
pub fn write_file_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".tmp.{}", std::process::id()));
    let tmp = PathBuf::from(tmp);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.flush()?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Expande `~` no inicio para a pasta pessoal.
pub fn expand_user_path(input: &str) -> String {
    if input == "~" || input.starts_with("~/") || input.starts_with("~\\") {
        let home = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok());
        if let Some(h) = home {
            return format!("{}{}", h, &input[1..]);
        }
    }
    input.to_string()
}

/// Remove espacos e aspas das pontas (colar caminho costuma trazer aspas).
pub fn clean_dir_input(raw: &str) -> String {
    let s = expand_user_path(raw);
    let s = s.trim_matches(|c| c == ' ' || c == '\t');
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
}

/// Importa os `.md/.markdown/.mdown/.txt/.text` (ou sem extensao) de uma pasta.
/// Reimportar atualiza o conteudo e preserva a marcacao de urgencia. Tudo numa
/// transacao: confirma com COMMIT verificado e faz ROLLBACK em qualquer erro.
pub fn import_notes_from_dir(db: &Db, dir: &str) -> Result<ImportReport, String> {
    if dir.is_empty() {
        return Err("nenhuma pasta informada".into());
    }
    let entries = fs::read_dir(dir).map_err(|_| {
        let shown: String = dir.chars().take(200).collect();
        format!("pasta nao encontrada: {}", shown)
    })?;
    if !db.begin() {
        return Err("nao foi possivel iniciar a transacao".into());
    }
    let (mut added, mut updated) = (0, 0);
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().into_owned();
        if fname.starts_with('.') || !is_markdown_ext(&fname) {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = fs::metadata(&path) else { continue };
        if meta.is_dir() {
            continue;
        }
        // Nome logico = nome do arquivo sem a ultima extensao.
        let name = match fname.rfind('.') {
            Some(i) => &fname[..i],
            None => &fname[..],
        };
        if name.is_empty() || name.len() >= 256 {
            continue;
        }
        let Ok(content) = read_text_file(&path) else { continue };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .filter(|&t| t > 0)
            .unwrap_or_else(now_secs);
        if let Some(id) = db.find_id_by_name(name) {
            if db.update_content(id, &content, mtime) {
                updated += 1;
            }
        } else if db.insert_note(name, &content, mtime, 0).is_some() {
            added += 1;
        }
    }
    if !db.commit() {
        db.rollback();
        return Err("falha ao confirmar a importacao".into());
    }
    Ok(ImportReport { added, updated })
}

/// Exporta cada nota como `<nome>.md` (nome higienizado) com escrita atomica.
/// Devolve quantas foram gravadas; erro se nada pode ser gravado.
///
/// Nomes diferentes podem higienizar para o mesmo arquivo (`a:b` e `a*b` viram
/// `a_b`): o segundo recebe um sufixo, em vez de sobrescrever o primeiro.
pub fn export_notes_to_dir(db: &Db, dir: &str) -> Result<usize, String> {
    if dir.is_empty() {
        return Err("nenhuma pasta informada".into());
    }
    fs::create_dir_all(dir).map_err(|_| {
        let shown: String = dir.chars().take(200).collect();
        format!("nao foi possivel criar a pasta: {}", shown)
    })?;
    let (mut written, mut failed) = (0usize, 0usize);
    let mut usados: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, content) in db.all_name_content() {
        let base = sanitize_note_name(&name);
        if base.is_empty() {
            failed += 1;
            continue;
        }
        // Windows e macOS nao diferenciam maiusculas no nome do arquivo.
        let mut safe = base.clone();
        let mut n = 2;
        while !usados.insert(safe.to_lowercase()) {
            safe = format!("{base} ({n})");
            n += 1;
        }
        let path = Path::new(dir).join(format!("{}.md", safe));
        if write_file_atomic(&path, content.as_bytes()).is_ok() {
            written += 1;
        } else {
            failed += 1;
        }
    }
    if failed > 0 && written == 0 {
        return Err(format!("{} nota(s) nao puderam ser exportadas", failed));
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.md");
        write_file_atomic(&p, b"um").unwrap();
        write_file_atomic(&p, b"dois").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"dois");
        let n = fs::read_dir(d.path()).unwrap().count();
        assert_eq!(n, 1, "nenhum .tmp sobra");
    }

    #[test]
    fn import_and_export_survive_awkward_names() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("entrada");
        fs::create_dir(&src).unwrap();
        for (f, body) in [
            ("Ação, ñ e 日本語.md", "acentos e ideogramas"),
            ("nota com espacos.markdown", "espacos"),
            ("sem-extensao", "sem extensao"),
            ("-traco no inicio.md", "traco"),
            (".oculta.md", "nao deve entrar"),
        ] {
            fs::write(src.join(f), body).unwrap();
        }
        let db = Db::open_memory();
        let r = import_notes_from_dir(&db, src.to_str().unwrap()).unwrap();
        assert_eq!((r.added, r.updated), (4, 0), "arquivo oculto fica de fora");

        // Nomes que so existem em banco (a versao em C ou um banco vindo do
        // Linux podem te-los) higienizam para o mesmo arquivo.
        db.insert_note("a:b", "dois pontos", 1, 0).unwrap();
        db.insert_note("a*b", "asterisco", 2, 0).unwrap();

        let out = d.path().join("saida");
        let n = export_notes_to_dir(&db, out.to_str().unwrap()).unwrap();
        let arquivos = fs::read_dir(&out).unwrap().count();
        assert_eq!(n, 6);
        assert_eq!(arquivos, 6, "nenhuma nota pode sobrescrever a outra");
        assert_eq!(fs::read_to_string(out.join("Ação, ñ e 日本語.md")).unwrap(), "acentos e ideogramas");
    }

    #[test]
    fn import_export_roundtrip_and_reimport_updates() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("Alfa.md"), "# Alfa\n").unwrap();
        fs::write(d.path().join("Beta.txt"), "beta ção\n").unwrap();
        fs::write(d.path().join("x.pdf"), "nao").unwrap();
        fs::write(d.path().join(".oculto.md"), "nao").unwrap();
        fs::create_dir(d.path().join("sub.md")).unwrap();
        let db = Db::open_memory();
        let r = import_notes_from_dir(&db, d.path().to_str().unwrap()).unwrap();
        assert_eq!((r.added, r.updated), (2, 0));
        let r = import_notes_from_dir(&db, d.path().to_str().unwrap()).unwrap();
        assert_eq!((r.added, r.updated), (0, 2), "reimportar atualiza, nao duplica");
        assert_eq!(db.count_notes(), 2);

        let out = d.path().join("saida");
        assert_eq!(export_notes_to_dir(&db, out.to_str().unwrap()).unwrap(), 2);
        assert_eq!(fs::read_to_string(out.join("Beta.md")).unwrap(), "beta ção\n");
    }

    #[test]
    fn import_of_missing_dir_fails_cleanly() {
        let db = Db::open_memory();
        assert!(import_notes_from_dir(&db, "Z:/nao/existe/mesmo").is_err());
        assert_eq!(db.count_notes(), 0);
    }

    #[test]
    fn dir_input_is_cleaned() {
        assert_eq!(clean_dir_input("  \"C:\\x y\"  "), "C:\\x y");
        assert_eq!(clean_dir_input("'/tmp/a'"), "/tmp/a");
        assert_eq!(clean_dir_input("plain"), "plain");
    }

    #[test]
    fn oversize_files_are_refused() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("big.txt");
        let f = fs::File::create(&p).unwrap();
        f.set_len(MAX_NOTE_FILE_BYTES + 1).unwrap();
        assert!(read_text_file(&p).is_err());
    }
}
