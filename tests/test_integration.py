"""Use the real C++ protocol parser through a subprocess transport."""
import shutil
import subprocess
from pathlib import Path
import pytest
from esp_gauge.connection import Connection
from esp_gauge.model import Settings


def test_desktop_commands_accepted_by_firmware(tmp_path, monkeypatch):
    compiler=shutil.which('g++')
    if not compiler:
        pytest.skip('C++ compiler unavailable; firmware CI runs this parser separately')
    root=Path(__file__).resolve().parents[1]
    executable=tmp_path/'bridge'
    subprocess.run([compiler,'-std=c++11','-I'+str(root/'firmware/include'),str(root/'tests/firmware_bridge.cpp'),'-o',str(executable)],check=True)
    process=subprocess.Popen([str(executable)],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    class Transport:
        def open(self): pass
        def reset_input_buffer(self): pass
        def write(self,data): process.stdin.write(data); process.stdin.flush()
        def read_until(self,*args): return process.stdout.readline()
        def close(self): process.stdin.close()
    monkeypatch.setattr('esp_gauge.connection.serial.Serial',Transport)
    monkeypatch.setattr('esp_gauge.connection.time.sleep',lambda _:None)
    connection=Connection()
    try:
        settings=Settings(port='simulated')
        for output in settings.outputs: output.enabled=True
        connection.connect(settings)
        connection.send([0,200,400,600,800,1000])
        connection.rest()
        connection.close()
        assert process.wait(timeout=5)==0
    finally:
        if process.poll() is None: process.kill(); process.wait()
