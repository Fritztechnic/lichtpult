#pragma once

// WLAN-Zugang: in secrets.h eintragen (siehe secrets.example.h, nicht einchecken).
#include "secrets.h"

// DMX-Ausgang ohne RS-485-Treiber: symmetrisches Signal aus zwei Pins.
// P -> XLR Pin 3 (Data +), N -> XLR Pin 2 (Data -, invertiert), GND -> XLR Pin 1.
#define DMX_PIN_P 18
#define DMX_PIN_N 19

// Art-Net: Lichtpult sendet Universum 1 als Art-Net-Universum 0.
#define ARTNET_UNIVERSE 0
