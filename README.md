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

| ESP32-C3 GPIO | LED Matrix Signal | Matrix Connector Pin | Notes                        |
| ------------- | ----------------- | -------------------- | ---------------------------- |
| GPIO0         | GCLK              | 13                   | Multiplex clock output       |
| GPIO1         | DCLK              | 14                   | Data clock output            |
| GPIO2         | LE                | 31                   | Latch Enable output          |
| GPIO3         | A0                | 9                    | Address bit 0                |
| GPIO4         | A1                | 10                   | Address bit 1                |
| GPIO5         | A2                | 11                   | Address bit 2                |
| GPIO6         | A3                | 12                   | Address bit 3                |
| GPIO7         | DR1               | 15                   | Red data chain 1             |
| GPIO8         | DG1               | 32                   | Green data chain 1 (Boot)    |
| GPIO9         | DB1               | 16                   | Blue data chain 1 (Boot)     |
| GPIO10        | DR2               | 33                   | Red data chain 2             |
| GPIO20        | DG2               | 17                   | Green data chain 2 (RXD)     |
| GPIO21        | DB2               | 34                   | Blue data chain 2 (TXD)      |
| GND           | GND               | 1, 2, 18, 19         | Common ground (4 pins)       |
| 5V (ext)      | +5V               | 3–8, 20–25           | External 5V supply (12 pins) |

> **Note:** ESP32-C3 SuperMini exposes GPIO 0-10 and GPIO 20-21 (13 pins). This uses ALL available GPIOs! GPIO8/9 are boot pins but work with matrix pull-ups. GPIO20/21 are UART pins - serial logging may interfere with display.

> ⚠️ **Power Warning**: Do NOT power the LED matrix from USB 5V! The matrix can draw up to 10A. Use an external 5V power supply rated for at least 10A. Connect ESP32-C3 GND to matrix GND.

### 34-Pin Connector Pinout

The matrix uses a 34-pin (2×17) 0.1" pitch pin header. Pin 1 sits at the
upper-left of the connector when the matrix is viewed from behind with the
triangles next to the handle pointing up (towards the top of the screen).
See the [Helsinki Hacklab wiki][wiki-connector] for the orientation diagram.

| Pin | Signal | ESP32-C3 GPIO     | Notes                         |
| --- | ------ | ----------------- | ----------------------------- |
| 1   | GND    | GND               | Common ground                 |
| 2   | GND    | GND               |                               |
| 3   | +5V    | (external supply) | External 5V, ≥10 A            |
| 4   | +5V    | (external supply) |                               |
| 5   | +5V    | (external supply) |                               |
| 6   | +5V    | (external supply) |                               |
| 7   | +5V    | (external supply) |                               |
| 8   | +5V    | (external supply) |                               |
| 9   | A0     | GPIO3             | Address bit 0                 |
| 10  | A1     | GPIO4             | Address bit 1                 |
| 11  | A2     | GPIO5             | Address bit 2                 |
| 12  | A3     | GPIO6             | Address bit 3                 |
| 13  | GCLK   | GPIO0             | Multiplex clock               |
| 14  | DCLK   | GPIO1             | Data clock                    |
| 15  | DR1    | GPIO7             | Red data chain 1              |
| 16  | DB1    | GPIO9             | Blue data chain 1 (boot pin)  |
| 17  | DG2    | GPIO20            | Green data chain 2 (UART RXD) |
| 18  | GND    | GND               | Common ground                 |
| 19  | GND    | GND               |                               |
| 20  | +5V    | (external supply) | External 5V, ≥10 A            |
| 21  | +5V    | (external supply) |                               |
| 22  | +5V    | (external supply) |                               |
| 23  | +5V    | (external supply) |                               |
| 24  | +5V    | (external supply) |                               |
| 25  | +5V    | (external supply) |                               |
| 26  | NC     | —                 | Not connected                 |
| 27  | NC     | —                 |                               |
| 28  | NC     | —                 |                               |
| 29  | NC     | —                 |                               |
| 30  | NC     | —                 |                               |
| 31  | LE     | GPIO2             | Latch enable                  |
| 32  | DG1    | GPIO8             | Green data chain 1 (boot pin) |
| 33  | DR2    | GPIO10            | Red data chain 2              |
| 34  | DB2    | GPIO21            | Blue data chain 2 (UART TXD)  |

[wiki-connector]: https://wiki.helsinki.hacklab.fi/Ledimatriisin_ohjaaminen

> ⚡ **Why so many +5V and GND pins?** The matrix can pull close to **10 A**
> at full white. The connector dedicates **12 pins to +5V** and **4 pins to
> GND** (16 pins total for power), so the per-pin current stays around
> ~0.8 A — within the safe range of 22 AWG jumper wire. **Wire every single
> +5V and GND pin to your external supply** — skipping pins to "save time"
> will cause voltage drop and dim/wrong colors at high brightness.

