"""Sleeping worker owns sampling and serial; GUI thread never waits for USB."""
import copy
import threading
import time
from PySide6.QtCore import QThread, Signal
from .connection import Connection
from .metrics import Sampler
from .model import positions

class Worker(QThread):
    sample_ready = Signal(dict, list)
    status = Signal(str)

    def __init__(self, settings):
        super().__init__()
        self.condition = threading.Condition()
        self.settings = copy.deepcopy(settings)
        self.revision = 0
        self.stopping = False
        self.want_connection = settings.auto_connect
        self.paused = False
        self.test = None
        self.sample_requested = False

    def test_position(self, index, value):
        with self.condition:
            self.test = (index, value, time.monotonic() + 5)
            self.sample_requested = True
            self.condition.notify()

    def configure(self, settings, connect=None):
        with self.condition:
            self.settings = copy.deepcopy(settings)
            if connect is not None:
                self.want_connection = connect
            self.revision += 1
            self.test = None
            self.condition.notify()

    def pause(self, paused):
        with self.condition:
            self.paused = paused
            self.test = None
            self.revision += 1
            self.condition.notify()

    def stop(self):
        with self.condition:
            self.stopping = True
            self.condition.notify()

    def run(self):
        sampler, connection = Sampler(), Connection()
        revision, retry, next_sample = -1, 0.0, time.monotonic() + 1
        try:
            while True:
                with self.condition:
                    if self.stopping:
                        break
                    settings = copy.deepcopy(self.settings)
                    changed = revision != self.revision
                    revision = self.revision
                    wanted, paused = self.want_connection, self.paused
                    test = self.test
                    requested, self.sample_requested = self.sample_requested, False
                now = time.monotonic()
                if changed:
                    connection.rest()
                    connection.close()
                    retry = 0
                    self.status.emit("Paused · needles resting" if paused else "Disconnected")
                if wanted and not paused and settings.port and not connection.serial and now >= retry:
                    self.status.emit(f"Connecting to {settings.port}…")
                    try:
                        connection.connect(settings)
                        self.status.emit(f"Connected · {settings.port}")
                    except (OSError, ValueError) as error:
                        self.status.emit(f"Retrying in 5 s · {error}")
                        retry = time.monotonic() + 5
                if (now >= next_sample or requested) and not paused:
                    try:
                        metrics = sampler.sample()
                        values = positions(settings, metrics)
                        if test and time.monotonic() < test[2] and settings.outputs[test[0]].enabled:
                            values[test[0]] = test[1]
                        self.sample_ready.emit(metrics, values)
                        if connection.serial:
                            connection.send(values)
                    except (OSError, RuntimeError) as error:
                        connection.close()
                        retry = time.monotonic() + 5
                        self.status.emit(f"Retrying in 5 s · {error}")
                    next_sample = time.monotonic() + settings.sample_ms / 1000
                    if test and time.monotonic() < test[2]:
                        next_sample = min(next_sample, test[2])
                with self.condition:
                    if self.stopping:
                        break
                    if revision == self.revision and not self.sample_requested:
                        deadline = next_sample if not paused else time.monotonic() + 60
                        if wanted and settings.port and not paused and not connection.serial:
                            deadline = min(deadline, retry)
                        self.condition.wait(max(0.05, deadline - time.monotonic()))
        finally:
            connection.rest()
            connection.close()
