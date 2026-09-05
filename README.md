# ESP Gauge

A six-channel physical resource monitor. Tauri 2 + Rust, Svelte, your PCB’s Blender linework, and ESP32 firmware. No Electron or bundled browser. System metrics are sampled by native libraries on macOS, Windows and Linux.

Click a physical header, calibrate the gauge with a live slider, and choose a source. Changes are stored on the board automatically. Closing the window keeps the monitor in the tray; **Pause** rests the needles, and **Quit** ends the connection. **Start at login** is optional in settings.

## Sources

- Computer: CPU, memory, swap, system-drive space, download, upload, battery where available.
- Clock: a 24-hour day, 12-hour hand, minutes, seconds.
- On board: nearby Wi-Fi networks, Bluetooth LE advertisers, internal chip temperature, Wi-Fi signal strength, fixed position.

Sources have a small registry and a sampling adapter. See [adding sources](docs/extending.md).

## First connection

1. Open **Settings → Updates** to install firmware on your selected CH340C board. Firmware and app versions come from signed GitHub releases.
2. Connect a suitably rated gauge to a header and attach USB. The app discovers CH340C bridges and checks the gauge protocol before streaming. Other CH340 boards are released.
3. Select **PWM1–PWM6 → Add gauge**. Output begins at zero. Move the range slider’s upper end slowly to the full-scale mark, and its lower end to the zero mark if needed. Choose **Use this range**.
4. Choose a source. Change sources, response and direction directly; no save/apply button.

The rear row is PWM1, PWM2, PWM3 left to right; the front row is PWM4, PWM5, PWM6. GPIOs are 16, 17, 18, 19, 21 and 22. The NeoPixel is GPIO23. PWM is 5 kHz at 12-bit resolution; the range is 0–100% duty. Size the series resistor for your particular meter.

The board has no load/needle feedback, so gauge presence cannot be reliably detected. The preview is commanded position. Empty ports remain off until explicitly added. Calibration expires after 1.5 seconds without renewal; PC-driven gauges rest after three seconds without readings. Standalone sources continue while powered.

## Independent clocks and sensing

The complete configuration is held in ESP32 NVS, including extension fields the firmware does not interpret. Move the board to a different computer and its assignments and calibration move with it.

Clocks keep running when the app disconnects, while power remains available. Set a 2.4 GHz network in **Board settings** to recover time from NTP after power loss. Without a network, a cold-started clock waits for the desktop to supply time. The last computer UTC offset is retained; reconnect after daylight-saving changes. Wi-Fi credentials stay on the board and are not included in configuration responses.

Bluetooth sensing counts advertising BLE addresses (up to 128), not all nearby Bluetooth devices. Chip temperature is experimental on the original ESP32 and measures the die, not the room. The NeoPixel breathes gently: neutral when powered, green for the desktop connection, blue for independent sources, amber during calibration, red for a configuration storage error.

## Run and build

Requirements: Node 22.12+ or 24, Rust, and the [Tauri platform dependencies](https://v2.tauri.app/start/prerequisites/). Linux packages need WebKitGTK 4.1 and an AppIndicator implementation. GNOME users may need its AppIndicator extension for a visible tray icon; launching the app again always opens the existing instance.

Linux users also need access to the serial device, normally through the distribution's `dialout` group. The tested KDE account already has this access. Windows needs a working CH340 driver and WebView2; the NSIS installer provisions WebView2 when needed.

```sh
cd desktop
npm ci
npm run tauri dev
# Installable app / DMG, Windows NSIS / MSI, Linux deb / AppImage / RPM:
npm run tauri build
```

Use the Tauri build command to create a release with embedded assets. For a bare release binary, include the feature explicitly:

```sh
cargo build --release --features custom-protocol --manifest-path desktop/src-tauri/Cargo.toml
```

The finished executable accepts `--background` to open only the tray and `--diagnose` for a read-only JSON report of system metrics and candidate bridges. The diagnosis does not open serial ports.

### Firmware

Python tooling uses `uv`:

```sh
uv tool install platformio==6.1.19
pio run -d firmware
pio run -d firmware -t upload --upload-port /dev/cu.usbserial-110
```

Change the upload port for your machine. The firmware’s dependencies are pinned. A 3 MB application partition leaves room for Wi-Fi and BLE. Uploading leaves the NVS configuration partition intact. The desktop app installs firmware directly using its native ESP32 flasher; neither Python nor PlatformIO is needed by app users. It verifies signed release metadata and every binary checksum before writing, and leaves the NVS region untouched.

### Verification

```sh
npm --prefix desktop run build
cargo test --locked --manifest-path desktop/src-tauri/Cargo.toml
cargo clippy --locked --manifest-path desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
c++ -std=c++17 -Wall -Wextra -Werror -Ifirmware/include tests/engine_test.cpp -o /tmp/gauge-test
/tmp/gauge-test
# Close the desktop app first. Briefly drives PWM6 at only 1% duty, then restores configuration:
uv run --with pyserial python tools/hardware_check.py --port /dev/cu.usbserial-110
```

See [protocol](docs/protocol.md), [design](docs/design.md), [verification record](docs/verification.md), and [third-party notices](THIRD_PARTY.md). CI is configured for all three desktop platforms and firmware. The Mac bundle receives an ad-hoc integrity signature; distribution packages have no publisher signature or notarization. Public distribution should add platform signing and notarization.

## Releases and updates

The app checks GitHub releases on launch and every six hours. **Settings → Updates** installs available app updates or board firmware. App installation restarts ESP Gauge; configuration edits are committed first. Unidentified CH340C bridges are offered for explicit initial installation and never flashed automatically. Existing 2.0 firmware connects normally but requires an update before using range calibration.

Signed release assets are produced by `.github/workflows/release.yml` on `v*` tags. All four desktop targets must build and test before publication. `firmware.json` is signed with the same updater key and lists each binary’s offset, size and SHA-256; `latest.json` maps desktop platforms to signed bundles. The public key is in Tauri configuration; the private key stays in the repository’s GitHub Actions secret. Never replace that key for an existing installation base.

Use the Linux **AppImage** for in-app updates. Debian/RPM packages use the system package manager. Platform publisher signing/notarization is separate from the mandatory update signatures.
