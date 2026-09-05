#include "gauge_engine.h"
#include <cassert>
#include <cstdio>
int main() {
  gauge::Engine e;
  for (unsigned i = 0; i < 6; i++)
    assert(e.duty(i) == 0);
  assert(!e.calibrate(6, 10, 0));
  assert(!e.calibrate(5, 881, 0));
  assert(e.calibrate(5, 20, 10));
  assert(e.duty(5) == 81);
  for (unsigned i = 0; i < 5; i++)
    assert(e.duty(i) == 0);
  e.tick(1511, 10);
  assert(e.calibration == -1);
  assert(e.duty(5) == 0);
  assert(e.calibrate(5, 880, 0xfffffff0));
  e.tick(0x00000010, 32);
  assert(e.calibration == 5);
  e.tick(0x00000600, 1520);
  assert(e.calibration == -1);
  auto &c = e.channels[5];
  c.enabled = true;
  c.available = true;
  c.maxDuty = 100;
  c.target = 1;
  c.response = 0;
  e.tick(2000, 10);
  assert(e.duty(5) == 409);
  c.reverse = true;
  c.available = false;
  e.tick(2010, 10);
  assert(e.duty(5) == 0);
  c.available = true;
  c.reverse = false;
  c.response = 500;
  c.position = 0;
  e.tick(2020, 10);
  assert(c.position > 0 && c.position < .03);
  for (int i = 0; i < 1000; i++)
    e.tick(2030 + i * 10, 10);
  assert(c.position > .999 && e.duty(5) <= 409);
  c.target = 10;
  c.maxDuty = 65000;
  e.tick(13000, 10000);
  assert(e.duty(5) <= 880 * 4095 / 1000);
  assert(gauge::clockValue('d', 0, 0) == 0);
  assert(gauge::clockValue('d', 43200, 0) == .5f);
  assert(gauge::clockValue('h', 21600, 0) == .5f);
  assert(gauge::clockValue('m', 1800, 0) == .5f);
  assert(gauge::clockValue('s', 30, 0) == .5f);
  assert(gauge::clockValue('d', 43200, -43200) == 0);
  assert(gauge::clockValue('d', -1, 0) > .999f);
  puts("engine: boot, port isolation, limits, watchdog, rollover, smoothing, missing sources, reversal, "
       "clocks OK");
}
