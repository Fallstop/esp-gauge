from types import SimpleNamespace
import pytest
from esp_gauge.connection import ports, Discovery


def port(device, vid=0x1a86, pid=0x7523, description='USB Serial'):
    return SimpleNamespace(device=device,vid=vid,pid=pid,description=description)


@pytest.mark.parametrize('name',['/dev/ttyUSB0','COM12','/dev/cu.wchusbserial1430'])
def test_ch340_usb_metadata_is_platform_independent(monkeypatch,name):
    monkeypatch.setattr('esp_gauge.connection.list_ports.comports',lambda:[
        port(name), port('unrelated',0x10c4,0xea60), port('legacy',None,None),
        port('misleading description',None,None,'CH340C')])
    assert [p.device for p in ports()]==[name]


def test_macos_prefers_callout_alias_and_deduplicates(monkeypatch):
    monkeypatch.setattr('esp_gauge.connection.list_ports.comports',lambda:[
        port('/dev/tty.wchusbserial1'),port('/dev/cu.wchusbserial1'),
        port('/dev/cu.wchusbserial1'),port('/dev/tty.wchusbserial2')])
    assert [p.device for p in ports()]==['/dev/cu.wchusbserial1','/dev/tty.wchusbserial2']


def test_discovery_rotates_and_recovers_from_hotplug_renaming(monkeypatch):
    found=[]
    monkeypatch.setattr('esp_gauge.connection.list_ports.comports',lambda:found)
    discovery=Discovery()
    assert discovery.next_port() is None
    found[:]=[port('COM4'),port('COM5')]
    assert [discovery.next_port() for _ in range(3)]==['COM4','COM5','COM4']
    found[:]=[]
    assert discovery.next_port() is None
    found[:]=[port('COM9')]
    assert discovery.next_port()=='COM9'
