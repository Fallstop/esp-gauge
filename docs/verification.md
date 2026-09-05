# Verification record — 5–6 September 2026

## Physical board

ESP32-D0WD-V3 revision 3.1, CH340C `1a86:7523`, protocol-2 device `107872DF948C`, connected to macOS over USB. GPIO mapping checked against the existing pin map. Only PWM6 has a gauge. A full 4 MB flash backup was taken before updating firmware and remains in the local, ignored `artifacts` directory.

Final firmware was flashed at 230400 baud and verified by esptool. Actual hardware checks passed:

- Strict identity handshake, six-channel configuration roundtrip, opaque metadata preservation.
- Invalid port, over-limit duty, extreme UTC offset and future config version rejection.
- PWM6 at 1% raw duty produces 40/4095; the other five output registers stay zero.
- Calibration expires and rests; host readings drive the configured output and expire after three seconds; unavailable readings rest.
- All four clock modes run after host release. An unknown future clock source remains unavailable.
- Wi-Fi scan found 75 networks; BLE scan found 11 advertising addresses. Chip temperature returned 60.6°C; this is an experimental die reading.
- Oversized frames and malformed JSON recover at the next newline.
- Calibration acknowledgement: median 10.97 ms, p95 11.34 ms, maximum 12.44 ms over 40 commands.
- A real EN reset retained the entire configuration and opaque metadata; disabled outputs stayed zero.

These checks validate commanded output registers, not measured pin voltage or physical needle position. A 1% calibration was used only during UI testing. It was removed before handoff. All six ports are disabled until the user completes calibration against an actual gauge.

Final firmware uses 65,392 bytes of static RAM (20.0%) and 1,522,125 bytes of the 3 MB application partition (48.4%). The full hardware test log is in the ignored local `artifacts/hardware-check.log`.

## Desktop

The native macOS application was exercised with computer use. Confirmed startup discovery after CH340-induced reset, PWM6 selection, live zero-to-1% calibration, completion, CPU/memory/clock/Bluetooth source changes, live BLE counts, pause/resume, reverse direction and optional login startup. Login startup was tested on and returned to off. Closing the window during calibration and reopening the same process returned to the unassigned gauge, with calibration ended.

A normal keyboard edit immediately followed by Command-Q was verified independently by reading its exact name back from device NVS. This caught and corrected AppKit’s predefined Quit bypassing Tauri’s exit callback; the application uses a custom Quit action to commit pending edits and release the board.

Native-window screenshot review corrected compositing, duplicate macOS serial aliases, source selection, switch styling and compact-height layout. The final UI uses the supplied Blender assets and packaged fonts.

Linux x86-64 was built, installed and run on the user’s KDE/Ubuntu 25.10 desktop. Native WebKitGTK UI pixels and live system metrics were verified through the app’s own XWayland window. A screenshot caught an early direct-Cargo build accidentally using the development URL; the packaged releases now embed the frontend through Tauri. The existing POC checkout on Linux was left intact; the new source is in `~/Documents/projects/esp-gauge-studio`.

Windows x86-64 MSVC compilation and NSIS installer generation passed on macOS using cargo-xwin and the Windows SDK. The portable executable and installer are included in local artifacts. No Windows runtime or attached Windows USB hardware was available for native execution in this session.

## Automated checks

- Portable C++ engine tests: boot, isolation, bounds, watchdogs, unsigned time rollover, smoothing, unavailable readings, reverse and clock units.
- Five Rust tests: config validation, unknown metadata, normalization, strict identity and real pseudoterminal framing with fragmentation, unrelated IDs and boot noise.
- Svelte/TypeScript validation: zero errors and warnings.
- Rust Clippy across all targets: no warnings.
- Firmware and desktop dependency lockfiles/pins included. A three-platform build workflow is configured; it has not been run on GitHub or published.

## Installed outputs and remaining validation

- Mac: `~/Applications/ESP Gauge.app`.
- Linux: `~/.local/bin/esp-gauge`, launcher `~/.local/share/applications/esp-gauge.desktop`.
- Local packages: macOS ARM64 DMG, Linux amd64 deb, Windows x64 NSIS installer/portable zip, firmware zip in `artifacts`.

Wi-Fi credential provisioning and forgetting were exercised without logging the password. The existing OS-saved network was not visible in the board’s returned 2.4 GHz scan results and did not connect. That trial credential was removed. Real network association and cold-start NTP recovery therefore remain unverified. Clock continuation while powered and host time synchronization did pass.

Physical full-scale calibration still requires observing the attached needle. Windows native execution, platform signing/notarization and a broader OS/hardware compatibility matrix remain release validation work. Windows/Linux packages are unsigned. The Mac bundle has an ad-hoc integrity signature, with no Developer ID signature or notarization.

## 2.1.0 continuation — 6 September 2026

The connected board retained its user-configured PWM6 full-scale setting of 82.5% through the refactored firmware upload. All hardware checks passed, including 0.5–1% range mapping, unavailable-source rest, invalid ranges, clocks, radio sensing, reset persistence and fragmented serial frames. Live calibration acknowledgement measured 10.95 ms median and 11.42 ms p95. Original configuration was restored after the tests.

Native macOS computer-use checks verified the embedded title bar, PWM6 range editor, live 1% upper and 0.5% lower endpoints, cancellation and the new Updates settings. Signed firmware-manifest tests verify the valid signature and reject a changed byte; fixed flash-region validation rejects writes overlapping NVS. Release installation and automatic-update end-to-end checks follow publication.
