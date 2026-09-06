# Verification record — 5–6 September 2026

## 2.3.0 source and update polish

Local verification on 2026-09-06: Svelte/TypeScript and production UI build passed with no warnings; 19 Rust tests and strict Clippy passed; the firmware engine suite (including curved endpoints, monotonicity, reversal and unavailable outputs), three packaging tests and PlatformIO firmware build passed. Five Playwright checks cover category/search navigation, hidden battery, separate disk assignments, product/store locking, update progress through disconnection, small-window layout and the old-firmware curve guard. Browser screenshots were inspected at 880×580.

Read-only native sampling on this Apple Silicon Mac returned GPU usage, CPU temperature, GPU temperature and the system volume setting. Playback capture was not exercised against live audio. Windows/Linux hardware readings and a physical firmware flash were not exercised locally; the cross-platform workflows build and test their native adapters. Super Tracker production DNS was unavailable: its product/store contract was checked against the local server's current OpenAPI definition, with parser and browser fixtures covering price freshness and strict store selection.


The first sections record the original 2.0 implementation. The continuation sections below record the released updates and supersede the original installation and calibration state.

## Windows driver bundling and 2.2.4 release

The NSIS package now includes the original WCH CH341SER.EXE 4.0, offers interactive driver setup, and creates a Start menu shortcut for retrying it. Updates and silent/passive setup skip the driver prompt. MSI resources include the same executable for separate administrator installation. The driver package contains the board's `1a86:7523` hardware ID and WCH's embedded publisher certificate.

On macOS, the frontend build passed with zero Svelte diagnostics. Tauri generated an NSIS installer with the new hooks and resources using the existing cross-compiled Windows executable; this was a packaging check, not a fresh Windows application build. Extracting its driver reproduced the vendor file byte for byte. The packaging checksum check accepted the original and rejected modified and missing files.

