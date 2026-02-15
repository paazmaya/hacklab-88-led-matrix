# ESP32-C3 LED Matrix Controller

A **pure Rust** ESP32-C3 application for controlling an 88x88 RGB LED matrix display with WiFi connectivity and HTTP web interface.

> **Built with esp-hal** - No ESP-IDF installation required! Works on Windows, Linux, and macOS.
>
> **Configured for ESP32-C3 SuperMini** - Compact RISC-V board with WiFi/BLE. Uses GPIO pins 0-10, 20-21 (13 pins total for LED matrix control)

https://docs.espressif.com/projects/rust/book/

## Overview

This project implements a complete solution for driving the "bonk" LED matrix displays from Helsinki Hacklab.
The ESP32-C3 SuperMini connects to your local WiFi network and serves a web page where you can input text to display on the LED matrix.

### Why ESP32-C3 SuperMini?

The ESP32-C3 is Espressif's **RISC-V** based chip with several advantages:

- **RISC-V architecture**: Standard LLVM backend, better Rust support, no Xtensa linker issues
- **Compact form factor**: SuperMini board is tiny (22.52×18mm), perfect for embedded projects
- **WiFi + BLE**: Built-in 2.4GHz WiFi and Bluetooth 5.0 LE
- **Better Rust ecosystem**: Uses stable Rust toolchain, not custom ESP fork
- **Limited pins**: Only 13 usable GPIOs on SuperMini - exactly what we need for the LED matrix!

**Pin constraints:**

- **ESP32-C3 SuperMini**: GPIO 0-10, 20-21 (13 pins total)
- **This project**: Uses all 13 available GPIOs for LED matrix control
- **Boot pins**: GPIO8 and GPIO9 are used but work fine with pull-ups on the matrix

If you have a different ESP32 variant, you'll need to modify the pin assignments in [src/main.rs](src/main.rs).

## Hardware Requirements

> **Note:** This project is configured for **ESP32-C3 SuperMini** - a compact RISC-V board with exactly 13 usable GPIOs, which is the minimum required for LED matrix control.

### ESP32-C3 SuperMini Board

- **Chip**: ESP32-C3FH4 (RISC-V 32-bit, 160MHz)
- **Flash**: 4MB
- **RAM**: 400KB SRAM
- **Size**: 22.52mm × 18mm (ultra-compact!)
- **USB**: Type-C (CH340C serial chip)
- **Available on**: AliExpress, Amazon (~$2-3 USD)
- **GPIO Pins**: 13 usable (GPIO 0-10, 20-21)

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

### ESP32-C3 SuperMini to LED Matrix Connection

```
                    LED MATRIX (34-pin connector)
                    ┌─────────────────────────────────────┐
                    │  Pin 1 (top left when oriented)     │
                    │                                     │
   ESP32-C3         │    PINOUT                           │
   SuperMini        │    ┌─────────────────────────────┐ │
   ┌──────┐         │    │                             │ │
   │ GPIO0 ├────────┼────┤ GCLK (Pin 13)               │ │
   │ GPIO1 ├────────┼────┤ DCLK (Pin 14)               │ │
   │ GPIO2 ├────────┼────┤ LE (Pin 31)                 │ │
   │ GPIO3 ├────────┼────┤ A0 (Pin 9)                  │ │
   │ GPIO4 ├────────┼────┤ A1 (Pin 10)                 │ │
   │ GPIO5 ├────────┼────┤ A2 (Pin 11)                 │ │
   │ GPIO6 ├────────┼────┤ A3 (Pin 12)                 │ │
   │ GPIO7 ├────────┼────┤ DR1 (Pin 15)                │ │
   │ GPIO8 ├────────┼────┤ DG1 (Pin 32)  ⚠️ Boot pin   │ │
   │ GPIO9 ├────────┼────┤ DB1 (Pin 16)  ⚠️ Boot pin   │ │
   │ GPIO10├────────┼────┤ DR2 (Pin 33)                │ │
   │ GPIO20├────────┼────┤ DG2 (Pin 17)  (UART RXD)    │ │
   │ GPIO21├────────┼────┤ DB2 (Pin 34)  (UART TXD)    │ │
   │  GND  ├────────┼────┤ GND (Pins 1-2, 18-19)       │ │
   │  5V** ├────────┼────┤ +5V (Pins 3-8, 20-25)       │ │
   └───────┘        │    └─────────────────────────────┘ │
                    │  **Use external 5V supply for matrix│
                    │  IMPORTANT: Verify pinout with wiki!│
                    └─────────────────────────────────────┘
```

