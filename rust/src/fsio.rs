//! Arquivos: leitura limitada, gravacao atomica, importacao e exportacao.

use crate::db::{now_secs, Db};
use crate::text::{is_markdown_ext, sanitize_note_name};
use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::io::{self, Read, Write};
use std::path::Path;

/// Teto para ler um arquivo inteiro (importacao e previa de arquivo avulso).
pub const MAX_NOTE_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn read_note_bytes(path: &Path) -> io::Result<Vec<u8>> {
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
    if buf.len() as u64 > MAX_NOTE_FILE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "arquivo grande demais"));
    }
    Ok(buf)
}

pub fn read_text_file(path: &Path) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&read_note_bytes(path)?).into_owned())
}

// Os blocos têm o mesmo tamanho ao calcular a versão em memória e no disco.
const HASH_BLOCK: usize = 32 * 1024;

pub fn file_version(data: &[u8]) -> u64 {
    let mut hash = DefaultHasher::new();
    for block in data.chunks(HASH_BLOCK) {
        hash.write(block);
    }
    hash.finish()
}

pub fn file_snapshot(path: &Path) -> io::Result<(String, u64)> {
    let bytes = read_note_bytes(path)?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), file_version(&bytes)))
}

pub enum FileWrite {
    Overwrite,
    Create,
    IfUnchanged(u64),
}

fn check_file_version(path: &Path, expected: u64) -> io::Result<()> {
    // O buffer pode ter crescido além do limite de importação desde a abertura.
    // Compare em blocos para que os próximos salvamentos continuem possíveis.
    let mut file = fs::File::open(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            io::Error::new(e.kind(), "arquivo removido; :w! permite recriá-lo")
        } else {
            e
        }
    })?;
    let mut remaining = file.metadata()?.len();
    let mut block = [0u8; HASH_BLOCK];
    let mut hash = DefaultHasher::new();
    while remaining != 0 {
        let n = remaining.min(HASH_BLOCK as u64) as usize;
        file.read_exact(&mut block[..n])?;
        hash.write(&block[..n]);
        remaining -= n as u64;
    }
    if file.read(&mut block[..1])? != 0 || hash.finish() != expected {
        return Err(io::Error::new(io::ErrorKind::InvalidData,
            "nota alterada em outro lugar; :e! recarrega ou :w! sobrescreve"));
    }
    Ok(())
}

/// Publica um temporário completo no mesmo diretório. Ao criar uma nota nova,
/// a própria publicação falha se o destino já existir.
pub fn write_file(path: &Path, data: &[u8], mode: FileWrite) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    match mode {
        FileWrite::Create => tmp.persist_noclobber(path),
        FileWrite::IfUnchanged(expected) => {
            check_file_version(path, expected)?;
            tmp.persist(path)
        }
        FileWrite::Overwrite => tmp.persist(path),
    }
    .map(|_| ())
    .map_err(|e| match e.error.kind() {
        io::ErrorKind::AlreadyExists => io::Error::new(e.error.kind(), "já existe um arquivo neste destino"),
        _ => e.error,
    })
}

