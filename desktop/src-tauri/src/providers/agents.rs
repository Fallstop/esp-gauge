use super::{local_data, rpc, Feed, Source};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub struct Agents {
    system: System,
    server: local_data::LocalServer,
}
impl Agents {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            server: local_data::LocalServer::default(),
        }
    }
    pub fn sample(&mut self) -> Feed {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_memory()
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_exe(UpdateKind::OnlyIfNotSet),
        );
        let mut feed = Feed::default();
        let Some(home) = dirs::home_dir() else {
            return feed;
        };
        for (provider, label) in [
            ("codex", "Codex"),
            ("claude", "Claude Code"),
            ("opencode", "OpenCode"),
        ] {
            let processes: Vec<_> = self
                .system
                .processes()
                .values()
                .filter(|p| {
                    let name = p.name().to_string_lossy().to_lowercase();
                    let is_native = name == provider
                        || name == format!("{provider}.exe")
                        || name == format!("{provider}-cli");
                    let args = p
                        .cmd()
                        .iter()
                        .take(4)
                        .map(|s| s.to_string_lossy())
                        .collect::<Vec<_>>();
                    let wrapper = name.starts_with("node")
                        && args.iter().any(|s| {
                            s.ends_with(&format!("/{provider}.js"))
                                || s.ends_with(&format!("\\{provider}.js"))
                        });
                    let is_wrapper_parent = self.system.processes().values().any(|child| {
                        child.parent() == Some(p.pid())
                            && child.name().to_string_lossy().to_lowercase() == provider
                    });
                    (is_native || (wrapper && !is_wrapper_parent))
                        && !args.iter().any(|a| {
                            ["app-server", "mcp-server", "--version", "--help"]
                                .contains(&a.as_ref())
                        })
                })
                .collect();
            if rpc::executable(provider).is_some() || !processes.is_empty() {
                feed.add(Source::new(&format!("{provider}_sessions"), "Open CLI sessions", label, "sessions", 10.0, "Local CLI processes, including idle sessions. Background app servers are excluded."), Some(processes.len() as f64));
                feed.add(
                    Source::new(
                        &format!("{provider}_memory"),
                        "CLI memory",
                        label,
                        "MiB",
                        2048.0,
                        "Memory held by the local CLI processes.",
                    ),
                    Some(
                        processes
                            .iter()
                            .map(|p| p.memory() as f64 / 1_048_576.0)
                            .sum::<f64>()
                            .max(0.0),
                    ),
                );
            }
        }
        local_data::codex(&mut feed, &home);
        local_data::codeslop(&mut feed, &home, &self.system, &mut self.server);
        local_data::opencode(&mut feed, &home);
        feed
    }
}
