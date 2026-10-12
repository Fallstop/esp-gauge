use super::{
    local_data::{open, scalar},
    Feed, Source,
};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use sysinfo::{Pid, System};

const GROUP: &str = "codeslop / T3 Code";
// Local midnight as an ISO UTC string, so timestamp columns compare without per-row date().
macro_rules! today {
    () => {
        "strftime('%Y-%m-%dT%H:%M:%fZ','now','localtime','start of day','utc')"
    };
}

// The server's own definition of an active run; `waiting` is a finished turn draining background work.
macro_rules! active_run {
    () => {
        "r.status IN ('preparing','starting','running','waiting') AND t.archived_at IS NULL AND t.deleted_at IS NULL"
    };
}

struct Queries {
    running: &'static str,
    waiting: &'static str,
    completed: &'static str,
    tools: &'static str,
    context: &'static str,
}
// Orchestration v2 (statev2.sqlite). Legacy projection tables are still present there but frozen.
const V2: Queries = Queries {
    running: concat!("SELECT count(DISTINCT r.thread_id)*1.0 FROM orchestration_v2_projection_runs r JOIN orchestration_v2_projection_threads t ON t.thread_id=r.thread_id WHERE ", active_run!()),
    waiting: "SELECT count(*)*1.0 FROM orchestration_v2_projection_threads t WHERE t.archived_at IS NULL AND t.deleted_at IS NULL AND (EXISTS (SELECT 1 FROM orchestration_v2_projection_runtime_requests q WHERE q.thread_id=t.thread_id AND q.status='pending' AND q.kind<>'auth_refresh') OR EXISTS (SELECT 1 FROM orchestration_v2_projection_plans p WHERE p.thread_id=t.thread_id AND p.kind='proposed_plan' AND p.status='active'))",
    completed: concat!("SELECT count(*)*1.0 FROM orchestration_v2_projection_runs WHERE status='completed' AND completed_at >= ", today!()),
    tools: concat!("SELECT count(*)*1.0 FROM orchestration_v2_projection_nodes WHERE kind='tool_call' AND status='completed' AND completed_at >= ", today!()),
    context: concat!("SELECT max(json_extract(pt.payload_json,'$.tokenUsage.usedTokens')*100.0/nullif(json_extract(pt.payload_json,'$.tokenUsage.maxTokens'),0)) FROM orchestration_v2_projection_runs r JOIN orchestration_v2_projection_threads t ON t.thread_id=r.thread_id JOIN orchestration_v2_projection_provider_turns pt ON pt.provider_turn_id=(SELECT p.provider_turn_id FROM orchestration_v2_projection_provider_turns p WHERE p.provider_thread_id=r.provider_thread_id AND json_extract(p.payload_json,'$.tokenUsage.usedTokens') IS NOT NULL ORDER BY p.ordinal DESC LIMIT 1) WHERE ", active_run!()),
};
const LEGACY: Queries = Queries {
    running: "SELECT count(*)*1.0 FROM projection_thread_sessions s JOIN projection_threads t ON t.thread_id=s.thread_id WHERE s.status IN ('running','starting') AND t.archived_at IS NULL AND t.deleted_at IS NULL",
    waiting: "SELECT count(*)*1.0 FROM projection_threads WHERE deleted_at IS NULL AND archived_at IS NULL AND (pending_approval_count>0 OR pending_user_input_count>0 OR has_actionable_proposed_plan=1)",
    completed: "SELECT count(*)*1.0 FROM projection_turns WHERE state='completed' AND date(completed_at,'localtime')=date('now','localtime')",
    tools: "SELECT count(*)*1.0 FROM projection_thread_activities WHERE kind='tool.completed' AND date(created_at,'localtime')=date('now','localtime')",
    context: "WITH readings AS (SELECT a.thread_id, a.payload_json, row_number() OVER (PARTITION BY a.thread_id ORDER BY a.created_at DESC) n FROM projection_thread_activities a JOIN projection_threads t ON t.thread_id=a.thread_id JOIN projection_thread_sessions s ON s.thread_id=a.thread_id WHERE a.kind='context-window.updated' AND t.deleted_at IS NULL AND t.archived_at IS NULL AND s.status IN ('running','starting','ready')) SELECT max(json_extract(payload_json,'$.usedTokens') * 100.0 / nullif(json_extract(payload_json,'$.maxTokens'),0)) FROM readings WHERE n=1",
};
const SUBAGENTS: &str = "SELECT count(*)*1.0 FROM orchestration_v2_projection_subagents WHERE status IN ('pending','running','waiting')";
const QUEUED: &str =
    "SELECT count(*)*1.0 FROM orchestration_v2_projection_runs WHERE status='queued'";