> 🔧 **Changing the pin map?** If you rewire to a different ESP32 variant
> (e.g. ESP32-C6, ESP32-S2/S3), you must update **three** places to keep
> them in sync:
>
> 1. The GPIO Pin Assignment table above (ESP32-C3 GPIO → matrix signal)
> 2. The 34-Pin Connector Pinout table above (matrix signal → connector pin)
> 3. [`src/main.rs`](src/main.rs) — the `LedMatrix::new(...)` call near the
>    top of `fn main`. The driver in [`src/led_matrix.rs`](src/led_matrix.rs)
>    takes the 13 GPIO outputs positionally, so swapping a wire without
>    swapping the matching constructor argument will scramble the display.

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

3. **Build the project** (the embedded build uses the `esp` toolchain via `cargo +esp` and the `build-esp32`/`release-esp32` aliases defined in `.cargo/config.toml`, which set `--target riscv32imc-unknown-none-elf --features esp32`):

   ```bash
   cargo +esp build-esp32          # debug build
   cargo +esp release-esp32        # optimised release build
   ```

   The resulting ELF is at `target/riscv32imc-unknown-none-elf/{release,debug}/esp32-led-matrix`.

### Flashing

1. **Connect ESP32-C3 SuperMini** via USB-C cable

2. **Flash the firmware**:

   **Automatic port detection (uses the runner from `.cargo/config.toml`):**

   ```bash
   cargo +esp run --release
   ```

   **Or specify port manually:**

   **Linux/macOS:**

   ```bash
   cargo +esp espflash flash --release --monitor /dev/ttyUSB0
   ```

   **Windows:**

   ```powershell
   cargo +esp espflash flash --release --monitor COM3
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

| Endpoint | Method | Query Parameters | Description |
| --- | --- | --- | --- |
| `/` | GET | — | Interactive web interface with position, color picker, and clear toggles |
| `/text` | GET | `msg`, `x`, `y`, `color`, `clear` | Render text to the LED matrix |
| `/clear` | GET | — | Clear the LED matrix to black |

### `/text` Query Parameters

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `msg` | string | `""` | Text to render (up to 32 characters, ASCII 32–126). Supports `\n` for line breaks. |
| `x` | integer | `4` (or auto-centered) | Starting X column coordinate ($0..87$). Supports negative values for partial clipping. |
| `y` | integer | `40` (vertically centered) | Starting Y row coordinate ($0..87$). Supports negative values for partial clipping. |
| `color` | hex string | `#FFFFFF` (white) | 24-bit RGB hex color (e.g. `FF8000`, `#00FF88`, or `%23FF0000`). Scaled to 16-bit PWM channels. |
| `clear` | boolean (`1`/`0`) | `1` (true) | When `1`, clears the matrix before drawing. When `0`, overlays text onto existing content. |

#### Examples

- **Default centered text**:
  ```text
  GET /text?msg=Hello+World
  ```
- **Custom coordinate and hex color**:
  ```text
  GET /text?msg=TopLeft&x=0&y=0&color=FF5500
  ```
- **Multi-line overlay without clearing previous content**:
  ```text
  GET /text?msg=Line+1&x=0&y=0&color=FF0000&clear=1
  GET /text?msg=Line+2&x=0&y=10&color=00FF00&clear=0
  GET /text?msg=Line+3&x=0&y=20&color=0088FF&clear=0
  ```

## Wokwi Simulation

