"""Exercise a protocol-2 board. Only PWM6 is driven, at 1% duty, briefly.

Run with: uv run --with pyserial python tools/hardware_check.py --port /dev/cu.usbserial-110
Close ESP Gauge first. The original configuration is restored even after failure.
"""
import argparse
import copy
import json
import statistics
import time
from pathlib import Path

import serial

parser = argparse.ArgumentParser()
parser.add_argument('--port', required=True)
args = parser.parse_args()
port = serial.Serial(port=None, baudrate=115200, timeout=.05, exclusive=True)
port.dtr = False
port.rts = False
port.port = args.port
port.open()
time.sleep(2.3)
sequence = 0

def request(op, **data):
    global sequence
    sequence += 1
    port.write((json.dumps(dict(op=op, id=sequence, **data), separators=(',', ':')) + '\n').encode())
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        line = port.readline()
        try:
            reply = json.loads(line)
        except (ValueError, UnicodeError):
            continue
        if reply.get('id') == sequence:
            return reply
    raise TimeoutError(op)

def ok(op, **data):
    reply = request(op, **data)
    assert reply.get('ok'), reply
    return reply

original = None
try:
    hello = ok('hello')
    assert hello['product'] == 'ESP Gauge' and hello['protocol'] == 2 and hello['channels'] == 6
    print('Identity:', hello['device'], hello['firmware'])
    original = ok('get_config')['config']
    Path('artifacts').mkdir(exist_ok=True)
    Path('artifacts/config-before-hardware-test.json').write_text(json.dumps(original, indent=2))
    safe = copy.deepcopy(original)
    for c in safe['channels']:
        c.update(enabled=False, min_duty=0, max_duty=0, source='cpu', scale=100, reverse=False, response_ms=0)
    safe['verification_metadata'] = {'opaque': ['retained', 2]}
    ok('config', config=safe)
    assert ok('get_config')['config'] == safe
    bad = copy.deepcopy(safe)
    bad['channels'][5]['max_duty'] = 1001
    assert not request('config', config=bad)['ok']
    assert ok('get_config')['config'] == safe
    bad['channels'][5]['max_duty'] = 0
    bad['version'] = 3
    assert not request('config', config=bad)['ok']
    assert not request('calibrate', port=6, duty=10)['ok']
    assert not request('calibrate', port=5, duty=1001)['ok']
    print('Configuration roundtrip, unknown metadata, invalid requests: OK')

    ok('calibrate', port=5, duty=10)
    snapshot = ok('status')
    assert snapshot['duties'] == [0, 0, 0, 0, 0, 40], snapshot
    started = time.monotonic()
    while time.monotonic() - started < 1.9:
        time.sleep(.1)
    snapshot = ok('status')
    assert snapshot['calibrating'] == -1 and snapshot['duties'] == [0]*6, snapshot
    print('PWM6 1% output, other ports zero, calibration expiry: OK')

    host = copy.deepcopy(safe)
    host['channels'][5].update(enabled=True, max_duty=10)
    ok('config', config=host)
    ok('live', values=[None]*5+[1.0], paused=False)
    time.sleep(.03)
    assert ok('status')['duties'][5] == 40
    ok('live', values=[None]*6, paused=False)
    time.sleep(.03)
    assert ok('status')['duties'][5] == 0
    ok('live', values=[None]*5+[1.0], paused=False)
    time.sleep(3.15)
    assert ok('status')['duties'] == [0]*6
    host['channels'][5].update(min_duty=5)
    ok('config', config=host)
    ok('live', values=[None]*5+[0.0], paused=False)
    time.sleep(.03)
    assert ok('status')['duties'][5] == 20
    ok('live', values=[None]*5+[1.0], paused=False)
    time.sleep(.03)
    assert ok('status')['duties'][5] == 40
    ok('live', values=[None]*6, paused=False)
    time.sleep(.03)
    assert ok('status')['duties'][5] == 0
    bad = copy.deepcopy(host)
    bad['channels'][5]['min_duty'] = 11
    assert not request('config', config=bad)['ok']
    print('Host readings, range endpoints, unavailable readings and timeout: OK')

    clocks = copy.deepcopy(safe)
    for i, source in enumerate(['time_day', 'time_hours', 'time_minutes', 'time_seconds']):
        clocks['channels'][i].update(source=source, enabled=True)
    clocks['channels'][4].update(source='time_future', enabled=True)
    ok('config', config=clocks)
    ok('time', epoch=1788602430, offset=0)
    assert not request('time', epoch=1788602430, offset=-2147483648)['ok']
    ok('release')
    time.sleep(.1)
    status = ok('status')
    assert status['clock_valid'] and all(status['available'][:4]), status
    assert not status['available'][4], status
    assert status['duties'] == [0]*6
    assert status['positions'][3] >= .5
    print('All four clock sources run after host release: OK')

    if tuple(map(int, hello['firmware'].split('.')[:2])) >= (2, 2):
        waves = copy.deepcopy(safe)
        for i, source in enumerate(['wave_sine', 'wave_triangle', 'wave_saw', 'wave_square']):
            waves['channels'][i].update(source=source, enabled=True, period_s=1.0, phase_deg=0)
        waves['channels'][5].update(source='wave_sine', enabled=True, max_duty=10, period_s=1.0)
        ok('config', config=waves)
        bad = copy.deepcopy(waves)
        bad['channels'][5]['period_s'] = 0
        assert not request('config', config=bad)['ok']
        ok('release')
        samples = []
        for _ in range(30):
            time.sleep(.05)
            status = ok('status')
            assert all(status['available'][i] for i in [0,1,2,3,5]), status
            assert status['duties'][:5] == [0]*5 and 0 <= status['duties'][5] <= 40, status
            samples.append(status['positions'][5])
        assert max(samples) > .95 and min(samples) < .05, samples
        ok('live', values=[1]*6, paused=True)
        time.sleep(.05)
        assert ok('status')['duties'] == [0]*6
        ok('release')
        print('Four autonomous waveforms, complete PWM6 sweep at 1%, invalid period and pause: OK')

    sense = copy.deepcopy(safe)
    for i, source in enumerate(['esp_wifi', 'esp_ble', 'esp_temperature']):
        sense['channels'][i].update(source=source, enabled=True)
    ok('config', config=sense)
    ok('wifi_scan')
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        status = ok('status')
        if status['wifi_count'] >= 0 and status['ble_count'] >= 0 and status.get('temperature') is not None:
            break
        time.sleep(.4)
    assert status['wifi_count'] >= 0, status
    assert status['ble_count'] >= 0, status
    assert status.get('temperature') is not None, status
    assert status['duties'] == [0]*6
    print('ESP senses:', {k: status.get(k) for k in ['wifi_count', 'ble_count', 'temperature', 'free_heap']})

    ok('config', config=safe)
    port.write(b'x'*6400+b'\n{bad json}\n')
    assert ok('hello')['protocol'] == 2
    timings=[]
    for i in range(40):
        start=time.perf_counter();ok('calibrate',port=5,duty=0);timings.append((time.perf_counter()-start)*1000)
    ok('calibrate_end')
    print('Framing recovery: OK. Calibration ACK ms:', {'median':round(statistics.median(timings),2),'p95':round(sorted(timings)[37],2),'max':round(max(timings),2)})

    # Reset EN without asserting the boot strap, then verify NVS independently of RAM.
    port.rts=True;time.sleep(.1);port.rts=False;time.sleep(1.5);port.reset_input_buffer()
    ok('hello')
    assert ok('get_config')['config'] == safe
    assert ok('status')['duties'] == [0]*6
    print('Configuration and opaque metadata survived a real ESP32 reset: OK')
finally:
    if original is not None:
        try:
            ok('hello');ok('calibrate_end');ok('config',config=original);ok('release')
            print('Original configuration restored.')
        except Exception as error:
            print('RESTORE REQUIRED: see artifacts/config-before-hardware-test.json;',error)
    port.close()
