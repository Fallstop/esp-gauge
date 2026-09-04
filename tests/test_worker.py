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
    worker=Worker(Settings(port='test',sample_ms=250))
    thread=threading.Thread(target=worker.run)
    thread.start()
    try:
        assert sent.wait(9)
        assert len(connected)==2
    finally:
        worker.stop(); thread.join(3)
    assert not thread.is_alive() and closed
