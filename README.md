# ESP Gauge

A quiet desktop activity monitor for the six-output ESP32 gauge board. Native
Qt Widgets on Linux, macOS and Windows, with a tray/menu-bar icon. No Electron,
webview, browser runtime, or Wi-Fi requirement.

One CPU gauge is enabled by default. Add up to five more and assign CPU, memory,
network upload/download, or disk read/write. Sample timing is independent of
firmware needle smoothing. Closing the window keeps monitoring in the tray;
Pause rests the needles, and Quit closes the app. On desktops without a tray,
closing quits so the app cannot become inaccessible.

## Run from source

Python 3.11–3.13, with 3.12 used by CI:

```sh
python -m venv .venv
# Linux / macOS
. .venv/bin/activate
# Windows PowerShell instead: .venv\Scripts\Activate.ps1
python -m pip install -e ".[dev]"
esp-gauge
```

GitHub Actions builds a portable Linux x86-64 tarball, Windows x86-64 ZIP, macOS
Apple Silicon DMG and ESP32 binaries on every push/PR. Download the artifact for
your platform, unpack it, and launch ESP-Gauge (on macOS, copy ESP-Gauge.app to
Applications). These development packages are unsigned and not notarized.
macOS Intel users can build from source on their machine; CI currently packages
Apple Silicon only. Linux builds target glibc 2.35 or newer. See
[platform notes](docs/platforms.md) for tray/USB prerequisites.

## First connection

1. Build the firmware with `pio run -d firmware` (PlatformIO 6.1.18). This command
   does **not** flash. Upload separately with PlatformIO when you intend to replace
   the board firmware. Upload is always a separate, intentional action.
2. Attach your gauge to the correct board output with power disconnected. Confirm
   its electrical rating and any required series resistance for the board.
3. Plug the board into USB. The app automatically finds its CH340 UART and
   verifies gauge firmware before applying your saved settings. There is no port
   picker or auto-connect switch. It checks again every five seconds when
   disconnected, including when the OS assigns a different port name.
4. Enable only physically connected outputs. Open **Calibration / response**.
   The initial upper limit is 20% duty; the firmware enforces an absolute 88%
   board ceiling inherited from the bench code. Neither number guarantees a safe
   current for every meter. Start lower when the gauge rating requires it.
5. Set endpoints, save the dialog, then **Save & apply**. Reopen calibration and
   use the five-second 0%, 50%, 100% tests, increasing the saved upper endpoint
   gradually until the scale matches. The tests use **saved** settings, and
   automatically return to the assigned metric. A reversed gauge can use Reverse.
6. Pick a resting position and needle response. 500 ms gives a damped response;
   0 ms applies samples immediately. Choose a timeout of at least three sampling
   intervals. Rate metrics use the configurable MiB/s full-scale value.

Preview displays normalized target position, not sensed needle position. The
board has no needle feedback. Configuration is saved atomically in the OS app
config directory (`ESP Gauge/ESP Gauge/settings.json` on Linux), and reapplied
on reconnect. Invalid settings are reported without silently replacing the file.
Only one application instance can own the settings/serial connection.

## Board and firmware

The existing schematic identifies an ESP32-WROOM-32E-N4 and CH340C USB bridge.
The `PWM1`–`PWM6` net names and [pin map](firmware/pins.md) correspond to GPIO
16, 17, 18, 19, 21 and 22. GPIO 23 is the NeoPixel; this release leaves it unused.
The outputs sometimes called “PDM” are driven here using the board's existing
5 kHz LEDC PWM approach. No PCB files were changed.

The interactive top view uses static white linework derived from the supplied
3D model. Click a connector to select its output settings. See
[artwork provenance and mapping](docs/board-artwork.md).

The pre-existing uncommitted sweep firmware is preserved verbatim at
[firmware/examples/bench/main.cpp](firmware/examples/bench/main.cpp). It is not
compiled by the default build and drives all outputs; use it only intentionally.
The new default never sweeps at boot. See [protocol](docs/protocol.md) for commands,
limits, failsafe behavior and extending the transport for club projects.

## Development and validation

```sh
python -m pytest -q
g++ -std=c++11 -Wall -Wextra -Werror -Ifirmware/include tests/core_test.cpp -o core-test
./core-test
pio run -d firmware
python tools/benchmark.py --seconds 30
python tools/benchmark.py --seconds 30 --visible
python -m PyInstaller --noconfirm --clean --windowed --name ESP-Gauge --paths desktop --collect-data esp_gauge desktop/launch.py
```

The tests include C++ framing/calibration/smoothing/timeout/wraparound checks,
real desktop commands against the C++ parser, reconnects after lost ACKs,
settings/rate validation and hidden-window rendering behavior. See
[verification record](docs/verification.md) for measured results and remaining
hardware/platform checks, and [third-party notices](THIRD_PARTY.md).
