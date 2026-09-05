import json
import math
import pytest
from esp_gauge.model import Settings, positions, configuration


def test_defaults_and_roundtrip(tmp_path):
    settings = Settings()
    assert sum(o.enabled for o in settings.outputs) == 1
    path = tmp_path / "nested/settings.json"
    settings.save(path)
    assert Settings.load(path) == settings
    assert not path.with_suffix('.tmp').exists()
    assert configuration(settings)[0] == 'C 0 1 0 200 0 0 500 1'

@pytest.mark.parametrize('low,high', [(500, 200), (-1, 200), (0, 881)])
def test_unsafe_endpoints(low, high):
    s=Settings(); s.outputs[0].low=low; s.outputs[0].high=high
    with pytest.raises(ValueError): s.validate()

@pytest.mark.parametrize('field,value', [('sample_ms', 0), ('timeout_ms', 2000), ('version', 2), ('port', None)])
def test_invalid_settings(field,value):
    s=Settings(); setattr(s,field,value)
    with pytest.raises(ValueError): s.validate()


def test_rates_unavailable_and_clamping():
    s=Settings(); s.outputs[0].metric='net_rx'; s.outputs[0].rest=100
    assert positions(s, {'net_rx': 5*1024**2})[0]==500
    assert positions(s, {'net_rx': 100*1024**2})[0]==1000
    assert positions(s, {'net_rx': -5})[0]==0
    assert positions(s, {'net_rx': math.nan})[0]==100
    assert positions(s, {})[0]==100


def test_corruption_does_not_overwrite(tmp_path):
    path=tmp_path/'settings.json'; path.write_text('{bad')
    with pytest.raises(ValueError): Settings.load(path)
    assert path.read_text()=='{bad'