[Release 2.2.4](https://github.com/Fallstop/esp-gauge/releases/tag/v2.2.4) was published after the [full release workflow](https://github.com/Fallstop/esp-gauge/actions/runs/34002890975) passed for firmware, Apple Silicon macOS, Windows x64 and Linux x64. Windows verified WCH's Authenticode signature and publisher and built both NSIS and MSI installers. Desktop tests and Clippy passed on all three platforms, as did the Linux packaged AppImage check and the three release packaging tests. The main commit is `b9b00bd`; no pull request was used.

All 20 published assets were downloaded and checked against GitHub's sizes and SHA-256 digests. All seven update/firmware signatures were verified with the app's existing public key. OS-labelled download names, release-note links, updater URLs and signatures, and firmware segment checksums matched. The published Windows setup contained the exact pinned WCH executable. Windows installer UI, UAC acceptance/cancellation, and clean-machine driver/USB operation still require physical Windows validation.

## Original implementation: physical board

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

The published 2.1.0 macOS package was installed from GitHub. Its native updater downloaded the signed firmware, reflashed and verified the board, and reconnected with PWM6's 82.5% calibration intact. Initial installation was tested by erasing only the application header sector, leaving the configuration partition intact: the UI identified an unknown CH340C, installed the release, then recovered the original device configuration. The normal disconnected view was visually checked with its receded sidebars and dimmed, centred PCB.

## 2.2 monitor sources

Twelve Rust tests pass, including quota-window selection, missing versus zero readings, stale public data, incompatible databases and active versus abandoned Codex writer locks. C++ tests cover waveform endpoints and phase offsets. TypeScript/Svelte and Clippy checks pass.

The actual PCB ran all four new waveform generators after host release. PWM6 made a complete sine sweep limited to 1% electrical duty, while all other output registers stayed zero. Invalid periods, pause/rest and reset persistence passed. Calibration acknowledgement was 11.08 ms median, 11.64 ms p95 in the first waveform test. Original configuration was restored after testing.

Native macOS controls were exercised for discovered Codex working agents, real weekly quota readings, sine period/phase edits and Claude's unavailable-login explanation. The local codeslop environment descriptor and read-only database adapters returned live data. Super Tracker's development endpoint returned its documented index schema; the release uses supertracker.nz, whose DNS was not yet live during these checks. Claude's stored OAuth credential was expired, so no live Claude subscription percentage was claimed.

The published Linux 2.1.0 interface was exercised through a temporary Xvfb/noVNC display on ssh-kde, including disconnected rendering and release discovery. Its newer static AppImage runtime conflicted with the desktop's AppImageLauncher 2 installation. A checksum-pinned dynamically linked AppImageKit runtime with gzip compression was then verified on that machine; 2.2.2 packages incorporate this fix. The temporary display is for validation and is removed before handoff.

Linux returned real Claude five-hour and weekly subscription readings using the existing login. Codex, OpenCode and codeslop sources were also discovered. A minimal macOS GUI search path still found the installed CLIs and returned Codex quota data. Provider tests distinguish a held writer lock from stale files on both Unix and Windows.

The final waveform-engine hardware run passed with calibration acknowledgement of 11.32 ms median, 12.05 ms p95 and 12.38 ms maximum. PWM6's original CPU source and 0–82.5% calibration were restored; the other five ports remained disabled. The local log is `artifacts/hardware-check-v2.2.0-final.log`.

## Published 2.2.2 and installed update tests

[Release 2.2.2](https://github.com/Fallstop/esp-gauge/releases/tag/v2.2.2) was published after the [complete release workflow](https://github.com/Fallstop/esp-gauge/actions/runs/33995384524) passed: ESP32 firmware, Apple Silicon macOS, Intel macOS, Windows x64 and Linux x64. Rust tests and Clippy passed on all desktop runners; Windows runs 11 tests and Unix runs 12, including the pseudoterminal check. The Linux job also executed the final packaged AppImage through its extraction fallback. The AppImage's gzip filesystem is made with the system compressor because the upstream bundled compressor only supports zstd; its mandatory update signature is generated after this conversion.

Both installed 2.1.0 applications discovered the public 2.2.2 release, downloaded their signed update through **Settings → Updates**, installed it and restarted. Their native interfaces then displayed 2.2.2 and reported that they were up to date. The Linux starting application was the extracted, published 2.1.0 bundle, with its original AppImage as the updater target; the restart ran directly from the newly installed 2.2.2 AppImage.

The updated Mac application installed firmware 2.2.2 from GitHub on the connected board, verified it and reconnected. The UI showed firmware 2.2.2 and PWM6's unchanged CPU source, 0–82.5% calibration and 0.5-second response. The other five channels stayed unassigned. The connected PCB view and all new source groups were checked in the released native application.

The installed Linux AppImage's read-only diagnosis returned version 2.2.2 and 21 discovered sources, with live Claude five-hour/weekly readings and Codex, OpenCode and codeslop/T3 local readings. It is running in the normal KDE session from `~/.local/lib/esp-gauge/ESP-Gauge.AppImage`, launched by `~/.local/bin/esp-gauge`. The preceding native executable is retained as `~/.local/lib/esp-gauge/esp-gauge-before-appimage`. The current Git checkout is `~/Documents/projects/esp-gauge-current`.

The temporary Xvfb display, VNC/proxy processes, SSH forwarding and test browser tab were stopped after verification. Mac installation remains `~/Applications/ESP Gauge.app`. Super Tracker's production endpoint was still unavailable, and the Mac's Claude login was expired; these sources correctly remain unavailable. Publisher signing/notarization and native Windows GUI/USB execution remain outside the validation completed here.

## Header interface refinement, 2.2.3

The six target paths were checked against the labelled `PWM-Targets.svg` source. Native macOS checks exercised every physical header, the shared PWM1/PWM2 boundary, clicks on bare board, and keyboard selection. Hover and selection highlights follow the connector outlines. A geometric check confirmed all six leader routes avoid every neighbouring header; the closest clearance is 12.6 scene units between the PWM2 trace and PWM3.

The supplied `Header.svg` linework is preserved over new face fills. The vector asset was reviewed enlarged and at its actual inspector size, then checked in the installed native Mac application. Svelte diagnostics, production builds and app integrity verification passed. The board reconnected normally, with PWM6 still assigned to CPU usage at 0–82.5% calibration and a 0.5-second response. This update changes the desktop interface; firmware only receives the matching release version.

## Unreleased Windows driver and layout refinement

The Connect board page exposes the bundled WCH setup on Windows. A native driver-store query matches the board's exact hardware ID, including when unplugged; detection failure leaves setup available. App startup and the NSIS install/update/uninstall hooks remove the legacy driver shortcut. Interactive setup skips its driver offer when a compatible package is found, and unattended updates still skip the vendor installer.

Local Rust tests and Clippy pass, including exact hardware-ID matching and window bounds for a 1080p work area at 100%, 125%, 150%, and 200% scaling. Windows code and test targets pass cross-compilation checks. The NSIS hooks compile and the bundled WCH checksum passes. This packaging-only check used the existing local Windows executable, so its installer is not a release candidate. Driver-store detection, administrator prompts and actual driver installation still require native Windows execution.

Browser checks with the actual components and temporary Windows/connected-board fixtures cover Connect board, gauge controls, the source picker, calibration, Wi-Fi and Updates. The default 1160 × 720 and compact 880 × 580 layouts have no document overflow; longer inspector content scrolls independently. Shared text colours and PCB linework have increased contrast. Windows startup now bounds the entire window, including decorations, to the monitor's scaled work area.
