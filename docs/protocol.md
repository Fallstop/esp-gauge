# ESP Gauge protocol 2

115200 baud, 8N1, UTF-8 JSON, one object per line. Commands carry a positive `id`; replies repeat it with `ok: true` or `ok: false, error: string`. Lines above 6143 bytes are discarded in full until the next newline. Malformed JSON is ignored. Serial parsing is bounded so malformed input cannot prevent output watchdogs.

## Discovery

Enumerate CH340C USB bridges (VID `1a86`, PID `7523`). Open exclusively where supported with DTR/RTS deasserted. Allow 2.2 seconds for drivers that reset the ESP32 on open, then send one read-only `{"id":1,"op":"hello"}` and wait at most 700 ms. Only keep a port that replies with all of:

```
product = "ESP Gauge"
protocol = 2
channels = 6
max_duty = 880
device = nonempty stable chip identifier
```

Unrecognised bridges are released, then may receive another single identity query after 60 seconds, reinsertion, or an explicit retry. Discovery uses a separate worker, probes one candidate at a time, and starts no new probe during calibration. Opening another bridge cannot block active gauge readings. USB bridge identity alone never permits streaming. Operating systems and drivers may briefly assert reset lines on open despite requested settings; this was observed on the test CH340C. Multiple genuine gauge boards appear in a board selector; only the selected board receives host readings.

## Operations

| `op` | Payload | Result |
| --- | --- | --- |
| `hello` | — | Identity and firmware version; does not drive outputs |
| `get_config` | — | Complete `config` object |
| `config` | `config` | Validate, durably store, then apply; unchanged blobs do not write flash |
| `time` | `epoch` (UTC seconds), `offset` (seconds east of UTC) | Set clock; store offset only if it changes |
| `live` | `values`: six normalized numbers 0–1 or null, `paused`: bool | Refresh host lease and return board readings; optional `include_networks` |
| `status` | — | Readings and Wi-Fi scan results without refreshing the host lease |
| `calibrate` | `port`: 0–5, `duty`: 0–880 per mille | Immediate raw output on one port; renewable 1.5-second lease |
| `calibrate_end` | — | End calibration and resume assigned source |
| `pause` | `paused`: bool | Temporarily rest all outputs |
| `release` | — | End host lease/calibration, rest PC sources, leave standalone sources running |
| `wifi_scan` | — | Request an asynchronous nearby-network scan |
| `wifi` | `ssid`, `password` | Store credentials atomically and begin connecting |
| `wifi_forget` | — | Remove credentials and disconnect |

The desktop adds a `device` field to mutations; its worker checks this against the verified active chip before sending. This prevents edits for a previously selected board reaching its replacement. Unknown telemetry is null and rests the corresponding PC-driven output. Stored zero calibration or disabled ports always produce zero output. A missed host lease rests PC sources after three seconds. Calibration is independent and expires after 1.5 seconds without renewal. Closing the window ends calibration; quitting releases the board. Abrupt process death is covered by the device watchdogs.

## Configuration

```json
{
  "version": 2,
  "channels": [
    {"enabled":false,"name":"","source":"cpu","max_duty":0,"response_ms":500,"scale":100,"reverse":false}
  ]
}
```

Exactly six channel objects are required. `max_duty` is PWM per mille (0–880), `response_ms` is the exponential response time constant (0–5000), and `scale` is the source value corresponding to full-scale deflection. The blob may contain arbitrary additional fields and is preserved up to 4096 bytes. The ESP understands only the subset needed to run outputs. NVS stores the complete JSON atomically; it never silently replaces corrupt or future-version settings.

Host sources are supplied as normalized readings. Built-in standalone source IDs are `time_day`, `time_hours`, `time_minutes`, `time_seconds`, `esp_wifi`, `esp_ble`, `esp_temperature`, `esp_rssi`, and `constant`. `constant` uses `scale` as a position percentage. Wi-Fi and Bluetooth scans run only when needed; only advertising BLE addresses are visible. Time is synchronized from the host every 30 seconds and optionally over NTP. Without Wi-Fi, time survives USB disconnection while powered but must be supplied again after a cold boot. Time zone is the last host UTC offset; a standalone board needs a host reconnection to pick up daylight-saving offset changes.
