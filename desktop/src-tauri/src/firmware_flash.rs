use crate::{releases::Firmware, transport, updates::Updates, worker::Service};
use espflash::{
    connection::{Connection, ResetAfterOperation, ResetBeforeOperation},
    flasher::Flasher,
    image_format::Segment,
    target::{Chip, ProgressCallbacks},
};
use serialport::SerialPortType;
use std::{borrow::Cow, time::Duration};
use tauri::{AppHandle, Manager};

struct Progress {
    app: AppHandle,
    total: usize,
    part: usize,
}
impl ProgressCallbacks for Progress {
    fn init(&mut self, addr: u32, total: usize) {
        self.total = total;
        self.part = match addr {
            0x1000 => 0,
            0x8000 => 1,
            0xe000 => 2,
            _ => 3,
        };
    }
    fn update(&mut self, current: usize) {
        let percent = ((self.part as f64 + current as f64 / self.total.max(1) as f64) / 4.0
            * 100.0)
            .min(100.0);
        self.app
            .state::<Updates>()
            .progress(&self.app, "Writing firmware", percent);
    }
    fn verifying(&mut self) {
        self.app.state::<Updates>().progress(
            &self.app,
            "Verifying firmware",
            (self.part + 1) as f64 * 25.0,
        );
    }
    fn finish(&mut self, _: bool) {}
}
struct Resume(Service);
impl Drop for Resume {
    fn drop(&mut self) {
        let _ = self
            .0
            .execute(serde_json::json!({"op":"maintenance","active":false}));
    }
}
pub fn install(
    app: &AppHandle,
    service: &Service,
    path: &str,
    firmware: &Firmware,
    data: &[Vec<u8>],
) -> Result<(), String> {
    if !transport::candidates().contains(&path.to_owned()) {
        return Err("The selected CH340C is no longer connected.".into());
    }
    service.execute(serde_json::json!({"op":"maintenance","active":true}))?;
    let _resume = Resume(service.clone());
    let info = serialport::available_ports()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|p| p.port_name == path)
        .ok_or("USB device disconnected")?;
    let SerialPortType::UsbPort(usb) = info.port_type else {
        return Err("Choose a CH340C USB board.".into());
    };
    if usb.vid != 0x1a86 || usb.pid != 0x7523 {
        return Err("The USB device changed.".into());
    }
    app.state::<Updates>()
        .progress(app, "Connecting to bootloader", 0.0);
    let port = serialport::new(path, 115200)
        .timeout(Duration::from_secs(3))
        .dtr_on_open(false)
        .open_native()
        .map_err(|e| e.to_string())?;
    let connection = Connection::new(
        port,
        usb,
        ResetAfterOperation::HardReset,
        ResetBeforeOperation::DefaultReset,
        115200,
    );
    let mut flasher = Flasher::connect(
        connection,
        true,
        true,
        true,
        Some(Chip::Esp32),
        Some(230400),
    )
    .map_err(|e| e.to_string())?;
    if flasher
        .device_info()
        .map_err(|e| e.to_string())?
        .flash_size
        .size()
        < 4 * 1024 * 1024
    {
        return Err("This firmware needs an ESP32 with at least 4 MB flash.".into());
    }
    // Separate segments never erase the NVS gap at 0x9000–0xdfff.
    let segments: Vec<_> = firmware
        .segments
        .iter()
        .zip(data)
        .map(|(s, bytes)| Segment {
            addr: s.offset,
            data: Cow::Borrowed(bytes.as_slice()),
        })
        .collect();
    flasher
        .write_bins_to_flash(
            &segments,
            &mut Progress {
                app: app.clone(),
                total: 1,
                part: 0,
            },
        )
        .map_err(|e| format!("{e}. Reconnect the board and retry installation."))?;
    drop(flasher);
    app.state::<Updates>()
        .progress(app, "Restarting board", 100.0);
    let mut link = transport::Link::probe(path)?;
    if link.firmware != firmware.version {
        return Err("The board did not start the expected firmware. Retry installation.".into());
    }
    link.request(serde_json::json!({"op":"release"}))?;
    Ok(())
}
