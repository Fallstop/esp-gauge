#include <Arduino.h>
#include <ArduinoJson.h>
#include <Preferences.h>
#include <Adafruit_NeoPixel.h>
#include <sys/time.h>
#include "gauge_engine.h"
#include "senses.h"

static constexpr uint8_t PINS[] = {16, 17, 18, 19, 21, 22};
static gauge::Engine engine;
static Adafruit_NeoPixel pixel(1, 23, NEO_GRB + NEO_KHZ800);
static JsonDocument config;
static String configBlob, deviceId;
static bool configError = false, handshake = false, hostLive = false, paused = false;
static uint32_t hostAt = 0;
static int timeOffset = 0;
static char frame[6144];
static size_t frameLength = 0;
static bool overflow = false;
static String sources[6];
static float scales[6];

static bool validConfig(JsonVariantConst c) {
  if (!c.is<JsonObjectConst>() || c["version"] != 2 || !c["channels"].is<JsonArrayConst>() ||
      c["channels"].size() != 6 || measureJson(c) > 4096)
    return false;
  for (JsonObjectConst ch : c["channels"].as<JsonArrayConst>()) {
    if (!ch["enabled"].is<bool>() || !ch["reverse"].is<bool>() || !ch["name"].is<const char *>() ||
        strlen(ch["name"]) > 64 || !ch["source"].is<const char *>())
      return false;
    const char *source = ch["source"];
    size_t length = strlen(source);
    if (!length || length > 48)
      return false;
    for (size_t i = 0; i < length; i++)
      if (!isalnum(static_cast<unsigned char>(source[i])) && source[i] != '_')
        return false;
    if (!ch["max_duty"].is<unsigned>() || ch["max_duty"].as<unsigned>() > gauge::MAX_DUTY ||
        !ch["response_ms"].is<unsigned>() || ch["response_ms"].as<unsigned>() > 5000)
      return false;
    if (!ch["scale"].is<double>())
      return false;
    double scale = ch["scale"];
    if (!isfinite(scale) || scale <= 0 || scale > 1e9)
      return false;
  }
  return true;
}
static void applyConfig() {
  for (unsigned i = 0; i < 6; i++) {
    JsonObject c = config["channels"][i];
    auto &out = engine.channels[i];
    bool changed = sources[i] != (c["source"] | "");
    out.enabled = c["enabled"];
    out.maxDuty = c["max_duty"];
    out.response = c["response_ms"];
    out.reverse = c["reverse"];
    sources[i] = c["source"].as<String>();
    scales[i] = c["scale"];
    if (changed || !out.enabled) {
      out.position = 0;
      out.target = 0;
      out.available = false;
    }
  }
}
static void defaults() {
  config.clear();
  config["version"] = 2;
  auto array = config["channels"].to<JsonArray>();
  for (unsigned i = 0; i < 6; i++) {
    auto c = array.add<JsonObject>();
    c["enabled"] = false;
    c["name"] = "";
    c["source"] = "cpu";
    c["max_duty"] = 0;
    c["response_ms"] = 500;
    c["scale"] = 100;
    c["reverse"] = false;
  }
  configBlob = "";
  serializeJson(config, configBlob);
  applyConfig();
}
static void writeOutputs() {
  for (unsigned i = 0; i < 6; i++)
    ledcWrite(i, engine.duty(i));
}
static void sampleSources() {
  time_t now = time(nullptr);
  bool clockValid = now > 1700000000;
  for (unsigned i = 0; i < 6; i++) {
    auto &out = engine.channels[i];
    const auto &s = sources[i];
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
      out.target = gauge::clockValue(unit, now, timeOffset);
    } else if (s.startsWith("esp_")) {
      float value = 0;
      out.available = senses::sample(s, value);
      out.target = constrain(value / scales[i], 0, 1);
    } else if (s == "constant") {
      out.available = true;
      out.target = constrain(scales[i] / 100, 0, 1);
    } else if (!hostLive) {
      out.available = false;
      out.target = 0;
    }
    if (paused) {
      out.available = false;
      out.target = 0;
    }
  }
}
static void status(JsonObject out, bool includeNetworks = false) {
  auto duty = out["duties"].to<JsonArray>();
  auto position = out["positions"].to<JsonArray>();
  auto valid = out["available"].to<JsonArray>();
  for (unsigned i = 0; i < 6; i++) {
    duty.add(engine.duty(i));
    position.add(engine.channels[i].position);
    valid.add(engine.channels[i].available);
  }
  out["calibrating"] = engine.calibration;
  out["clock_valid"] = time(nullptr) > 1700000000;
  out["uptime"] = millis() / 1000;
  senses::status(out, includeNetworks);
  out["free_heap"] = ESP.getFreeHeap();
}
static void command(const char *line) {
  JsonDocument cmd, reply;
  auto err = deserializeJson(cmd, line);
  if (err)
    return;
  if (!cmd["id"].is<uint32_t>() || !cmd["op"].is<const char *>())
    return;
  reply["id"] = cmd["id"];
  reply["ok"] = false;
  const char *op = cmd["op"];
  String error;
  if (!strcmp(op, "hello")) {
    handshake = true;
    reply["product"] = "ESP Gauge";
    reply["protocol"] = 2;
    reply["firmware"] = "2.0.0";
    reply["channels"] = 6;
    reply["max_duty"] = gauge::MAX_DUTY;
    reply["device"] = deviceId;
    reply["config_error"] = configError;
  } else if (!handshake) {
    error = "Identify the board first";
  } else if (!strcmp(op, "get_config")) {
    if (configError)
      error = "Stored configuration is invalid or newer than this firmware; it has been preserved";
    else
      reply["config"] = config.as<JsonObject>();
  } else if (!strcmp(op, "config")) {
    if (!validConfig(cmd["config"]))
      error = "Invalid gauge configuration";
    else {
      String blob;
      serializeJson(cmd["config"], blob);
      if (blob != configBlob) {
        Preferences p;
        if (!p.begin("gauge-v2", false))
          error = "Configuration storage unavailable";
        else {
          if (p.putString("config", blob) != blob.length())
            error = "Could not store configuration";
          p.end();
        }
        if (error.isEmpty()) {
          configBlob = blob;
          deserializeJson(config, blob);
          configError = false;
          applyConfig();
        }
      }
    }
  } else if (!strcmp(op, "time")) {
    if (!cmd["epoch"].is<int64_t>() || cmd["epoch"].as<int64_t>() < 1700000000 ||
        cmd["epoch"].as<int64_t>() > 4102444800LL || !cmd["offset"].is<int>() ||
        cmd["offset"].as<int>() < -50400 || cmd["offset"].as<int>() > 50400)
      error = "Invalid clock time";
    else {
      timeval tv = {};
      tv.tv_sec = cmd["epoch"].as<int64_t>();
      settimeofday(&tv, nullptr);
      int next = cmd["offset"];
      if (timeOffset != next) {
        timeOffset = next;
        Preferences p;
        if (p.begin("gauge-v2", false)) {
          p.putInt("offset", next);
          p.end();
        }
      }
    }
  } else if (!strcmp(op, "live")) {
    auto values = cmd["values"].as<JsonArray>();
    bool valid = values.size() == 6 && cmd["paused"].is<bool>();
    for (JsonVariant v : values)
      if (!v.isNull() &&
          (!v.is<double>() || !isfinite(v.as<double>()) || v.as<double>() < 0 || v.as<double>() > 1))
        valid = false;
    if (!valid)
      error = "Invalid live readings";
    else {
      hostAt = millis();
      hostLive = true;
      paused = cmd["paused"];
      for (unsigned i = 0; i < 6; i++)
        if (!sources[i].startsWith("time_") && !sources[i].startsWith("esp_") && sources[i] != "constant") {
          engine.channels[i].target = values[i].isNull() ? 0 : values[i].as<float>();
          engine.channels[i].available = !values[i].isNull();
        }
      sampleSources();
      status(reply.as<JsonObject>(), cmd["include_networks"] | false);
    }
  } else if (!strcmp(op, "status")) {
    status(reply.as<JsonObject>(), true);
  } else if (!strcmp(op, "calibrate")) {
    if (!cmd["port"].is<unsigned>() || !cmd["duty"].is<unsigned>() ||
        !engine.calibrate(cmd["port"], cmd["duty"], millis()))
      error = "Invalid calibration output";
    else
      writeOutputs();
  } else if (!strcmp(op, "calibrate_end")) {
    engine.endCalibration();
    writeOutputs();
  } else if (!strcmp(op, "pause")) {
    if (!cmd["paused"].is<bool>())
      error = "Invalid pause state";
    else {
      paused = cmd["paused"];
      hostAt = millis();
      hostLive = true;
      if (paused)
        engine.endCalibration();
      sampleSources();
      writeOutputs();
    }
  } else if (!strcmp(op, "release")) {
    hostLive = false;
    paused = false;
    engine.endCalibration();
    sampleSources();
    writeOutputs();
  } else if (!strcmp(op, "wifi_scan")) {
    senses::requestScan();
  } else if (!strcmp(op, "wifi")) {
    if (!cmd["ssid"].is<const char *>() || !cmd["password"].is<const char *>() ||
        !senses::wifi(cmd["ssid"].as<String>(), cmd["password"].as<String>()))
      error = "Wi-Fi needs a network name and an 8–63 character password (or an open network)";
  } else if (!strcmp(op, "wifi_forget")) {
    senses::forgetWifi();
  } else
    error = "Unknown operation";
  if (error.length())
    reply["error"] = error;
  else
    reply["ok"] = true;
  serializeJson(reply, Serial);
  Serial.print('\n');
}
void setup() {
  for (unsigned i = 0; i < 6; i++) {
    pinMode(PINS[i], OUTPUT);
    digitalWrite(PINS[i], LOW);
    ledcSetup(i, 5000, 12);
    ledcAttachPin(PINS[i], i);
    ledcWrite(i, 0);
  }
  pixel.begin();
  pixel.setPixelColor(0, pixel.Color(5, 7, 5));
  pixel.show();
  Serial.begin(115200);
  char id[17];
  snprintf(id, sizeof(id), "%012llX", ESP.getEfuseMac());
  deviceId = id;
  defaults();
  Preferences p;
  if (p.begin("gauge-v2", true)) {
    String blob = p.getString("config", "");
    timeOffset = p.getInt("offset", 0);
    p.end();
    if (blob.length()) {
      JsonDocument saved;
      if (deserializeJson(saved, blob) || !validConfig(saved.as<JsonVariantConst>()))
        configError = true;
      else {
        configBlob = blob;
        config.set(saved);
        applyConfig();
      }
    }
  }
  senses::begin();
}
void loop() {
  for (unsigned n = 0; n < 512 && Serial.available(); n++) {
    char c = char(Serial.read());
    if (c == '\n') {
      if (!overflow && frameLength) {
        frame[frameLength] = 0;
        command(frame);
      }
      frameLength = 0;
      overflow = false;
    } else if (c != '\r') {
      if (frameLength < sizeof(frame) - 1 && !overflow)
        frame[frameLength++] = c;
      else
        overflow = true;
    }
  }
  auto now = millis();
  if (hostLive && uint32_t(now - hostAt) > gauge::HOST_TIMEOUT) {
    hostLive = false;
    paused = false;
  }
  static uint32_t previous = 0, ledAt = 0;
  uint32_t dt = now - previous;
  if (dt >= 10) {
    previous = now;
    sampleSources();
    engine.tick(now, dt);
    writeOutputs();
  }
  bool wifi = false, ble = false, standalone = false;
  for (unsigned i = 0; i < 6; i++)
    if (engine.channels[i].enabled) {
      wifi |= sources[i] == "esp_wifi";
      ble |= sources[i] == "esp_ble";
      standalone |=
          sources[i].startsWith("esp_") || sources[i].startsWith("time_") || sources[i] == "constant";
    }
  // Radio work is deferred during live calibration to keep the slider responsive.
  if (engine.calibration < 0)
    senses::tick(wifi, ble);
  if (uint32_t(now - ledAt) > 50) {
    ledAt = now;
    float breath = .65f + .35f * sinf(now / 1800.0f);
    uint8_t r = 5, g = 7, b = 5;
    if (configError) {
      r = 35;
      g = 1;
      b = 1;
    } else if (engine.calibration >= 0) {
      r = 35;
      g = 16;
      b = 2;
    } else if (hostLive) {
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
  delay(1);
}
