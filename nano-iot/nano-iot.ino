#include <WiFiNINA.h>

#define NUCLEO_SERIAL Serial1

char ssid[] = "Jonesies";              // your network SSID (name)
char pass[] = "a88b90c91d";            // your network password (use for WPA, or use as key for WEP)
int status = WL_IDLE_STATUS;           // the Wi-Fi radio's status
int ledState = LOW;                    //ledState used to set the LED
unsigned long previousMillisInfo = 0;  //will store last time Wi-Fi information was updated
unsigned long previousMillisLED = 0;   // will store the last time LED was updated
const int intervalInfo = 5000;         // interval at which to update the board information

void toggleLED() {
  ledState = ledState == LOW ? HIGH : LOW;

  digitalWrite(LED_BUILTIN, ledState);
}
void LEDOff() {
  digitalWrite(LED_BUILTIN, LOW);
}
void LEDOn() {
  digitalWrite(LED_BUILTIN, HIGH);
}

void setup() {
  NUCLEO_SERIAL.begin(9600);
  Serial1.setTimeout(-1);

  // set the LED as output
  pinMode(LED_BUILTIN, OUTPUT);

  LEDOn();
  delay(1000);
  LEDOff();

  // attempt to connect to Wi-Fi network:
  while (status != WL_CONNECTED) {
    Serial.print("Attempting to connect to network: ");
    Serial.println(ssid);
    // Connect to WPA/WPA2 network:
    status = WiFi.begin(ssid, pass);

    // wait 10 seconds for connection:
    delay(2000);
  }

  // you're connected now, so print out the data:
  Serial.println("You're connected to the network");
  Serial.println("---------------------------------------");

  LEDOn();
  delay(100);
  LEDOff();
  delay(100);
  LEDOn();
  delay(100);
  LEDOff();
  delay(100);
}

void loop() {
  if (NUCLEO_SERIAL.available()) {
    char b = 0;
    NUCLEO_SERIAL.readBytes(&b, 1);

    switch (b) {
      case 0b00000000:
        LEDOff();
        break;
      case 0b00000001:
        LEDOn();
        break;
    }
  }
  //if (NUCLEO_SERIAL.available()) {
  //  String request = NUCLEO_SERIAL.readStringUntil('\n');
  //  sendHttpRequest(request);
  //}
}

//void sendHttpRequest(String url) {
//  WiFiClient client;
//  if (client.connect("jjthejjpro.com", 80)) {
//    client.println("GET " + url + " HTTP/1.1");
//    client.println("Host: your-api-endpoint.com");
//    client.println("Connection: close");
//    client.println();
//    while (client.connected() || client.available()) {
//      if (client.available()) {
//        char c = client.read();
//        NUCLEO_SERIAL.print(c);
//      }
//    }
//    client.stop();
//  }
//}