# Verification record

Verified on Linux/KDE, 5 September 2026 (NZ time). No board was flashed or driven.

| Check | Result |
|---|---|
| Desktop tests | 18 passing: settings, metrics, serial identity/ACK, reconnect, hidden paints, board assets/mapping, and desktop commands against real C++ parser |
| C++ host test | Passed with `-Wall -Wextra -Werror`; framing overflow, invalid fields, atomic positions, safe limits, smoothing, timeout, unsigned clock rollover |
| ESP32 build | PlatformIO espressif32 6.10.0 / Arduino 2.0.17; 21,876 bytes RAM, 275,337 bytes flash |
| Linux packaging | PyInstaller succeeds; native library bundle ~151 MiB before archive compression |
| Initial remote CI | Run 33929860304 passed Linux, Windows, macOS and firmware jobs |
| Native KDE final board-view UI | Tray detected and visible, menu restores window, Pause changes worker state |
| Hidden native UI | Zero paint events across all dials and board view over 20.017 s while 20 metric samples continued |
| Native hidden overhead | 0.200% of one logical CPU core; 86.8 MiB RSS, 1-second sample interval, no USB attached |
| Earlier offscreen baseline | Hidden 0.134% one-core / 65.0 MiB; visible 0.504% / 71.1 MiB, ~30 s, before board asset integration |

CPU is process CPU-time delta divided by wall-time, multiplied by 100. These are
short measurements on this machine, not universal bounds. Startup/import cost is
excluded. The native run includes actual Qt event handling, tray registration,
metrics, and the final static board widget, but excludes real serial traffic.
`tools/verify_desktop.py` reproduces the native test; `tools/benchmark.py` offers
an offscreen comparison. The native test approach was adapted from the preserved
duplicate task's verification script, but the measurements above were rerun on
this implementation.

The final CI workflow additionally launches every frozen desktop executable with
`--smoke-test`, checking bundled artwork and clean exit without USB. Consult the
latest checks on [draft PR #1](https://github.com/Fallstop/esp-gauge/pull/1) for the
final commit's results.

Still requires physical/user platform verification: real gauge safe current and
endpoints, polarity/needle dynamics, USB cable removal and driver-specific reset
behavior, macOS and Windows native tray interactions, and GNOME/Wayland tray
behavior. Remote compile/package/smoke tests do not establish those hardware and
interactive behaviors. Firmware is pinned/tested on Arduino 2.0.17; the 3.x LEDC
adapter branch is included for reuse but is not exercised by the pinned build.

Board geometry is derived from the supplied OBJ, with source hash and connector
bounds in the asset metadata. Electrical connector mapping is from the existing
schematic, not assumed connector numbering. See `board-artwork.md`.
