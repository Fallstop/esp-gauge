use std::collections::BTreeMap;
use windows_sys::Win32::System::Performance::*;
pub struct Gpu {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}
impl Gpu {
    pub fn new() -> Self {
        let mut result = Self {
            query: std::ptr::null_mut(),
            counter: std::ptr::null_mut(),
        };
        let path: Vec<_> = "\\GPU Engine(*)\\Utilization Percentage\0"
            .encode_utf16()
            .collect();
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut result.query) == 0 {
                if PdhAddEnglishCounterW(result.query, path.as_ptr(), 0, &mut result.counter) == 0 {
                    PdhCollectQueryData(result.query);
                } else {
                    PdhCloseQuery(result.query);
                    result.query = std::ptr::null_mut();
                }
            }
        }
        result
    }
    pub fn sample(&mut self) -> BTreeMap<String, f64> {
        let mut engines = BTreeMap::<(String, String), f64>::new();
        unsafe {
            if self.query.is_null() || PdhCollectQueryData(self.query) != 0 {
                return BTreeMap::new();
            }
            let (mut bytes, mut count) = (0, 0);
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                std::ptr::null_mut(),
            );
            if bytes == 0 || bytes > 4 * 1024 * 1024 {
                return BTreeMap::new();
            }
            let mut buffer = vec![0u64; (bytes as usize).div_ceil(8)];
            if PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                buffer.as_mut_ptr().cast(),
            ) != 0
            {
                return BTreeMap::new();
            }
            let items = std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            );
            for item in items {
                if item.FmtValue.CStatus > 1 || item.szName.is_null() {
                    continue;
                }
                let mut len = 0;
                while *item.szName.add(len) != 0 {
                    len += 1;
                }
                let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, len));
                let Some((_, rest)) = name.split_once("luid_") else {
                    continue;
                };
                let Some((gpu, engine)) = rest.split_once("_eng_") else {
                    continue;
                };
                let value = item.FmtValue.Anonymous.doubleValue;
                if value.is_finite() && value >= 0.0 {
                    *engines.entry((gpu.into(), engine.into())).or_default() += value;
                }
            }
        }
        let mut gpus = BTreeMap::<String, f64>::new();
        for ((gpu, _), value) in engines {
            gpus.entry(gpu)
                .and_modify(|v| *v = v.max(value.min(100.0)))
                .or_insert(value.min(100.0));
        }
        gpus
    }
}
impl Drop for Gpu {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Sensor {
    pub identifier: String,
    pub name: String,
    pub value: f64,
}
pub fn temperatures() -> Vec<Sensor> {
    let Ok(connection) = wmi::WMIConnection::with_namespace_path("ROOT\\LibreHardwareMonitor")
    else {
        return Vec::new();
    };
    connection
        .raw_query::<Sensor>(
            "SELECT Identifier, Name, Value FROM Sensor WHERE SensorType = 'Temperature'",
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.value > 0.0 && s.value < 150.0)
        .collect()
}
