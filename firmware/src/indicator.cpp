#include "indicator.h"
#include "board.h"
#include <Adafruit_NeoPixel.h>

static Adafruit_NeoPixel pixel(1, 23, NEO_GRB + NEO_KHZ800);
void indicator::begin() {
  pixel.begin();
  pixel.setPixelColor(0, pixel.Color(5, 7, 5));
  pixel.show();
}
void indicator::tick() {
  const auto now = millis();
  static uint32_t ledAt = 0;
  bool standalone = false;
  for (unsigned i = 0; i < 6; ++i)
    standalone |=
        board::state.engine.channels[i].enabled &&
        (board::state.sources[i].startsWith("esp_") || board::state.sources[i].startsWith("wave_") ||
         board::state.sources[i].startsWith("time_") || board::state.sources[i] == "constant");
  if (uint32_t(now - ledAt) > 50) {
    ledAt = now;
    float breath = .65f + .35f * sinf(now / 1800.0f);
    uint8_t r = 5, g = 7, b = 5;
    if (board::state.configError) {
      r = 35;
      g = 1;
      b = 1;
    } else if (board::state.engine.calibration >= 0) {
      r = 35;
      g = 16;
      b = 2;
    } else if (board::state.hostLive) {
      r = 5;
      g = 24;
      b = 14;
    } else if (standalone) {
      r = 5;
      g = 12;
      b = 25;
    }
    pixel.setPixelColor(0, pixel.Color(r * breath, g * breath, b * breath));
    pixel.show();
  }
}
