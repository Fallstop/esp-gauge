# Platform and metric notes

- **Linux:** a StatusNotifier/AppIndicator or XEmbed tray host is needed. KDE
  supplies one; GNOME typically needs its AppIndicator extension. Without a tray
  the window remains usable and closing quits. Serial access usually requires
  membership of the distro's `dialout` or `uucp` group (log out/in afterward).
  Qt xcb needs the packages listed in `.github/workflows/build.yml`. Wayland and
  X11 tray integration still require desktop testing.
- **macOS:** the icon uses QSystemTrayIcon in the menu bar. Discovery prefers `/dev/cu.*` callout aliases when available. A CH340 driver may be needed depending on OS/device support.
  The CI DMG is unsigned; signing/notarization credentials are not included.
- **Windows:** COM ports are identified automatically by USB metadata. Install a trusted CH340 driver if the
  device is absent in Device Manager. Extract the entire ZIP, retaining the
  adjacent `_internal` folder. The tray icon may appear in the overflow area.

Closing/hiding stops gauge paints, not metrics. The worker sleeps between samples
(default 1 s, minimum 250 ms) and reconnect attempts (5 s), and serial operations
have bounded read/write timeouts off the GUI thread. There is no animation timer,
per-process enumeration, disk scan, or network request. The tray icon is static.

CPU is whole-system utilization, primed before its first display. Memory is
psutil's physical-memory used percentage (OS definitions differ). Network rates
aggregate interfaces; VPN/virtual interfaces may double-count traffic. Disk rates
aggregate OS disk-I/O counters and can include virtual-device double counting.
These are throughput, not a hardware saturation/utilization percentage. Set a
useful full-scale MiB/s per gauge. Missing/unsupported counters show “waiting” and
send the gauge to its configured rest. Windows disk counters may require enabling
OS performance counters. Counter resets clamp to zero, and the first rate sample
uses a primed baseline.

The app always discovers one board automatically, matching USB VID:PID
`1A86:7523` through pyserial's OS backends. Port names and human-readable device
descriptions are not used as identity, so Linux ttyUSB, Windows COM and macOS
callout paths follow the same code. macOS tty/callout duplicates are collapsed.
CH340C shares this USB identity with other CH340 variants; the app sends only `H`
until the exact gauge firmware response is verified, then applies configuration.
Unknown USB metadata is not guessed from a description: if no device appears,
check the CH340 driver and permissions. Linux serial group permissions still apply.

No port is required in settings. Legacy `port` and `auto_connect` fields are read
for compatibility but ignored, and cleared/reset on save. While disconnected,
discovery re-enumerates every five seconds and probes one matching device per
attempt, rotating past busy/non-gauge CH340 devices. A connected board is kept
without further scans; unplugging or a failed ACK resumes discovery, including
renamed ports. With several gauge boards attached, the first responding candidate
is used (one board at a time). Pause intentionally rests/disconnects and suspends
discovery until resumed. The tray's Find board now triggers an immediate retry.
No automatic firmware upload occurs.

Metadata API: https://pyserial.readthedocs.io/en/latest/tools.html#serial.tools.list_ports
USB identity: https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ch341.c
