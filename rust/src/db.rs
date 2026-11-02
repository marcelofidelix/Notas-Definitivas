//! Camada SQLite. Mesmo esquema e mesmas regras da versao em C: o `notas.db`
//! e intercambiavel entre as duas versoes.

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::Path;

pub const DB_SCHEMA_VERSION: i32 = 1;

/// Metadados de uma nota (sem o conteudo).
#[derive(Clone, Debug)]
pub struct NoteRow {
    pub id: i64,
    pub name: String,
    pub mtime: i64,
    pub attention: u8,
}

pub struct Db {
    conn: Connection,
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
        let _ = conn.query_row("PRAGMA journal_mode=WAL;", [], |_| Ok(()));
        let _ = conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 name      TEXT    NOT NULL UNIQUE,
                 content   TEXT    NOT NULL DEFAULT '',
                 mtime     INTEGER NOT NULL DEFAULT 0,
                 attention INTEGER NOT NULL DEFAULT 0);
             CREATE INDEX IF NOT EXISTS idx_notes_mtime ON notes(mtime DESC);",
        )
        .map_err(|e| e.to_string())?;

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
        Ok(Db { conn })
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
                 attention INTEGER NOT NULL DEFAULT 0);",
        )
        .unwrap();
        Db { conn }
    }

    pub fn begin(&self) -> bool {
        self.conn.execute_batch("BEGIN TRANSACTION;").is_ok()
    }
    pub fn commit(&self) -> bool {
        self.conn.execute_batch("COMMIT;").is_ok()
    }
    pub fn rollback(&self) {
        let _ = self.conn.execute_batch("ROLLBACK;");
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
        self.conn
            .prepare_cached("INSERT INTO notes(name, content, mtime, attention) VALUES(?1,?2,?3,?4);")
            .and_then(|mut st| st.execute(params![name, content, mtime, attention.min(6)]))
            .ok()?;
        Some(self.conn.last_insert_rowid())
    }

    pub fn update_content(&self, id: i64, content: &str, mtime: i64) -> bool {
        if id <= 0 {
            return false;
        }
        let mtime = if mtime > 0 { mtime } else { now_secs() };
        self.conn
            .prepare_cached("UPDATE notes SET content=?1, mtime=?2 WHERE id=?3;")
            .and_then(|mut st| st.execute(params![content, mtime, id]))
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    /// Falha (false) se ja existir outra nota com esse nome (UNIQUE).
    pub fn update_name(&self, id: i64, new_name: &str) -> bool {
        if id <= 0 || new_name.is_empty() {
            return false;
        }
        self.conn
            .prepare_cached("UPDATE notes SET name=?1 WHERE id=?2;")
            .and_then(|mut st| st.execute(params![new_name, id]))
            .map(|n| n > 0)
            .unwrap_or(false)
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
    fn crud_roundtrip() {
        let db = Db::open_memory();
        let id = db.insert_note("Alfa", "# Alfa\n", 1000, 2).unwrap();
        assert_eq!(db.find_id_by_name("Alfa"), Some(id));
        assert_eq!(db.get_content(id).as_deref(), Some("# Alfa\n"));
        assert_eq!(db.get_name(id).as_deref(), Some("Alfa"));
        assert!(db.update_content(id, "novo", 2000));
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
        assert!(!db.update_name(a, "B"), "UNIQUE impede o nome repetido");
        assert!(db.update_name(a, "C"));
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
        assert!(db.begin());
        db.insert_note("T", "", 1, 0).unwrap();
        db.rollback();
        assert_eq!(db.count_notes(), 0);
    }
}
