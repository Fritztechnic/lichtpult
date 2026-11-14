// Art-Net -> DMX512 Node für den ESP32 (ein Universum).
#include <Arduino.h>
#include <WiFi.h>
#include <WiFiUdp.h>
#include "soc/gpio_sig_map.h"
#include "esp_rom_gpio.h"
#include "config.h"

static uint8_t dmx[513]; // [0] = Startcode
static WiFiUDP udp;
static uint8_t packet[600];

static void dmxBegin() {
  Serial2.begin(250000, SERIAL_8N2, -1, DMX_PIN_P);
  // Zweiter Pin trägt dasselbe Signal invertiert -> symmetrisches DMX ohne MAX485.
  esp_rom_gpio_pad_select_gpio(DMX_PIN_N);
  pinMode(DMX_PIN_N, OUTPUT);
  esp_rom_gpio_connect_out_signal(DMX_PIN_N, U2TXD_OUT_IDX, true, false);
}

static void dmxSend() {
  // Break: ein 0-Byte bei niedriger Baudrate hält die Leitung ca. 100 µs auf Low.
  Serial2.updateBaudRate(90000);
  Serial2.write((uint8_t)0);
  Serial2.flush();
  Serial2.updateBaudRate(250000);
  Serial2.write(dmx, sizeof dmx);
  Serial2.flush();
}

static void artnetPoll() {
  int n = udp.parsePacket();
  if (n < 18 || n > (int)sizeof packet) return;
  udp.read(packet, n);
  if (memcmp(packet, "Art-Net\0", 8) != 0 || packet[8] != 0x00 || packet[9] != 0x50) return;
  uint16_t universe = packet[14] | (packet[15] << 8);
  uint16_t len = (packet[16] << 8) | packet[17];
  if (universe != ARTNET_UNIVERSE || len > 512 || n < 18 + len) return;
  memcpy(dmx + 1, packet + 18, len);
}

void setup() {
  Serial.begin(115200);
  dmxBegin();
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASS);
  while (WiFi.status() != WL_CONNECTED) {
    dmxSend();
    delay(25);
  }
  Serial.printf("WLAN verbunden, IP %s\n", WiFi.localIP().toString().c_str());
  udp.begin(6454);
}

void loop() {
  artnetPoll();
  static uint32_t last;
  if (millis() - last >= 25) { // ca. 40 Hz
    last = millis();
    dmxSend();
  }
}