> ⚠️ **Boot pin warning**: GPIO8 and GPIO9 are boot-mode strapping pins. The LED matrix has pull-ups, which keeps them HIGH during boot (normal mode). This works fine but be aware during debugging.

> ⚠️ **UART sharing**: GPIO20/21 are also used for USB serial debugging. Disable serial logging if you see interference with DB2/DG2 data lines.

### GPIO Pin Assignment (ESP32-C3 SuperMini)

| ESP32-C3 GPIO | LED Matrix Signal | Notes                       |
| ------------- | ----------------- | --------------------------- |
| GPIO0         | GCLK              | Multiplex clock output      |
| GPIO1         | DCLK              | Data clock output           |
| GPIO2         | LE                | Latch Enable output         |
| GPIO3         | A0                | Address bit 0               |
| GPIO4         | A1                | Address bit 1               |
| GPIO5         | A2                | Address bit 2               |
| GPIO6         | A3                | Address bit 3               |
| GPIO7         | DR1               | Red data chain 1            |
| GPIO8         | DG1               | Green data chain 1 (Boot)   |
| GPIO9         | DB1               | Blue data chain 1 (Boot)    |
| GPIO10        | DR2               | Red data chain 2            |
| GPIO20        | DG2               | Green data chain 2 (RXD)    |
| GPIO21        | DB2               | Blue data chain 2 (TXD)     |
| GND           | GND               | Common ground               |
| 5V (ext)      | +5V               | **External 5V supply only** |

> **Note:** ESP32-C3 SuperMini exposes GPIO 0-10 and GPIO 20-21 (13 pins). This uses ALL available GPIOs! GPIO8/9 are boot pins but work with matrix pull-ups. GPIO20/21 are UART pins - serial logging may interfere with display.

> ⚠️ **Power Warning**: Do NOT power the LED matrix from USB 5V! The matrix can draw up to 10A. Use an external 5V power supply rated for at least 10A. Connect ESP32-C3 GND to matrix GND.

## Build Instructions

### Prerequisites

1. **Install Rust** (if not already installed):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Add RISC-V target** (ESP32-C3 uses standard Rust, no custom toolchain needed!):

   ```bash
   rustup target add riscv32imc-unknown-none-elf
   ```

3. **Install espflash** for flashing:

   ```bash
   cargo install espflash
   ```

   Or on Windows:

   ```powershell
   cargo install espflash
   ```

> 🎉 **No espup needed!** ESP32-C3 uses RISC-V with standard LLVM backend. Just use stable Rust toolchain!

### Building

1. **Navigate to project**:

   ```bash
   cd esp32-led-matrix
   ```

2. **Configure WiFi credentials** in `src/main.rs`:

   ```rust
   const WIFI_SSID: &str = "YOUR_WIFI_SSID";
   const WIFI_PASSWORD: &str = "YOUR_WIFI_PASSWORD";
   ```

3. **Build the project**:
   ```bash
   cargo build --release
   ```

### Flashing

1. **Connect ESP32-C3 SuperMini** via USB-C cable

2. **Flash the firmware**:

   **Automatic port detection:**

   ```bash
   cargo run --release
   ```

   **Or specify port manually:**

   **Linux/macOS:**

   ```bash
   espflash flash --release --monitor /dev/ttyUSB0
   ```

   **Windows:**

   ```powershell
   espflash flash --release --monitor COM3
   ```

   (Replace `COM3` with your actual COM port)

3. **Monitor serial output** to see the assigned IP address

> **Tip**: The ESP32-C3 SuperMini has auto-reset, so you don't need to manually press BOOT+RESET buttons for flashing!

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

## Project Structure

```
esp32-led-matrix/
├── Cargo.toml          # Project dependencies (esp-hal, esp-wifi)
├── rust-toolchain.toml # Rust toolchain configuration
├── .cargo/
│   └── config.toml     # Build target configuration
└── src/
    ├── main.rs         # Main application entry point
    ├── led_matrix.rs   # LED matrix driver
    ├── http_server.rs  # HTTP server implementation
    ├── wifi.rs         # WiFi connectivity
    └── font.rs         # 5x7 bitmap font
```

## Dependencies

This project uses **pure Rust** crates (no ESP-IDF!):

| Crate              | Purpose                       |
| ------------------ | ----------------------------- |
| `esp-hal`          | Hardware abstraction layer    |
| `esp-hal-embassy`  | Embassy async runtime support |
| `esp-wifi`         | WiFi driver                   |
| `embassy-executor` | Async task executor           |
| `embassy-net`      | TCP/IP networking             |
| `smoltcp`          | Network stack                 |
| `esp-backtrace`    | Panic handling                |

