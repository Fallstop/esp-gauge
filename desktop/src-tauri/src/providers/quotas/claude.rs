use super::local;
use crate::providers::{rpc, Feed, Source};
use serde_json::Value;
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

// Cadences follow CodexBar and Claude Code's own /usage: the usage endpoint is
// undocumented and rate-limits clients that poll it every minute.
const ACTIVE: Duration = Duration::from_secs(5 * 60);
const IDLE: Duration = Duration::from_secs(20 * 60);
const MINIMUM: Duration = Duration::from_secs(2 * 60);

enum Failure {
    Login(&'static str),
    Limited(Option<Duration>),
    Unavailable,
}
#[derive(Default)]
pub struct Claude {
    token: Option<(String, Option<i64>)>,
    fetched: Option<Instant>,
    retry: Option<(Instant, Duration)>,
    last: Option<(Value, chrono::DateTime<chrono::Utc>)>,
    error: Option<&'static str>,
}
impl Claude {
    pub fn sample(&mut self, feed: &mut Feed) {
        if rpc::executable("claude").is_none() {
            return;
        }
        if self.due() {
            self.fetch();
        }
        let now = chrono::Utc::now();
        for (field, id, name) in [
            ("five_hour", "claude_5h", "5-hour usage"),
            ("seven_day", "claude_weekly", "Weekly usage"),
        ] {
            let window = self.last.as_ref().map(|(data, _)| &data[field]);
            let reset = window.and_then(resets_at);
            let mut description = match (self.error, &self.last) {
                (Some(error), None) => error.to_string(),
                _ => "Claude subscription quota used, read with your existing Claude Code login. Checked every 5 minutes while Claude Code is in use, and every 20 minutes otherwise.".into(),
            };
            if let Some(reset) = reset {
                description.push_str(&format!(" Resets {}.", local(reset)));
            }
            if let (Some(error), Some((_, at))) = (self.error, &self.last) {
                description.push_str(&format!(
                    " {error} Showing the reading from {}.",
                    local(*at)
                ));
            }
            let value = window
                .and_then(|w| w["utilization"].as_f64())
                .filter(|n| (0.0..=100.0).contains(n))
                .map(|n| {
                    if reset.is_some_and(|r| r <= now) {
                        0.0
                    } else {
                        n
                    }
                });
            feed.add(
                Source::new(id, name, "Claude Code", "%", 100.0, &description),
                value,
            );
        }
    }
    fn due(&self) -> bool {
        if let Some((at, _)) = self.retry {
            return Instant::now() >= at;
        }
        let Some(fetched) = self.fetched else {
            return true;
        };
        let elapsed = fetched.elapsed();
        let interval = if claude_active() { ACTIVE } else { IDLE };
        let reset_passed = self.last.as_ref().is_some_and(|(data, _)| {
            ["five_hour", "seven_day"]
                .iter()
                .filter_map(|f| resets_at(&data[f]))
                .any(|r| r <= chrono::Utc::now())
        });
        elapsed >= interval || (reset_passed && elapsed >= MINIMUM)
    }
    fn fetch(&mut self) {
        self.fetched = Some(Instant::now());
        let now = chrono::Utc::now().timestamp_millis();
        let cached = self
            .token
            .as_ref()
            .is_some_and(|(_, expires)| expires.is_none_or(|t| t > now));
        let mut result = self.request();
        // Claude Code rotates its token; one re-read covers a refresh since we cached it.
        if cached && matches!(result, Err(Failure::Login(_))) {
            self.token = None;
            result = self.request();
        }
        let (start, cap) = match result {
            Ok(data) => {
                self.last = Some((data, chrono::Utc::now()));
                self.error = None;
                self.retry = None;
                return;
            }
            Err(Failure::Login(message)) => {
                self.token = None;
                self.last = None;
                self.error = Some(message);
                self.retry = None;
                return;
            }
            Err(Failure::Limited(Some(wait))) => {
                self.error = Some("Anthropic asked ESP Gauge to check less often.");
                self.retry = Some((Instant::now() + wait, wait));
                return;
            }
            Err(Failure::Limited(None)) => {
                self.error = Some("Anthropic asked ESP Gauge to check less often.");
                (Duration::from_secs(5 * 60), Duration::from_secs(60 * 60))
            }
            Err(Failure::Unavailable) => {
                self.error = Some("Claude subscription usage is temporarily unavailable.");
                (Duration::from_secs(60), Duration::from_secs(15 * 60))
            }
        };
        let wait = self
            .retry
            .map_or(start, |(_, previous)| (previous * 2).clamp(start, cap));
        self.retry = Some((Instant::now() + wait, wait));
    }
    fn request(&mut self) -> Result<Value, Failure> {
        let now = chrono::Utc::now().timestamp_millis();
        if self
            .token
            .as_ref()
            .is_none_or(|(_, expires)| expires.is_some_and(|t| t <= now))
        {
            self.token = Some(claude_token()?);
        }
        let (token, expires) = self.token.as_ref().unwrap();
        if expires.is_some_and(|t| t <= now) {
            return Err(Failure::Login(
                "Claude Code’s login has expired. Open Claude Code to refresh it.",
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Failure::Unavailable)?;
        let response = client
            .get("https://api.anthropic.com/api/oauth/usage")
            .bearer_auth(token)
            .header("anthropic-beta", "oauth-2025-04-20")
            .header(
                "user-agent",
                concat!("ESP-Gauge/", env!("CARGO_PKG_VERSION")),
            )
            .send()
            .map_err(|_| Failure::Unavailable)?;
        match response.status().as_u16() {
            401 | 403 => Err(Failure::Login(LOGIN)),
            429 => Err(Failure::Limited(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()?.trim().parse::<u64>().ok())
                    .filter(|s| *s > 0)
                    .map(|s| Duration::from_secs(s.min(6 * 60 * 60))),
            )),
            _ => response
                .error_for_status()
                .map_err(|_| Failure::Unavailable)?
                .json()
                .map_err(|_| Failure::Unavailable),
        }
    }
}
const LOGIN: &str = "Open Claude Code and sign in to read subscription usage.";
fn resets_at(window: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(window["resets_at"].as_str()?)
        .ok()
        .map(|t| t.to_utc())
}
fn claude_dir() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
}
// A transcript written in the last ten minutes means Claude Code is being used.
fn claude_active() -> bool {
    let Some(projects) = claude_dir().map(|d| d.join("projects")) else {
        return false;
    };
    let cutoff = SystemTime::now() - Duration::from_secs(10 * 60);
    std::fs::read_dir(projects)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|project| {
            std::fs::read_dir(project.path())
                .into_iter()
                .flatten()
                .flatten()
        })
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .any(|t| t >= cutoff)
}
fn claude_token() -> Result<(String, Option<i64>), Failure> {
    let login = Failure::Login(LOGIN);
    let credential_file = claude_dir()
        .ok_or(Failure::Login(LOGIN))?
        .join(".credentials.json");
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
    let credential: Value = serde_json::from_slice(&bytes).map_err(|_| Failure::Login(LOGIN))?;
    let token = credential["claudeAiOauth"]["accessToken"]
        .as_str()
        .ok_or(Failure::Login(LOGIN))?;
    Ok((
        token.into(),
        credential["claudeAiOauth"]["expiresAt"].as_i64(),
    ))
}
