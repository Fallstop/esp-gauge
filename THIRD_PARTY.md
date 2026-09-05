# Third-party notices

The desktop application uses Tauri (MIT/Apache-2.0), Svelte (MIT), serialport (MPL-2.0), sysinfo (MIT), chrono (MIT/Apache-2.0), and battery (Apache-2.0/MIT). Dependency sources and license metadata are available from crates.io and npm using the exact lockfile versions. WebKit/WebView2 is provided by the operating system.

Manrope and IBM Plex Mono are distributed under the SIL Open Font License 1.1. Their full license notices are included in `desktop/public/licenses` and in the packaged interface.

Firmware uses the Espressif Arduino core (LGPL-2.1), ArduinoJson (MIT), and Adafruit NeoPixel (LGPL-3.0). The exact PlatformIO dependencies are pinned in `firmware/platformio.ini`; dependency source, notices, and link objects are available in the normal PlatformIO build tree to permit rebuilding and relinking.

Board renders are supplied by the project owner. The original line and shadow passes are preserved.

Native updating and discovery also use espflash (MIT/Apache-2.0), reqwest (MIT/Apache-2.0), rusqlite (MIT), SQLite (public domain), fs2 (MIT/Apache-2.0), dirs (MIT/Apache-2.0), sha2 (MIT/Apache-2.0), base64 (MIT/Apache-2.0), semver (MIT/Apache-2.0), and minisign-verify (MIT). These are pinned in Cargo.lock with transitive dependency license metadata available from their published sources.

Super Tracker's public data is credited to [Super Tracker and its source retailers](https://supertracker.nz), under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). It is a nowcast, not an official statistic.

Linux AppImages include the [AppImageKit runtime](https://github.com/AppImage/AppImageKit), distributed under the upstream MIT license and bundled component notices. CI pins the runtime download by SHA-256; runtime source and dependency licensing remain available from that upstream project.
