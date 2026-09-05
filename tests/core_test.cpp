#include "gauge_core.h"
#include <cassert>
#include <string>
int main() {
  gauge::Controller c;
  for (unsigned i=0; i<6; ++i) assert(c.duty(i)==0);
  assert(c.command("C 0 1 100 800 0 250 500 1", 0));
  assert(c.command("V 1000 0 0 0 0 0", 0));
  c.tick(500, 500);
  assert(c.outputs[0].position > 700 && c.outputs[0].position < 730);
  c.tick(6000, 5500);
  assert(!c.live && c.outputs[0].position < 251);
  assert(c.command("C 0 1 100 800 1 0 0 1", 0));
  c.tick(0,20);
  assert(c.duty(0)==3276);
  assert(c.command("V 1000 0 0 0 0 0", UINT32_MAX-10));
  c.tick(10,20); assert(c.live); // wraparound
  assert(c.duty(0)==409);
  for (const char* bad : {"V 1001 0 0 0 0 0", "V -1 0 0 0 0 0", "V 1 2", "V 1 2 3 4 5 6 x", "T 0", "T 60001", "C 6 1 0 800 0 0 0 1", "C 0 1 800 100 0 0 0 1", "C 0 1 0 881 0 0 0 1", "C 0 1 0 880 0 0 0 2", "V 99999999999999999999 0 0 0 0 0"}) assert(!c.command(bad, 0));
  assert(c.outputs[0].target==1000); // invalid values never partially apply
  assert(c.command("S",0)); assert(!c.live);
  assert(c.command("R",0)); assert(c.duty(0)==0);
  gauge::Framer f; int calls=0;
  auto callback=[&](const char* line){ ++calls; assert(std::string(line)=="H"); };
  for(char ch: std::string(200,'x')+"H\nH\r\n") f.feed(ch,callback);
  assert(calls==1);
}
