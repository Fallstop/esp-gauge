"""Measure the real worker and Qt window with visible/hidden UI, no USB hardware.
CPU percent is one logical core (process CPU seconds / wall seconds * 100).
"""
import argparse
import json
import os
import tempfile
import time
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
import psutil
from PySide6.QtCore import QTimer
from PySide6.QtWidgets import QApplication
from esp_gauge.app import Window
from esp_gauge.model import Settings

parser=argparse.ArgumentParser()
parser.add_argument('--seconds',type=int,default=30)
parser.add_argument('--visible',action='store_true')
parser.add_argument('--sample-ms',type=int,default=1000)
args=parser.parse_args()
app=QApplication([])
window=Window(Settings(auto_connect=False,sample_ms=args.sample_ms),Path(tempfile.gettempdir())/'esp-gauge-benchmark.json', hardware_enabled=False)
if args.visible: window.show()
process=psutil.Process()
start=time.monotonic(); cpu=process.cpu_times(); samples=[]
window.worker.sample_ready.connect(lambda *_: samples.append(time.monotonic()))
QTimer.singleShot(args.seconds*1000,app.quit)
app.exec()
elapsed=time.monotonic()-start
end=process.cpu_times()
window.worker.stop(); window.worker.wait()
print(json.dumps({'visible':args.visible,'sample_ms':args.sample_ms,'wall_seconds':round(elapsed,3),
'cpu_percent_one_core':round((end.user+end.system-cpu.user-cpu.system)/elapsed*100,3),
'rss_mib':round(process.memory_info().rss/1024**2,1),'samples':len(samples)},indent=2))
