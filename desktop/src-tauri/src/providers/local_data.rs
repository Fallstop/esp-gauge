use super::{Feed, Source};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use sysinfo::{Pid, System};

pub fn open(path: &Path) -> Option<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    db.busy_timeout(Duration::from_millis(100)).ok()?;
    Some(db)
}
fn scalar(db: &Connection, query: &str) -> Option<f64> {
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
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => count += 1,
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

pub struct LocalServer {
    path: PathBuf,
    checked: Instant,
    paired: bool,
}
impl Default for LocalServer {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            checked: Instant::now() - Duration::from_secs(60),
            paired: false,
        }
    }
}
impl LocalServer {
    fn pair(&mut self, path: &Path, runtime: &Value) -> bool {
        if self.path != path || self.checked.elapsed() >= Duration::from_secs(30) {
            self.path = path.to_owned();
            self.checked = Instant::now();
            self.paired = (|| -> Option<bool> {
                let port = runtime["port"]
                    .as_u64()
                    .filter(|p| (1..=65535).contains(p))?;
                // Only contact loopback, never a hostname supplied by a runtime file.
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_millis(500))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .ok()?;
                let response = client
                    .get(format!(
                        "http://127.0.0.1:{port}/.well-known/t3/environment"
                    ))
                    .send()
                    .ok()?
                    .error_for_status()
                    .ok()?;
                use std::io::Read;
                let mut bytes = Vec::new();
                response.take(16385).read_to_end(&mut bytes).ok()?;
                if bytes.len() > 16384 {
                    return None;
                }
                let descriptor: Value = serde_json::from_slice(&bytes).ok()?;
                Some(
                    descriptor["environmentId"].as_str().is_some()
                        && descriptor["serverVersion"].as_str().is_some(),
                )
            })()
            .unwrap_or(false);
        }
        self.paired
    }
}
pub fn codeslop(feed: &mut Feed, home: &Path, system: &System, server: &mut LocalServer) {
    let mut roots = vec![
        home.join(".codeslop"),
        home.join(".t3"),
        home.join(".t3code"),
    ];
    for variable in ["T3_HOME", "CODESLOP_HOME"] {
        if let Some(path) = std::env::var_os(variable) {
            roots.insert(0, path.into());
        }
    }
    for root in roots {
        for variant in ["userdata", "dev"] {
            let dir = root.join(variant);
            let runtime: Value = match std::fs::read(dir.join("server-runtime.json"))
                .ok()
                .filter(|b| b.len() < 16384)
                .and_then(|b| serde_json::from_slice(&b).ok())
            {
                Some(v) => v,
                None => continue,
            };
            let live = runtime["pid"]
                .as_u64()
                .and_then(|pid| system.process(Pid::from_u32(pid as u32)))
                .is_some_and(|p| {
                    let cmd = p
                        .cmd()
                        .iter()
                        .map(|s| s.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase();
                    cmd.contains("codeslop")
                        || cmd.contains("t3")
                        || p.name()
                            .to_string_lossy()
                            .to_lowercase()
                            .contains("codeslop")
                })
                && server.pair(&dir, &runtime);
            let Some(db) = open(&dir.join("state.sqlite")) else {
                continue;
            };
            feed.sources.retain(|s| !s.id.starts_with("t3_"));
            let value = |query: &str| if live { scalar(&db, query) } else { None };
            feed.add(Source::new("t3_running", "Working agents", "codeslop / T3 Code", "agents", 10.0, "Running or starting sessions in the discovered local codeslop/T3 server."), value("SELECT count(*)*1.0 FROM projection_thread_sessions s JOIN projection_threads t ON t.thread_id=s.thread_id WHERE s.status IN ('running','starting') AND t.archived_at IS NULL AND t.deleted_at IS NULL"));
            feed.add(Source::new("t3_waiting", "Needs your attention", "codeslop / T3 Code", "tasks", 10.0, "Tasks waiting for approval, input or an actionable plan."), value("SELECT count(*)*1.0 FROM projection_threads WHERE deleted_at IS NULL AND archived_at IS NULL AND (pending_approval_count>0 OR pending_user_input_count>0 OR has_actionable_proposed_plan=1)"));
            feed.add(Source::new("t3_completed", "Turns completed today", "codeslop / T3 Code", "turns", 100.0, "Completed local turns since midnight."), value("SELECT count(*)*1.0 FROM projection_turns WHERE state='completed' AND date(completed_at,'localtime')=date('now','localtime')"));
            feed.add(Source::new("t3_tools", "Tool calls today", "codeslop / T3 Code", "calls", 1000.0, "Completed tool calls recorded by the local server today."), value("SELECT count(*)*1.0 FROM projection_thread_activities WHERE kind='tool.completed' AND date(created_at,'localtime')=date('now','localtime')"));
            feed.add(Source::new("t3_context", "Busiest context window", "codeslop / T3 Code", "%", 100.0, "Largest context window in use among active local sessions, using each thread’s latest provider reading."), value("WITH readings AS (SELECT a.thread_id, a.payload_json, row_number() OVER (PARTITION BY a.thread_id ORDER BY a.created_at DESC) n FROM projection_thread_activities a JOIN projection_threads t ON t.thread_id=a.thread_id JOIN projection_thread_sessions s ON s.thread_id=a.thread_id WHERE a.kind='context-window.updated' AND t.deleted_at IS NULL AND t.archived_at IS NULL AND s.status IN ('running','starting','ready')) SELECT max(json_extract(payload_json,'$.usedTokens') * 100.0 / nullif(json_extract(payload_json,'$.maxTokens'),0)) FROM readings WHERE n=1"));
            if live {
                return;
            }
        }
    }
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
