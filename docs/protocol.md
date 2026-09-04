# ESP Gauge serial protocol v1

115200 baud, 8 data bits, no parity, one stop bit, no flow control. ASCII lines
terminated by LF; CR before LF is accepted. Host sends one command and waits for
one response before sending the next. The desktop allows 300 ms per response.
Firmware input is bounded at 127 characters and discards an oversized or invalid
line through the next LF. Bad numeric fields reject the whole command.

| Command | Response / meaning |
|---|---|
| `H` | `ESPGAUGE 1 6 880`: protocol, channels, maximum electrical duty in per mille |
| `R` | `OK`: reset session; disable every output immediately |
| `C i enabled low high reverse rest response 1` | `OK`: configure zero-based channel; final `1` is calibration format version |
| `T milliseconds` | `OK`: watchdog timeout, 2000–60000 ms |
| `V p0 p1 p2 p3 p4 p5` | `OK`: atomically set six normalized positions and refresh watchdog |
| `S` | `OK`: go to calibrated rest using configured response |

Rejected recognized or unknown commands return `ERR`. Empty and discarded frames
produce no reply. Startup emits no application logs. Identification never enables
outputs. Desktop sends H, R, six C commands, T, then V each sample. Unexpected
responses close the connection and trigger a five-second reconnect backoff. A
new connection reapplies all configuration. Only valid V refreshes the watchdog;
H, malformed traffic and configuration commands do not keep a stale needle alive.

`i`: 0–5. `enabled`, `reverse`: 0 or 1. `low`, `high`: electrical duty
0–880 per mille, with low <= high. `rest`, positions: normalized 0–1000.
`response`: 0–5000 ms, exponential time constant (63% of a step in one time
constant, about 95% in three). Zero applies immediately. Position is reversed
before mapping between low/high. Disabled outputs always have zero electrical
duty, regardless of rest. Pins use 5 kHz, 12-bit LEDC PWM.

Example: one output, conservative 20% maximum electrical duty:

```text
H
R
C 0 1 0 200 0 0 500 1
T 5000
V 500 0 0 0 0 0
```

Normalized 500 becomes 10% electrical duty in this example. This is PWM averaged
by the board/gauge circuitry, not hobby servo pulses or a stepper driver.

Configuration is deliberately RAM-only on the ESP32: power-up always disables
outputs, and continuous desktop samples never wear flash. Desktop settings are
the source of truth. While powered, cable loss moves to rest after the watchdog;
removing power cannot actively position a needle. `gauge_core.h` is independent
of Arduino and serial so a future Wi-Fi or club-project adapter can call the same
validated controller. There is no network listener in this release.
