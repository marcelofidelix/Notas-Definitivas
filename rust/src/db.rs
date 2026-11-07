//! Camada SQLite. Mesmo esquema e mesmas regras da versao em C: o `notas.db`
//! e intercambiavel entre as duas versoes.

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

pub const DB_SCHEMA_VERSION: i32 = 1;

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

impl Db {
    /// Abre (criando se preciso) o banco e garante o esquema. Em qualquer falha
    /// o handle e descartado: nunca devolve um banco meio-aberto.
    pub fn open(path: &Path) -> Result<Db, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
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
        let _ = conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 name      TEXT    NOT NULL UNIQUE,
                 content   TEXT    NOT NULL DEFAULT '',
                 mtime     INTEGER NOT NULL DEFAULT 0,
                 attention INTEGER NOT NULL DEFAULT 0,
                 content_norm TEXT);
             CREATE INDEX IF NOT EXISTS idx_notes_mtime ON notes(mtime DESC);",
        )
        .map_err(|e| e.to_string())?;

        // Coluna do indice de busca: presente em bancos novos, ausente nos antigos
        // (inclusive os criados pela versao em C, que continua abrindo este banco
        // porque a coluna e apenas adicional). Nao mexemos em `user_version` para
        // preservar essa compatibilidade.
        if !conn
            .prepare("PRAGMA table_info(notes);")
            .and_then(|mut st| {
                let cols: Vec<String> = st
                    .query_map([], |r| r.get::<_, String>(1))?
                    .filter_map(|c| c.ok())
                    .collect();
                Ok(cols)
            })
            .map(|cols| cols.iter().any(|c| c == "content_norm"))
            .unwrap_or(false)
        {
            conn.execute_batch("ALTER TABLE notes ADD COLUMN content_norm TEXT;")
                .map_err(|e| e.to_string())?;
        }

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
        conn.execute_batch(
            "CREATE TABLE notes (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 name      TEXT    NOT NULL UNIQUE,
                 content   TEXT    NOT NULL DEFAULT '',
                 mtime     INTEGER NOT NULL DEFAULT 0,
                 attention INTEGER NOT NULL DEFAULT 0,
                 content_norm TEXT);",
        )
        .unwrap();
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

    /// Reserva o escritor antes de conferir a versão. O nome, o texto e o índice
    /// de busca passam para a nova versão no mesmo commit.
    pub fn save_note(
        &self,
        id: i64,
        expected: Option<u64>,
        rename: Option<&str>,
        content: &str,
    ) -> Result<u64, String> {
        let tx = self.transaction(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
        let (name, _, current) = self.snapshot(id).map_err(|e| e.to_string())?;
        if expected.is_some_and(|v| v != current) {
            return Err("nota alterada em outro lugar; :e! recarrega ou :w! sobrescreve".into());
        }
        let name = rename.unwrap_or(&name);
        let mtime = now_secs();
        let rows = tx.execute(
            "UPDATE notes SET name=?1, content=?2, mtime=?3, content_norm=?4 WHERE id=?5",
            params![name, content, mtime, crate::notes::build_content_norm(content), id],
        ).map_err(|e| e.to_string())?;
        if rows != 1 {
            return Err("nenhuma nota foi gravada".into());
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(note_version(name, mtime, content))
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
    /// normalizado de todas as notas so para montar a lista lateral.
    pub fn all_content_norms(&self) -> Vec<(i64, Option<String>)> {
        let Ok(mut st) = self.conn.prepare_cached("SELECT id, content_norm FROM notes;") else {
            return Vec::new();
        };
        let Ok(rows) = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))) else {
            return Vec::new();
        };
        rows.filter_map(|r| r.ok()).collect()
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

    pub fn get_content(&self, id: i64) -> Option<String> {
        self.conn
            .prepare_cached("SELECT content FROM notes WHERE id=?1;")
            .and_then(|mut st| st.query_row([id], |r| r.get::<_, String>(0)).optional())
            .ok()
            .flatten()
    }

    pub fn count_notes(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM notes;", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Todas as notas (sem conteudo), da mais recente para a mais antiga.
    pub fn list_notes(&self) -> Vec<NoteRow> {
        let Ok(mut st) = self
            .conn
            .prepare_cached("SELECT id, name, mtime, attention FROM notes ORDER BY mtime DESC;")
        else {
            return Vec::new();
        };
        let Ok(rows) = st.query_map([], |r| {
            Ok(NoteRow {
                id: r.get(0)?,
                name: r.get(1)?,
                mtime: r.get(2)?,
                attention: clamp_attention(r.get::<_, i64>(3)?),
            })
        }) else {
            return Vec::new();
        };
        rows.filter_map(|r| r.ok()).filter(|n| !n.name.is_empty()).collect()
    }

    /// (nome, conteudo) de todas as notas, para exportar.
    pub fn all_name_content(&self) -> Vec<(String, String)> {
        let Ok(mut st) = self
            .conn
            .prepare_cached("SELECT name, content FROM notes ORDER BY mtime DESC;")
        else {
            return Vec::new();
        };
        let Ok(rows) = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))) else {
            return Vec::new();
        };
        rows.filter_map(|r| r.ok()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_norm_follows_insert_and_update() {
        let db = Db::open_memory();
        let id = db.insert_note("Alfa", "# Título\nconteúdo inicial", 1, 0).unwrap();
        let norm = |db: &Db| db.all_content_norms().into_iter().find(|r| r.0 == id).and_then(|r| r.1).unwrap_or_default();
        assert!(norm(&db).contains("conteudo"), "{:?}", norm(&db));
        assert!(norm(&db).contains("titulo"), "o acento e dobrado na normalizacao");
        // Gravar conteudo novo atualiza o indice na mesma instrucao.
        db.save_note(id, None, None, "outro texto sem acento").unwrap();
        assert!(norm(&db).contains("outro"));
        assert!(!norm(&db).contains("conteudo"));
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
        let rows = db.list_notes();
        assert_eq!(rows.len(), 1);
        assert!(
            db.all_content_norms()[0].1.is_none(),
            "nota antiga ainda nao tem indice"
        );
        assert!(db.set_content_norm(rows[0].id, "texto"));
        assert_eq!(db.all_content_norms()[0].1.as_deref(), Some("texto"));
    }

    #[test]
    fn crud_roundtrip() {
        let db = Db::open_memory();
        let id = db.insert_note("Alfa", "# Alfa\n", 1000, 2).unwrap();
        assert_eq!(db.find_id_by_name("Alfa"), Some(id));
        assert_eq!(db.get_content(id).as_deref(), Some("# Alfa\n"));
        assert_eq!(db.get_name(id).as_deref(), Some("Alfa"));
        db.save_note(id, None, None, "novo").unwrap();
        assert_eq!(db.get_content(id).as_deref(), Some("novo"));
        assert!(db.set_attention(id, 9));
        assert_eq!(db.list_notes()[0].attention, 6, "nivel e limitado a 0..6");
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
        assert_eq!(db.list_notes()[0].attention, 6);
        db.conn.execute("UPDATE notes SET attention=-4 WHERE id=?1", [id]).unwrap();
        assert_eq!(db.list_notes()[0].attention, 0);
    }

    #[test]
    fn transaction_rollback_discards_inserts() {
        let db = Db::open_memory();
        let tx = db.transaction(TransactionBehavior::Deferred).unwrap();
        db.insert_note("T", "", 1, 0).unwrap();
        tx.rollback().unwrap();
        assert_eq!(db.count_notes(), 0);
    }
}
