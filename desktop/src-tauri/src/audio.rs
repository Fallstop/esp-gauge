use crate::providers::{Feed, Source};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub struct Audio {
    wanted: Arc<AtomicBool>,
    feed: Arc<Mutex<Feed>>,
}
impl Audio {
    pub fn start() -> Self {
        let wanted = Arc::new(AtomicBool::new(false));
        let feed = Arc::new(Mutex::new(catalog(None, None)));
        let active = wanted.clone();
        let shared = feed.clone();
        std::thread::spawn(move || {
            let peak = Arc::new(AtomicU32::new(0));
            let failed = Arc::new(AtomicBool::new(false));
            let mut stream = None;
            let mut device_id = String::new();
            let mut last_check = Instant::now() - Duration::from_secs(10);
            let mut last_error = None;
            loop {
                if !active.load(Ordering::Relaxed) {
                    stream = None;
                    last_error = None;
                    peak.store(0, Ordering::Relaxed);
                    *shared.lock().unwrap() = catalog(None, None);
                } else {
                    if last_check.elapsed() >= Duration::from_secs(2) {
                        last_check = Instant::now();
                        match output() {
                            Ok(device) => {
                                let id = device.id().map(|id| id.to_string()).unwrap_or_default();
                                if id != device_id {
                                    stream = None;
                                    last_error = None;
                                    device_id = id;
                                }
                                if failed.swap(false, Ordering::Relaxed) {
                                    stream = None;
                                    last_error = Some(
                                        "Audio output disconnected. Reselect Audio level to retry."
                                            .into(),
                                    );
                                }
                                if stream.is_none() && last_error.is_none() {
                                    match capture(&device, peak.clone(), failed.clone()) {
                                        Ok(next) => stream = Some(next),
                                        Err(e) => last_error = Some(e),
                                    }
                                }
                            }
                            Err(e) => {
                                stream = None;
                                last_error = Some(e);
                            }
                        }
                    }
                    let value = stream
                        .as_ref()
                        .map(|_| f32::from_bits(peak.swap(0, Ordering::Relaxed)) as f64 * 100.0);
                    *shared.lock().unwrap() = catalog(value, last_error.as_deref());
                }
                std::thread::sleep(Duration::from_millis(125));
            }
        });
        Self { wanted, feed }
    }
    pub fn snapshot(&self, wanted: bool) -> Feed {
        self.wanted.store(wanted, Ordering::Relaxed);
        self.feed.lock().unwrap().clone()
    }
}
fn catalog(value: Option<f64>, error: Option<&str>) -> Feed {
    let mut feed = Feed::default();
    let description = error.unwrap_or("Live peak level of sound playing through your default output. Audio is measured in memory and never recorded. On macOS, allow system audio access when asked.");
    feed.add(
        Source::new(
            "audio",
            "Audio level",
            "This computer",
            "%",
            100.0,
            description,
        ),
        value,
    );
    feed
}
fn output() -> Result<cpal::Device, String> {
    #[cfg(target_os = "linux")]
    {
        let host = cpal::host_from_id(cpal::HostId::PulseAudio).map_err(|e| e.to_string())?;
        let output = host
            .default_output_device()
            .ok_or("No default audio output. Start PulseAudio or PipeWire’s PulseAudio service.")?;
        let monitor = format!("{}.monitor", output.id().map_err(|e| e.to_string())?);
        host.input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.id().is_ok_and(|id| id.to_string() == monitor))
            .ok_or("No monitor for the default audio output.".into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        cpal::default_host()
            .default_output_device()
            .ok_or("No default audio output is available.".into())
    }
}
fn capture(
    device: &cpal::Device,
    peak: Arc<AtomicU32>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    // CPAL treats duplex macOS devices as microphone inputs; never fall back to recording one.
    #[cfg(target_os = "macos")]
    if device.supports_input() {
        return Err("This audio device combines input and output. Select an output-only device in Sound settings to meter playback.".into());
    }
    #[cfg(target_os = "linux")]
    let config = device.default_input_config();
    #[cfg(not(target_os = "linux"))]
    let config = device.default_output_config();
    let config = config.map_err(|e| e.to_string())?;
    let format = config.sample_format();
    let stream_failed = failed.clone();
    let stream = device.build_input_stream_raw(config.config(), format, move |data, _| {
        let value = match data.sample_format() {
            cpal::SampleFormat::F32 => data.as_slice::<f32>().map(|s| s.iter().copied().map(f32::abs).fold(0.0, f32::max)),
            cpal::SampleFormat::F64 => data.as_slice::<f64>().map(|s| s.iter().map(|v| v.abs() as f32).fold(0.0, f32::max)),
            cpal::SampleFormat::I16 => data.as_slice::<i16>().map(|s| s.iter().map(|v| (*v as f32 / 32768.0).abs()).fold(0.0, f32::max)),
            cpal::SampleFormat::I32 => data.as_slice::<i32>().map(|s| s.iter().map(|v| (*v as f32 / 2147483648.0).abs()).fold(0.0, f32::max)),
            _ => None,
        };
        if let Some(v) = value.filter(|v| v.is_finite()) { peak.fetch_max(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed); }
        else { failed.store(true, Ordering::Relaxed); }
    }, {
        let failed = stream_failed; move |_| { failed.store(true, Ordering::Relaxed); }
    }, Some(Duration::from_secs(3))).map_err(|e| format!("Audio access unavailable: {e}. Check system audio permissions, then reselect Audio level."))?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
