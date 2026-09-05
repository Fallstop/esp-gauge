use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

pub fn executable(name: &str) -> Option<PathBuf> {
    let mut paths: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    if let Some(home) = dirs::home_dir() {
        paths.extend(
            [
                ".local/bin",
                ".cargo/bin",
                ".opencode/bin",
                ".npm-global/bin",
                "AppData/Roaming/npm",
            ]
            .map(|p| home.join(p)),
        );
        for root in [
            home.join(".local/share/fnm/node-versions"),
            home.join(".nvm/versions/node"),
        ] {
            if let Ok(entries) = std::fs::read_dir(root) {
                let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
                entries.sort();
                entries.reverse();
                for p in entries.into_iter().take(8) {
                    paths.push(p.join("installation/bin"));
                    paths.push(p.join("bin"));
                }
            }
        }
    }
    paths.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    for dir in paths {
        for suffix in if cfg!(windows) {
            vec![".exe", ".cmd"]
        } else {
            vec![""]
        } {
            let path = dir.join(format!("{name}{suffix}"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}
pub fn quiet_command(path: &std::path::Path) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = if path.extension().is_some_and(|s| s == "cmd") {
            let mut c = Command::new("cmd.exe");
            c.args(["/D", "/C"]).arg(path);
            c
        } else {
            Command::new(path)
        };
        cmd.creation_flags(0x08000000);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(path)
    }
}
#[cfg(target_os = "macos")]
pub fn bounded_output(command: &mut Command) -> Option<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let output = child.stdout.take()?;
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output.take(65537).read_to_end(&mut bytes);
        let _ = tx.send(result.ok().filter(|_| bytes.len() <= 65536).map(|_| bytes));
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let result = rx.recv_timeout(Duration::from_secs(3)).ok().flatten();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            _ if std::time::Instant::now() >= deadline => break false,
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    result.filter(|_| success)
}
pub struct Rpc {
    child: Child,
    input: ChildStdin,
    output: mpsc::Receiver<Value>,
    next: u32,
}
impl Rpc {
    pub fn codex() -> Result<Self, String> {
        let path = executable("codex").ok_or("Codex CLI is not installed")?;
        let mut child = quiet_command(&path)
            .args(["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::sync_channel(32);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut bytes = Vec::new();
                match std::io::Read::by_ref(&mut reader)
                    .take(2 * 1024 * 1024)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Ok(value) = serde_json::from_slice(&bytes) {
                            if tx.send(value).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        let mut rpc = Self {
            child,
            input,
            output: rx,
            next: 0,
        };
        rpc.call("initialize", Some(json!({"clientInfo":{"name":"esp-gauge","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})))?;
        writeln!(rpc.input, "{{\"method\":\"initialized\"}}").map_err(|e| e.to_string())?;
        Ok(rpc)
    }
    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, String> {
        self.next += 1;
        let mut message = json!({"id":self.next,"method":method});
        if let Some(params) = params {
            message["params"] = params;
        }
        writeln!(self.input, "{message}").map_err(|e| e.to_string())?;
        let end = std::time::Instant::now() + Duration::from_secs(12);
        while std::time::Instant::now() < end {
            let value = self
                .output
                .recv_timeout(end.saturating_duration_since(std::time::Instant::now()))
                .map_err(|_| "Codex did not return usage data")?;
            if value["id"] == self.next {
                return value
                    .get("result")
                    .cloned()
                    .ok_or_else(|| "Codex usage is unavailable for this account.".into());
            }
        }
        Err("Codex usage request timed out".into())
    }
}
impl Drop for Rpc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
