import os
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
from PySide6.QtWidgets import QApplication
from esp_gauge.app import Window
from esp_gauge.model import Settings


def test_window_hidden_does_not_schedule_dial_paints(tmp_path):
    app=QApplication.instance() or QApplication([])
    window=Window(Settings(auto_connect=False),tmp_path/'settings.json')
    try:
        window.show(); app.processEvents()
        assert len(window.dials)==6
        assert sum(box.isChecked() for box in window.enables)==1
        window.hide(); app.processEvents()
        calls=[]
        window.repaint_dials=lambda: calls.append(True)
        window.sample({'cpu':50},[500,0,0,0,0,0])
        assert not calls
        window.show_window(); assert calls
    finally:
        window.worker.stop(); assert window.worker.wait(5000)
        window.tray.hide(); window.hide()
