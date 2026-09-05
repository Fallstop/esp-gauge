#pragma once
// No Arduino dependency: reusable by USB, a future Wi-Fi adapter, and host tests.
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

namespace gauge {
constexpr unsigned COUNT = 6;
constexpr unsigned MAX_DUTY = 880; // per mille; preserve bench's 88% ceiling
struct Output {
  bool enabled = false, reverse = false;
  unsigned low = 0, high = 200, rest = 0, response = 500, target = 0;
  float position = 0;
};
struct Controller {
  Output outputs[COUNT];
  uint32_t timeout = 5000, last = 0;
  bool live = false;
  // Strict unsigned ASCII integers; reject overflow, negatives and trailing junk.
  bool command(const char *line, uint32_t now) {
    if (!strcmp(line, "R")) {
      for (auto &o : outputs) o = Output{};
      live = false;
      return true;
    }
    char kind = *line++;
    unsigned values[8], n = 0;
    while (*line) {
      if (*line++ != ' ' || n == 8 || *line < '0' || *line > '9') return false;
      unsigned value = 0;
      while (*line >= '0' && *line <= '9') {
        if (value > 60000) return false;
        value = value * 10 + (*line++ - '0');
      }
      if (value > 60000) return false;
      values[n++] = value;
    }
    if (kind == 'C' && n == 8) {
      auto *v = values;
      if (v[0] >= COUNT || v[1] > 1 || v[2] > v[3] || v[3] > MAX_DUTY || v[4] > 1 || v[5] > 1000 || v[6] > 5000 || v[7] != 1) return false;
      auto &o = outputs[v[0]];
      o.enabled = v[1]; o.low = v[2]; o.high = v[3]; o.reverse = v[4]; o.rest = v[5]; o.response = v[6];
      o.target = o.rest;
      o.position = o.rest;
      return true;
    }
    if (kind == 'T' && n == 1 && values[0] >= 2000) { timeout = values[0]; return true; }
    if (kind == 'V' && n == COUNT) {
      for (unsigned i = 0; i < COUNT; ++i) if (values[i] > 1000) return false;
      for (unsigned i = 0; i < COUNT; ++i) outputs[i].target = values[i];
      last = now; live = true; return true;
    }
    if (kind == 'S' && n == 0) { live = false; return true; }
    return false;
  }
  void tick(uint32_t now, unsigned dt) {
    if (live && uint32_t(now - last) > timeout) live = false;
    for (auto &o : outputs) {
      float target = live ? o.target : o.rest;
      float alpha = o.response ? 1.0f - expf(-float(dt) / o.response) : 1.0f;
      o.position += (target - o.position) * alpha;
    }
  }
  unsigned duty(unsigned i) const {
    const auto &o = outputs[i];
    if (!o.enabled) return 0;
    float p = o.reverse ? 1000 - o.position : o.position;
    p = fmaxf(0, fminf(1000, p));
    return unsigned((o.low + (o.high - o.low) * p / 1000) * 4095 / 1000);
  }
};
// Oversized/invalid lines are discarded through newline, never parsed as suffixes.
struct Framer {
  char line[128]; unsigned length = 0; bool dropping = false;
  template<class Callback> void feed(char c, Callback callback) {
    if (c == '\n') {
      if (!dropping && length) { line[length] = 0; callback(line); }
      length = 0; dropping = false;
    } else if (c != '\r') {
      if (c < 32 || c > 126 || length >= sizeof(line)-1) dropping = true;
      if (!dropping) line[length++] = c;
    }
  }
};
}
