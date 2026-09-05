#include "senses.h"
#include <WiFi.h>
#include <Preferences.h>
#include <esp_bt.h>
#include <esp_bt_main.h>
#include <esp_gap_ble_api.h>
#include <atomic>

// Arduino otherwise releases Bluetooth RAM before a source can be selected.
extern "C" bool btInUse() { return true; }

namespace senses {
static bool scanning = false, requested = false, bleStarted = false, bleAttempted = false;
static std::atomic<bool> bleScanning{false};
static int wifiCount = -1;
static std::atomic<int> bleCount{-1};
static uint32_t lastWifi = 0, lastBle = 0, lastTemp = 0, lastConnect = 0;
static float chipTemperature = NAN;
static String ssid, password;
static JsonDocument networks;
static uint8_t addresses[128][6];
static unsigned addressCount = 0;
static portMUX_TYPE bleMux = portMUX_INITIALIZER_UNLOCKED;
static void gap(esp_gap_ble_cb_event_t event, esp_ble_gap_cb_param_t *p) {
  if (event == ESP_GAP_BLE_SCAN_PARAM_SET_COMPLETE_EVT) {
    esp_ble_gap_start_scanning(4);
    return;
  }
  if (event != ESP_GAP_BLE_SCAN_RESULT_EVT)
    return;
  portENTER_CRITICAL(&bleMux);
  if (p->scan_rst.search_evt == ESP_GAP_SEARCH_INQ_RES_EVT) {
    bool found = false;
    for (unsigned i = 0; i < addressCount; i++)
      if (!memcmp(addresses[i], p->scan_rst.bda, 6)) {
        found = true;
        break;
      }
    if (!found && addressCount < 128)
      memcpy(addresses[addressCount++], p->scan_rst.bda, 6);
  } else if (p->scan_rst.search_evt == ESP_GAP_SEARCH_INQ_CMPL_EVT) {
    bleCount = int(addressCount);
    bleScanning = false;
  }
  portEXIT_CRITICAL(&bleMux);
}
static bool startBle() {
  esp_bt_controller_mem_release(ESP_BT_MODE_CLASSIC_BT);
  esp_bt_controller_config_t cfg = BT_CONTROLLER_INIT_CONFIG_DEFAULT();
  cfg.mode = ESP_BT_MODE_BLE;
  if (esp_bt_controller_init(&cfg) != ESP_OK)
    return false;
  if (esp_bt_controller_enable(ESP_BT_MODE_BLE) != ESP_OK)
    return false;
  if (esp_bluedroid_init() != ESP_OK || esp_bluedroid_enable() != ESP_OK)
    return false;
  return esp_ble_gap_register_callback(gap) == ESP_OK;
}
void begin() {
  WiFi.persistent(false);
  WiFi.setAutoReconnect(true);
  Preferences p;
  if (p.begin("gauge-wifi", true)) {
    JsonDocument doc;
    if (!deserializeJson(doc, p.getString("credentials", "{}"))) {
      ssid = doc["ssid"] | "";
      password = doc["password"] | "";
    }
    p.end();
  }
  if (ssid.length()) {
    WiFi.mode(WIFI_STA);
    WiFi.begin(ssid.c_str(), password.c_str());
    configTime(0, 0, "pool.ntp.org", "time.cloudflare.com");
  }
}
bool wifi(const String &nextSsid, const String &nextPassword) {
  if (!nextSsid.length() || nextSsid.length() > 32 || nextPassword.length() > 63 ||
      (nextPassword.length() > 0 && nextPassword.length() < 8))
    return false;
  Preferences p;
  if (!p.begin("gauge-wifi", false))
    return false;
  // A single NVS blob is atomic, including both credentials.
  JsonDocument doc;
  doc["ssid"] = nextSsid;
  doc["password"] = nextPassword;
  String blob;
  serializeJson(doc, blob);
  bool ok = p.putString("credentials", blob) == blob.length();
  p.end();
  if (!ok)
    return false;
  ssid = nextSsid;
  password = nextPassword;
  WiFi.mode(WIFI_STA);
  WiFi.disconnect();
  WiFi.begin(ssid.c_str(), password.c_str());
  configTime(0, 0, "pool.ntp.org", "time.cloudflare.com");
  return true;
}
void forgetWifi() {
  Preferences p;
  if (p.begin("gauge-wifi", false)) {
    p.clear();
    p.end();
  }
  ssid = "";
  password = "";
  WiFi.disconnect(true);
}
void requestScan() {
  requested = true;
  lastWifi = millis() - 15001;
}
void tick(bool wantWifi, bool wantBle) {
  const auto now = millis();
  if ((wantWifi || requested) && !scanning && uint32_t(now - lastWifi) > 15000) {
    if (WiFi.getMode() == WIFI_OFF)
      WiFi.mode(WIFI_STA);
    WiFi.scanDelete();
    WiFi.scanNetworks(true, true, false, 300);
    scanning = true;
    requested = false;
    lastWifi = now;
  }
  if (scanning) {
    int count = WiFi.scanComplete();
    if (count >= 0) {
      wifiCount = count;
      networks.clear();
      auto a = networks.to<JsonArray>();
      for (int i = 0; i < count && i < 24; i++) {
        auto n = a.add<JsonObject>();
        n["ssid"] = WiFi.SSID(i);
        n["rssi"] = WiFi.RSSI(i);
        n["open"] = WiFi.encryptionType(i) == WIFI_AUTH_OPEN;
      }
      WiFi.scanDelete();
      scanning = false;
    } else if (count == WIFI_SCAN_FAILED) {
      scanning = false;
      wifiCount = -1;
    }
  }
  if (wantBle && !bleAttempted) {
    bleAttempted = true;
    bleStarted = startBle();
    lastBle = now;
    if (bleStarted) {
      esp_ble_scan_params_t p = {};
      p.scan_type = BLE_SCAN_TYPE_PASSIVE;
      p.own_addr_type = BLE_ADDR_TYPE_PUBLIC;
      p.scan_filter_policy = BLE_SCAN_FILTER_ALLOW_ALL;
      p.scan_interval = 0x80;
      p.scan_window = 0x40;
      p.scan_duplicate = BLE_SCAN_DUPLICATE_ENABLE;
      bleScanning = true;
      esp_ble_gap_set_scan_params(&p);
    }
  }
  if (wantBle && bleStarted && !bleScanning && uint32_t(now - lastBle) > 15000) {
    portENTER_CRITICAL(&bleMux);
    addressCount = 0;
    bleScanning = true;
    portEXIT_CRITICAL(&bleMux);
    lastBle = now;
    if (esp_ble_gap_start_scanning(4) != ESP_OK)
      bleScanning = false;
  }
  if (uint32_t(now - lastTemp) > 5000) {
    lastTemp = now;
    chipTemperature = temperatureRead();
  }
  if (ssid.length() && WiFi.status() != WL_CONNECTED && uint32_t(now - lastConnect) > 30000) {
    lastConnect = now;
    WiFi.reconnect();
  }
}
void status(JsonObject out, bool includeNetworks) {
  out["wifi_connected"] = WiFi.status() == WL_CONNECTED;
  out["ssid"] = ssid;
  out["wifi_count"] = wifiCount;
  out["ble_count"] = bleCount.load();
  out["scanning"] = scanning;
  if (includeNetworks)
    out["networks"] = networks.as<JsonArray>();
  if (isfinite(chipTemperature))
    out["temperature"] = chipTemperature;
  if (WiFi.status() == WL_CONNECTED)
    out["rssi"] = WiFi.RSSI();
}
bool sample(const String &source, float &value) {
  if (source == "esp_wifi") {
    value = wifiCount;
    return wifiCount >= 0;
  }
  if (source == "esp_ble") {
    portENTER_CRITICAL(&bleMux);
    value = bleCount;
    portEXIT_CRITICAL(&bleMux);
    return value >= 0;
  }
  if (source == "esp_temperature") {
    value = chipTemperature;
    return isfinite(value);
  }
  if (source == "esp_rssi") {
    value = constrain(2 * (WiFi.RSSI() + 100), 0, 100);
    return WiFi.status() == WL_CONNECTED;
  }
  return false;
}
} // namespace senses
