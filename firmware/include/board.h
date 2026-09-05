#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include "gauge_engine.h"

namespace board {
struct State {
  gauge::Engine engine;
  JsonDocument config;
  String configBlob, deviceId;
  bool configError = false, handshake = false, hostLive = false, paused = false;
  uint32_t hostAt = 0;
  int timeOffset = 0;
  String sources[6];
  float scales[6] = {};
  float inputMin[6] = {}, periods[6] = {}, phases[6] = {};
};
extern State state;
} // namespace board
