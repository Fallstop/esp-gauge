"""Validated, versioned configuration and transport-independent normalization."""
from dataclasses import dataclass, field, asdict
import json
import math
from pathlib import Path

METRICS = {"cpu": "CPU usage", "memory": "Memory used", "net_rx": "Network download", "net_tx": "Network upload", "disk_read": "Disk read", "disk_write": "Disk write"}

@dataclass
class Output:
    enabled: bool = False
    metric: str = "cpu"
    low: int = 0
    high: int = 200  # conservative 20% electrical duty until calibrated
    reverse: bool = False
    rest: int = 0
    response_ms: int = 500
    full_scale: float = 10.0  # MiB/s for rate metrics

    def validate(self):
        if type(self.enabled) is not bool or type(self.reverse) is not bool:
            raise ValueError("Output switches must be booleans")
        if self.metric not in METRICS:
            raise ValueError("Unknown metric")
        for value in (self.low, self.high, self.rest, self.response_ms):
            if type(value) is not int:
                raise ValueError("Calibration values must be integers")
        if not 0 <= self.low <= self.high <= 880:
            raise ValueError("Duty endpoints must be ordered within 0–88%")
        if not 0 <= self.rest <= 1000 or not 0 <= self.response_ms <= 5000:
            raise ValueError("Invalid resting position or response time")
        if not isinstance(self.full_scale, (int, float)) or not math.isfinite(self.full_scale) or not 0.01 <= self.full_scale <= 100000:
            raise ValueError("Rate full scale must be 0.01–100000 MiB/s")

@dataclass
class Settings:
    version: int = 1
    # Legacy fields retained for v1 settings compatibility; discovery ignores them.
    port: str = ""
    auto_connect: bool = True
    sample_ms: int = 1000
    timeout_ms: int = 5000
    outputs: list[Output] = field(default_factory=lambda: [Output(enabled=i == 0) for i in range(6)])

    def validate(self):
        if self.version != 1 or len(self.outputs) != 6:
            raise ValueError("Unsupported settings version or output count")
        if not isinstance(self.port, str) or len(self.port) > 512 or type(self.auto_connect) is not bool:
            raise ValueError("Invalid connection settings")
        if type(self.sample_ms) is not int or not 250 <= self.sample_ms <= 10000:
            raise ValueError("Sampling must be 250–10000 ms")
        if type(self.timeout_ms) is not int or not max(2000, self.sample_ms * 3) <= self.timeout_ms <= 60000:
            raise ValueError("Timeout must allow at least three samples (2–60 seconds)")
        for output in self.outputs:
            output.validate()
        return self

    @classmethod
    def load(cls, path: Path):
        if not path.exists():
            return cls()
        data = json.loads(path.read_text())
        data["outputs"] = [Output(**o) for o in data["outputs"]]
        return cls(**data).validate()

    def save(self, path: Path):
        self.validate()
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(".tmp")
        temporary.write_text(json.dumps(asdict(self), indent=2) + "\n")
        temporary.replace(path)


def positions(settings, metrics):
    result = []
    for output in settings.outputs:
        value = metrics.get(output.metric)
        if not output.enabled or value is None or not math.isfinite(value):
            result.append(output.rest)
        else:
            scale = 100 if output.metric in ("cpu", "memory") else output.full_scale * 1024**2
            result.append(round(max(0, min(1, value / scale)) * 1000))
    return result


def configuration(settings):
    settings.validate()
    lines = [f"C {i} {int(o.enabled)} {o.low} {o.high} {int(o.reverse)} {o.rest} {o.response_ms} 1" for i, o in enumerate(settings.outputs)]
    return lines + [f"T {settings.timeout_ms}"]
