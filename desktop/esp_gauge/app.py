"""Native Qt Widgets application: paints only on samples while visible."""
import copy
import math
import sys
from pathlib import Path
from PySide6.QtCore import Qt, QRectF, QPointF, QStandardPaths, QLockFile
from PySide6.QtGui import QColor, QPainter, QPen, QFont, QIcon, QPixmap, QAction
from PySide6.QtWidgets import (QApplication, QWidget, QVBoxLayout, QHBoxLayout, QLabel,
    QPushButton, QComboBox, QCheckBox, QSpinBox, QDoubleSpinBox, QGroupBox, QFormLayout,
    QScrollArea, QGridLayout, QSystemTrayIcon, QMenu, QMessageBox, QDialog, QDialogButtonBox)
from .model import Settings, METRICS
from .connection import ports
from .worker import Worker

STYLE = """
QWidget { background: #131d26; color: #e5eef2; font-size: 13px; }
QLabel#eyebrow { color: #5cd8bc; font-weight: 700; letter-spacing: 2px; }
QLabel#title { font-size: 30px; font-weight: 700; }
QLabel#muted { color: #9aaebc; }
QGroupBox { border: 1px solid #344551; border-radius: 12px; margin-top: 12px; padding: 16px 10px 10px; font-weight: 600; }
QGroupBox::title { subcontrol-origin: margin; left: 14px; padding: 0 6px; }
QPushButton { background: #293d4a; border: 1px solid #415866; border-radius: 6px; padding: 9px 16px; }
QPushButton:hover { background: #355364; }
QPushButton#primary { background: #5cd8bc; color: #102c29; font-weight: 700; }
QComboBox, QSpinBox, QDoubleSpinBox { background: #1d2b36; border: 1px solid #415866; border-radius: 5px; padding: 6px; min-height: 20px; }
QCheckBox { spacing: 8px; }
QScrollArea { border: none; }
"""

class Dial(QWidget):
    def __init__(self, number):
        super().__init__()
        self.number, self.value, self.label, self.active = number, 0, "CPU usage", True
        self.setMinimumSize(220, 170)

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.scale(self.width() / 260, self.height() / 180)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.setBrush(QColor("#1d2b36"))
        painter.drawRoundedRect(QRectF(2, 2, 256, 176), 12, 12)
        center = QPointF(130, 126)
        for tick in range(21):
            angle = math.radians(210 - tick * 12)
            a, b = 75, 66 if tick % 5 == 0 else 71
            painter.setPen(QPen(QColor("#89a0ad"), 2 if tick % 5 == 0 else 1))
            painter.drawLine(QPointF(center.x()+a*math.cos(angle), center.y()-a*math.sin(angle)),
                             QPointF(center.x()+b*math.cos(angle), center.y()-b*math.sin(angle)))
        angle = math.radians(210 - self.value * .24)
        painter.setPen(QPen(QColor("#5cd8bc" if self.active else "#536472"), 3))
        painter.drawLine(center, QPointF(130+61*math.cos(angle), 126-61*math.sin(angle)))
        painter.setBrush(QColor("#5cd8bc" if self.active else "#536472"))
        painter.drawEllipse(center, 5, 5)
        painter.setPen(QColor("#e5eef2"))
        painter.setFont(QFont("sans-serif", 10))
        painter.drawText(QRectF(12, 12, 236, 24), Qt.AlignmentFlag.AlignCenter, f"{self.number:02d}  /  {self.label}")
        painter.setFont(QFont("sans-serif", 12, QFont.Weight.Bold))
        painter.drawText(QRectF(12, 145, 236, 26), Qt.AlignmentFlag.AlignCenter, f"{self.value/10:.0f}% of scale" if self.active else "Output off")


def icon():
    pixmap = QPixmap(64, 64)
    pixmap.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pixmap)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setBrush(QColor("#173a39"))
    painter.setPen(QPen(QColor("#5cd8bc"), 4))
    painter.drawEllipse(5, 5, 54, 54)
    painter.drawArc(14, 14, 36, 36, 0, 180*16)
    painter.drawLine(32, 38, 44, 21)
    painter.drawEllipse(29, 35, 6, 6)
    painter.end()
    return QIcon(pixmap)


def spin(low, high, value, suffix=""):
    box = QSpinBox()
    box.setRange(low, high)
    box.setValue(value)
    box.setSuffix(suffix)
    return box

