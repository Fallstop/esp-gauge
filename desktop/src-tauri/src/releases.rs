use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, time::Duration};

const RELEASE_API: &str = "https://api.github.com/repos/Fallstop/esp-gauge/releases/latest";
const RELEASE_PREFIX: &str = "https://github.com/Fallstop/esp-gauge/releases/download/";

#[derive(Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
#[derive(Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub offset: u32,
    pub size: usize,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Firmware {
    pub version: String,
    pub chip: String,
    pub layout: String,
    pub segments: Vec<Segment>,
}
pub fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("ESP-Gauge/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(90))
        .connect_timeout(Duration::from_secs(12))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())
}
pub fn bounded_get(client: &Client, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    bounded_get_progress(client, url, limit, &mut |_| {})
}
fn bounded_get_progress(
    client: &Client,
    url: &str,
    limit: usize,
    progress: &mut dyn FnMut(usize),
) -> Result<Vec<u8>, String> {
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    if response.status() == 404 {
        return Err("No published release is available yet.".into());
    }
    let mut response = response.error_for_status().map_err(|e| e.to_string())?;
    read_download(&mut response, limit, progress)
}
fn read_download(
    response: &mut dyn Read,
    limit: usize,
    progress: &mut dyn FnMut(usize),
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let length = response.read(&mut chunk).map_err(|e| e.to_string())?;
        if length == 0 {
            break;
        }
        if bytes.len() + length > limit {
            return Err("Release download exceeded its size limit.".into());
        }
        bytes.extend_from_slice(&chunk[..length]);
        progress(bytes.len());
    }
    Ok(bytes)
}

