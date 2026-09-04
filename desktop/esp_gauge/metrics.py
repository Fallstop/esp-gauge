"""One nonblocking system-wide sample; no process enumeration."""
import time
import psutil

class Sampler:
    def __init__(self):
        psutil.cpu_percent(None)  # prime the delta; do not present this value
        self.previous = {}
        self.last = time.monotonic()
        self._counters()

    def _counters(self):
        result = {}
        for call, fields in ((psutil.net_io_counters, {"net_rx": "bytes_recv", "net_tx": "bytes_sent"}),
                             (psutil.disk_io_counters, {"disk_read": "read_bytes", "disk_write": "write_bytes"})):
            try:
                counters = call()
                if counters:
                    result.update({key: getattr(counters, field) for key, field in fields.items()})
            except (OSError, RuntimeError, NotImplementedError, psutil.Error):
                pass
        previous, self.previous = self.previous, result
        return result, previous

    def sample(self):
        now = time.monotonic()
        elapsed, self.last = max(now - self.last, 0.001), now
        result = {"cpu": psutil.cpu_percent(None), "memory": psutil.virtual_memory().percent}
        current, previous = self._counters()
        for key in ("net_rx", "net_tx", "disk_read", "disk_write"):
            result[key] = max(0, current[key] - previous[key]) / elapsed if key in current and key in previous else None
        return result
