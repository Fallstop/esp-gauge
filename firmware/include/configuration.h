#pragma once
#include <ArduinoJson.h>
#include <Arduino.h>
namespace configuration {
bool valid(JsonVariantConst value);
void begin();
void apply();
String store(JsonVariantConst value);
} // namespace configuration