class Calibration(QDialog):
    def __init__(self, output, number, parent):
        super().__init__(parent)
        self.setWindowTitle(f"Output {number} · calibration")
        layout = QVBoxLayout(self)
        note = QLabel("Start with a low upper endpoint. Increase gradually for your gauge.\n88% is the board ceiling, not a guarantee that a gauge can tolerate it.\nChanges take effect only after Save & apply in the main window.")
        note.setWordWrap(True)
        layout.addWidget(note)
        form = QFormLayout()
        self.low = spin(0, 880, output.low, " ‰ duty")
        self.high = spin(0, 880, output.high, " ‰ duty")
        self.rest = spin(0, 1000, output.rest, " ‰ scale")
        self.response = spin(0, 5000, output.response_ms, " ms")
        self.reverse = QCheckBox("Reverse needle direction")
        self.reverse.setChecked(output.reverse)
        self.scale = QDoubleSpinBox()
        self.scale.setRange(.01, 100000)
        self.scale.setValue(output.full_scale)
        self.scale.setSuffix(" MiB/s")
        for name, widget in (("Minimum duty", self.low), ("Maximum duty", self.high), ("Rest on connection loss", self.rest), ("Needle response (time constant)", self.response), ("Rate at full scale", self.scale), ("Direction", self.reverse)):
            form.addRow(name, widget)
        layout.addLayout(form)
        test_note = QLabel("Test the saved calibration for 5 seconds, then resume the metric.\nSave changes in the main window before testing new endpoints.")
        test_note.setWordWrap(True)
        layout.addWidget(test_note)
        tests = QHBoxLayout()
        for label, value in (("Test 0%", 0), ("Test 50%", 500), ("Test 100%", 1000)):
            button = QPushButton(label)
            button.clicked.connect(lambda checked=False, v=value: parent.worker.test_position(number-1, v))
            tests.addWidget(button)
        layout.addLayout(tests)
        buttons = QDialogButtonBox(QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def apply(self, output):
        output.low, output.high = self.low.value(), self.high.value()
        output.rest, output.response_ms = self.rest.value(), self.response.value()
        output.reverse, output.full_scale = self.reverse.isChecked(), self.scale.value()
        output.validate()

class Window(QWidget):
    def __init__(self, settings, path, load_error=""):
        super().__init__()
        self.settings, self.draft, self.path = settings, copy.deepcopy(settings), path
        self.values, self.metrics = [0]*6, {}
        self.setWindowTitle("ESP Gauge")
        self.setWindowIcon(icon())
        self.resize(930, 760)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(26, 22, 26, 22)
        eyebrow = QLabel("ESP GAUGE  /  DESKTOP INSTRUMENTS")
        eyebrow.setObjectName("eyebrow")
        layout.addWidget(eyebrow)
        title = QLabel("Your computer. In motion.")
        title.setObjectName("title")
        layout.addWidget(title)
        self.status_label = QLabel(load_error or "Choose a serial port to connect your board.")
        self.status_label.setWordWrap(True)
        self.status_label.setObjectName("muted")
        layout.addWidget(self.status_label)
        link = QHBoxLayout()
        self.port = QComboBox()
        self.port.setMinimumWidth(250)
        self.refresh_ports()
        refresh = QPushButton("Refresh ports")
        refresh.clicked.connect(self.refresh_ports)
        self.auto = QCheckBox("Connect automatically")
        self.auto.setChecked(settings.auto_connect)
        link.addWidget(self.port, 1)
        link.addWidget(refresh)
        link.addWidget(self.auto)
        layout.addLayout(link)
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        content = QWidget()
        grid = QGridLayout(content)
        self.dials, self.enables, self.assignments = [], [], []
        for i, output in enumerate(settings.outputs):
            card = QGroupBox(f"OUTPUT {i+1}  ·  GPIO {[16,17,18,19,21,22][i]}")
            stack = QVBoxLayout(card)
            dial = Dial(i+1)
            stack.addWidget(dial)
            enabled = QCheckBox("Connected gauge")
            enabled.setChecked(output.enabled)
            enabled.toggled.connect(self.dirty)
            stack.addWidget(enabled)
            assignment = QComboBox()
            for key, name in METRICS.items():
                assignment.addItem(name, key)
            assignment.setCurrentIndex(list(METRICS).index(output.metric))
            assignment.currentIndexChanged.connect(self.dirty)
            stack.addWidget(assignment)
            calibration = QPushButton("Calibration / response")
            calibration.clicked.connect(lambda checked=False, index=i: self.calibrate(index))
            stack.addWidget(calibration)
            grid.addWidget(card, i//3, i%3)
            self.dials.append(dial)
            self.enables.append(enabled)
            self.assignments.append(assignment)
        scroll.setWidget(content)
        layout.addWidget(scroll, 1)
        options = QHBoxLayout()
        options.addWidget(QLabel("Sample every"))
        self.interval = spin(250, 10000, settings.sample_ms, " ms")
        options.addWidget(self.interval)
        options.addWidget(QLabel("Connection loss after"))
        self.timeout = spin(2000, 60000, settings.timeout_ms, " ms")
        options.addWidget(self.timeout)
        options.addStretch()
        layout.addLayout(options)
        footer = QHBoxLayout()
        self.notice = QLabel("Preview shows target position; firmware smooths the physical needle.")
        self.notice.setWordWrap(True)
        self.notice.setObjectName("muted")
        footer.addWidget(self.notice, 1)
        save = QPushButton("Save && apply")
        save.setObjectName("primary")
        save.clicked.connect(self.save)
        footer.addWidget(save)
        layout.addLayout(footer)
        for signal in (self.interval.valueChanged, self.timeout.valueChanged, self.auto.toggled, self.port.currentIndexChanged):
            signal.connect(self.dirty)
        self.worker = Worker(settings)
        self.worker.sample_ready.connect(self.sample)
        self.worker.status.connect(self.status_label.setText)
        self.tray = QSystemTrayIcon(icon(), self)
        self.tray.setToolTip("ESP Gauge")
        menu = QMenu()
        menu.addAction("Show instruments", self.show_window)
        pause = QAction("Pause / rest needles", self)
        pause.setCheckable(True)
        pause.toggled.connect(self.worker.pause)
        menu.addAction(pause)
        menu.addAction("Disconnect", lambda: self.worker.configure(self.settings, False))
        menu.addAction("Reconnect", lambda: self.worker.configure(self.settings, True))
        menu.addSeparator()
        menu.addAction("Quit", self.quit)
        self.tray.setContextMenu(menu)
        self.tray.activated.connect(lambda reason: self.show_window() if reason in (QSystemTrayIcon.ActivationReason.Trigger, QSystemTrayIcon.ActivationReason.DoubleClick) else None)
        self.tray.show()
        if not QSystemTrayIcon.isSystemTrayAvailable():
            self.notice.setText("No system tray detected. Closing quits; install a tray extension on GNOME to run hidden.")
        if load_error:
            self.notice.setText(load_error)
        self.worker.start()
        self.repaint_dials()

    def dirty(self, *_):
        self.notice.setText("Unsaved changes · Save & apply to update the board.")

    def refresh_ports(self):
        selected = self.port.currentData() or self.settings.port
        self.port.clear()
        self.port.addItem("Select a USB serial port…", "")
        try:
            found = ports()
            for port in found:
                self.port.addItem(f"{port.device} · {port.description}", port.device)
            if selected and self.port.findData(selected) < 0:
                self.port.addItem(f"{selected} · unplugged", selected)
            self.port.setCurrentIndex(max(0, self.port.findData(selected)))
        except OSError:
            pass

    def calibrate(self, index):
        dialog = Calibration(self.draft.outputs[index], index+1, self)
        if dialog.exec():
            candidate = copy.deepcopy(self.draft.outputs[index])
            try:
                dialog.apply(candidate)
                self.draft.outputs[index] = candidate
                self.dirty()
            except ValueError as error:
                QMessageBox.warning(self, "Calibration", str(error))

    def save(self):
        candidate = copy.deepcopy(self.draft)
        candidate.port = self.port.currentData() or ""
        candidate.auto_connect = self.auto.isChecked()
        candidate.sample_ms, candidate.timeout_ms = self.interval.value(), self.timeout.value()
        for i, output in enumerate(candidate.outputs):
            output.enabled = self.enables[i].isChecked()
            output.metric = self.assignments[i].currentData()
        try:
            candidate.save(self.path)
        except (ValueError, OSError) as error:
            QMessageBox.warning(self, "Settings could not be saved", str(error))
            return
        self.settings, self.draft = candidate, copy.deepcopy(candidate)
        self.worker.configure(candidate, bool(candidate.port))
        self.notice.setText("Saved · target preview; physical needle response runs on the board.")
        self.repaint_dials()

    def sample(self, metrics, values):
        self.metrics, self.values = metrics, values
        if self.isVisible():
            self.repaint_dials()

    def repaint_dials(self):
        for i, dial in enumerate(self.dials):
            output = self.settings.outputs[i]
            dial.value, dial.active = self.values[i], output.enabled
            dial.label = METRICS[output.metric] if self.metrics.get(output.metric) is not None else METRICS[output.metric] + " · waiting"
            dial.update()

    def show_window(self):
        self.showNormal()
        self.raise_()
        self.activateWindow()
        self.repaint_dials()

    def closeEvent(self, event):
        if QSystemTrayIcon.isSystemTrayAvailable():
            self.hide()
            event.ignore()
        else:
            self.quit()
            event.accept()

    def quit(self):
        self.worker.stop()
        self.worker.wait()
        self.tray.hide()
        QApplication.instance().quit()


def main():
    app = QApplication(sys.argv)
    app.setOrganizationName("ESP Gauge")
    app.setApplicationName("ESP Gauge")
    app.setQuitOnLastWindowClosed(False)
    app.setStyle("Fusion")
    app.setStyleSheet(STYLE)
    root = Path(QStandardPaths.writableLocation(QStandardPaths.StandardLocation.AppConfigLocation))
    root.mkdir(parents=True, exist_ok=True)
    lock = QLockFile(str(root / "instance.lock"))
    if not lock.tryLock(0):
        QMessageBox.information(None, "ESP Gauge", "ESP Gauge is already running. Open it from the tray/menu-bar icon.")
        return 0
    path, error = root / "settings.json", ""
    try:
        settings = Settings.load(path)
    except (ValueError, TypeError, KeyError, OSError) as exception:
        settings = Settings(auto_connect=False)
        error = f"Settings were not loaded: {exception}. Original file kept until you save."
    window = Window(settings, path, error)
    window.show()
    result = app.exec()
    window.worker.stop()
    window.worker.wait()
    return result

if __name__ == "__main__":
    sys.exit(main())
