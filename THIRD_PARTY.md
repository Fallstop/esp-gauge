# Third-party software

The desktop uses PySide6 Essentials and Shiboken6 (Qt for Python), psutil, and
pyserial. Qt for Python is available under LGPLv3/GPL/commercial terms; psutil
uses BSD-3-Clause and pyserial BSD. PyInstaller is a build-time tool with its
bootloader exception. See the distributions' full license texts bundled by
`tools/licenses.py` in CI artifacts; Qt component licenses can differ.

The portable build retains dynamically loaded libraries. Preserve their license
notices and the ability to replace LGPL libraries when redistributing. Qt source
and build information: https://code.qt.io/ and https://doc.qt.io/qtforpython-6/.
ESP32 firmware uses the Arduino-ESP32 and ESP-IDF components distributed through
the pinned PlatformIO platform; their upstream licenses apply independently.

Stack references: native tray support and deployment are documented at
https://doc.qt.io/qtforpython-6/PySide6/QtWidgets/QSystemTrayIcon.html and
https://doc.qt.io/qtforpython-6/deployment/index.html. Metric semantics are at
https://psutil.readthedocs.io/.
