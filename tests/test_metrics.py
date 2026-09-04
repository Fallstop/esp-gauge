from types import SimpleNamespace
from esp_gauge.metrics import Sampler


def test_rate_deltas_missing_counters_and_reset(monkeypatch):
    clock=iter([10,12,14])
    monkeypatch.setattr('esp_gauge.metrics.time.monotonic',lambda:next(clock))
    monkeypatch.setattr('esp_gauge.metrics.psutil.cpu_percent',lambda _:25)
    monkeypatch.setattr('esp_gauge.metrics.psutil.virtual_memory',lambda:SimpleNamespace(percent=40))
    counters=iter([SimpleNamespace(bytes_recv=100,bytes_sent=200),SimpleNamespace(bytes_recv=300,bytes_sent=400),SimpleNamespace(bytes_recv=0,bytes_sent=0)])
    monkeypatch.setattr('esp_gauge.metrics.psutil.net_io_counters',lambda:next(counters))
    monkeypatch.setattr('esp_gauge.metrics.psutil.disk_io_counters',lambda:None)
    sampler=Sampler(); sample=sampler.sample()
    assert sample['net_rx']==100 and sample['net_tx']==100
    assert sample['disk_read'] is None and sample['cpu']==25
    assert sampler.sample()['net_rx']==0
