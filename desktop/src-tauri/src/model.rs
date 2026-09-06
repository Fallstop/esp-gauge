use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_DUTY: u16 = 1000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    pub enabled: bool,
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub min_duty: u16,
    pub max_duty: u16,
    pub response_ms: u16,
    pub scale: f64,
    #[serde(default)]
    pub input_min: f64,
    pub reverse: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            enabled: false,
            name: String::new(),
            source: "cpu".into(),
            min_duty: 0,
            max_duty: 0,
            response_ms: 500,
            scale: 100.0,
            input_min: 0.0,
            reverse: false,
            extra: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: u8,
    pub channels: [Channel; 6],
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            channels: std::array::from_fn(|_| Channel::default()),
            extra: BTreeMap::new(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 2 {
            return Err(
                "This board uses a newer configuration. Update ESP Gauge before editing it.".into(),
            );
        }
        for c in &self.channels {
            for (key, min, max) in [
                ("period_s", 0.1, 86400.0),
                ("phase_deg", 0.0, 360.0),
                ("curve", -4.0, 4.0),
            ] {
                if let Some(value) = c.extra.get(key) {
                    if !value
                        .as_f64()
                        .is_some_and(|v| v.is_finite() && (min..=max).contains(&v))
                    {
                        return Err("Source settings are outside the supported range.".into());
                    }
                }
            }
            for key in ["device", "product_name", "store_name"] {
                if c.extra
                    .get(key)
                    .is_some_and(|v| v.as_str().is_none_or(|s| s.len() > 160))
                {
                    return Err("Source selection is too long.".into());
                }
            }
            for key in ["product_id", "store_id"] {
                if c.extra
                    .get(key)
                    .is_some_and(|v| v.as_u64().is_none_or(|n| n > i32::MAX as u64))
                {
                    return Err("Invalid product or store selection.".into());
                }
            }
            if c.min_duty > c.max_duty
                || c.max_duty > MAX_DUTY
                || c.response_ms > 5000
                || !c.input_min.is_finite()
                || c.input_min.abs() > 1e9
                || c.input_min >= c.scale
                || !c.scale.is_finite()
                || c.scale <= 0.0
                || c.scale > 1e9
                || c.name.len() > 64
                || c.source.is_empty()
                || c.source.len() > 48
                || c.source
                    .chars()
                    .any(|x| !x.is_ascii_alphanumeric() && x != '_')
            {
                return Err("Gauge settings are outside the supported range.".into());
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 4096 {
            return Err("The board configuration is too large.".into());
        }
        Ok(())
    }
}
pub fn sample_key(channel: &Channel) -> String {
    match channel.source.as_str() {
        "supertracker_product" => format!(
            "product:{}:{}",
            channel
                .extra
                .get("product_id")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            channel
                .extra
                .get("store_id")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        "disk" | "network_down" | "network_up" | "gpu" | "cpu_temperature" | "gpu_temperature" => {
            match channel
                .extra
                .get("device")
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
            {
                Some(device) => format!("{}:{device}", channel.source),
                None => channel.source.clone(),
            }
        }
        _ => channel.source.clone(),
    }
}
pub fn normalize(value: f64, scale: f64) -> f64 {
    if !value.is_finite() || !scale.is_finite() || scale <= 0.0 {
        0.0
    } else {
        (value / scale).clamp(0.0, 1.0)
    }
}
pub fn autonomous(source: &str) -> bool {
    source.starts_with("time_")
        || source.starts_with("esp_")
        || source == "constant"
        || source.starts_with("wave_")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_config_rejected() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        c.channels[5].max_duty = 1001;
        assert!(c.validate().is_err());
        c.channels[5].max_duty = 1000;
        c.channels[5].scale = f64::NAN;
        assert!(c.validate().is_err());
        c.channels[5].scale = 1.0;
        c.version = 3;
        assert!(c.validate().is_err());
    }
    #[test]
    fn unknown_metadata_survives_roundtrip() {
        let mut c = Config::default();
        c.extra.insert("future".into(), serde_json::json!({"x":3}));
        c.channels[0]
            .extra
            .insert("custom".into(), serde_json::json!("kept"));
        assert_eq!(
            c,
            serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap()
        );
    }
    #[test]
    fn source_options_and_curves_are_validated_and_isolated() {
        let mut config = Config::default();
        config.channels[0].source = "disk".into();
        config.channels[0]
            .extra
            .insert("device".into(), serde_json::json!("/Volumes/External"));
        assert_eq!(sample_key(&config.channels[0]), "disk:/Volumes/External");
        assert_eq!(sample_key(&config.channels[1]), "cpu");
        config.channels[1].source = "supertracker_product".into();
        config.channels[1]
            .extra
            .insert("product_id".into(), serde_json::json!(285));
        config.channels[1]
            .extra
            .insert("store_id".into(), serde_json::json!(42));
        assert_eq!(sample_key(&config.channels[1]), "product:285:42");
        config.channels[0]
            .extra
            .insert("curve".into(), serde_json::json!(-4));
        assert!(config.validate().is_ok());
        config.channels[0]
            .extra
            .insert("curve".into(), serde_json::json!(4.1));
        assert!(config.validate().is_err());
        config.channels[0]
            .extra
            .insert("curve".into(), serde_json::json!(0));
        config.channels[1]
            .extra
            .insert("store_id".into(), serde_json::json!(-1));
        assert!(config.validate().is_err());
    }
    #[test]
    fn normalization_is_bounded() {
        assert_eq!(normalize(f64::NAN, 100.0), 0.0);
        assert_eq!(normalize(42.0, 0.0), 0.0);
        assert_eq!(normalize(-1.0, 10.0), 0.0);
        assert_eq!(normalize(100.0, 10.0), 1.0);
        assert_eq!(normalize(25.0, 100.0), 0.25);
    }
}