/// Preserva a origem até publicar o conteúdo completo num destino exclusivo.
pub fn rename_note_file(src: &Path, dest: &Path, version: Option<u64>, data: &[u8]) -> Result<u64, String> {
    if let Some(expected) = version {
        check_file_version(src, expected).map_err(|e| format!("{}: {e}", src.display()))?;
    } else if src.try_exists().map_err(|e| e.to_string())? {
        return Err(format!("{} foi criado em outro lugar", src.display()));
    }
    write_file(dest, data, FileWrite::Create)
        .map_err(|e| format!("falha ao criar {}: {e}", dest.display()))?;
    if let Some(expected) = version {
        check_file_version(src, expected).and_then(|_| fs::remove_file(src)).map_err(|e|
            format!("{} foi gravado; falha ao verificar ou remover {}: {e}", dest.display(), src.display())
        )?;
    }
    Ok(file_version(data))
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

#[derive(Debug)]
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
    let entries = fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))?;
    let tx = db.transaction(rusqlite::TransactionBehavior::Deferred).map_err(|e| e.to_string())?;
    let (mut added, mut updated) = (0, 0);
    for entry in entries {
        let entry = entry.map_err(|e| format!("{dir}: {e}"))?;
        let fname = entry.file_name().to_string_lossy().into_owned();
        if fname.starts_with('.') || !is_markdown_ext(&fname) {
            continue;
        }
        let path = entry.path();
        let meta = fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.is_dir() {
            continue;
        }
        // Nome logico = nome do arquivo sem a ultima extensao.
        let name = match fname.rfind('.') {
            Some(i) => &fname[..i],
            None => &fname[..],
        };
        if name.is_empty() || name.len() >= 256 {
            return Err(format!("{}: nome deve ter de 1 a 255 bytes", path.display()));
        }
        let content = read_text_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .filter(|&t| t > 0)
            .unwrap_or_else(now_secs);
        if db.import_note(name, &content, mtime).map_err(|e| format!("{}: {e}", path.display()))? {
            added += 1;
        } else {
            updated += 1;
        }
    }
    tx.commit().map_err(|e| format!("falha ao confirmar a importação de {dir}: {e}"))?;
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
        if write_file(&path, content.as_bytes(), FileWrite::Overwrite).is_ok() {
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
        write_file(&p, b"um", FileWrite::Create).unwrap();
        write_file(&p, b"dois", FileWrite::Overwrite).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"dois");
        let n = fs::read_dir(d.path()).unwrap().count();
        assert_eq!(n, 1, "nenhum .tmp sobra");
    }

    #[test]
    fn file_versions_compare_raw_bytes_across_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nota.md");
        let mut bytes = vec![b'a'; HASH_BLOCK * 2 + 1];
        bytes[HASH_BLOCK] = 0xff;
        fs::write(&path, &bytes).unwrap();
        let (text, version) = file_snapshot(&path).unwrap();
        check_file_version(&path, version).unwrap();
        bytes[HASH_BLOCK] = 0xfe;
        fs::write(&path, &bytes).unwrap();
        assert_eq!(text, file_snapshot(&path).unwrap().0);
        assert!(write_file(&path, b"novo", FileWrite::IfUnchanged(version)).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
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
    fn oversized_eligible_file_aborts_import() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("válida.md"), "corpo").unwrap();
        fs::File::create(dir.path().join("grande.md")).unwrap().set_len(MAX_NOTE_FILE_BYTES + 1).unwrap();
        let db = Db::open_memory();
        let err = import_notes_from_dir(&db, dir.path().to_str().unwrap()).unwrap_err();
        assert!(err.contains("grande.md") && err.contains("grande demais"), "{err}");
        assert_eq!(db.count_notes(), 0);
    }

    #[test]
    fn import_database_error_rolls_back_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("notas.db");
        let db = Db::open(&db_path).unwrap();
        let external = rusqlite::Connection::open(&db_path).unwrap();
        external.execute_batch(
            "CREATE TRIGGER fail_second BEFORE INSERT ON notes WHEN EXISTS (SELECT 1 FROM notes)
             BEGIN SELECT RAISE(ABORT, 'falha simulada'); END;"
        ).unwrap();
        for filename in ["A.md", "B.md"] {
            fs::write(dir.path().join(filename), filename).unwrap();
        }
        let err = import_notes_from_dir(&db, dir.path().to_str().unwrap()).unwrap_err();
        assert!(err.contains(".md") && err.contains("falha simulada"), "{err}");
        assert_eq!(db.count_notes(), 0);
        external.execute_batch("DROP TRIGGER fail_second").unwrap();
        assert_eq!(import_notes_from_dir(&db, dir.path().to_str().unwrap()).unwrap().added, 2);
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
