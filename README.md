# ESP32 LED Matrix Controller

A Rust-based ESP32 application for controlling an 88x88 RGB LED matrix display with WiFi connectivity and HTTP web interface.

## Overview

This project implements a complete solution for driving the "bonk" LED matrix displays from Helsinki Hacklab. 
The ESP32 connects to your local WiFi network and serves a web page where you can input text to display on the LED matrix.

## Hardware Requirements

### LED Matrix Specifications

Based on the [Helsinki Hacklab documentation](https://wiki.helsinki.hacklab.fi/Ledimatriisin_ohjaaminen):

| Specification  | Value                               |
| -------------- | ----------------------------------- |
| Resolution     | 88 × 88 pixels                      |
| Pixel Type     | RGB LEDs                            |
| Color Depth    | 16-bit PWM per channel              |
| Multiplexing   | 11:1 (11 scanlines)                 |
| Control Chains | 6 parallel (R1, G1, B1, R2, G2, B2) |
| Power Supply   | 5V DC, up to 10A at full white      |
| Connector      | 34-pin (2×17) 0.1" pitch header     |

### Control Signals

The LED matrix requires 13 control signals:

| Signal        | Function                                          |
| ------------- | ------------------------------------------------- |
| GCLK          | Multiplex clock (~1 MHz, 256 pulses per scanline) |
| DCLK          | Data clock for shift registers                    |
| LE            | Latch Enable (combined with DCLK for commands)    |
| A0-A3         | Scanline address (4 bits, selects rows 0-10)      |
| DR1, DG1, DB1 | RGB data for chain 1 (rows 0-43)                  |
| DR2, DG2, DB2 | RGB data for chain 2 (rows 44-87)                 |

### Commands (via LE + DCLK pulses)

| Pulses | Command                          |
| ------ | -------------------------------- |
| 1      | Data Latch (strobe)              |
| 2      | VSYNC (buffer swap)              |
| 4      | Write Configuration Register     |
| 10     | Reset                            |
| 14     | Pre-Active (enable config write) |

## Wiring Diagram

### ESP32 to LED Matrix Connection

```
                     LED MATRIX (34-pin connector, viewed from back)
                     ┌─────────────────────────────────────────┐
                     │  ◄──── Pin 1 (top left when oriented)   │
                     │                                         │
    ESP32            │    PINOUT                               │
   ┌───────┐         │    ┌──────────────────────────────────┐ │
   │       │         │    │  1  GND    18  GND               │ │
   │ GPIO4 ├─────────┼────┤  2  GND    19  GND               │ │
   │       │   GCLK  │    │  3  +5V    20  +5V               │ │
   │ GPIO5 ├─────────┼────┤  4  +5V    21  +5V               │ │
   │       │   DCLK  │    │  5  +5V    22  +5V               │ │
   │ GPIO18├─────────┼────┤  6  +5V    23  +5V               │ │
   │       │    LE   │    │  7  +5V    24  +5V               │ │
   │ GPIO19├─────────┼────┤  8  +5V    25  +5V               │ │
   │       │    A0   │    │  9  A0     26  NC                │ │
   │ GPIO21├─────────┼────┤ 10  A1     27  NC                │ │
   │       │    A1   │    │ 11  A2     28  NC                │ │
   │ GPIO22├─────────┼────┤ 12  A3     29  NC                │ │
   │       │    A2   │    │ 13  GCLK   30  NC                │ │
   │ GPIO23├─────────┼────┤ 14  DCLK   31  LE                │ │
   │       │    A3   │    │ 15  DR1    32  DG1               │ │
   │ GPIO25├─────────┼────┤ 16  DB1    33  DR2               │ │
   │       │   DR1   │    │ 17  DG2    34  DB2               │ │
   │ GPIO26├─────────┼────┤                                  │ │
   │       │   DG1   │    └──────────────────────────────────┘ │
   │ GPIO27├─────────┼────┤   Note: Actual pinout may vary!    │
   │       │   DB1   │    │   Verify with your module!         │
   │ GPIO32├─────────┼────┤                                    │
   │       │   DR2   │    └────────────────────────────────────┘
   │ GPIO33├─────────┼────┤
   │       │   DG2   │     IMPORTANT:
   │ GPIO13├─────────┼────┤  - GPIO34-39 are INPUT ONLY!
   │       │   DB2   │       - Use GPIO13 for DB2 instead
   │  GND  ├─────────┼────┤  - Verify pinout from wiki
   │       │         │    │  - Connect all GND and +5V pins
   │  VIN  ├─────────┼────┤
   │       │  +5V    │    Power Requirements:
   └───────┘         │    - External 5V supply required
                     │    - Can draw up to 10A at full white
                     │    - ESP32 powered separately or via
                     │      buck converter from main supply
                     └─────────────────────────────────────────┘
```

### Verified Pin Mapping

| ESP32 GPIO | LED Matrix Signal | Notes                             |
| ---------- | ----------------- | --------------------------------- |
| GPIO4      | GCLK              | Multiplex clock output            |
| GPIO5      | DCLK              | Data clock output                 |
| GPIO18     | LE                | Latch Enable output               |
| GPIO19     | A0                | Address bit 0                     |
| GPIO21     | A1                | Address bit 1                     |
| GPIO22     | A2                | Address bit 2                     |
| GPIO23     | A3                | Address bit 3                     |
| GPIO25     | DR1               | Red data chain 1                  |
| GPIO26     | DG1               | Green data chain 1                |
| GPIO27     | DB1               | Blue data chain 1                 |
| GPIO32     | DR2               | Red data chain 2                  |
| GPIO33     | DG2               | Green data chain 2                |
| GPIO13     | DB2               | Blue data chain 2 (avoid GPIO34!) |
| GND        | GND               | Common ground                     |
| VIN/5V     | +5V               | External 5V supply                |

## Software Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Main Application                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   WiFi      │  │ HTTP Server │  │     LED Matrix Driver   │  │
│  │  Module     │  │   Module    │  │        Module           │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│         │               │                      │                │
│         ▼               ▼                      ▼                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ ESP-IDF     │  │  Web Page   │  │   Display Refresh       │  │
│  │ WiFi Stack  │  │  (HTML/JS)  │  │   (Multiplex + Data)    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│         │               │                      │                │
│         └───────────────┴──────────────────────┘                │
│                         │                                       │
│                         ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    ESP32 Hardware                           ││
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     ││
│  │  │   WiFi   │  │   GPIO   │  │  Timers  │  │   RAM    │     ││
│  │  │  Radio   │  │  Output  │  │  (RMT)   │  │  (SRAM)  │     ││
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘     ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Build Instructions

### Prerequisites

1. **Install Rust** with espup:

   ```bash
   cargo install espup
   espup install
   source $HOME/export-esp.sh  # Linux/macOS
   ```

2. **Install additional tools**:
   ```bash
   cargo install espflash cargo-espflash
   ```

### Building

1. **Clone and navigate to project**:

   ```bash
   cd esp32-led-matrix
   ```

2. **Configure WiFi credentials**:
   Edit `src/main.rs` and update:

   ```rust
   const WIFI_SSID: &str = "YOUR_WIFI_SSID";
   const WIFI_PASSWORD: &str = "YOUR_WIFI_PASSWORD";
   ```

3. **Build the project**:
   ```bash
   cargo build --release
   ```

### Flashing

1. **Connect ESP32** via USB

2. **Flash the firmware**:

   ```bash
   cargo espflash flash --release --monitor /dev/ttyUSB0
   ```

   On Windows, use the appropriate COM port:

   ```bash
   cargo espflash flash --release --monitor COM3
   ```

3. **Monitor serial output** to see the assigned IP address

## Usage

1. **Power on** the ESP32 and LED matrix
2. **Wait for WiFi connection** (check serial monitor for IP address)
3. **Open web browser** on your phone/computer
4. **Navigate to** `http://<ESP32_IP_ADDRESS>/`
5. **Enter text** in the input field and click "Display Text"

## API Endpoints

| Endpoint              | Method | Description               |
| --------------------- | ------ | ------------------------- |
| `/`                   | GET    | Web interface (HTML page) |
| `/text?msg=YOUR_TEXT` | GET    | Update display text       |
| `/clear`              | GET    | Clear the display         |

## Troubleshooting

### Display Shows Nothing

1. **Check power supply** - The matrix needs adequate 5V power
2. **Verify wiring** - Double-check all GPIO connections
3. **Check serial output** - Look for initialization errors

### WiFi Connection Fails

1. **Verify credentials** - Check SSID and password
2. **Check signal strength** - ESP32 antenna may need better positioning
3. **Try 2.4GHz** - ESP32 only supports 2.4GHz WiFi

### Text Not Displaying Correctly

1. **Check character support** - Only ASCII characters are supported
2. **Reduce text length** - Maximum ~14 characters fit on screen
3. **Adjust brightness** - Modify PWM values in code

## Technical Notes

### Timing Considerations

The display requires precise timing for both multiplexing and data transfer:

- **GCLK**: ~1 MHz minimum, 256 pulses per scanline
- **DCLK**: Can be slower, limited by desired frame rate
- **Refresh rate**: 12.5 FPS achievable with optimized code

### Memory Usage

- Frame buffer: 88 × 88 × 3 × 2 = 46,464 bytes (16-bit RGB)
- ESP32 has ~520KB SRAM, sufficient for double buffering

### Performance Optimization

For better frame rates, consider:

1. Using RMT (Remote Control) peripheral for GCLK generation
2. DMA for data transfer
3. SPI for parallel data output

## References

- [Helsinki Hacklab LED Matrix Documentation](https://wiki.helsinki.hacklab.fi/Ledimatriisin_ohjaaminen)
- [Pacman Project](https://wiki.helsinki.hacklab.fi/Pacman_ja_ledimatriisi)
- [ESP-IDF Programming Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/)
- [esp-rs Community](https://github.com/esp-rs)

## File Structure

| File                  | Purpose                                               |
| --------------------- | ----------------------------------------------------- |
| `Cargo.toml`          | Project dependencies (esp-idf-sys, esp-idf-hal, etc.) |
| `src/main.rs`         | Main application entry point                          |
| `src/led_matrix.rs`   | LED matrix driver with multiplexing & data transfer   |
| `src/http_server.rs`  | HTTP server with embedded web interface               |
| `src/wifi.rs`         | WiFi connectivity module                              |
| `src/font.rs`         | 5x7 bitmap font for text rendering                    |
| `README.md`           | Comprehensive documentation with wiring diagrams      |
| `rust-toolchain.toml` | Rust ESP32 toolchain configuration                    |
| `build.rs`            | ESP-IDF build script                                  |
| `.cargo/config.toml`  | Build target configuration                            |

## License

MIT License

## Acknowledgments

- Helsinki Hacklab for the LED matrix documentation and reference designs
- The esp-rs community for the excellent Rust ESP32 support
