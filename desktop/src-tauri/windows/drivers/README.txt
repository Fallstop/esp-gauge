ESP Gauge USB driver for Windows

CH341SER.EXE is WCH's original, unmodified CH340/CH341 driver installer.
It supports the CH340C USB bridge used by ESP Gauge (USB VID 1A86, PID 7523).

Installation
------------
Run CH341SER.EXE, approve the Windows administrator prompt, and choose
INSTALL in the WCH window. Follow its result/restart instructions, then
reconnect the board if necessary. No driver download is needed.

ESP Gauge checks Windows for a compatible driver, even with the board
unplugged. If none is found (or the check fails), interactive setup.exe
offers this step. You can also choose "USB driver setup" on ESP Gauge's
Connect board page. It remains available there for repair if needed.
Setup, app updates, and app startup remove the old Start menu driver
shortcut from ESP Gauge 2.2.4. No new driver shortcut is created.
App updates and silent/passive setup skip the interactive driver step.
MSI deployments include this file and CH341SER.EXE in the app's drivers
folder; an administrator must run the driver installer separately.

Uninstalling ESP Gauge leaves the installed Windows driver in place,
because other CH340/CH341 devices may use it.

Source and attribution
----------------------
Vendor: Nanjing Qinheng Microelectronics Co., Ltd. (WCH)
Copyright (C) WCH 2001-2026. This driver is not covered by ESP Gauge's license.
Package version: 4.0, published 2026-06-24
INF DriverVer: 02/11/2026, 4.0.2026.02
Downloaded: 2026-09-06
Product page: https://www.wch-ic.com/downloads/CH341SER_EXE.html
Download: https://www.wch-ic.com/download/file?id=65

WCH describes this package as "Used to distribute to user with the product."
The executable retains WCH's Authenticode signature and the original
driver catalog. ESP Gauge's build verifies the pinned SHA-256; Windows
release CI also verifies the executable's Authenticode signature.

SHA-256:
458c37bdafbe4ce3cd0baf728c232b4b765b36d7463956e9b94cdf099212cad1

Maintainers: replace only with an original WCH package. Verify its vendor
signature and hardware support, then update this notice and the checksum
in tools/verify_windows_driver.mjs together. Test on Windows with a board
before releasing a new driver version.
