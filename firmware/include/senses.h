#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>

namespace senses {
void begin();
void tick(bool wantWifi, bool wantBle);
void requestScan();
bool wifi(const String &ssid, const String &password);
void forgetWifi();
void status(JsonObject out, bool includeNetworks = false);
bool sample(const String &source, float &value);
} // namespace senses
