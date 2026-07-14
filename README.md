# cloudy-with-a-chance-of-rust

A weather-station firmware written in Rust for the **ESP32-C6**. It connects to
Wi-Fi, fetches the current forecast from the [WeatherAPI.com](https://www.weatherapi.com/)
service, and renders the result on a **Waveshare 2.13" e-Paper HAT (B)**
(250×122, black & white).

The runtime is built on [embassy](https://embassy.dev/) (async/await on the
ESP32-C6), the HTTP/TLS stack is [reqwless](https://crates.io/crates/reqwless)
backed by `embedded-tls`, and the framebuffer is drawn with
[embedded-graphics](https://github.com/embedded-graphics/embedded-graphics).
Weather condition icons are pre-rasterized BMPs bundled in `icons/`.

## Required hardware

- **[Waveshare ESP32-C6-DevKit-N8](https://docs.waveshare.com/ESP32-C6-DevKit-N8)**
  (or any ESP32-C6 board with Wi-Fi and an exposed SPI-capable GPIO header).
- **[Waveshare 2.13" e-Paper HAT (B)](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT_(B)_Manual)**,
  250×122 resolution, black & white.
- A USB cable to power the board and flash firmware.

The default pinout assumes the Waveshare ESP32-C6 dev kit. The display is
wired to the ESP32 over SPI plus three GPIO control lines.


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

## Compile and build

### 1. Install the Rust toolchain

The project pins its toolchain in `rust-toolchain.toml`, so once `rustup` is
installed the right toolchain and target are picked up automatically. The
required setup is:

```bash
# Install rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install the stable Rust toolchain
rustup install stable

# Add the RISC-V target used by the ESP32-C6
rustup target add riscv32imac-unknown-none-elf

# Install the rust-src component (required for build-std / core+alloc)
rustup component add rust-src
```

The matching `rust-toolchain.toml` will then activate stable + the
`riscv32imac-unknown-none-elf` target + `rust-src` on every `cargo` invocation
in this directory.

### 2. Set the required environment variables

The firmware reads three values at compile time from the environment, so they
must be exported in the same shell where you run `cargo`:

| Variable   | Purpose                                                         |
|------------|-----------------------------------------------------------------|
| `SSID`     | Wi-Fi network name (2.4 GHz)                                    |
| `PASSWORD` | Wi-Fi WPA2 passphrase                                           |
| `API_KEY`  | WeatherAPI.com API key ([sign up](https://www.weatherapi.com/)) |

```bash
export SSID="your-wifi-ssid"
export PASSWORD="your-wifi-password"
export API_KEY="your-weatherapi-key"
```

A free WeatherAPI.com tier is sufficient for the current forecast endpoint
used by this project.

### 3. Build

```bash
# Debug build (smaller compile time, larger binary)
cargo build

# Release build (LTO, opt-level "s", single codegen unit — slower to compile,
# fits better in flash and runs cooler on the ESP32-C6)
cargo build --release
```

The ELF lands at `target/riscv32imac-unknown-none-elf/{debug,release}/cloudy-with-a-chance-of-rust`.

## Flashing and running

The project's `.cargo/config.toml` is configured to use
[probe-rs](https://probe.rs/) as the runner, so the simplest way to flash and
run is via `cargo run`. Install it first:

```bash
# Install probe-rs (provides the `probe-rs` CLI used as the cargo runner)
curl -fsSL https://probe.rs/install | sh
```

Connect the ESP32-C6 over USB, then:

```bash
# Flash and start the firmware (release build recommended for actual use)
cargo run --release

# Or just flash without launching a debug session
probe-rs run --chip=esp32c6 target/riscv32imac-unknown-none-elf/release/cloudy-with-a-chance-of-rust
```

`probe-rs` uses the on-board USB-serial/JTAG bridge on the ESP32-C6 — no
external programmer is needed. The default config also passes
`--preverify`, `--always-print-stacktrace`, and `--no-location`, which makes
panics easier to read in the RTT log.

If you prefer [espflash](https://github.com/esp-rs/espflash):

```bash
cargo install espflash
espflash flash --chip esp32c6 target/riscv32imac-unknown-none-elf/release/cloudy-with-a-chance-of-rust
```

## Project layout

```
src/
  main.rs            - entry point, Wi-Fi bring-up, task spawning
  lib.rs             - shared modules
  display/           - embedded-graphics framebuffer + e-paper driver
  weatherapi/        - WeatherAPI.com client (reqwless + serde)
    condition_icons.rs - maps WeatherAPI condition codes to icon BMPs
  wifi/              - embassy-net Wi-Fi configuration helpers
  icons/             - generated icon index module
icons/               - 20 weather condition BMPs (250x122, B/W)
build.rs             - esp-idf app descriptor + metadata
```

## References

- [Waveshare ESP32-C6 dev kit doc](https://docs.waveshare.com/ESP32-C6-DEV-KIT-N8)
- [Waveshare e-Paper doc](https://www.waveshare.com/wiki/2.13inch_e-Paper_HAT_(B)_Manual)
- [Wifi connection example](https://github.com/esp-rs/esp-hal/blob/esp-hal-v1.1.0/examples/wifi/embassy_dhcp/src/main.rs)
- [probe-rs](https://probe.rs/) — runner used to flash and debug

## Weather API

https://www.weatherapi.com/docs/
