#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
use crate::providers::{Choice, Feed, Source};

pub struct Hardware {
    components: sysinfo::Components,
    #[cfg(target_os = "macos")]
    smc: Option<macpow::smc::SmcConnection>,
    #[cfg(any(windows, target_os = "linux"))]
    nvml: Option<nvml_wrapper::Nvml>,
    #[cfg(windows)]
    gpu: windows::Gpu,
}
impl Hardware {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        let smc = macpow::smc::SmcConnection::open().ok().map(|mut smc| {
            let discovery = smc.start_temp_discovery();
            smc.finish_temp_discovery(discovery);
            smc
        });
        Self {
            components: sysinfo::Components::new_with_refreshed_list(),
            #[cfg(target_os = "macos")]
            smc,
            #[cfg(any(windows, target_os = "linux"))]
            nvml: nvml_wrapper::Nvml::init().ok(),
            #[cfg(windows)]
            gpu: windows::Gpu::new(),
        }
    }
    pub fn sample(&mut self) -> Feed {
        let mut feed = Feed::default();
        let mut gpu = Source::new("gpu", "GPU usage", "This computer", "%", 100.0, "How hard your graphics processor is working. Availability depends on the graphics driver.");
        let mut cpu_temp = Source::new("cpu_temperature", "CPU temperature", "This computer", "°C", 100.0, "Hottest CPU sensor, or choose one sensor. Requires a temperature sensor exposed by your operating system. On Windows, run Libre Hardware Monitor to expose CPU sensors.");
        let mut gpu_temp = Source::new("gpu_temperature", "GPU temperature", "This computer", "°C", 100.0, "Graphics processor temperature. Requires a sensor exposed by your GPU driver or operating system.");
        self.components.refresh(true);
        for component in &self.components {
            let label = component.label();
            let lower = label.to_lowercase();
            let source = if lower.contains("gpu") {
                &mut gpu_temp
            } else if lower.contains("cpu")
                || lower.contains("core")
                || lower.contains("package")
                || lower.starts_with("k10temp")
                || lower.starts_with("zenpower")
            {
                &mut cpu_temp
            } else {
                continue;
            };
            add(
                &mut feed,
                source,
                label,
                label,
                component.temperature().map(f64::from),
            );
        }
        #[cfg(target_os = "macos")]
        if let Some(smc) = &mut self.smc {
            for sensor in smc.read_temperatures().into_iter().filter(|s| !s.stale) {
                let source = match sensor.category.as_str() {
                    "CPU" => &mut cpu_temp,
                    "GPU" => &mut gpu_temp,
                    _ => continue,
                };
                add(
                    &mut feed,
                    source,
                    &sensor.key,
                    &format!("{} · {}", sensor.category, sensor.key),
                    Some(sensor.value_celsius as f64),
                );
            }
        }
        #[cfg(windows)]
        for sensor in windows::temperatures() {
            let source = if sensor.identifier.contains("cpu") {
                &mut cpu_temp
            } else if sensor.identifier.contains("gpu") {
                &mut gpu_temp
            } else {
                continue;
            };
            add(
                &mut feed,
                source,
                &sensor.identifier,
                &sensor.name,
                Some(sensor.value),
            );
        }
        #[cfg(target_os = "macos")]
        for (id, name, value) in macos::gpus() {
            add(&mut feed, &mut gpu, &id, &name, Some(value));
        }
        #[cfg(windows)]
        for (id, value) in self.gpu.sample() {
            add(&mut feed, &mut gpu, &id, &format!("GPU {id}"), Some(value));
        }
        #[cfg(any(windows, target_os = "linux"))]
        if let Some(nvml) = &self.nvml {
            for index in 0..nvml.device_count().unwrap_or(0) {
                let Ok(device) = nvml.device_by_index(index) else {
                    continue;
                };
                let Ok(id) = device.uuid() else { continue };
                let name = device
                    .name()
                    .unwrap_or_else(|_| format!("NVIDIA GPU {}", index + 1));
                #[cfg(not(windows))]
                add(
                    &mut feed,
                    &mut gpu,
                    &id,
                    &name,
                    device.utilization_rates().ok().map(|r| r.gpu as f64),
                );
                add(
                    &mut feed,
                    &mut gpu_temp,
                    &id,
                    &name,
                    device
                        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .ok()
                        .map(f64::from),
                );
            }
        }
        #[cfg(target_os = "linux")]
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("card") || !name[4..].chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let path = entry.path().join("device");
                let Ok(id) = std::fs::canonicalize(&path) else {
                    continue;
                };
                let id = id.file_name().unwrap_or_default().to_string_lossy();
                let value = std::fs::read_to_string(path.join("gpu_busy_percent"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok());
                if value.is_some() {
                    add(&mut feed, &mut gpu, &id, &format!("{name} · {id}"), value);
                }
            }
        }
        let volume = volumecontrol::AudioDevice::from_default()
            .and_then(|d| d.get_vol())
            .ok()
            .map(f64::from);
        feed.add(Source::new("volume", "System volume", "This computer", "%", 100.0, "The volume setting of your computer’s default output, even when nothing is playing. Hardware outputs with no volume control are unavailable."), volume);
        for source in [gpu, cpu_temp, gpu_temp] {
            feed.sources.push(source);
        }
        feed
    }
}
fn add(feed: &mut Feed, source: &mut Source, id: &str, name: &str, value: Option<f64>) {
    source.options.push(Choice {
        id: id.into(),
        name: name.into(),
    });
    if let Some(value) = value.filter(|v| v.is_finite() && *v >= 0.0) {
        feed.values.insert(format!("{}:{id}", source.id), value);
        feed.values
            .entry(source.id.clone())
            .and_modify(|v| *v = v.max(value))
            .or_insert(value);
    }
}
