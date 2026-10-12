use super::local;
use crate::providers::{rpc, Feed, Source};
use serde_json::{json, Map, Value};
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

// Spawning app-server costs a process launch; local session files carry fresher limits.
const SERVER_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub fn codex_limits(feed: &mut Feed, data: &Value) {
    let now = chrono::Utc::now().timestamp();
    for (bucket, value) in &buckets(data) {
        if bucket.len() > 30
            || !bucket
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            continue;
        }
        for window in ["primary", "secondary"] {
            let Some(minutes) = value[window]["windowDurationMins"].as_u64() else {
                continue;
            };
            let Some(used) = value[window]["usedPercent"]
                .as_f64()
                .filter(|n| (0.0..=100.0).contains(n))
            else {
                continue;
            };
            let (suffix, label) = match minutes {
                300 => ("5h".into(), "5-hour usage".into()),
                10080 => ("weekly".into(), "Weekly usage".into()),
                _ => (format!("{minutes}m"), format!("{minutes}-minute usage")),
            };
            let prefix = if bucket == "codex" {
                String::new()
            } else {
                format!("{} · ", value["limitName"].as_str().unwrap_or(bucket))
            };
            let reset = value[window]["resetsAt"].as_i64();
            let description = format!(
                "Share of this quota used.{} Read from Codex activity on this computer, and checked with your account every 15 minutes.",
                reset
                    .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                    .map(|t| format!(" Resets {}.", local(t)))
                    .unwrap_or_default()
            );
            feed.add(
                Source::new(
                    &format!("{bucket}_{suffix}"),
                    &format!("{prefix}{label}"),
                    "Codex",
                    "%",
                    100.0,
                    &description,
                ),
                // A passed reset means the window has emptied since the last reading.
                Some(if reset.is_some_and(|t| t <= now) {
                    0.0
                } else {
                    used
                }),
            );
        }
    }
}
fn buckets(data: &Value) -> Map<String, Value> {
    match data["rateLimitsByLimitId"].as_object() {
        Some(buckets) => buckets.clone(),
        None if data["rateLimits"].is_object() => {
            Map::from_iter([("codex".into(), data["rateLimits"].clone())])
        }
        None => Map::new(),
    }
}
#[derive(Default)]
pub struct Codex {
    checked: Option<Instant>,
    server: Option<(SystemTime, Value)>,
    tokens: Option<Option<f64>>,
}
impl Codex {
    pub fn sample(&mut self, feed: &mut Feed) {
        if self.checked.is_none_or(|t| t.elapsed() >= SERVER_INTERVAL) {
            self.checked = Some(Instant::now());
            self.server = None;
            self.tokens = None;
            if let Ok(mut rpc) = rpc::Rpc::codex() {
                if let Ok(data) = rpc.call("account/rateLimits/read", None) {
                    self.server = Some((SystemTime::now(), data));
                }
                if let Ok(data) = rpc.call("account/usage/read", None) {
                    if let Some(days) = data["dailyUsageBuckets"].as_array() {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        self.tokens = Some(
                            days.iter()
                                .find(|d| d["startDate"] == today)
                                .and_then(|d| d["tokens"].as_f64()),
                        );
                    }
                }
            }
        }
        let mut limits = Map::new();
        let mut since = SystemTime::UNIX_EPOCH;
        if let Some((at, data)) = &self.server {
            limits = buckets(data);
            since = *at;
        }
        // Codex records limits with each turn, so local activity is fresher than the last account check.
        if let Some((at, id, bucket)) = codex_home().and_then(|dir| session_limits(&dir)) {
            if at > since {
                limits.insert(id, bucket);
            }
        }
        codex_limits(feed, &json!({ "rateLimitsByLimitId": limits }));
        if let Some(tokens) = self.tokens {
            feed.add(Source::new("codex_tokens_today","Tokens today","Codex","tokens",1000000.0,"Account token activity for the current UTC day, as reported by Codex. Checked every 15 minutes, and reporting can lag."),tokens);
        }
    }
}
fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")))
}
fn newest_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.reverse();
    dirs
}
fn session_limits(codex: &Path) -> Option<(SystemTime, String, Value)> {
    // Rollouts live in sessions/YYYY/MM/DD; the two newest days cover sessions spanning midnight.
    let days: Vec<PathBuf> = newest_dirs(&codex.join("sessions"))
        .iter()
        .flat_map(|y| newest_dirs(y))
        .flat_map(|m| newest_dirs(&m))
        .take(2)
        .collect();
    let (modified, path) = days
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .filter(|e| e.file_name().to_string_lossy().starts_with("rollout-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max()?;
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(512 * 1024)))
        .ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    let limits = tail
        .split(|b| *b == b'\n')
        .rev()
        .filter(|line| line.windows(13).any(|w| w == b"\"token_count\""))
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .map(|event| event["payload"]["rate_limits"].clone())
        .find(Value::is_object)?;
    let window = |w: &Value| {
        if w.is_object() {
            json!({"usedPercent":w["used_percent"],"windowDurationMins":w["window_minutes"],"resetsAt":w["resets_at"]})
        } else {
            Value::Null
        }
    };
    Some((
        modified,
        limits["limit_id"].as_str().unwrap_or("codex").into(),
        json!({"limitName":limits["limit_name"],"primary":window(&limits["primary"]),"secondary":window(&limits["secondary"])}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_duration_determines_quota_and_null_is_not_zero() {
        let mut feed = Feed::default();
        codex_limits(
            &mut feed,
            &json!({"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":17,"windowDurationMins":10080},"secondary":null},"spark":{"limitName":"Spark","primary":{"usedPercent":0,"windowDurationMins":300}}}}),
        );
        assert_eq!(feed.values.get("codex_weekly"), Some(&17.0));
        assert!(!feed.values.contains_key("codex_5h"));
        assert_eq!(feed.values.get("spark_5h"), Some(&0.0));
    }
    #[test]
    fn passed_reset_empties_the_window() {
        let mut feed = Feed::default();
        codex_limits(
            &mut feed,
            &json!({"rateLimits":{"primary":{"usedPercent":40,"windowDurationMins":300,"resetsAt":1}}}),
        );
        assert_eq!(feed.values.get("codex_5h"), Some(&0.0));
    }
    #[test]
    fn session_rollout_limits_are_read_from_the_newest_token_count() {
        let dir = std::env::temp_dir().join(format!("gauge-codex-{}", std::process::id()));
        let day = dir.join("sessions/2026/10/12");
        std::fs::create_dir_all(&day).unwrap();
        let line = |used: f64| {
            json!({"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":used,"window_minutes":10080,"resets_at":4102444800i64},"secondary":null}}})
                .to_string()
        };
        std::fs::write(
            day.join("rollout-a.jsonl"),
            format!(
                "{}\n{{\"type\":\"response_item\"}}\n{}\n",
                line(3.0),
                line(9.0)
            ),
        )
        .unwrap();
        let (_, id, bucket) = session_limits(&dir).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(id, "codex");
        let mut feed = Feed::default();
        codex_limits(&mut feed, &json!({"rateLimitsByLimitId":{id:bucket}}));
        assert_eq!(feed.values.get("codex_weekly"), Some(&9.0));
    }
}
