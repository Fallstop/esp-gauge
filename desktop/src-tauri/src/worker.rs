use crate::{
    metrics::{Metrics, Samples},
    model::{autonomous, normalize, Config},
    transport::{self, Link},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
pub struct Device {
    pub path: String,
    pub id: String,
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub connected: bool,
    pub device: String,
    pub firmware: String,
    pub candidates: Vec<String>,
    pub path: String,
    pub devices: Vec<Device>,
    pub config: Option<Config>,
    pub metrics: Samples,
    pub board: Value,
    pub paused: bool,
    pub error: Option<String>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            connected: false,
            device: String::new(),
            firmware: String::new(),
            candidates: Vec::new(),
            path: String::new(),
            devices: Vec::new(),
            config: None,
            metrics: Samples::new(),
            board: json!({}),
            paused: false,
            error: None,
        }
    }
}
pub struct Job {
    pub command: Value,
    pub reply: mpsc::Sender<Result<Value, String>>,
}
#[derive(Clone)]
pub struct Service {
    pub state: Arc<Mutex<Snapshot>>,
    pub tx: mpsc::Sender<Job>,
}
impl Service {
    pub fn start(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let shared = state.clone();
        thread::spawn(move || run(app, rx, shared));
        Self { state, tx }
    }
    pub fn execute(&self, command: Value) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Job { command, reply: tx })
            .map_err(|_| "Monitor stopped")?;
        rx.recv_timeout(Duration::from_secs(8))
            .map_err(|_| "The board is busy. Try again.".to_string())?
    }
}
fn connect(link: &mut Link) -> Result<Config, String> {
    let response = link.request(json!({"op":"get_config"}))?;
    let config: Config = serde_json::from_value(response["config"].clone())
        .map_err(|_| "The board configuration could not be read. It has been preserved.")?;
    config.validate()?;
    sync_time(link)?;
    Ok(config)
}
fn sync_time(link: &mut Link) -> Result<Value, String> {
    let now = chrono::Local::now();
    link.request(
        json!({"op":"time","epoch":now.timestamp(),"offset":now.offset().local_minus_utc()}),
    )
}
fn publish(app: &AppHandle, shared: &Arc<Mutex<Snapshot>>, state: &Snapshot) {
    *shared.lock().unwrap() = state.clone();
    if app
        .get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false))
    {
        let _ = app.emit("state", state);
    }
}
fn run(app: AppHandle, rx: mpsc::Receiver<Job>, shared: Arc<Mutex<Snapshot>>) {
    let mut state = Snapshot::default();
    let mut metrics = Metrics::new();
    let mut link: Option<Link> = None;
    let mut seen: HashMap<String, (Option<String>, Instant)> = HashMap::new();
    let mut probing: Option<(String, mpsc::Receiver<Result<Link, String>>)> = None;
    let mut last_scan = Instant::now() - Duration::from_secs(10);
    let mut last_sample = Instant::now() - Duration::from_secs(1);
    let mut last_clock = Instant::now();
    let mut wifi_scan = Instant::now() - Duration::from_secs(60);
    let mut calibrating = false;
    let mut maintenance = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(job) => {
                let op = job.command["op"].as_str().unwrap_or("").to_owned();
                let result = (|| -> Result<Value, String> {
                    if op == "maintenance" {
                        let active = job.command["active"]
                            .as_bool()
                            .ok_or("Invalid maintenance state")?;
                        if active && probing.is_some() {
                            return Err("Finishing device discovery. Try again in a moment.".into());
                        }
                        if active {
                            if let Some(l) = link.as_mut() {
                                let _ = l.request(json!({"op":"release"}));
                            }
                            link = None;
                            calibrating = false;
                            state.connected = false;
                            state.board = json!({});
                        }
                        maintenance = active;
                        seen.clear();
                        state.error = None;
                        last_scan = Instant::now() - Duration::from_secs(10);
                        return Ok(json!({"ok":true}));
                    }
                    if maintenance {
                        return Err("Firmware installation is in progress.".into());
                    }
                    if op == "retry" {
                        seen.clear();
                        last_scan = Instant::now() - Duration::from_secs(10);
                        state.error = None;
                        return Ok(json!({"ok":true}));
                    }
                    if op == "select" {
                        let path = job.command["path"].as_str().ok_or("Choose a board")?;
                        if !state.devices.iter().any(|d| d.path == path) {
                            return Err("This board is no longer connected".into());
                        }
                        if let Some(l) = link.as_mut() {
                            let _ = l.request(json!({"op":"release"}));
                        }
                        link = None;
                        state.connected = false;
                        state.config = None;
                        let mut next = Link::probe(path)?;
                        let config = connect(&mut next)?;
                        state.path = next.path.clone();
                        state.device = next.device.clone();
                        state.firmware = next.firmware.clone();
                        state.config = Some(config);
                        state.connected = true;
                        state.paused = false;
                        link = Some(next);
                        return Ok(json!({"ok":true}));
                    }
                    let l = link
                        .as_mut()
                        .ok_or("Connect your ESP Gauge to change its settings.")?;
                    if op != "release" && job.command["device"].as_str() != Some(l.device.as_str())
                    {
                        return Err("The connected board changed. Select it before editing.".into());
                    }
                    if op == "calibrate" && l.max_duty < 1000 {
                        return Err("Update this board’s firmware to calibrate its range.".into());
                    }
                    let config = if op == "config" {
                        let c: Config = serde_json::from_value(job.command["config"].clone())
                            .map_err(|e| e.to_string())?;
                        c.validate()?;
                        if l.max_duty < 1000
                            && c.channels
                                .iter()
                                .any(|c| c.min_duty > 0 || c.max_duty > l.max_duty)
                        {
                            return Err(
                                "Update this board’s firmware to use this calibration range."
                                    .into(),
                            );
                        }
                        Some(c)
                    } else {
                        None
                    };
                    if op == "pause" {
                        state.paused = job.command["paused"]
                            .as_bool()
                            .ok_or("Invalid pause setting")?;
                    }
                    let reply = l.request(job.command)?;
                    if op == "wifi_scan" {
                        wifi_scan = Instant::now();
                    }
                    if op == "calibrate" {
                        calibrating = true;
                    }
                    if op == "calibrate_end" || op == "release" || op == "pause" {
                        calibrating = false;
                    }
                    if let Some(c) = config {
                        state.config = Some(c);
                    }
                    state.error = None;
                    Ok(reply)
                })();
                if let Err(e) = &result {
                    state.error = Some(e.clone());
                }
                let _ = job.reply.send(result);
                publish(&app, &shared, &state);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some((path, receiver)) = &probing {
            if let Ok(result) = receiver.try_recv() {
                let path = path.clone();
                probing = None;
                match result {
                    Ok(mut l) => {
                        seen.insert(path.clone(), (Some(l.device.clone()), Instant::now()));
                        if !state.devices.iter().any(|d| d.path == path) {
                            state.devices.push(Device {
                                path: path.clone(),
                                id: l.device.clone(),
                            });
                        }
                        if link.is_none() {
                            match connect(&mut l) {
                                Ok(c) => {
                                    state.config = Some(c);
                                    state.device = l.device.clone();
                                    state.firmware = l.firmware.clone();
                                    state.path = path;
                                    state.connected = true;
                                    state.error = None;
                                    state.paused = false;
                                    link = Some(l);
                                }
                                Err(e) => state.error = Some(e),
                            }
                        }
                    }
                    Err(e) => {
                        seen.insert(path, (None, Instant::now()));
                        if link.is_none()
                            && (e.contains("Permission")
                                || e.contains("denied")
                                || e.contains("busy"))
                        {
                            state.error = Some(format!("USB port unavailable: {e}"));
                        }
                    }
                }
                publish(&app, &shared, &state);
            }
        }
        if !maintenance
            && !calibrating
            && probing.is_none()
            && last_scan.elapsed() >= Duration::from_secs(3)
        {
            last_scan = Instant::now();
            let paths = transport::candidates();
            state.candidates = paths.clone();
            seen.retain(|p, (id, at)| {
                paths.contains(p) && (id.is_some() || at.elapsed() < Duration::from_secs(60))
            });
            state.devices.retain(|d| paths.contains(&d.path));
            if let Some(path) = paths.into_iter().find(|path| {
                !link.as_ref().is_some_and(|l| l.path == *path) && !seen.contains_key(path)
            }) {
                let (sender, receiver) = mpsc::channel();
                let probe_path = path.clone();
                // Opening another bridge must never stall the active board's readings.
                thread::spawn(move || {
                    let _ = sender.send(Link::probe(&probe_path));
                });
                probing = Some((path, receiver));
            }
            publish(&app, &shared, &state);
        }
        if last_sample.elapsed() >= Duration::from_millis(750) {
            last_sample = Instant::now();
            state.metrics = metrics.sample();
            if let Some(l) = link.as_mut() {
                let config = state.config.as_ref().unwrap();
                let values: Vec<Value> = config
                    .channels
                    .iter()
                    .map(|c| {
                        if autonomous(&c.source) {
                            Value::Null
                        } else {
                            state
                                .metrics
                                .get(&c.source)
                                .map(|v| json!(normalize(*v, c.scale)))
                                .unwrap_or(Value::Null)
                        }
                    })
                    .collect();
                let result=l.request(json!({"op":"live","values":values,"paused":state.paused,"include_networks":wifi_scan.elapsed()<Duration::from_secs(30)}));
                match result {
                    Ok(v) => {
                        calibrating = v["calibrating"].as_i64().is_some_and(|p| p >= 0);
                        state.board = v;
                        if last_clock.elapsed() >= Duration::from_secs(30) {
                            let _ = sync_time(l);
                            last_clock = Instant::now();
                        }
                    }
                    Err(e) => {
                        seen.remove(&l.path);
                        state.connected = false;
                        calibrating = false;
                        state.board = json!({});
                        state.error = Some(e);
                        link = None;
                    }
                }
            }
            publish(&app, &shared, &state);
        }
    }
}
