//! Camada SQLite. Mesmo esquema e mesmas regras da versao em C: o `notas.db`
//! e intercambiavel entre as duas versoes.

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

pub const DB_SCHEMA_VERSION: i32 = 1;

/// Pasta (ao lado do banco) dos backups diarios.
pub const BACKUP_DIR: &str = "notas-backups";
/// Quantos backups diarios ficam guardados.
const BACKUPS_KEPT: usize = 7;
/// Quantas copias de recuperacao (versoes substituidas num salvamento de
/// emergencia) ficam na tabela `recovery`.
const RECOVERY_KEPT: usize = 20;

/// Esquema base. `content_norm` e adicional: a versao em C ignora a coluna.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS notes (
         id        INTEGER PRIMARY KEY AUTOINCREMENT,
         name      TEXT    NOT NULL UNIQUE,
         content   TEXT    NOT NULL DEFAULT '',
         mtime     INTEGER NOT NULL DEFAULT 0,
         attention INTEGER NOT NULL DEFAULT 0,
         content_norm TEXT);
     CREATE INDEX IF NOT EXISTS idx_notes_mtime ON notes(mtime DESC);";

/// Quem grava `content` sem recalcular `content_norm` (a versao em C, o
/// `sqlite3` na linha de comando) deixaria um indice velho para sempre: o gatilho
/// o invalida (NULL) e o Rust o reconstroi na proxima busca.
const NORM_TRIGGER: &str = "CREATE TRIGGER IF NOT EXISTS notes_norm_stale
     AFTER UPDATE OF content ON notes
     WHEN NEW.content_norm IS OLD.content_norm AND NEW.content IS NOT OLD.content
     BEGIN UPDATE notes SET content_norm=NULL WHERE id=NEW.id; END;";

/// Versoes que um salvamento de emergencia substituiu (rede de seguranca para o
/// caso de o buffer estar inconsistente depois de um panico).
const RECOVERY_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS recovery (
         id      INTEGER PRIMARY KEY AUTOINCREMENT,
         note_id INTEGER,
         name    TEXT    NOT NULL,
         content TEXT    NOT NULL,
         ts      INTEGER NOT NULL);";

/// Metadados de uma nota (sem o conteudo e sem o indice de busca, que e lido
/// sob demanda por `all_content_norms`).
#[derive(Clone, Debug)]
pub struct NoteRow {
    pub id: i64,
    pub name: String,
    pub mtime: i64,
    pub attention: u8,
}

pub struct Db {
    conn: Connection,
    /// Avisos de degradacao (ex.: modo WAL indisponivel). O banco continua
    /// utilizavel; a interface mostra o aviso no status inicial.
    pub warnings: Vec<String>,
}

