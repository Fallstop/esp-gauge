use super::{Feed, Source};
use rusqlite::{Connection, OpenFlags};
use std::{fs::File, path::Path, time::Duration};

pub fn open(path: &Path) -> Option<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    db.busy_timeout(Duration::from_millis(100)).ok()?;
    Some(db)
}
pub fn scalar(db: &Connection, query: &str) -> Option<f64> {
    db.query_row(query, [], |r| r.get::<_, f64>(0)).ok()
}
pub fn codex(feed: &mut Feed, home: &Path) {
    let dir = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    if let Some(db) = open(&dir.join("thread_history_1.sqlite")) {
        let count = active_codex(&db, &dir);
        feed.add(Source::new("codex_running", "Working agents", "Codex", "agents", 10.0, "Active Codex turns whose local writer still holds the session lock. Includes desktop and CLI work."), count);
        feed.add(Source::new("codex_completed", "Turns completed today", "Codex", "turns", 100.0, "Completed turns in the local Codex history, since local midnight."), scalar(&db, "SELECT count(*) * 1.0 FROM thread_turns WHERE status='completed' AND date(completed_at,'unixepoch','localtime')=date('now','localtime')"));
    }
}
fn active_codex(db: &Connection, dir: &Path) -> Option<f64> {
    let mut statement = db.prepare("SELECT DISTINCT thread_id FROM thread_turns WHERE status = 'inProgress' AND completed_at IS NULL").ok()?;
    let rows = statement.query_map([], |r| r.get::<_, String>(0)).ok()?;
    let mut count = 0;
    for id in rows.flatten() {
        if id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
            if let Ok(file) = File::open(dir.join("thread-writer-locks").join(format!("{id}.lock")))
            {
                match fs2::FileExt::try_lock_shared(&file) {
                    Err(e) if e.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                        count += 1
                    }
                    Ok(()) => {
                        let _ = fs2::FileExt::unlock(&file);
                    }
                    _ => {}
                }
            }
        }
    }
    Some(count as f64)
}

pub fn opencode(feed: &mut Feed, home: &Path) {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let Some(db) = open(&root.join("opencode/opencode.db")) else {
        return;
    };
    let query = "FROM message WHERE json_extract(data,'$.role')='assistant' AND date(time_created / 1000,'unixepoch','localtime')=date('now','localtime')";
    feed.add(
        Source::new(
            "opencode_tokens",
            "Output tokens today",
            "OpenCode",
            "tokens",
            100000.0,
            "Assistant output tokens in the local OpenCode database since midnight.",
        ),
        scalar(
            &db,
            &format!("SELECT coalesce(sum(json_extract(data,'$.tokens.output')),0)*1.0 {query}"),
        ),
    );
    feed.add(
        Source::new(
            "opencode_cost",
            "Estimated cost today",
            "OpenCode",
            "USD",
            10.0,
            "OpenCode’s recorded cost estimate for today’s assistant messages.",
        ),
        scalar(
            &db,
            &format!("SELECT coalesce(sum(json_extract(data,'$.cost')),0)*1.0 {query}"),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crashed_or_unlocked_codex_turns_are_not_running() {
        let dir = std::env::temp_dir().join(format!(
            "gauge-lock-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("thread-writer-locks")).unwrap();
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE thread_turns(thread_id TEXT, status TEXT, completed_at INTEGER);
            INSERT INTO thread_turns VALUES ('a1','inProgress',NULL), ('a2','inProgress',NULL),
                ('a3','inProgress',NULL), ('../bad','inProgress',NULL), ('a1','completed',12);",
        )
        .unwrap();
        let held = File::create(dir.join("thread-writer-locks/a1.lock")).unwrap();
        fs2::FileExt::lock_exclusive(&held).unwrap();
        File::create(dir.join("thread-writer-locks/a2.lock")).unwrap();
        assert_eq!(active_codex(&db, &dir), Some(1.0));
        fs2::FileExt::unlock(&held).unwrap();
        assert_eq!(active_codex(&db, &dir), Some(0.0));
        drop(held);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn incompatible_schema_is_unavailable() {
        let db = Connection::open_in_memory().unwrap();
        assert_eq!(active_codex(&db, Path::new(".")), None);
    }
}
