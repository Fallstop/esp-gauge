"""Native-session tray registration and hidden-window overhead check. No serial I/O.
Run with QT_QPA_PLATFORM=offscreen for a headless lifecycle/CPU check instead.
"""
import argparse
import json
import os
from pathlib import Path
import sys
import time
import psutil
from PySide6.QtCore import QObject, QEvent, QTimer
from PySide6.QtWidgets import QApplication, QSystemTrayIcon
from esp_gauge.model import Settings
from esp_gauge.app import Window, STYLE

parser = argparse.ArgumentParser()
parser.add_argument("--seconds", type=int, default=15)
parser.add_argument("--output", type=Path, default=Path("desktop-validation.json"))
parser.add_argument("--screenshot", type=Path)
args = parser.parse_args()
app = QApplication([])
app.setQuitOnLastWindowClosed(False)
app.setStyle("Fusion")
app.setStyleSheet(STYLE)
window = Window(Settings(auto_connect=False), Path("/tmp/esp-gauge-validation-settings.json"))
sample_count = [0]
window.worker.sample_ready.connect(lambda *_: sample_count.__setitem__(0, sample_count[0]+1))
process = psutil.Process()

class PaintCounter(QObject):
    count = 0
    def eventFilter(self, watched, event):
        if event.type() == QEvent.Type.Paint:
            self.count += 1
        return False

counter = PaintCounter()
for dial in window.dials:
    dial.installEventFilter(counter)
window.board.installEventFilter(counter)
window.show()
report = {"platform": sys.platform, "pid": os.getpid(), "sampling_ms": 1000}
start = {}

def cpu():
    times = process.cpu_times()
    return times.user + times.system

def begin():
    report["tray_available"] = QSystemTrayIcon.isSystemTrayAvailable()
    report["tray_visible"] = window.tray.isVisible()
    if sys.platform.startswith("linux"):
        from PySide6.QtDBus import QDBusInterface, QDBusConnection
        watcher = QDBusInterface("org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher", "org.kde.StatusNotifierWatcher", QDBusConnection.sessionBus())
        report["status_notifier_items"] = watcher.property("RegisteredStatusNotifierItems") if watcher.isValid() else []
    if args.screenshot:
        window.grab().save(str(args.screenshot))
    if report["tray_available"]:
        window.close()
    else:
        window.hide()
    app.processEvents()
    start.update(cpu=cpu(), time=time.monotonic(), paints=counter.count, samples=sample_count[0])
    report["hidden_timer_active"] = False
    QTimer.singleShot(args.seconds * 1000, finish)

def finish():
    elapsed = time.monotonic() - start["time"]
    report.update(hidden_seconds=round(elapsed, 3), cpu_percent_one_core=round((cpu()-start["cpu"])/elapsed*100, 3), rss_mib=round(process.memory_info().rss/1048576, 1), hidden_widget_paints=counter.count-start["paints"], samples_while_hidden=sample_count[0]-start["samples"])
    window.tray.contextMenu().actions()[0].trigger()
    app.processEvents()
    report["tray_open_action_restores_window"] = window.isVisible()
    window.tray.contextMenu().actions()[1].trigger()
    report["tray_pause_action_pauses_engine"] = window.worker.paused
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    window.quit()

QTimer.singleShot(2000, begin)
app.exec()
