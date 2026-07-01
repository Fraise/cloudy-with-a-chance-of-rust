# cloudy-with-a-chance-of-rust

## E-paper

Resolution: 250x122

### Pinout

Reference: https://esp32.implrust.com/e-ink/circuit.html

| E-Paper    | ESP32      |
|------------|------------|
| VCC        | 3.3V       |
| GND        | GND        |
| DIN (MOSI) | IO23       |
| CLK (SCK)  | IO18       |
| CS         | IO10       |
| DC         | RXD (IO17) |
| RST        | TXD (IO16) |
| BUSY       | IO22       |

## References

- [Waveshare ESP32-C6 dev kit doc](https://docs.waveshare.com/ESP32-C6-DEV-KIT-N8)
- [Waveshare e-Paper doc](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT_(B)_Manual)
- [Wifi connection example](https://github.com/esp-rs/esp-hal/blob/esp-hal-v1.1.0/examples/wifi/embassy_dhcp/src/main.rs)