This project includes a complete [Wokwi](https://wokwi.com/) simulation configuration mirroring the real hardware setup as closely as possible.

### Simulated Components

- **ESP32-C3 DevKit** (`board-esp32-c3-devkitm-1`): RISC-V microcontroller running the exact embedded firmware with serial monitor connected to TX/RX.
- **8-Channel Digital Logic Analyzer** (`wokwi-logic-analyzer`): Monitors the 7 control pins (GCLK on D0, DCLK on D1, LE on D2, A0–A3 on D3–D6) and red data chain 1 (DR1 on D7). Signal waveforms are automatically saved to `signals.vcd` upon stopping the simulation (can be opened in VS Code using Surfer or WaveTrace).
- **Driver Shift Registers** (`wokwi-74hc595`): 6 parallel shift registers representing the driver IC chains (R1, G1, B1, R2, G2, B2), clocked by DCLK and latched by LE.
- **RGB Activity Indicators** (`wokwi-rgb-led`): Display live output from the shift register chains.
- **Custom 88×88 Matrix Chip** (`chips/matrix-88x88.chip.*`): A custom Wokwi chip implementing the 34-pin connector and an $88 \times 88$ RGBA framebuffer.

### Running in VS Code

1. Install the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension and activate your license.
2. Build the firmware:
   ```bash
   cargo release-esp32
   ```
3. Open the Command Palette (`F1` or `Cmd+Shift+P`) and choose **Wokwi: Start Simulator**.
4. The virtual WiFi connects via `Wokwi-GUEST`. Port 80 on the simulated ESP32 is forwarded to port 8080 on your host machine.
5. Open your web browser at `http://localhost:8080/` to interact with the simulated web server!

> 💡 **WiFi in Wokwi**: When running in the simulator, set `const WIFI_SSID: &str = "Wokwi-GUEST";` and `const WIFI_PASSWORD: &str = "";` in `src/main.rs`.

## Project Structure

```
esp32-led-matrix/
├── Cargo.toml          # Dependencies, feature flags, and bin/lib targets
├── rust-toolchain.toml # Stable Rust toolchain configuration
├── .cargo/
│   └── config.toml     # Build aliases (build-esp32, release-esp32) and runner
├── wokwi.toml          # Wokwi simulator configuration & port forwarding
├── diagram.json        # Circuit diagram with ESP32-C3, logic analyzer & shift registers
├── chips/              # Custom 88x88 LED matrix chip definition for Wokwi
│   ├── matrix-88x88.chip.json
│   ├── matrix-88x88.chip.c
│   ├── wokwi-api.h
│   └── Makefile
├── src/
│   ├── main.rs         # Embedded binary entry point, pin setup & refresh loop
│   ├── lib.rs          # Testable library root (font, framebuffer, parser, mapper)
│   ├── frame_buffer.rs # 88x88 RGB pixel buffer, draw_text_at, draw_char & clipping
│   ├── font.rs         # 5x7 ASCII bitmap font (ASCII 32–126)
│   ├── chain_mapper.rs # 88x88 matrix to 6 parallel shift register chains mapping
│   ├── bit_stream.rs   # Bit-level serialization for GPIO pin pulses
│   ├── http_request.rs # Pure HTTP parsing, query parameters (x, y, color, clear)
│   ├── http_page.html  # Modern glassmorphism web UI with position & color controls
│   ├── http_server.rs  # Async HTTP server task (embassy-net)
│   ├── led_matrix.rs   # Hardware driver & GPIO pulse multiplexer
│   └── wifi.rs         # WiFi initialization & DHCP stack management
└── tests/
    └── integration_tests.rs # End-to-end pipeline & query parsing tests
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

2. **`portable_atomic_unsafe_assume_single_core` compile error**

   The `unsafe-assume-single-core` feature on `portable-atomic` is only legal on the embedded target — it must not be enabled when building for the host. Use the project's `build-esp32` / `release-esp32` aliases (or pass `--target riscv32imc-unknown-none-elf` explicitly) instead of bare `cargo build`:

   ```bash
   cargo +esp build-esp32
   ```

3. **Compilation errors with esp-hal**
   - Make sure you are using the `esp` toolchain: `cargo +esp …`
   - Try cleaning and rebuilding: `cargo clean && cargo +esp build-esp32`

4. **"unstable feature required" error**
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

1. **Check character support** - Only ASCII characters are supported (ASCII 32–126; lowercase characters automatically map to uppercase glyphs)
2. **Text length & wrapping** - Up to 32 characters per message. About 14 characters fit on a single horizontal line; use `\n` to break text onto multiple lines or supply explicit `x` and `y` coordinates
3. **Check font rendering** - Font uses 5×7 pixels per glyph with 1-pixel horizontal and vertical spacing

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

## Running Tests

This project includes comprehensive unit and integration tests for all pure Rust components (`font`, `frame_buffer`, `chain_mapper`, `bit_stream`, and `http_request`).

Because the embedded ESP32 HAL dependencies are behind the optional `esp32` feature flag (`default = []`), all tests run directly on your host machine:

```bash
# Run all unit and integration tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration_tests
```

### Build Binary for ESP32

To build the embedded binary for ESP32:

```bash
cargo build-esp32        # debug build
cargo release-esp32      # optimised release build
```

These aliases defined in `.cargo/config.toml` invoke `cargo build --target riscv32imc-unknown-none-elf --features esp32`. The `.cargo/config.toml` also configures `runner = "espflash flash --monitor"` so `cargo run --release` automatically flashes the connected board.

## Continuous Integration

Tests should be run before updating dependencies. Use these commands:

```bash
# Run all host tests (font module)
cargo test --no-default-features --target x86_64-pc-windows-msvc
# or on macOS:
cargo test --no-default-features --target aarch64-apple-darwin

# Build for embedded
cargo +esp build-esp32
cargo +esp release-esp32
```

All tests must pass and the embedded build must succeed before deploying to ESP32.

## License

MIT License

## Acknowledgments

- Helsinki Hacklab for the LED matrix documentation and reference designs
- The esp-rs community for the excellent pure Rust ESP32 support
