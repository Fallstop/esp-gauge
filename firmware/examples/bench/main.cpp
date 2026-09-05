// Bench test firmware for the ESP gauge board.
//
//  - Steps all six PWM gauge outputs together in 25% increments, up and back.
//  - Every output shares the same 95% of full-scale ceiling, so all six needles
//    should track each other exactly.
//  - The status NeoPixel tracks the ramp: brightness follows the level, red on
//    the way up and blue on the way down.

#include <Arduino.h>

// Pin map: see pins.md
static const uint8_t PWM_PINS[] = {16, 17, 18, 19, 21, 22};
static const uint8_t NEOPIXEL_PIN = 23;
static const size_t PWM_COUNT = sizeof(PWM_PINS) / sizeof(PWM_PINS[0]);

// Full deflection as a percentage of full duty: 95% is as far as the hardware
// usefully goes. Shared by every channel.
static const uint8_t PWM_MAX_PCT = 88;

static const uint32_t PWM_FREQ_HZ = 5000;
static const uint8_t PWM_RES_BITS = 12;
static const uint16_t PWM_FULL = (1u << PWM_RES_BITS) - 1;

// The ramp moves in coarse increments of each channel's own range. Fewer, bigger
// steps over the same cycle time means each one holds longer, so the needles
// settle and the movement reads as stepped rather than smooth.
static const uint8_t STEP_PCT = 25;
static const uint8_t STEP_COUNT = 100 / STEP_PCT;
static const uint32_t CYCLE_MS = 5000;
static const uint32_t STEP_HOLD_MS = CYCLE_MS / (2u * STEP_COUNT);

// Status pixel brightness at 0% and at 100% of the ramp.
static const uint8_t PIXEL_MIN = 5;
static const uint8_t PIXEL_MAX = 64;

// The LEDC and RGB LED helpers were renamed in Arduino-ESP32 3.x.
#if ESP_ARDUINO_VERSION_MAJOR >= 3
#define pwmWrite(index, duty) ledcWrite(PWM_PINS[index], (duty))
#define statusPixel(r, g, b) rgbLedWrite(NEOPIXEL_PIN, (r), (g), (b))
#else
#define pwmWrite(index, duty) ledcWrite((index), (duty))
#define statusPixel(r, g, b) neopixelWrite(NEOPIXEL_PIN, (r), (g), (b))
#endif

// Duty at 0..100% of the usable range.
static uint16_t scaledDuty(uint32_t percent) {
  uint32_t ceiling = ((uint32_t)PWM_FULL * PWM_MAX_PCT) / 100;
  return (uint16_t)((ceiling * percent) / 100);
}

static void attachOutputs() {
#if ESP_ARDUINO_VERSION_MAJOR >= 3
  for (size_t i = 0; i < PWM_COUNT; i++) {
    if (!ledcAttach(PWM_PINS[i], PWM_FREQ_HZ, PWM_RES_BITS)) {
      Serial.printf("PWM%u: failed to attach GPIO %u\n", (unsigned)(i + 1), PWM_PINS[i]);
    }
  }
#else
  for (size_t i = 0; i < PWM_COUNT; i++) {
    ledcSetup(i, PWM_FREQ_HZ, PWM_RES_BITS);
    ledcAttachPin(PWM_PINS[i], i);
  }
#endif
  for (size_t i = 0; i < PWM_COUNT; i++) {
    pwmWrite(i, 0);
  }
}

// Red, green, blue in turn, so a swapped colour order is visible on the pixel
// before the ramp starts using red and blue to mean something.
static void pixelSelfTest() {
  const uint8_t level = PIXEL_MAX;
  Serial.println("self-test pixel: red, green, blue");
  statusPixel(level, 0, 0);
  delay(400);
  statusPixel(0, level, 0);
  delay(400);
  statusPixel(0, 0, level);
  delay(400);
  statusPixel(0, 0, 0);
}

// Drive each channel to its own ceiling on its own, so a miswired or dead
// output shows up before the channels start moving together.
static void channelSelfTest() {
  Serial.printf("self-test PWMs -> %u%% of full scale\n", PWM_MAX_PCT);
  statusPixel(PIXEL_MIN, PIXEL_MIN, PIXEL_MIN);
  for (uint32_t p = 0; p <= 100; p += 2) {
    for (size_t i = 0; i < PWM_COUNT; i++) {
      pwmWrite(i, scaledDuty(p));
    }
    delay(15);
  }
  delay(200);
  for (int32_t p = 100; p >= 0; p -= 2) {
    for (size_t i = 0; i < PWM_COUNT; i++) {
      pwmWrite(i, scaledDuty((uint32_t)p));
    }
    delay(15);
  }
  for (size_t i = 0; i < PWM_COUNT; i++) {
    pwmWrite(i, 0);
  }
  statusPixel(0, 0, 0);
  delay(150);
}

void setup() {
  Serial.begin(115200);
  delay(200);
  Serial.println();
  Serial.println("esp-gauge test firmware");
  Serial.printf("PWM: %u channels, %lu Hz, %u-bit, max %u%% (duty %u of %u)\n",
                (unsigned)PWM_COUNT, (unsigned long)PWM_FREQ_HZ, PWM_RES_BITS, PWM_MAX_PCT,
                scaledDuty(100), PWM_FULL);

  attachOutputs();
  pixelSelfTest();
  channelSelfTest();

  Serial.printf("running: synchronised ramp in %u%% steps, %lu ms per step\n", STEP_PCT,
                (unsigned long)STEP_HOLD_MS);
}

void loop() {
  // Triangle over 2 * STEP_COUNT ticks: 0..4 up, then 3..1 back down.
  const uint32_t tick = (millis() / STEP_HOLD_MS) % (2u * STEP_COUNT);
  const uint32_t level = (tick <= STEP_COUNT) ? tick : (2u * STEP_COUNT - tick);
  const bool rising = tick < STEP_COUNT;

  static uint32_t lastTick = UINT32_MAX;
  if (tick == lastTick) {
    delay(5);
    return;
  }
  lastTick = tick;

  const uint32_t percent = level * STEP_PCT;
  const uint16_t duty = scaledDuty(percent);
  for (size_t i = 0; i < PWM_COUNT; i++) {
    pwmWrite(i, duty);
  }

  const uint8_t brightness =
      PIXEL_MIN + (uint8_t)(((uint32_t)(PIXEL_MAX - PIXEL_MIN) * percent) / 100);
  if (rising) {
    statusPixel(brightness, 0, 0);
  } else {
    statusPixel(0, 0, brightness);
  }

  Serial.printf("step: %3lu%% %s\n", (unsigned long)percent, rising ? "rising" : "falling");
}
