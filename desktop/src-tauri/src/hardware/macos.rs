use core_foundation::{
    base::{CFType, TCFType},
    dictionary::CFDictionary,
    number::CFNumber,
    string::CFString,
};
use std::ffi::{c_char, c_void};
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingServices(port: u32, matching: *mut c_void, iterator: *mut u32) -> i32;
    fn IOIteratorNext(iterator: u32) -> u32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IORegistryEntryCreateCFProperty(
        entry: u32,
        key: *const c_void,
        allocator: *const c_void,
        options: u32,
    ) -> *const c_void;
    fn IORegistryEntryGetRegistryEntryID(entry: u32, id: *mut u64) -> i32;
}
struct Object(u32);
impl Drop for Object {
    fn drop(&mut self) {
        unsafe {
            IOObjectRelease(self.0);
        }
    }
}
fn property(entry: u32, name: &str) -> Option<CFType> {
    let key = CFString::new(name);
    let value = unsafe {
        IORegistryEntryCreateCFProperty(
            entry,
            key.as_concrete_TypeRef().cast(),
            std::ptr::null(),
            0,
        )
    };
    if value.is_null() {
        None
    } else {
        Some(unsafe { CFType::wrap_under_create_rule(value) })
    }
}
pub fn gpus() -> Vec<(String, String, f64)> {
    let mut result = Vec::new();
    let mut iterator = 0;
    unsafe {
        let matching = IOServiceMatching(c"IOAccelerator".as_ptr());
        if matching.is_null() || IOServiceGetMatchingServices(0, matching, &mut iterator) != 0 {
            return result;
        }
    }
    let iterator = Object(iterator);
    loop {
        let entry = unsafe { IOIteratorNext(iterator.0) };
        if entry == 0 {
            break;
        }
        let entry = Object(entry);
        let Some(stats) =
            property(entry.0, "PerformanceStatistics").and_then(|p| p.downcast::<CFDictionary>())
        else {
            continue;
        };
        let stats: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(stats.as_concrete_TypeRef()) };
        let value = ["Device Utilization %", "GPU Activity(%)"]
            .into_iter()
            .find_map(|key| {
                stats
                    .find(CFString::new(key))
                    .and_then(|v| v.downcast::<CFNumber>())
                    .and_then(|n| n.to_f64())
            });
        if let Some(value) = value.filter(|v| (0.0..=100.0).contains(v)) {
            let mut id = 0;
            unsafe {
                IORegistryEntryGetRegistryEntryID(entry.0, &mut id);
            }
            result.push((format!("{id:x}"), "Apple graphics processor".into(), value));
        }
    }
    result
}
