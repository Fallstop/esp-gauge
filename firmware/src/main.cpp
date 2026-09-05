#include <Arduino.h>
#include "gauge_core.h"

// Board-specific adapter. pins.md and the schematic call these PWM1–PWM6.
constexpr uint8_t PINS[] = {16, 17, 18, 19, 21, 22};
gauge::Controller controller;
gauge::Framer framer;

void writeOutputs() {
  for (unsigned i = 0; i < gauge::COUNT; ++i) {
#if ESP_ARDUINO_VERSION_MAJOR >= 3
    ledcWrite(PINS[i], controller.duty(i));
#else
    ledcWrite(i, controller.duty(i));
#endif
  }
}
void setup() {
  Serial.begin(115200);
  for (unsigned i = 0; i < gauge::COUNT; ++i) {
#if ESP_ARDUINO_VERSION_MAJOR >= 3
    if (!ledcAttach(PINS[i], 5000, 12)) { while (true) delay(1000); }
#else
    ledcSetup(i, 5000, 12);
    ledcAttachPin(PINS[i], i);
#endif
  }
  writeOutputs(); // no sweep or surprise full deflection at startup
}
void loop() {
  // Bound serial work so malformed floods cannot starve timeout/smoothing.
  for (unsigned i = 0; i < 128 && Serial.available(); ++i) {
    framer.feed(Serial.read(), [](const char *line) {
      if (!strcmp(line, "H")) { Serial.println("ESPGAUGE 1 6 880"); return; }
      bool ok = controller.command(line, millis());
      if (ok && !strcmp(line, "R")) writeOutputs();
      Serial.println(ok ? "OK" : "ERR");
    });
  }
  static uint32_t previous = 0;
  uint32_t now = millis(), dt = now - previous;
  if (dt >= 20) {
    previous = now;
    controller.tick(now, dt);
    writeOutputs();
  }
  delay(2);
}
