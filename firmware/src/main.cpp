#include <Arduino.h>
#include "board.h"
#include "configuration.h"
#include "outputs.h"
#include "protocol.h"
#include "indicator.h"
#include "senses.h"

board::State board::state;

void setup() {
  outputs::begin();
  indicator::begin();
  protocol::begin();
  configuration::begin();
  senses::begin();
}

void loop() {
  protocol::tick();
  outputs::tick();
  indicator::tick();
  delay(1);
}
