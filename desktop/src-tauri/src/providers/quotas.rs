use super::{rpc, Feed, Source};
use serde_json::Value;
use std::time::Duration;

pub fn codex_limits(feed: &mut Feed, data: &Value) {
    let fallback;
    let buckets = if let Some(buckets) = data["rateLimitsByLimitId"].as_object() {
        buckets
    } else {
        fallback = serde_json::Map::from_iter([("codex".into(), data["rateLimits"].clone())]);
        &fallback
    };
    for (bucket, value) in buckets {
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
            let reset = value[window]["resetsAt"]
                .as_i64()
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0));
            let description = reset
                .map(|t| {
                    format!(
                        "Share of this quota used. Resets {}. Refreshed every minute.",
                        t.with_timezone(&chrono::Local).format("%a %H:%M")
                    )
                })
                .unwrap_or_else(|| "Share of this quota used, refreshed every minute.".into());
            feed.add(
                Source::new(
                    &format!("{bucket}_{suffix}"),
                    &format!("{prefix}{label}"),
                    "Codex",
                    "%",
                    100.0,
                    &description,
                ),
                Some(used),
            );
        }
    }
}
fn claude_usage() -> Result<Value, &'static str> {
    let login = "Open Claude Code and sign in to read subscription usage.";
    let home = dirs::home_dir().ok_or(login)?;
    let credential_file = home.join(".claude/.credentials.json");
    let bytes = if credential_file.exists() {
        std::fs::read(credential_file)
            .ok()
            .filter(|b| b.len() <= 65536)
            .ok_or(login)?
    } else {
        #[cfg(target_os = "macos")]
        {
            rpc::bounded_output(
                rpc::quiet_command(std::path::Path::new("/usr/bin/security")).args([
                    "find-generic-password",
                    "-s",
                    "Claude Code-credentials",
                    "-w",
                ]),
            )
            .ok_or(login)?
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Err(login);
        }
    };
    let credential: Value = serde_json::from_slice(&bytes).map_err(|_| login)?;
    let token = credential["claudeAiOauth"]["accessToken"]
        .as_str()
        .ok_or(login)?;
    if credential["claudeAiOauth"]["expiresAt"]
        .as_i64()
        .is_some_and(|t| t <= chrono::Utc::now().timestamp_millis())
    {
        return Err("Claude Code’s login has expired. Open Claude Code to refresh it.");
    }
    let unavailable = "Claude subscription usage is temporarily unavailable.";
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| unavailable)?;
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("user-agent", "ESP-Gauge")
        .send()
        .map_err(|_| unavailable)?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(login);
    }
    response
        .error_for_status()
        .map_err(|_| unavailable)?
        .json()
        .map_err(|_| unavailable)
}
pub fn sample() -> Feed {
    let mut feed = Feed::default();
    if let Ok(mut rpc) = rpc::Rpc::codex() {
        if let Ok(data) = rpc.call("account/rateLimits/read", None) {
            codex_limits(&mut feed, &data);
        }
        if let Ok(data) = rpc.call("account/usage/read", None) {
            if let Some(days) = data["dailyUsageBuckets"].as_array() {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let tokens = days
                    .iter()
                    .find(|d| d["startDate"] == today)
                    .and_then(|d| d["tokens"].as_f64());
                feed.add(Source::new("codex_tokens_today","Tokens today","Codex","tokens",1000000.0,"Account token activity for the current UTC day, as reported by Codex. Reporting can lag."),tokens);
            }
        }
    }
    if rpc::executable("claude").is_some() {
        let data = claude_usage();
        for (field, id, name) in [
            ("five_hour", "claude_5h", "5-hour usage"),
            ("seven_day", "claude_weekly", "Weekly usage"),
        ] {
            let description = data.as_ref().err().copied().unwrap_or("Claude subscription quota used. Refreshed every minute using your existing Claude Code login.");
            feed.add(
                Source::new(id, name, "Claude Code", "%", 100.0, description),
                data.as_ref()
                    .ok()
                    .and_then(|d| d[field]["utilization"].as_f64())
                    .filter(|n| (0.0..=100.0).contains(n)),
            );
        }
    }
    feed
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
}
