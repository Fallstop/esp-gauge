use serde_json::{json, Value};
use serialport::{SerialPort, SerialPortType};
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

pub struct Link {
    port: Box<dyn SerialPort>,
    next: u64,
    buffer: Vec<u8>,
    pub path: String,
    pub device: String,
    pub firmware: String,
    pub max_duty: u16,
}
pub fn candidates() -> Vec<String> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !cfg!(target_os = "macos") || p.port_name.starts_with("/dev/cu."))
        .filter_map(|p| match p.port_type {
            SerialPortType::UsbPort(u) if u.vid == 0x1a86 && u.pid == 0x7523 => Some(p.port_name),
            _ => None,
        })
        .collect()
}
pub fn is_identity(v: &Value) -> bool {
    v["ok"] == true
        && v["product"] == "ESP Gauge"
        && v["protocol"] == 2
        && v["channels"] == 6
        && matches!(v["max_duty"].as_u64(), Some(880 | 1000))
        && v["device"].as_str().is_some_and(|s| !s.is_empty())
}
impl Link {
    pub fn probe(path: &str) -> Result<Self, String> {
        let mut port = serialport::new(path, 115200)
            .timeout(Duration::from_millis(25))
            .dtr_on_open(false)
            .open()
            .map_err(|e| e.to_string())?;
        let _ = port.write_request_to_send(false);
        // Some CH340 drivers pulse EN while opening despite the requested line levels.
        // Let a possible ESP32 reboot finish before the single identity query.
        std::thread::sleep(Duration::from_millis(2200));
        let mut link = Self {
            port,
            next: 1,
            buffer: Vec::new(),
            path: path.into(),
            device: String::new(),
            firmware: String::new(),
            max_duty: 0,
        };
        // One bounded read-only query. Non-gauge CH340s are released and not streamed to.
        let reply = link.request_timeout(json!({"op":"hello"}), Duration::from_millis(700))?;
        if !is_identity(&reply) {
            return Err("Not ESP Gauge protocol 2".into());
        }
        link.device = reply["device"].as_str().unwrap().into();
        link.firmware = reply["firmware"].as_str().unwrap_or("2.0.0").into();
        link.max_duty = reply["max_duty"].as_u64().unwrap() as u16;
        Ok(link)
    }
    pub fn request(&mut self, cmd: Value) -> Result<Value, String> {
        self.request_timeout(cmd, Duration::from_millis(1200))
    }
    fn request_timeout(&mut self, mut cmd: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next;
        self.next += 1;
        cmd["id"] = json!(id);
        let mut bytes = serde_json::to_vec(&cmd).map_err(|e| e.to_string())?;
        if bytes.len() > 6000 {
            return Err("Command exceeds the board's message limit".into());
        }
        bytes.push(b'\n');
        self.port.write_all(&bytes).map_err(|e| e.to_string())?;
        let start = Instant::now();
        let mut chunk = [0u8; 1024];
        while start.elapsed() < timeout {
            match self.port.read(&mut chunk) {
                Ok(0) => {}
                Ok(n) => {
                    self.buffer.extend_from_slice(&chunk[..n]);
                    while let Some(pos) = self.buffer.iter().position(|b| *b == b'\n') {
                        let line: Vec<_> = self.buffer.drain(..=pos).collect();
                        if let Ok(v) = serde_json::from_slice::<Value>(&line) {
                            if v["id"].as_u64() == Some(id) {
                                if v["ok"] == true {
                                    return Ok(v);
                                }
                                return Err(v["error"]
                                    .as_str()
                                    .unwrap_or("The board rejected this command")
                                    .into());
                            }
                        }
                    }
                    if self.buffer.len() > 8192 {
                        self.buffer.clear();
                        return Err("Unexpected serial data".into());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("The board stopped responding. Reconnecting…".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_is_not_identity() {
        assert!(!is_identity(&json!({"ok":true,"product":"Other board"})));
        let v = json!({"ok":true,"product":"ESP Gauge","protocol":2,"channels":6,"max_duty":880,"device":"1234"});
        assert!(is_identity(&v));
        let mut bad = v;
        bad["protocol"] = json!(1);
        assert!(!is_identity(&bad));
    }
    #[cfg(unix)]
    #[test]
    fn serial_framing_correlates_replies_and_ignores_boot_noise() {
        let (master, mut slave) = serialport::TTYPort::pair().unwrap();
        let server = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let mut b = [0u8; 1];
            loop {
                if slave.read(&mut b).unwrap_or(0) > 0 {
                    bytes.push(b[0]);
                    if b[0] == b'\n' {
                        break;
                    }
                }
            }
            let req: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(req["op"], "hello");
            slave
                .write_all(b"boot log\n{\"id\":999,\"ok\":true}\n{invalid}\n")
                .unwrap();
            let reply = json!({"id":req["id"],"ok":true,"product":"ESP Gauge","protocol":2,"channels":6,"max_duty":880,"device":"test"});
            let line = format!("{reply}\n");
            let half = line.len() / 2;
            slave.write_all(&line.as_bytes()[..half]).unwrap();
            std::thread::sleep(Duration::from_millis(10));
            slave.write_all(&line.as_bytes()[half..]).unwrap();
            std::thread::sleep(Duration::from_millis(50));
        });
        let mut link = Link {
            port: Box::new(master),
            next: 1,
            buffer: Vec::new(),
            path: "test".into(),
            device: String::new(),
            firmware: String::new(),
            max_duty: 0,
        };
        let reply = link.request(json!({"op":"hello"})).unwrap();
        assert!(is_identity(&reply));
        server.join().unwrap();
    }
}
