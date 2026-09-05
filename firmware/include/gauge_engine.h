#pragma once
#include <stdint.h>
#include <math.h>

namespace gauge {
constexpr unsigned COUNT = 6, MAX_DUTY = 1000;
constexpr uint32_t HOST_TIMEOUT = 3000, CAL_TIMEOUT = 1500;
struct Channel {
  bool enabled = false, reverse = false;
  uint16_t minDuty = 0, maxDuty = 0, response = 500;
  float target = 0, position = 0;
  bool available = false;
};
struct Engine {
  Channel channels[COUNT];
  int calibration = -1;
  unsigned calibrationDuty = 0;
  uint32_t calibrationAt = 0;
  bool calibrate(unsigned port, unsigned duty, uint32_t now) {
    if (port >= COUNT || duty > MAX_DUTY)
      return false;
    calibration = int(port);
    calibrationDuty = duty;
    calibrationAt = now;
    return true;
  }
  void endCalibration() {
    if (calibration >= 0)
      channels[calibration].position = 0;
    calibration = -1;
    calibrationDuty = 0;
  }
  void tick(uint32_t now, uint32_t dt) {
    if (calibration >= 0 && uint32_t(now - calibrationAt) > CAL_TIMEOUT)
      endCalibration();
    for (auto &c : channels) {
      if (!c.enabled || !c.available) {
        c.position = 0;
        continue;
      }
      float target = fmaxf(0, fminf(1, c.target));
      c.position += (target - c.position) * (c.response ? 1 - expf(-float(dt) / c.response) : 1);
    }
  }
  unsigned duty(unsigned i) const {
    if (i >= COUNT)
      return 0;
    if (calibration == int(i))
      return calibrationDuty * 4095 / 1000;
    const auto &c = channels[i];
    if (!c.enabled || !c.available)
      return 0;
    float p = c.reverse ? 1 - c.position : c.position;
    float duty = c.minDuty + (float(c.maxDuty) - c.minDuty) * fmaxf(0, fminf(1, p));
    return unsigned(fmaxf(0, fminf(float(MAX_DUTY), duty)) * 4095 / 1000);
  }
};
inline float clockValue(const char unit, int64_t epoch, int offset) {
  int64_t local = epoch + offset;
  int64_t day = ((local % 86400) + 86400) % 86400;
  switch (unit) {
  case 'h':
    return float(day % 43200) / 43200;
  case 'm':
    return float(day % 3600) / 3600;
  case 's':
    return float(day % 60) / 60;
  default:
    return float(day) / 86400;
  }
}
} // namespace gauge

namespace gauge {
inline float waveform(const char shape, double seconds, double period, double phase) {
  double p = fmod(seconds / period + phase / 360.0, 1.0);
  if (p < 0)
    p += 1;
  switch (shape) {
  case 't':
    return float(1.0 - fabs(2.0 * p - 1.0));
  case 'r':
    return float(p);
  case 'q':
    return p < .5 ? 0 : 1;
  default:
    return float((1.0 - cos(6.283185307179586 * p)) * .5);
  }
}
} // namespace gauge
