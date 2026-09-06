fn matches_board(ids: &[u16]) -> bool {
    ids.split(|&c| c == 0)
        .any(|id| String::from_utf16_lossy(id).eq_ignore_ascii_case(r"USB\VID_1A86&PID_7523"))
}

#[cfg(windows)]
pub use native::{available, install, remove_legacy_shortcut};

#[cfg(windows)]
mod native {
    use std::{io, mem, path::PathBuf, ptr};
    use windows_sys::Win32::{
        Devices::DeviceAndDriverInstallation::*,
        Foundation::{
            CloseHandle, ERROR_CANCELLED, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS,
        },
        System::{
            Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED},
            Threading::{WaitForSingleObject, INFINITE},
        },
        UI::Shell::{
            FOLDERID_Programs, SHGetKnownFolderPath, ShellExecuteExW, SEE_MASK_NOASYNC,
            SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        },
    };

    struct DriverList(HDEVINFO);

    impl Drop for DriverList {
        fn drop(&mut self) {
            unsafe { SetupDiDestroyDeviceInfoList(self.0) };
        }
    }

    fn checked(result: i32) -> io::Result<()> {
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn available() -> io::Result<bool> {
        // A global class list reads staged packages without creating a device or needing admin.
        unsafe {
            let handle = SetupDiCreateDeviceInfoList(&GUID_DEVCLASS_PORTS, ptr::null_mut());
            if handle == -1 {
                return Err(io::Error::last_os_error());
            }
            let list = DriverList(handle);
            let mut params = SP_DEVINSTALL_PARAMS_W {
                cbSize: mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32,
                ..Default::default()
            };
            checked(SetupDiGetDeviceInstallParamsW(
                list.0,
                ptr::null(),
                &mut params,
            ))?;
            params.Flags |= DI_QUIETINSTALL;
            // WCH marks its Plug and Play drivers ExcludeFromSelect.
            params.FlagsEx |= DI_FLAGSEX_ALLOWEXCLUDEDDRVS | DI_FLAGSEX_SEARCH_PUBLISHED_INFS;
            checked(SetupDiSetDeviceInstallParamsW(list.0, ptr::null(), &params))?;
            checked(SetupDiBuildDriverInfoList(
                list.0,
                ptr::null_mut(),
                SPDIT_CLASSDRIVER,
            ))?;

            for index in 0.. {
                let mut driver = SP_DRVINFO_DATA_V2_W {
                    cbSize: mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32,
                    ..Default::default()
                };
                if SetupDiEnumDriverInfoW(
                    list.0,
                    ptr::null(),
                    SPDIT_CLASSDRIVER,
                    index,
                    &mut driver,
                ) == 0
                {
                    let error = io::Error::last_os_error();
                    return if error.raw_os_error() == Some(ERROR_NO_MORE_ITEMS as i32) {
                        Ok(false)
                    } else {
                        Err(error)
                    };
                }
                if driver_matches(&list, &driver)? {
                    return Ok(true);
                }
            }
            unreachable!()
        }
    }

    unsafe fn driver_matches(list: &DriverList, driver: &SP_DRVINFO_DATA_V2_W) -> io::Result<bool> {
        let mut needed = 0;
        let result = SetupDiGetDriverInfoDetailW(
            list.0,
            ptr::null(),
            driver,
            ptr::null_mut(),
            0,
            &mut needed,
        );
        if result == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32) {
                return Err(error);
            }
        }
        if needed < mem::size_of::<SP_DRVINFO_DETAIL_DATA_W>() as u32 || needed > 1024 * 1024 {
            return Err(io::Error::other("Invalid Windows driver information size"));
        }
        // u64 storage preserves the native structure's alignment, including its trailing IDs.
        let mut storage = vec![0u64; (needed as usize).div_ceil(8)];
        let detail = storage.as_mut_ptr().cast::<SP_DRVINFO_DETAIL_DATA_W>();
        (*detail).cbSize = mem::size_of::<SP_DRVINFO_DETAIL_DATA_W>() as u32;
        checked(SetupDiGetDriverInfoDetailW(
            list.0,
            ptr::null(),
            driver,
            detail,
            needed,
            ptr::null_mut(),
        ))?;
        let offset = mem::offset_of!(SP_DRVINFO_DETAIL_DATA_W, HardwareID);
        let ids = std::slice::from_raw_parts(
            storage.as_ptr().cast::<u8>().add(offset).cast::<u16>(),
            (needed as usize - offset) / 2,
        );
        Ok(super::matches_board(ids))
    }

    pub fn remove_legacy_shortcut() {
        let Some(shortcut) = shortcut_path() else {
            return;
        };
        let _ = std::fs::remove_file(shortcut);
    }

    pub fn install(path: PathBuf) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        if !path.is_file() {
            return Err("USB driver setup is missing. Reinstall ESP Gauge using the Windows setup installer.".into());
        }
        let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
        unsafe {
            let initialized = CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32) >= 0;
            let mut launch = SHELLEXECUTEINFOW {
                cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
                lpVerb: verb.as_ptr(),
                lpFile: file.as_ptr(),
                nShow: 1,
                ..Default::default()
            };
            let result = if ShellExecuteExW(&mut launch) == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_CANCELLED as i32) {
                    Err(
                        "Administrator approval was cancelled. You can try USB driver setup again."
                            .into(),
                    )
                } else {
                    Err(format!("Could not open USB driver setup: {error}"))
                }
            } else {
                if !launch.hProcess.is_null() {
                    WaitForSingleObject(launch.hProcess, INFINITE);
                    CloseHandle(launch.hProcess);
                }
                Ok(())
            };
            if initialized {
                CoUninitialize();
            }
            result
        }
    }

    fn shortcut_path() -> Option<PathBuf> {
        use std::os::windows::ffi::OsStringExt;
        unsafe {
            let mut path = ptr::null_mut();
            if SHGetKnownFolderPath(&FOLDERID_Programs, 0, ptr::null_mut(), &mut path) < 0 {
                return None;
            }
            let mut len = 0;
            while *path.add(len) != 0 {
                len += 1;
            }
            let folder = std::ffi::OsString::from_wide(std::slice::from_raw_parts(path, len));
            CoTaskMemFree(path.cast());
            Some(PathBuf::from(folder).join("ESP Gauge USB driver setup.lnk"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact_board_id_among_multiple_driver_ids() {
        for value in [
            "USB\\VID_1A86&PID_7523\0",
            "usb\\vid_1a86&pid_7523\0\0",
            "USB\\VID_1A86&PID_5523\0USB\\VID_1A86&PID_7523\0\0",
        ] {
            assert!(matches_board(&value.encode_utf16().collect::<Vec<_>>()));
        }
        for value in [
            "",
            "\0",
            "USB\\VID_1A86&PID_5523\0",
            "USB\\VID_1A86&PID_75230\0",
        ] {
            assert!(!matches_board(&value.encode_utf16().collect::<Vec<_>>()));
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_can_query_the_driver_store_without_a_board() {
        available().expect("Windows driver store query failed");
    }
}
