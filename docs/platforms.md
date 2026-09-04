# Platform and metric notes

- **Linux:** a StatusNotifier/AppIndicator or XEmbed tray host is needed. KDE
  supplies one; GNOME typically needs its AppIndicator extension. Without a tray
  the window remains usable and closing quits. Serial access usually requires
  membership of the distro's `dialout` or `uucp` group (log out/in afterward).
  Qt xcb needs the packages listed in `.github/workflows/build.yml`. Wayland and
  X11 tray integration still require desktop testing.
- **macOS:** the icon uses QSystemTrayIcon in the menu bar. Select `/dev/cu.*` for
  serial devices. A CH340 driver may be needed depending on OS/device support.
  The CI DMG is unsigned; signing/notarization credentials are not included.
- **Windows:** select the board's COM port. Install a trusted CH340 driver if the
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

Settings select one board at a time. Reconnection uses its saved port path, not a
unique USB identity (many CH340 boards lack unique serial numbers). If the OS
assigns a new port name, choose it again. No automatic firmware upload occurs.