impl Drop for Db {
    fn drop(&mut self) {
        // Deixa as estatisticas do planejador em dia (barato; falhar nao importa).
        let _ = self.conn.execute_batch("PRAGMA optimize;");
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn note_version(name: &str, mtime: i64, content: &str) -> u64 {
    let mut hash = DefaultHasher::new();
    (name, mtime, content).hash(&mut hash);
    hash.finish()
}

fn clamp_attention(level: i64) -> u8 {
    level.clamp(0, 6) as u8
}

/// Data civil (UTC) de um instante em segundos desde 1970 (algoritmo de Howard
/// Hinnant), sem depender de uma biblioteca de datas.
fn civil_from_secs(secs: i64) -> (i64, u32, u32) {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Cria o que falta do esquema. Devolve avisos (nunca impede o uso do banco, a
/// nao ser que a tabela principal nao possa ser criada).
fn ensure_schema(conn: &Connection) -> Result<Vec<String>, String> {
    conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;

    // Coluna do indice de busca: presente em bancos novos, ausente nos antigos
    // (inclusive os criados pela versao em C, que continua abrindo este banco
    // porque a coluna e apenas adicional). Nao mexemos em `user_version` para
    // preservar essa compatibilidade.
    let has_norm = conn
        .prepare("PRAGMA table_info(notes);")
        .and_then(|mut st| {
            let cols: Vec<String> = st
                .query_map([], |r| r.get::<_, String>(1))?
                .filter_map(|c| c.ok())
                .collect();
            Ok(cols)
        })
        .map(|cols| cols.iter().any(|c| c == "content_norm"))
        .unwrap_or(false);
    if !has_norm {
        conn.execute_batch("ALTER TABLE notes ADD COLUMN content_norm TEXT;")
            .map_err(|e| e.to_string())?;
    }

    let mut warnings = Vec::new();
    if let Err(e) = conn.execute_batch(NORM_TRIGGER) {
        warnings.push(format!("gatilho do indice de busca nao criado: {e}"));
    }
    if let Err(e) = conn.execute_batch(RECOVERY_SCHEMA) {
        warnings.push(format!("tabela de recuperacao nao criada: {e}"));
    }
    Ok(warnings)
}

impl Db {
    /// Abre (criando se preciso) o banco e garante o esquema. Em qualquer falha
    /// o handle e descartado: nunca devolve um banco meio-aberto.
    pub fn open(path: &Path) -> Result<Db, String> {
        // Uma unica conexao, usada so pela thread principal: sem trava interna.
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(3000))
            .map_err(|e| e.to_string())?;

        // Endurecimento contra um notas.db adulterado (nao afeta o CRUD).
        use rusqlite::config::DbConfig;
        let _ = conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true);
        let _ = conn.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false);

        // WAL: leitura/escrita concorrentes; se o sistema de arquivos nao
        // suportar, a SQLite mantem o journal padrao e seguimos funcionando.
        let mut warnings = Vec::new();
        if conn.query_row("PRAGMA journal_mode=WAL;", [], |_| Ok(())).is_err() {
            warnings.push("WAL indisponivel neste sistema de arquivos".to_string());
        }
        let _ = conn.execute_batch(
            "PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA temp_store=MEMORY; \
             PRAGMA journal_size_limit=67108864;",
        );

        warnings.extend(ensure_schema(&conn)?);

        let version: i32 = conn
            .query_row("PRAGMA user_version;", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > DB_SCHEMA_VERSION {
            return Err(format!(
                "banco com versao de esquema {} mais nova que a suportada ({})",
                version, DB_SCHEMA_VERSION
            ));
        }
        if version < DB_SCHEMA_VERSION {
            conn.execute_batch("PRAGMA user_version=1;")
                .map_err(|e| e.to_string())?;
        }
        Ok(Db { conn, warnings })
    }

    /// Banco em memoria (testes).
    #[cfg(test)]
    pub fn open_memory() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        Db { conn, warnings: Vec::new() }
    }

    pub fn transaction(&self, behavior: TransactionBehavior) -> rusqlite::Result<Transaction<'_>> {
        Transaction::new_unchecked(&self.conn, behavior)
    }

    /// Nome, conteúdo e versão pertencem à mesma leitura do SQLite.
    pub fn snapshot(&self, id: i64) -> rusqlite::Result<(String, String, u64)> {
        self.conn.prepare_cached("SELECT name, content, mtime FROM notes WHERE id=?1")?
            .query_row([id], |row| {
                let name: String = row.get(0)?;
                let content: String = row.get(1)?;
                let mtime: i64 = row.get(2)?;
                let version = note_version(&name, mtime, &content);
                Ok((name, content, version))
            })
    }

