use std::{fs, path::Path, time::Duration};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::lessons::LESSONS;

const SCHEMA_VERSION: i32 = 2;
const STORAGE_ERROR: &str = "Unable to access local progress. Check free disk space and restart the app.";
pub type Result<T> = std::result::Result<T, String>;

fn storage_error(_: impl std::fmt::Display) -> String { STORAGE_ERROR.into() }

#[derive(Debug, Serialize)]
pub struct Lesson {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub level: i32,
}

#[derive(Debug, Serialize)]
pub struct Progress {
    pub id: i64,
    pub lesson_id: i64,
    pub wpm: f64,
    pub accuracy: f64,
    pub completed_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressInput {
    pub username: String,
    pub lesson_id: i64,
    pub wpm: f64,
    pub accuracy: f64,
}

pub struct Database(Connection);

fn private_permissions(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(storage_error)?;
    }
    #[cfg(not(unix))] let _ = (path, mode);
    Ok(())
}

fn username(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 {
        return Err("Username must contain 1–80 characters.".into());
    }
    Ok(value)
}

impl Database {
    pub fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() { return Err("An absolute app-data directory is required.".into()); }
        fs::create_dir_all(directory).map_err(storage_error)?;
        private_permissions(directory, 0o700)?;
        let path = directory.join("colemak.db");
        if path.is_symlink() { return Err("The database must not be a symbolic link.".into()); }
        let existed = path.exists();
        let mut connection = Connection::open(&path).map_err(storage_error)?;
        private_permissions(&path, 0o600)?;
        connection.busy_timeout(Duration::from_secs(5)).map_err(storage_error)?;
        let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).map_err(storage_error)?;
        if version > SCHEMA_VERSION { return Err("This database requires a newer version of TypeDH.".into()); }
        if existed && version < SCHEMA_VERSION {
            return Err("Unsupported development database. Move the old colemak.db and its WAL/SHM files out of the app-data folder while the app is closed, then restart.".into());
        }
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(storage_error)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage_error)?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS user (
            id INTEGER PRIMARY KEY, username VARCHAR(80) NOT NULL UNIQUE, created_at DATETIME);
            CREATE TABLE IF NOT EXISTS lesson (
            id INTEGER PRIMARY KEY, title VARCHAR(100) NOT NULL, content TEXT NOT NULL, level INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS progress (
            id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES user(id),
            lesson_id INTEGER NOT NULL REFERENCES lesson(id), wpm FLOAT, accuracy FLOAT, completed_at DATETIME);
            CREATE INDEX IF NOT EXISTS ix_progress_user_id_id ON progress(user_id,id);").map_err(storage_error)?;
        for (index, (title, content, level)) in LESSONS.iter().enumerate() {
            tx.execute("INSERT INTO lesson(id,title,content,level) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO NOTHING",
                params![index as i64 + 1, title, content, level]).map_err(storage_error)?;
        }
        tx.execute("INSERT INTO lesson(id,title,content,level) VALUES (999,'Custom Practice','',99) ON CONFLICT(id) DO NOTHING", []).map_err(storage_error)?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(Self(connection))
    }

    pub fn lessons(&self) -> Result<Vec<Lesson>> {
        let mut statement = self.0.prepare("SELECT id,title,content,level FROM lesson WHERE id != 999 ORDER BY level,id").map_err(storage_error)?;
        let rows = statement.query_map([], |row| Ok(Lesson { id: row.get(0)?, title: row.get(1)?, content: row.get(2)?, level: row.get(3)? })).map_err(storage_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)
    }

    pub fn save_progress(&mut self, input: ProgressInput) -> Result<()> {
        let user = username(&input.username)?;
        if !(1..=i32::MAX as i64).contains(&input.lesson_id) || !input.wpm.is_finite()
            || !input.accuracy.is_finite() || !(0.0..=1000.0).contains(&input.wpm)
            || !(0.0..=100.0).contains(&input.accuracy) {
            return Err("Invalid lesson or progress values.".into());
        }
        let tx = self.0.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage_error)?;
        let exists = tx.query_row("SELECT id FROM lesson WHERE id=?1", [input.lesson_id], |row| row.get::<_, i64>(0)).optional().map_err(storage_error)?;
        if exists.is_none() { return Err("Unknown lesson.".into()); }
        tx.execute("INSERT INTO user(username,created_at) VALUES (?1,strftime('%Y-%m-%d %H:%M:%f','now')) ON CONFLICT(username) DO NOTHING", [user]).map_err(storage_error)?;
        tx.execute("INSERT INTO progress(user_id,lesson_id,wpm,accuracy,completed_at)
            SELECT id,?2,?3,?4,strftime('%Y-%m-%d %H:%M:%f','now') FROM user WHERE username=?1",
            params![user, input.lesson_id, input.wpm, input.accuracy]).map_err(storage_error)?;
        tx.commit().map_err(storage_error)
    }

    pub fn progress(&self, user: &str, limit: i64, before: Option<i64>) -> Result<Vec<Progress>> {
        let user = username(user)?;
        let before = before.unwrap_or(i64::MAX);
        if !(1..=200).contains(&limit) || before < 1 { return Err("Invalid history pagination.".into()); }
        let mut statement = self.0.prepare("SELECT p.id,p.lesson_id,p.wpm,p.accuracy,p.completed_at
            FROM progress p JOIN user u ON p.user_id=u.id
            WHERE u.username=?1 AND p.id<?2 ORDER BY p.id DESC LIMIT ?3").map_err(storage_error)?;
        let rows = statement.query_map(params![user,before,limit], |row| {
            let timestamp: String = row.get(4)?;
            Ok(Progress { id: row.get(0)?, lesson_id: row.get(1)?, wpm: row.get(2)?, accuracy: row.get(3)?,
                completed_at: format!("{}Z", timestamp.replace(' ', "T").trim_end_matches('Z')) })
        }).map_err(storage_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(storage_error)
    }
}

#[cfg(test)]
mod tests;
