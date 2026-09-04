#include "gauge_core.h"
#include <iostream>
#include <string>
int main() {
  gauge::Controller controller;
  std::string line;
  while(std::getline(std::cin,line)) {
    if(line=="H") std::cout << "ESPGAUGE 1 6 880" << std::endl;
    else std::cout << (controller.command(line.c_str(),0) ? "OK" : "ERR") << std::endl;
  }
}