    /// Nome e versão sem copiar o conteúdo: o texto é lido por referência e só
    /// passa pelo hash (salvar não precisa de uma cópia da nota antiga).
    fn name_and_version(&self, id: i64) -> rusqlite::Result<(String, u64)> {
        self.conn.prepare_cached("SELECT name, content, mtime FROM notes WHERE id=?1")?
            .query_row([id], |row| {
                let name: String = row.get(0)?;
                let content = row.get_ref(1)?.as_str().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
                })?;
                let mtime: i64 = row.get(2)?;
                let version = note_version(&name, mtime, content);
                Ok((name, version))
            })
    }

    /// Reserva o escritor antes de conferir a versão. O nome, o texto e o índice
    /// de busca (`norm`, já calculado por quem chama) passam para a nova versão
    /// no mesmo commit.
    pub fn save_note_with_norm(
        &self,
        id: i64,
        expected: Option<u64>,
        rename: Option<&str>,
        content: &str,
        norm: &str,
    ) -> Result<u64, String> {
        let tx = self.transaction(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
        let (name, current) = self.name_and_version(id).map_err(|e| e.to_string())?;
        if expected.is_some_and(|v| v != current) {
            return Err("nota alterada em outro lugar; :e! recarrega ou :w! sobrescreve".into());
        }
        let name = rename.unwrap_or(&name);
        let mtime = now_secs();
        let rows = tx.execute(
            "UPDATE notes SET name=?1, content=?2, mtime=?3, content_norm=?4 WHERE id=?5",
            params![name, content, mtime, norm, id],
        ).map_err(|e| e.to_string())?;
        if rows != 1 {
            return Err("nenhuma nota foi gravada".into());
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(note_version(name, mtime, content))
    }

    /// Como `save_note_with_norm`, calculando o indice (testes e usos simples).
    #[cfg(test)]
    pub fn save_note(
        &self,
        id: i64,
        expected: Option<u64>,
        rename: Option<&str>,
        content: &str,
    ) -> Result<u64, String> {
        self.save_note_with_norm(id, expected, rename, content, &crate::notes::build_content_norm(content))
    }

    /// Guarda uma copia da versao hoje gravada de uma nota, antes de um
    /// salvamento de emergencia sobrescreve-la.
    pub fn stash_recovery(&self, id: i64, now: i64) -> bool {
        let copied = self
            .conn
            .execute(
                "INSERT INTO recovery(note_id, name, content, ts) SELECT id, name, content, ?2 FROM notes WHERE id=?1",
                params![id, now],
            )
            .map(|n| n > 0)
            .unwrap_or(false);
        if copied {
            let _ = self.conn.execute(
                "DELETE FROM recovery WHERE id NOT IN (SELECT id FROM recovery ORDER BY id DESC LIMIT ?1)",
                [RECOVERY_KEPT as i64],
            );
        }
        copied
    }

    #[cfg(test)]
    pub fn recovery_count(&self) -> i64 {
        self.conn.query_row("SELECT COUNT(*) FROM recovery", [], |r| r.get(0)).unwrap_or(-1)
    }

    /// Backup diario consistente (`VACUUM INTO`) numa pasta ao lado do banco,
    /// mantendo os ultimos `BACKUPS_KEPT`. Devolve o arquivo criado, ou `None`
    /// se o de hoje ja existe ou o banco esta vazio.
    pub fn backup_if_due(&self, db_path: &Path, now: i64) -> Result<Option<PathBuf>, String> {
        if self.count_notes() == 0 {
            return Ok(None);
        }
        let base = db_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let dir = base.join(BACKUP_DIR);
        let (y, m, d) = civil_from_secs(now);
        let dest = dir.join(format!("notas-backup-{y:04}{m:02}{d:02}.db"));
        if dest.exists() {
            return Ok(None);
        }
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let dest_str = dest.to_str().ok_or_else(|| "caminho do backup nao e UTF-8".to_string())?;
        self.conn.execute("VACUUM INTO ?1", [dest_str]).map_err(|e| e.to_string())?;

        // Descarta os mais antigos (o nome carrega a data, entao ordenar basta).
        let mut backups: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("notas-backup-") && n.ends_with(".db"))
            })
            .collect();
        backups.sort();
        while backups.len() > BACKUPS_KEPT {
            let _ = std::fs::remove_file(backups.remove(0));
        }
        Ok(Some(dest))
    }

    pub fn find_id_by_name(&self, name: &str) -> Option<i64> {
        if name.is_empty() {
            return None;
        }
        self.conn
            .prepare_cached("SELECT id FROM notes WHERE name=?1 LIMIT 1;")
            .and_then(|mut st| st.query_row([name], |r| r.get(0)).optional())
            .ok()
            .flatten()
    }

    /// Insere e devolve o id; `None` se o nome ja existe ou o disco falhou.
    pub fn insert_note(&self, name: &str, content: &str, mtime: i64, attention: u8) -> Option<i64> {
        if name.is_empty() {
            return None;
        }
        let mtime = if mtime > 0 { mtime } else { now_secs() };
        let norm = crate::notes::build_content_norm(content);
        self.conn
            .prepare_cached(
                "INSERT INTO notes(name, content, mtime, attention, content_norm) VALUES(?1,?2,?3,?4,?5);",
            )
            .and_then(|mut st| st.execute(params![name, content, mtime, attention.min(6), norm]))
            .ok()?;
        Some(self.conn.last_insert_rowid())
    }

    pub fn import_note(&self, name: &str, content: &str, mtime: i64) -> rusqlite::Result<bool> {
        let norm = crate::notes::build_content_norm(content);
        let updated = self.conn.prepare_cached(
            "UPDATE notes SET content=?1, mtime=?2, content_norm=?3 WHERE name=?4",
        )?.execute(params![content, mtime, norm, name])?;
        if updated == 1 {
            return Ok(false);
        }
        let inserted = self.conn.prepare_cached(
            "INSERT INTO notes(name, content, mtime, content_norm, attention) VALUES(?1,?2,?3,?4,0)",
        )?.execute(params![name, content, mtime, norm])?;
        if inserted != 1 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(true)
    }

    /// Indice de busca de todas as notas (`None` quando ainda nao foi gerado).
    ///
    /// Carregado sob demanda: abrir o programa nao precisa mover o conteudo
    /// normalizado de todas as notas so para montar a lista lateral. Uma linha
    /// ilegivel fica de fora (a nota continua na lista, so sem busca por conteudo);
    /// uma falha da consulta e devolvida.
    pub fn all_content_norms(&self) -> Result<Vec<(i64, Option<String>)>, String> {
        let mut st = self
            .conn
            .prepare_cached("SELECT id, content_norm FROM notes;")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Grava o indice de busca reconstruido de uma nota antiga (sem a coluna).
    pub fn set_content_norm(&self, id: i64, norm: &str) -> bool {
        id > 0
            && self
                .conn
                .prepare_cached("UPDATE notes SET content_norm=?1 WHERE id=?2;")
                .and_then(|mut st| st.execute(params![norm, id]))
                .is_ok()
    }

    pub fn delete_note(&self, id: i64) -> bool {
        if id <= 0 {
            return false;
        }
        self.conn
            .prepare_cached("DELETE FROM notes WHERE id=?1;")
            .and_then(|mut st| st.execute([id]))
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    pub fn set_attention(&self, id: i64, level: u8) -> bool {
        if id <= 0 {
            return false;
        }
        self.conn
            .prepare_cached("UPDATE notes SET attention=?1 WHERE id=?2;")
            .and_then(|mut st| st.execute(params![level.min(6), id]))
            .is_ok()
    }

    /// Data de modificacao gravada (usada para detectar alteracao por outra instancia).
    #[cfg(test)]
    pub fn get_mtime(&self, id: i64) -> Option<i64> {
        self.conn
            .prepare_cached("SELECT mtime FROM notes WHERE id=?1;")
            .and_then(|mut st| st.query_row([id], |r| r.get(0)).optional())
            .ok()
            .flatten()
    }

    pub fn get_name(&self, id: i64) -> Option<String> {
        self.conn
            .prepare_cached("SELECT name FROM notes WHERE id=?1;")
            .and_then(|mut st| st.query_row([id], |r| r.get(0)).optional())
            .ok()
            .flatten()
    }

    /// Conteudo da nota: `Ok(None)` so quando a nota nao existe; um erro do
    /// SQLite (banco ocupado, linha ilegivel) e devolvido, nao disfarcado de "vazio".
    pub fn try_get_content(&self, id: i64) -> Result<Option<String>, String> {
        self.conn
            .prepare_cached("SELECT content FROM notes WHERE id=?1;")
            .and_then(|mut st| st.query_row([id], |r| r.get::<_, String>(0)).optional())
            .map_err(|e| e.to_string())
    }

    #[cfg(test)]
    pub fn get_content(&self, id: i64) -> Option<String> {
        self.try_get_content(id).ok().flatten()
    }

    pub fn count_notes(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM notes;", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Todas as notas (sem conteudo), da mais recente para a mais antiga, e
    /// quantas linhas nao puderam ser lidas (devem ser avisadas ao usuario).
    pub fn list_notes(&self) -> Result<(Vec<NoteRow>, usize), String> {
        let mut st = self
            .conn
            .prepare_cached("SELECT id, name, mtime, attention FROM notes ORDER BY mtime DESC;")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| {
                Ok(NoteRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    mtime: r.get(2)?,
                    attention: clamp_attention(r.get::<_, i64>(3)?),
                })
            })
            .map_err(|e| e.to_string())?;
        let (mut notes, mut unreadable) = (Vec::new(), 0usize);
        for row in rows {
            match row {
                Ok(n) if n.name.is_empty() => {}
                Ok(n) => notes.push(n),
                Err(_) => unreadable += 1,
            }
        }
        Ok((notes, unreadable))
    }

    /// Percorre (nome, conteudo) de todas as notas, uma por vez, sem carregar o
    /// banco inteiro na memoria (exportar). Uma falha de leitura interrompe e e
    /// devolvida: exportar so parte das notas sem avisar seria pior que falhar.
    pub fn for_each_note(&self, mut f: impl FnMut(&str, &str)) -> Result<usize, String> {
        let mut st = self
            .conn
            .prepare_cached("SELECT name, content FROM notes ORDER BY mtime DESC;")
            .map_err(|e| e.to_string())?;
        let mut rows = st.query([]).map_err(|e| e.to_string())?;
        let mut n = 0;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let name = row.get_ref(0).map_err(|e| e.to_string())?.as_str().map_err(|e| e.to_string())?;
            let content = row.get_ref(1).map_err(|e| e.to_string())?.as_str().map_err(|e| e.to_string())?;
            f(name, content);
            n += 1;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_norm_follows_insert_and_update() {
        let db = Db::open_memory();
        let id = db.insert_note("Alfa", "# Título\nconteúdo inicial", 1, 0).unwrap();
        let norm = |db: &Db| {
            db.all_content_norms().unwrap().into_iter().find(|r| r.0 == id).and_then(|r| r.1).unwrap_or_default()
        };
        assert!(norm(&db).contains("conteudo"), "{:?}", norm(&db));
        assert!(norm(&db).contains("titulo"), "o acento e dobrado na normalizacao");
        // Gravar conteudo novo atualiza o indice na mesma instrucao.
        db.save_note(id, None, None, "outro texto sem acento").unwrap();
        assert!(norm(&db).contains("outro"));
        assert!(!norm(&db).contains("conteudo"));
    }

    #[test]
    fn external_content_change_invalidates_the_search_index() {
        // Quem grava so `content` (a versao em C) nao toca em `content_norm`: o
        // gatilho zera o indice para o Rust reconstruir, em vez de buscar no texto velho.
        let db = Db::open_memory();
        let id = db.insert_note("A", "texto antigo", 1, 0).unwrap();
        assert!(db.all_content_norms().unwrap()[0].1.is_some());
        db.conn.execute("UPDATE notes SET content='texto novo' WHERE id=?1", [id]).unwrap();
        assert!(db.all_content_norms().unwrap()[0].1.is_none(), "indice velho foi invalidado");
        // Uma gravacao do proprio programa (conteudo e indice juntos) continua valida.
        db.save_note(id, None, None, "mais um").unwrap();
        assert_eq!(db.all_content_norms().unwrap()[0].1.as_deref(), Some("mais um"));
        // Gravar o mesmo conteudo nao invalida nada.
        db.conn.execute("UPDATE notes SET content='mais um' WHERE id=?1", [id]).unwrap();
        assert!(db.all_content_norms().unwrap()[0].1.is_some());
    }

    #[test]
    fn old_database_gains_the_search_index_column() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notas.db");
        {
            // Esquema anterior, como a versao em C deixava o banco.
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE notes (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     name TEXT NOT NULL UNIQUE,
                     content TEXT NOT NULL DEFAULT '',
                     mtime INTEGER NOT NULL DEFAULT 0,
                     attention INTEGER NOT NULL DEFAULT 0);
                 INSERT INTO notes(name, content, mtime, attention) VALUES('Antiga','texto','1',0);",
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let (rows, bad) = db.list_notes().unwrap();
        assert_eq!((rows.len(), bad), (1, 0));
        assert!(
            db.all_content_norms().unwrap()[0].1.is_none(),
            "nota antiga ainda nao tem indice"
        );
        assert!(db.set_content_norm(rows[0].id, "texto"));
        assert_eq!(db.all_content_norms().unwrap()[0].1.as_deref(), Some("texto"));
    }

    #[test]
    fn crud_roundtrip() {
        let db = Db::open_memory();
        let id = db.insert_note("Alfa", "# Alfa\n", 1000, 2).unwrap();
        assert_eq!(db.find_id_by_name("Alfa"), Some(id));
        assert_eq!(db.get_content(id).as_deref(), Some("# Alfa\n"));
        assert_eq!(db.try_get_content(id).unwrap().as_deref(), Some("# Alfa\n"));
        assert_eq!(db.try_get_content(id + 99).unwrap(), None, "nota inexistente nao e erro");
        assert_eq!(db.get_name(id).as_deref(), Some("Alfa"));
        db.save_note(id, None, None, "novo").unwrap();
        assert_eq!(db.get_content(id).as_deref(), Some("novo"));
        assert!(db.set_attention(id, 9));
        assert_eq!(db.list_notes().unwrap().0[0].attention, 6, "nivel e limitado a 0..6");
        assert_eq!(db.count_notes(), 1);
        assert!(db.delete_note(id));
        assert_eq!(db.count_notes(), 0);
        assert!(!db.delete_note(id));
    }

    #[test]
    fn names_are_unique_and_rename_collides() {
        let db = Db::open_memory();
        let a = db.insert_note("A", "", 1, 0).unwrap();
        let _b = db.insert_note("B", "", 1, 0).unwrap();
        assert!(db.insert_note("A", "", 1, 0).is_none());
        assert!(db.save_note(a, None, Some("B"), "").is_err());
        assert!(db.save_note(a, None, Some("C"), "").is_ok());
        assert_eq!(db.find_id_by_name("C"), Some(a));
    }

    #[test]
    fn corrupted_attention_reads_as_zero_to_six() {
        let db = Db::open_memory();
        let id = db.insert_note("X", "", 1, 0).unwrap();
        db.conn.execute("UPDATE notes SET attention=99 WHERE id=?1", [id]).unwrap();
        assert_eq!(db.list_notes().unwrap().0[0].attention, 6);
        db.conn.execute("UPDATE notes SET attention=-4 WHERE id=?1", [id]).unwrap();
        assert_eq!(db.list_notes().unwrap().0[0].attention, 0);
    }

    #[test]
    fn unreadable_rows_are_counted_not_hidden() {
        let db = Db::open_memory();
        db.insert_note("Boa", "", 1, 0).unwrap();
        // Tipo errado na coluna `mtime` (texto onde se espera inteiro).
        db.conn.execute("INSERT INTO notes(name, content, mtime) VALUES('Ruim','', 'x')", []).unwrap();
        let (notes, bad) = db.list_notes().unwrap();
        assert_eq!((notes.len(), bad), (1, 1), "a linha ruim e contada, a boa continua na lista");
    }

    #[test]
    fn transaction_rollback_discards_inserts() {
        let db = Db::open_memory();
        let tx = db.transaction(TransactionBehavior::Deferred).unwrap();
        db.insert_note("T", "", 1, 0).unwrap();
        tx.rollback().unwrap();
        assert_eq!(db.count_notes(), 0);
    }

    #[test]
    fn for_each_note_streams_every_note() {
        let db = Db::open_memory();
        db.insert_note("A", "um", 2, 0).unwrap();
        db.insert_note("B", "dois", 1, 0).unwrap();
        let mut visto = Vec::new();
        let n = db.for_each_note(|nome, texto| visto.push(format!("{nome}={texto}"))).unwrap();
        assert_eq!((n, visto), (2, vec!["A=um".to_string(), "B=dois".to_string()]));
    }

    #[test]
    fn recovery_keeps_the_replaced_version_and_prunes() {
        let db = Db::open_memory();
        let id = db.insert_note("R", "versao boa", 1, 0).unwrap();
        assert!(db.stash_recovery(id, 10));
        db.save_note(id, None, None, "versao corrompida").unwrap();
        let guardada: String = db.conn.query_row("SELECT content FROM recovery", [], |r| r.get(0)).unwrap();
        assert_eq!(guardada, "versao boa");
        for t in 0..(RECOVERY_KEPT as i64 + 10) {
            assert!(db.stash_recovery(id, 20 + t));
        }
        assert_eq!(db.recovery_count(), RECOVERY_KEPT as i64);
        assert!(!db.stash_recovery(id + 100, 1), "nota inexistente nao gera copia");
    }

    #[test]
    fn civil_dates_are_correct() {
        assert_eq!(civil_from_secs(0), (1970, 1, 1));
        assert_eq!(civil_from_secs(1_700_000_000), (2023, 11, 14));
        assert_eq!(civil_from_secs(951_782_400), (2000, 2, 29), "ano bissexto");
        assert_eq!(civil_from_secs(-86_400), (1969, 12, 31));
    }

    #[test]
    fn daily_backup_is_created_once_and_rotated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notas.db");
        let db = Db::open(&path).unwrap();
        assert!(db.backup_if_due(&path, 1_700_000_000).unwrap().is_none(), "banco vazio nao gera backup");
        db.insert_note("Importante", "conteudo", 1, 0).unwrap();
        let primeiro = db.backup_if_due(&path, 1_700_000_000).unwrap().expect("cria o de hoje");
        assert!(primeiro.ends_with("notas-backup-20231114.db"));
        assert!(db.backup_if_due(&path, 1_700_000_500).unwrap().is_none(), "um por dia");
        // O backup e um banco completo e abrivel.
        let copia = Db::open(&primeiro).unwrap();
        assert_eq!(copia.count_notes(), 1);
        drop(copia);
        for dia in 1..=10 {
            db.backup_if_due(&path, 1_700_000_000 + dia * 86_400).unwrap();
        }
        let restantes = std::fs::read_dir(dir.path().join(BACKUP_DIR))
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".db"))
            .count();
        assert_eq!(restantes, BACKUPS_KEPT, "so os mais recentes ficam");
        assert!(!primeiro.exists(), "o mais antigo foi descartado");
    }
}
