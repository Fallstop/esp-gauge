use crate::providers::{Choice, Source};
use std::{collections::BTreeMap, time::Instant};
use sysinfo::{Disks, Networks, System};
pub type Samples = BTreeMap<String, f64>;

pub struct Metrics {
    system: System,
    networks: Networks,
    disks: Disks,
    last: Instant,
    ticks: u64,
    battery: Option<f64>,
    pub sources: Vec<Source>,
}
impl Metrics {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last: Instant::now(),
            ticks: 0,
            battery: None,
            sources: Vec::new(),
        }
    }
    pub fn sample(&mut self) -> Samples {
        let seconds = self.last.elapsed().as_secs_f64().max(0.1);
        self.last = Instant::now();
        self.ticks += 1;
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        let mut out = Samples::new();
        self.sources.clear();
        out.insert("cpu".into(), self.system.global_cpu_usage() as f64);
        out.insert(
            "memory".into(),
            percent(self.system.used_memory(), self.system.total_memory()),
        );
        out.insert(
            "swap".into(),
            percent(self.system.used_swap(), self.system.total_swap()),
        );
        let (mut down, mut up) = (0u64, 0u64);
        let mut interfaces = Vec::new();
        for (name, n) in &self.networks {
            if name != "lo" && name != "lo0" {
                interfaces.push(Choice {
                    id: name.clone(),
                    name: name.clone(),
                });
                out.insert(
                    format!("network_down:{name}"),
                    n.received() as f64 / seconds / 1_048_576.0,
                );
                out.insert(
                    format!("network_up:{name}"),
                    n.transmitted() as f64 / seconds / 1_048_576.0,
                );
                down = down.saturating_add(n.received());
                up = up.saturating_add(n.transmitted());
            }
        }
        out.insert("network_down".into(), down as f64 / seconds / 1_048_576.0);
        out.insert("network_up".into(), up as f64 / seconds / 1_048_576.0);
        for (id, name) in [("network_down", "Download"), ("network_up", "Upload")] {
            let mut source = Source::new(
                id,
                name,
                "This computer",
                "MiB/s",
                10.0,
                "Traffic across all network interfaces, or select one interface.",
            );
            source.options = interfaces.clone();
            self.sources.push(source);
        }
        if self.ticks % 10 == 1 {
            self.disks.refresh(true);
        }
        let mut disk = Source::new(
            "disk",
            "Disk space",
            "This computer",
            "%",
            100.0,
            "Space used on the system drive, or choose a mounted drive.",
        );
        for d in &self.disks {
            let path = d.mount_point().to_string_lossy().to_string();
            disk.options.push(Choice {
                id: path.clone(),
                name: format!("{} · {path}", d.name().to_string_lossy()),
            });
            if d.total_space() > 0 {
                out.insert(
                    format!("disk:{path}"),
                    percent(
                        d.total_space().saturating_sub(d.available_space()),
                        d.total_space(),
                    ),
                );
            }
        }
        self.sources.push(disk);
        let root = if cfg!(windows) { "C:\\" } else { "/" };
        if let Some(d) = self
            .disks
            .iter()
            .find(|d| d.mount_point().to_string_lossy() == root)
        {
            out.insert(
                "disk".into(),
                percent(
                    d.total_space().saturating_sub(d.available_space()),
                    d.total_space(),
                ),
            );
        }
        // Some desktops have no battery. Omit it, rather than inventing a reading.
        if self.ticks % 20 == 1 {
            self.battery = None;
            if let Ok(manager) = battery::Manager::new() {
                if let Ok(batteries) = manager.batteries() {
                    if let Some(Ok(b)) = batteries.into_iter().next() {
                        self.battery = Some(b.state_of_charge().value as f64 * 100.0);
                    }
                }
            }
        }
        if let Some(battery) = self.battery {
            out.insert("battery".into(), battery);
        }
        out
    }
}
fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 / total as f64 * 100.0
    }
}
