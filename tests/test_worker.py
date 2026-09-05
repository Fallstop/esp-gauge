import threading
import time
from esp_gauge.worker import Worker
from esp_gauge.model import Settings


def test_worker_reconnects_after_failed_ack_and_stops(monkeypatch):
    connected=[]; sent=threading.Event(); closed=[]
    class Link:
        serial=None
        def connect(self, settings):
            connected.append(settings.port); self.serial=True
        def send(self, values):
            if len(connected)==1: raise OSError('cable lost')
            sent.set()
        def rest(self): pass
        def close(self): self.serial=None; closed.append(True)
    monkeypatch.setattr('esp_gauge.worker.Connection',Link)
    monkeypatch.setattr('esp_gauge.worker.Discovery.next_port',lambda _: 'test')
    worker=Worker(Settings(port='stale-port',auto_connect=False,sample_ms=250))
    thread=threading.Thread(target=worker.run)
    thread.start()
    try:
        assert sent.wait(9)
        assert len(connected)==2
    finally:
        worker.stop(); thread.join(3)
    assert not thread.is_alive() and closed


def test_discovery_skips_wrong_firmware_then_keeps_connection(monkeypatch):
    attempts=[]; sent=threading.Event(); scans=[]
    class Link:
        serial=None
        def connect(self,settings):
            attempts.append(settings.port)
            if settings.port=='busy': raise OSError('not gauge firmware')
            self.serial=True
        def send(self,values): sent.set()
        def rest(self): pass
        def close(self): self.serial=None
    def discover(_):
        scans.append(True)
        return 'busy' if len(scans)==1 else 'board'
    monkeypatch.setattr('esp_gauge.worker.Connection',Link)
    monkeypatch.setattr('esp_gauge.worker.Discovery.next_port',discover)
    worker=Worker(Settings(port='unrelated',auto_connect=False,sample_ms=250))
    thread=threading.Thread(target=worker.run); thread.start()
    try:
        assert sent.wait(8)
        time.sleep(.3)
        assert attempts==['busy','board']
        assert len(scans)==2
    finally:
        worker.stop(); thread.join(3)
    assert not thread.is_alive()


def test_empty_discovery_sleeps_and_can_be_stopped(monkeypatch):
    scans=[]; first=threading.Event()
    def discover(_): scans.append(True); first.set(); return None
    monkeypatch.setattr('esp_gauge.worker.Discovery.next_port',discover)
    worker=Worker(Settings(auto_connect=False))
    thread=threading.Thread(target=worker.run); thread.start()
    try:
        assert first.wait(2)
        time.sleep(.3)
        assert len(scans)==1
    finally:
        worker.stop(); thread.join(3)
    assert not thread.is_alive()
