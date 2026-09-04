"""Bounded serial I/O, explicit identification, and ACK verification."""
import time
import serial
from serial.tools import list_ports
from .model import configuration


def ports():
    return sorted(list_ports.comports(), key=lambda p: p.device)

class Connection:
    def __init__(self):
        self.serial = None

    def close(self):
        if self.serial:
            try:
                self.serial.close()
            finally:
                self.serial = None

    def request(self, line, expected="OK"):
        self.serial.write((line + "\n").encode("ascii"))
        answer = self.serial.read_until(b"\n", 128).decode("ascii", errors="replace").strip()
        if answer != expected:
            raise OSError(f"Board reply: {answer or 'timed out'}")

    def connect(self, settings):
        self.close()
        port = serial.Serial()
        port.port = settings.port
        port.baudrate = 115200
        port.timeout = 0.3
        port.write_timeout = 0.3
        port.dtr = False
        port.rts = False
        self.serial = port
        try:
            port.open()
            # Some CH340 drivers pulse reset on open. Allow boot and discard logs.
            time.sleep(1.5)
            port.reset_input_buffer()
            self.request("H", "ESPGAUGE 1 6 880")
            self.request("R")
            for line in configuration(settings):
                self.request(line)
        except Exception:
            self.close()
            raise

    def send(self, values):
        if len(values) != 6 or any(type(v) is not int or not 0 <= v <= 1000 for v in values):
            raise ValueError("Six normalized integer positions required")
        self.request("V " + " ".join(map(str, values)))

    def rest(self):
        if self.serial:
            try:
                self.request("S")
            except (OSError, serial.SerialException):
                pass
