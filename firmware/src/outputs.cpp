#include "outputs.h"
#include "board.h"
#include "senses.h"
#include <time.h>
#include <esp_timer.h>

static constexpr uint8_t PINS[] = {16, 17, 18, 19, 21, 22};
void outputs::write() {
  for (unsigned i = 0; i < 6; i++)
    ledcWrite(i, board::state.engine.duty(i));
}
void outputs::sample() {
  time_t now = time(nullptr);
  bool clockValid = now > 1700000000;
  for (unsigned i = 0; i < 6; i++) {
    auto &out = board::state.engine.channels[i];
    const auto &s = board::state.sources[i];
    if (s.startsWith("time_")) {
      out.available =
          clockValid && (s == "time_day" || s == "time_hours" || s == "time_minutes" || s == "time_seconds");
      char unit = 'd';
      if (s == "time_hours")
        unit = 'h';
      else if (s == "time_minutes")
        unit = 'm';
      else if (s == "time_seconds")
        unit = 's';
      out.target = gauge::clockValue(unit, now, board::state.timeOffset);
    } else if (s.startsWith("esp_")) {
      float value = 0;
      out.available = senses::sample(s, value);
      out.target = constrain(
          (value - board::state.inputMin[i]) / (board::state.scales[i] - board::state.inputMin[i]), 0, 1);
    } else if (s.startsWith("wave_")) {
      out.available = s == "wave_sine" || s == "wave_triangle" || s == "wave_saw" || s == "wave_square";
      const char shape = s == "wave_triangle" ? 't' : s == "wave_saw" ? 'r' : s == "wave_square" ? 'q' : 's';
      out.target = gauge::waveform(shape, esp_timer_get_time() / 1000000.0, board::state.periods[i],
                                   board::state.phases[i]);
    } else if (s == "constant") {
      out.available = true;
      out.target = constrain(board::state.scales[i] / 100, 0, 1);
    } else if (!board::state.hostLive) {
      out.available = false;
      out.target = 0;
    }
    if (board::state.paused) {
      out.available = false;
      out.target = 0;
    }
  }
}

void outputs::begin() {
  for (unsigned i = 0; i < 6; i++) {
    pinMode(PINS[i], OUTPUT);
    digitalWrite(PINS[i], LOW);
    ledcSetup(i, 5000, 12);
    ledcAttachPin(PINS[i], i);
    ledcWrite(i, 0);
  }
}

void outputs::tick() {
  auto now = millis();
  if (board::state.hostLive && uint32_t(now - board::state.hostAt) > gauge::HOST_TIMEOUT) {
    board::state.hostLive = false;
    board::state.paused = false;
  }
  static uint32_t previous = 0;
  uint32_t dt = now - previous;
  if (dt >= 10) {
    previous = now;
    outputs::sample();
    board::state.engine.tick(now, dt);
    outputs::write();
  }
  bool wifi = false, ble = false;
  for (unsigned i = 0; i < 6; i++)
    if (board::state.engine.channels[i].enabled) {
      wifi |= board::state.sources[i] == "esp_wifi";
      ble |= board::state.sources[i] == "esp_ble";
    }
  // Radio work is deferred during live calibration to keep the slider responsive.
  if (board::state.engine.calibration < 0)
    senses::tick(wifi, ble);
}
