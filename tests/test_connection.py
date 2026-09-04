import pytest
from esp_gauge.connection import Connection
from esp_gauge.model import Settings

class FakeSerial:
    def __init__(self):
        self.writes=[]; self.closed=False; self.answer=None
    def open(self): pass
    def close(self): self.closed=True
    def reset_input_buffer(self): pass
    def write(self, data): self.writes.append(data)
    def read_until(self, *args):
        return self.answer or (b'ESPGAUGE 1 6 880\n' if self.writes[-1]==b'H\n' else b'OK\n')


def test_handshake_configuration_and_position(monkeypatch):
    fake=FakeSerial()
    monkeypatch.setattr('esp_gauge.connection.serial.Serial', lambda:fake)
    monkeypatch.setattr('esp_gauge.connection.time.sleep', lambda _:None)
    c=Connection(); c.connect(Settings(port='test'))
    assert fake.writes[:2]==[b'H\n',b'R\n']
    assert len(fake.writes)==9
    c.send([1000,0,0,0,0,0]); assert fake.writes[-1]==b'V 1000 0 0 0 0 0\n'
    c.rest(); assert fake.writes[-1]==b'S\n'
    c.close(); assert fake.closed


def test_wrong_device_never_configured(monkeypatch):
    fake=FakeSerial(); fake.answer=b'Unrelated device\n'
    monkeypatch.setattr('esp_gauge.connection.serial.Serial', lambda:fake)
    monkeypatch.setattr('esp_gauge.connection.time.sleep', lambda _:None)
    c=Connection()
    with pytest.raises(OSError): c.connect(Settings(port='test'))
    assert fake.writes==[b'H\n'] and fake.closed


def test_lost_ack():
    c=Connection(); c.serial=FakeSerial(); c.serial.answer=b'ERR\n'
    with pytest.raises(OSError): c.send([0]*6)
    with pytest.raises(ValueError): c.send([1001]*6)
