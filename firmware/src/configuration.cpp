#include "configuration.h"
#include "board.h"
#include <Preferences.h>

bool configuration::valid(JsonVariantConst c) {
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
    if (!ch["min_duty"].isNull() &&
        (!ch["min_duty"].is<unsigned>() || ch["min_duty"].as<unsigned>() > ch["max_duty"].as<unsigned>()))
      return false;
    for (const char *field : {"period_s", "phase_deg", "input_min"})
      if (!ch[field].isNull() && (!ch[field].is<double>() || !isfinite(ch[field].as<double>())))
        return false;
    if ((!ch["period_s"].isNull() &&
         (ch["period_s"].as<double>() < .1 || ch["period_s"].as<double>() > 86400)) ||
        (!ch["phase_deg"].isNull() &&
         (ch["phase_deg"].as<double>() < 0 || ch["phase_deg"].as<double>() > 360)) ||
        (!ch["input_min"].isNull() && (fabs(ch["input_min"].as<double>()) > 1e9 ||
                                       ch["input_min"].as<double>() >= ch["scale"].as<double>())))
      return false;
    if (!ch["scale"].is<double>())
      return false;
    double scale = ch["scale"];
    if (!isfinite(scale) || scale <= 0 || scale > 1e9)
      return false;
  }
  return true;
}
void configuration::apply() {
  for (unsigned i = 0; i < 6; i++) {
    JsonObject c = board::state.config["channels"][i];
    auto &out = board::state.engine.channels[i];
    bool changed = board::state.sources[i] != (c["source"] | "");
    out.enabled = c["enabled"];
    out.minDuty = c["min_duty"] | 0;
    out.maxDuty = c["max_duty"];
    out.response = c["response_ms"];
    out.reverse = c["reverse"];
    board::state.sources[i] = c["source"].as<String>();
    board::state.scales[i] = c["scale"];
    board::state.inputMin[i] = c["input_min"] | 0.0f;
    board::state.periods[i] = c["period_s"] | 10.0f;
    board::state.phases[i] = c["phase_deg"] | 0.0f;
    if (changed || !out.enabled) {
      out.position = 0;
      out.target = 0;
      out.available = false;
    }
  }
}
static void defaults() {
  board::state.config.clear();
  board::state.config["version"] = 2;
  auto array = board::state.config["channels"].to<JsonArray>();
  for (unsigned i = 0; i < 6; i++) {
    auto c = array.add<JsonObject>();
    c["enabled"] = false;
    c["name"] = "";
    c["source"] = "cpu";
    c["min_duty"] = 0;
    c["max_duty"] = 0;
    c["response_ms"] = 500;
    c["scale"] = 100;
    c["reverse"] = false;
  }
  board::state.configBlob = "";
  serializeJson(board::state.config, board::state.configBlob);
  configuration::apply();
}

void configuration::begin() {
  defaults();
  Preferences p;
  if (p.begin("gauge-v2", true)) {
    String blob = p.getString("config", "");
    board::state.timeOffset = p.getInt("offset", 0);
    p.end();
    if (blob.length()) {
      JsonDocument saved;
      if (deserializeJson(saved, blob) || !configuration::valid(saved.as<JsonVariantConst>()))
        board::state.configError = true;
      else {
        board::state.configBlob = blob;
        board::state.config.set(saved);
        configuration::apply();
      }
    }
  }
}

String configuration::store(JsonVariantConst value) {
  String error;
  String blob;
  serializeJson(value, blob);
  if (blob != board::state.configBlob) {
    Preferences p;
    if (!p.begin("gauge-v2", false))
      error = "Configuration storage unavailable";
    else {
      if (p.putString("config", blob) != blob.length())
        error = "Could not store configuration";
      p.end();
    }
    if (error.isEmpty()) {
      board::state.configBlob = blob;
      deserializeJson(board::state.config, blob);
      board::state.configError = false;
      configuration::apply();
    }
  }
  return error;
}