**Toolchain:**

- Uses **stable Rust** (not custom ESP fork)
- Target: `riscv32imc-unknown-none-elf` (standard RISC-V)
- No Xtensa linker issues - RISC-V has excellent LLVM support!

## Troubleshooting

### Using Different ESP32 Board?

This project is optimized for **ESP32-C3 SuperMini**. For other boards:

**ESP32-S2/S3 (Xtensa):**

1. Update `Cargo.toml` features: `esp32s2` or `esp32s3`
2. Update `.cargo/config.toml` target: `xtensa-esp32s2-none-elf` or `xtensa-esp32s3-none-elf`
3. Update `rust-toolchain.toml`: `channel = "esp"`
4. Run `espup install` and source export script
5. Update GPIO pins in `src/main.rs`
6. ⚠️ May encounter Xtensa linker issues (windowed longcall problems)

**Original ESP32 (Xtensa):**

1. Update `Cargo.toml` features: `esp32`
2. Update `.cargo/config.toml` target: `xtensa-esp32-none-elf`
3. Update `rust-toolchain.toml`: `channel = "esp"`
4. Run `espup install`
5. Use GPIO 18-33 range (has more pins available)
6. ⚠️ May encounter Xtensa linker issues

**ESP32-C6 (RISC-V, recommended alternative):**

1. Update `Cargo.toml` features: `esp32c6`
2. Keep RISC-V target: `riscv32imac-unknown-none-elf`
3. More pins available than C3, same RISC-V benefits

### Build Errors

1. **"riscv32imc-unknown-none-elf target not found"**

   ```bash
   rustup target add riscv32imc-unknown-none-elf
   ```

2. **Compilation errors with esp-hal**
   - Ensure you're using stable Rust: `rustup default stable`
   - Try cleaning and rebuilding: `cargo clean && cargo build --release`

3. **"unstable feature required" error**
   - Make sure `Cargo.toml` includes `unstable` feature for esp-hal

### Display Shows Nothing

1. **Check power supply** - The matrix needs adequate 5V power (up to 10A) from **external supply**, NOT USB!
2. **Verify wiring** - Double-check all GPIO connections (especially boot pins GPIO8/9)
3. **Check serial output** - Look for initialization errors
4. **UART interference** - If GPIO20/21 show flickering, reduce serial logging
5. **Boot mode** - Ensure GPIO8/9 are not pulled LOW during power-on (matrix pull-ups should handle this)

### WiFi Connection Fails

1. **Verify credentials** - Check SSID and password in `src/main.rs`
2. **Check signal strength** - ESP32 antenna may need better positioning
3. **Use 2.4GHz network** - ESP32 only supports 2.4GHz WiFi

### Text Not Displaying Correctly

1. **Check character support** - Only ASCII characters are supported
2. **Reduce text length** - Maximum ~14 characters fit on screen
3. **Check font rendering** - Some special characters may not be defined

## Technical Notes

### Timing Considerations

The display requires precise timing for both multiplexing and data transfer:

- **GCLK**: ~1 MHz minimum, 256 pulses per scanline
- **DCLK**: Can be slower, limited by desired frame rate
- **Refresh rate**: Dependent on data transfer speed

### Memory Usage

- Frame buffer: 88 × 88 × 3 × 2 = 46,464 bytes (16-bit RGB)
- ESP32-C3 has 400KB SRAM, sufficient for the frame buffer and WiFi stack

### Pin Limitations on ESP32-C3 SuperMini

The SuperMini is _extremely_ compact but uses **all 13 available GPIOs**:

- **Cannot add more features** without pin sharing or external I/O expander
- **GPIO8/9** are boot strapping pins - matrix pull-ups keep them HIGH ✓
- **GPIO20/21** are UART - serial logging may interfere with DG2/DB2 data
- Consider **ESP32-C6** if you need more pins (30 GPIOs available)

## References

- [Helsinki Hacklab LED Matrix Documentation](https://wiki.helsinki.hacklab.fi/Ledimatriisin_ohjaaminen)
- [Pacman Project](https://wiki.helsinki.hacklab.fi/Pacman_ja_ledimatriisi)
- [esp-hal Documentation](https://docs.esp-rs.org/esp-hal/)
- [esp-rs Community](https://github.com/esp-rs)

## License

MIT License

## Acknowledgments

- Helsinki Hacklab for the LED matrix documentation and reference designs
- The esp-rs community for the excellent pure Rust ESP32 support
