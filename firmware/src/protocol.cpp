#include "protocol.h"
#include "board.h"
#include "configuration.h"
#include "outputs.h"
#include "senses.h"
#include "version.h"
#include <Preferences.h>
#include <sys/time.h>

static char frame[6144];
static size_t frameLength = 0;
static bool overflow = false;

static void status(JsonObject out, bool includeNetworks = false) {
  auto duty = out["duties"].to<JsonArray>();
  auto position = out["positions"].to<JsonArray>();
  auto valid = out["available"].to<JsonArray>();
  for (unsigned i = 0; i < 6; i++) {
    duty.add(board::state.engine.duty(i));
    position.add(board::state.engine.channels[i].position);
    valid.add(board::state.engine.channels[i].available);
  }
  out["calibrating"] = board::state.engine.calibration;
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
    board::state.handshake = true;
    reply["product"] = "ESP Gauge";
    reply["protocol"] = 2;
    reply["firmware"] = FIRMWARE_VERSION;
    reply["channels"] = 6;
    reply["max_duty"] = gauge::MAX_DUTY;
    reply["device"] = board::state.deviceId;
    reply["config_error"] = board::state.configError;
  } else if (!board::state.handshake) {
    error = "Identify the board first";
  } else if (!strcmp(op, "get_config")) {
    if (board::state.configError)
      error = "Stored configuration is invalid or newer than this firmware; it has been preserved";
    else
      reply["config"] = board::state.config.as<JsonObject>();
  } else if (!strcmp(op, "config")) {
    if (!configuration::valid(cmd["config"]))
      error = "Invalid gauge configuration";
    else {
      error = configuration::store(cmd["config"]);
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
      if (board::state.timeOffset != next) {
        board::state.timeOffset = next;
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
      board::state.hostAt = millis();
      board::state.hostLive = true;
      board::state.paused = cmd["paused"];
      for (unsigned i = 0; i < 6; i++)
        if (!board::state.sources[i].startsWith("time_") && !board::state.sources[i].startsWith("esp_") &&
            board::state.sources[i] != "constant") {
          board::state.engine.channels[i].target = values[i].isNull() ? 0 : values[i].as<float>();
          board::state.engine.channels[i].available = !values[i].isNull();
        }
      outputs::sample();
      status(reply.as<JsonObject>(), cmd["include_networks"] | false);
    }
  } else if (!strcmp(op, "status")) {
    status(reply.as<JsonObject>(), true);
  } else if (!strcmp(op, "calibrate")) {
    if (!cmd["port"].is<unsigned>() || !cmd["duty"].is<unsigned>() ||
        !board::state.engine.calibrate(cmd["port"], cmd["duty"], millis()))
      error = "Invalid calibration output";
    else
      outputs::write();
  } else if (!strcmp(op, "calibrate_end")) {
    board::state.engine.endCalibration();
    outputs::write();
  } else if (!strcmp(op, "pause")) {
    if (!cmd["paused"].is<bool>())
      error = "Invalid pause state";
    else {
      board::state.paused = cmd["paused"];
      board::state.hostAt = millis();
      board::state.hostLive = true;
      if (board::state.paused)
        board::state.engine.endCalibration();
      outputs::sample();
      outputs::write();
    }
  } else if (!strcmp(op, "release")) {
    board::state.hostLive = false;
    board::state.paused = false;
    board::state.engine.endCalibration();
    outputs::sample();
    outputs::write();
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

void protocol::begin() {
  Serial.begin(115200);
  char id[17];
  snprintf(id, sizeof(id), "%012llX", ESP.getEfuseMac());
  board::state.deviceId = id;
}

void protocol::tick() {
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
}