impl Release {
    pub fn latest(client: &Client) -> Result<Self, String> {
        serde_json::from_slice(&bounded_get(client, RELEASE_API, 512 * 1024)?)
            .map_err(|e| e.to_string())
    }
    pub fn asset(&self, client: &Client, name: &str, limit: usize) -> Result<Vec<u8>, String> {
        self.asset_progress(client, name, limit, &mut |_| {})
    }
    fn asset_progress(
        &self,
        client: &Client,
        name: &str,
        limit: usize,
        progress: &mut dyn FnMut(usize),
    ) -> Result<Vec<u8>, String> {
        let asset = self
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| format!("This release has no {name}."))?;
        let prefix = format!("{RELEASE_PREFIX}{}/", self.tag_name);
        if !asset.browser_download_url.starts_with(&prefix)
            || !asset.browser_download_url.ends_with(&format!("/{name}"))
        {
            return Err("Release asset points outside this release.".into());
        }
        bounded_get_progress(client, &asset.browser_download_url, limit, progress)
    }
    pub fn firmware(&self, client: &Client) -> Result<Firmware, String> {
        let data = self.asset(client, "firmware.json", 16384)?;
        let signature = self.asset(client, "firmware.json.sig", 2048)?;
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).map_err(|e| e.to_string())?;
        verify_signed(
            &data,
            &signature,
            config["plugins"]["updater"]["pubkey"]
                .as_str()
                .ok_or("Missing update key")?,
        )?;
        let manifest: Firmware = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        manifest.validate()?;
        if self.tag_name.trim_start_matches('v') != manifest.version {
            return Err("Firmware and release versions differ.".into());
        }
        Ok(manifest)
    }
}
fn verify_signed(data: &[u8], encoded_signature: &[u8], encoded_key: &str) -> Result<(), String> {
    let decode = |value: &[u8]| -> Result<String, String> {
        let value = std::str::from_utf8(value)
            .map_err(|e| e.to_string())?
            .trim();
        String::from_utf8(STANDARD.decode(value).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    };
    let key = minisign_verify::PublicKey::decode(&decode(encoded_key.as_bytes())?)
        .map_err(|e| e.to_string())?;
    let signature = minisign_verify::Signature::decode(&decode(encoded_signature)?)
        .map_err(|e| e.to_string())?;
    key.verify(data, &signature, false)
        .map_err(|_| "Firmware signature could not be verified.".into())
}
impl Firmware {
    pub fn validate(&self) -> Result<(), String> {
        semver::Version::parse(&self.version).map_err(|e| e.to_string())?;
        let expected = [
            ("bootloader.bin", 0x1000, 0x7000),
            ("partitions.bin", 0x8000, 0x1000),
            ("boot_app0.bin", 0xe000, 0x2000),
            ("firmware.bin", 0x10000, 0x300000),
        ];
        if self.chip != "esp32"
            || self.layout != "esp32-4mb-huge-app-v1"
            || self.segments.len() != expected.len()
        {
            return Err("This firmware does not match the six-output ESP32 board.".into());
        }
        for (segment, (name, offset, limit)) in self.segments.iter().zip(expected) {
            if segment.name != name
                || segment.offset != offset
                || segment.size == 0
                || segment.size > limit
                || segment.sha256.len() != 64
                || !segment.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(
                    "Firmware layout is invalid; configuration storage must remain untouched."
                        .into(),
                );
            }
        }
        Ok(())
    }
    pub fn download(
        &self,
        release: &Release,
        client: &Client,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<Vec<Vec<u8>>, String> {
        let total = self.segments.iter().map(|s| s.size).sum();
        let mut completed = 0;
        self.segments
            .iter()
            .map(|segment| {
                let bytes =
                    release.asset_progress(client, &segment.name, segment.size, &mut |n| {
                        progress(completed + n, total)
                    })?;
                if bytes.len() != segment.size
                    || format!("{:x}", Sha256::digest(&bytes)) != segment.sha256
                {
                    return Err(format!(
                        "{} failed its checksum. Download the update again.",
                        segment.name
                    ));
                }
                completed += bytes.len();
                Ok(bytes)
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_progress_counts_bytes_and_rejects_oversized_reads() {
        let bytes = vec![42; 50_000];
        let mut positions = Vec::new();
        assert_eq!(
            read_download(&mut bytes.as_slice(), bytes.len(), &mut |n| positions
                .push(n))
            .unwrap(),
            bytes
        );
        assert_eq!(positions.last(), Some(&50_000));
        assert!(positions.len() > 1);
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(read_download(&mut bytes.as_slice(), bytes.len() - 1, &mut |_| {}).is_err());
    }
    #[test]
    fn interrupted_download_does_not_report_completion() {
        struct Interrupted;
        impl Read for Interrupted {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("USB-independent network failure"))
            }
        }
        let mut calls = 0;
        assert!(read_download(&mut Interrupted, 100, &mut |_| calls += 1).is_err());
        assert_eq!(calls, 0);
    }
    #[test]
    fn firmware_cannot_write_into_nvs_or_unknown_regions() {
        let mut m = Firmware {
            version: "2.1.0".into(),
            chip: "esp32".into(),
            layout: "esp32-4mb-huge-app-v1".into(),
            segments: [
                ("bootloader.bin", 0x1000),
                ("partitions.bin", 0x8000),
                ("boot_app0.bin", 0xe000),
                ("firmware.bin", 0x10000),
            ]
            .into_iter()
            .map(|(name, offset)| Segment {
                name: name.into(),
                offset,
                size: 16,
                sha256: "a".repeat(64),
            })
            .collect(),
        };
        assert!(m.validate().is_ok());
        m.segments[1].offset = 0x9000;
        assert!(m.validate().is_err());
        m.segments[1].offset = 0x8000;
        m.segments[1].size = 4097;
        assert!(m.validate().is_err());
    }
    #[test]
    fn signed_manifest_accepts_original_and_rejects_tampering() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let key = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        let data = include_bytes!("../../../tests/fixtures/firmware.json");
        let signature = include_bytes!("../../../tests/fixtures/firmware.json.sig");
        assert!(verify_signed(data, signature, key).is_ok());
        let mut changed = data.to_vec();
        changed[10] ^= 1;
        assert!(verify_signed(&changed, signature, key).is_err());
    }
    #[test]
    fn invalid_signature_is_rejected() {
        assert!(verify_signed(b"firmware", b"bad", "bad").is_err());
    }
}