const TOKENS: &str = concat!("SELECT coalesce(sum(json_extract(payload_json,'$.turnTokenUsage.outputTokens')),0)*1.0 FROM orchestration_v2_projection_provider_turns WHERE completed_at >= ", today!());

// The server copies state.sqlite into statev2.sqlite once, then writes only v2; match on tables, not migration ids.
fn database(dir: &Path) -> Option<(Connection, bool)> {
    if let Some(db) = open(&dir.join("statev2.sqlite")) {
        if scalar(&db, "SELECT count(*)*1.0 FROM sqlite_master WHERE type='table' AND name='orchestration_v2_projection_threads'") == Some(1.0) {
            return Some((db, true));
        }
    }
    open(&dir.join("state.sqlite")).map(|db| (db, false))
}

pub struct LocalServer {
    path: PathBuf,
    checked: Instant,
    paired: bool,
    daily: HashMap<&'static str, (Instant, Option<f64>)>,
}
impl Default for LocalServer {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            checked: Instant::now() - Duration::from_secs(60),
            paired: false,
            daily: HashMap::new(),
        }
    }
}
impl LocalServer {
    // Daily totals can scan whole tables (gigabytes in the legacy schema); refresh them sparingly.
    fn daily(&mut self, db: &Connection, query: &'static str) -> Option<f64> {
        if let Some((at, value)) = self.daily.get(query) {
            if at.elapsed() < Duration::from_secs(30) {
                return *value;
            }
        }
        let value = scalar(db, query);
        self.daily.insert(query, (Instant::now(), value));
        value
    }
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
            let Some((db, v2)) = database(&dir) else {
                continue;
            };
            let queries = if v2 { &V2 } else { &LEGACY };
            feed.sources.retain(|s| !s.id.starts_with("t3_"));
            let value = |query: &str| if live { scalar(&db, query) } else { None };
            let mut daily = |query: &'static str| {
                if live {
                    server.daily(&db, query)
                } else {
                    None
                }
            };
            feed.add(Source::new("t3_running", "Working agents", GROUP, "agents", 10.0, "Tasks with an active run in the local codeslop/T3 server, including runs finishing background work."), value(queries.running));
            feed.add(
                Source::new(
                    "t3_waiting",
                    "Needs your attention",
                    GROUP,
                    "tasks",
                    10.0,
                    "Tasks waiting for an approval, your input or a decision on a proposed plan.",
                ),
                value(queries.waiting),
            );
            feed.add(
                Source::new(
                    "t3_completed",
                    "Turns completed today",
                    GROUP,
                    "turns",
                    100.0,
                    "Completed local turns since midnight.",
                ),
                daily(queries.completed),
            );
            feed.add(
                Source::new(
                    "t3_tools",
                    "Tool calls today",
                    GROUP,
                    "calls",
                    1000.0,
                    "Completed tool calls recorded by the local server today.",
                ),
                daily(queries.tools),
            );
            feed.add(Source::new("t3_context", "Busiest context window", GROUP, "%", 100.0, "Largest context window in use among active tasks, using each task’s latest provider reading."), value(queries.context));
            if v2 {
                feed.add(
                    Source::new(
                        "t3_subagents",
                        "Active subagents",
                        GROUP,
                        "agents",
                        10.0,
                        "Subagents pending, running or waiting across local tasks.",
                    ),
                    value(SUBAGENTS),
                );
                feed.add(
                    Source::new(
                        "t3_queued",
                        "Queued messages",
                        GROUP,
                        "messages",
                        10.0,
                        "Messages queued behind a task’s active run.",
                    ),
                    value(QUEUED),
                );
                feed.add(
                    Source::new(
                        "t3_tokens",
                        "Output tokens today",
                        GROUP,
                        "tokens",
                        1000000.0,
                        "Provider output tokens from local turns completed since midnight.",
                    ),
                    daily(TOKENS),
                );
            }
            if live {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v2_queries_count_live_work_only() {
        let dir = std::env::temp_dir().join(format!("gauge-t3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Connection::open(dir.join("statev2.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE orchestration_v2_projection_threads (thread_id TEXT, archived_at TEXT, deleted_at TEXT);
             CREATE TABLE orchestration_v2_projection_runs (thread_id TEXT, provider_thread_id TEXT, status TEXT, completed_at TEXT);
             CREATE TABLE orchestration_v2_projection_runtime_requests (thread_id TEXT, kind TEXT, status TEXT);
             CREATE TABLE orchestration_v2_projection_plans (thread_id TEXT, kind TEXT, status TEXT);
             CREATE TABLE orchestration_v2_projection_nodes (kind TEXT, status TEXT, completed_at TEXT);
             CREATE TABLE orchestration_v2_projection_provider_turns (provider_turn_id TEXT, provider_thread_id TEXT, ordinal INTEGER, payload_json TEXT, completed_at TEXT);
             CREATE TABLE orchestration_v2_projection_subagents (status TEXT);
             INSERT INTO orchestration_v2_projection_threads VALUES ('a',NULL,NULL),('b',NULL,NULL),('old','2026-01-01',NULL);
             INSERT INTO orchestration_v2_projection_runs VALUES
               ('a','pa','running',NULL),('old','po','running',NULL),
               ('b','pb','completed',strftime('%Y-%m-%dT%H:%M:%fZ','now')),('b','pb','completed','2000-01-01T00:00:00.000Z'),
               ('b','pb','queued',NULL);
             INSERT INTO orchestration_v2_projection_runtime_requests VALUES ('b','permission','pending'),('a','auth_refresh','pending');
             INSERT INTO orchestration_v2_projection_nodes VALUES ('tool_call','completed',strftime('%Y-%m-%dT%H:%M:%fZ','now')),('tool_call','completed','2000-01-01T00:00:00.000Z');
             INSERT INTO orchestration_v2_projection_provider_turns VALUES
               ('t1','pa',1,'{\"tokenUsage\":{\"usedTokens\":10,\"maxTokens\":100}}',NULL),
               ('t2','pa',2,'{\"tokenUsage\":{\"usedTokens\":50,\"maxTokens\":100},\"turnTokenUsage\":{\"outputTokens\":7}}',strftime('%Y-%m-%dT%H:%M:%fZ','now'));
             INSERT INTO orchestration_v2_projection_subagents VALUES ('running'),('completed');",
        )
        .unwrap();
        drop(db);
        let (db, v2) = database(&dir).unwrap();
        let read = |q| scalar(&db, q);
        assert!(v2);
        assert_eq!(read(V2.running), Some(1.0));
        assert_eq!(read(V2.waiting), Some(1.0));
        assert_eq!(read(V2.completed), Some(1.0));
        assert_eq!(read(V2.tools), Some(1.0));
        assert_eq!(read(V2.context), Some(50.0));
        assert_eq!(read(SUBAGENTS), Some(1.0));
        assert_eq!(read(QUEUED), Some(1.0));
        assert_eq!(read(TOKENS), Some(7.0));
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
