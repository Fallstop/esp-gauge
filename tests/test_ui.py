import os
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
from PySide6.QtWidgets import QApplication
from esp_gauge.app import Window
from esp_gauge.model import Settings


def test_window_hidden_does_not_schedule_dial_paints(tmp_path):
    app=QApplication.instance() or QApplication([])
    window=Window(Settings(auto_connect=False),tmp_path/'settings.json', hardware_enabled=False)
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


def test_board_connectors_follow_actual_schematic(tmp_path):
    from esp_gauge.board import CONNECTORS
    assert CONNECTORS==('J1','J3','J5','J2','J4','J6')
    app=QApplication.instance() or QApplication([])
    window=Window(Settings(auto_connect=False),tmp_path/'settings.json', hardware_enabled=False)
    try:
        assert not window.board.pixmap.isNull()
        window.show(); app.processEvents()
        window.select_output(4)
        assert window.board.index==4
        assert 'J4' in window.board_label.text()
        for name in CONNECTORS:
            rectangle=window.board.connector_rect(name)
            assert rectangle.width()>0 and rectangle.height()>0
    finally:
        window.worker.stop(); window.worker.wait(5000)
        window.tray.hide(); window.hide()
