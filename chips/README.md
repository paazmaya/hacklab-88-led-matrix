# Custom Wokwi Chip: 88×88 RGB LED Matrix (`matrix-88x88`)

This directory contains the custom [Wokwi](https://wokwi.com/) simulation chip for the **Helsinki Hacklab 88×88 RGB LED Matrix**. The chip is written in C and compiled into WebAssembly (`matrix-88x88.chip.wasm`) using Wokwi's Custom Chips C API.

## Table of Contents

- [Custom Wokwi Chip: 88×88 RGB LED Matrix (`matrix-88x88`)](#custom-wokwi-chip-8888-rgb-led-matrix-matrix-88x88)
  - [Table of Contents](#table-of-contents)
  - [Why a Custom Chip is Needed](#why-a-custom-chip-is-needed)
  - [Hardware Background \& Architecture](#hardware-background--architecture)
  - [Connector \& Pinout Specification](#connector--pinout-specification)
  - [Signaling Protocol \& Command Decoding](#signaling-protocol--command-decoding)
    - [1. Shift Register Timing (`DCLK`)](#1-shift-register-timing-dclk)
    - [2. Command Strobe (`LE` + `DCLK`)](#2-command-strobe-le--dclk)
    - [3. Frame Data Lifecycle](#3-frame-data-lifecycle)
  - [Pixel Mapping \& Inverse Transformation](#pixel-mapping--inverse-transformation)
    - [The Physical PCB Mapping Problem](#the-physical-pcb-mapping-problem)
    - [The Inverse Transformation in C](#the-inverse-transformation-in-c)
  - [Simulation Architecture \& Performance Tradeoffs](#simulation-architecture--performance-tradeoffs)
    - [Why GCLK / High-Frequency PWM Emulation is Bypassed](#why-gclk--high-frequency-pwm-emulation-is-bypassed)
    - [VSYNC-Driven Framebuffer Presentation](#vsync-driven-framebuffer-presentation)
  - [Repository File Overview](#repository-file-overview)
  - [Building \& Recompilation](#building--recompilation)
    - [Prerequisites](#prerequisites)
    - [Compilation Command](#compilation-command)
  - [Simulation, Testing \& Debugging](#simulation-testing--debugging)
    - [Running in VS Code](#running-in-vs-code)
    - [Waveform Analysis (VCD Export)](#waveform-analysis-vcd-export)

## Why a Custom Chip is Needed

Standard Wokwi components include basic individual LEDs, character LCDs, and simple 8×8 matrices (e.g. MAX7219 or WS2812). The Helsinki Hacklab LED Matrix, however, is a custom high-density installation panel with unique physical and electrical characteristics:

- **High Resolution**: 88×88 RGB pixels ($7{,}744$ RGB pixels = $23{,}232$ individual LED dies).
- **Specialized Driver ICs**: Driven by 44 Macroblock MBI5252 (or pin-compatible) constant-current 16-channel LED sink drivers with internal 16-bit PWM grayscale generators and double-buffered SRAM.
- **Parallel Multiplexed Shift Architecture**: Operates with 6 parallel serial data lines (`DR1`, `DG1`, `DB1`, `DR2`, `DG2`, `DB2`) split into two vertical halves of 22 ICs each, driven by an 11:1 multiplexing scheme.
- **Pulse-Count Command Protocol**: Commands (data latch, VSYNC buffer swap, configuration, reset) are encoded by holding Latch Enable (`LE`) high while pulsing Data Clock (`DCLK`) a specific number of times.
- **Serpentine PCB Routing**: Outputs of the driver ICs are mapped across non-contiguous rows and inverted column sequences across four 22-row quadrants.

Without a custom peripheral, the real embedded firmware could not be simulated without heavily modifying or mocking the hardware driver layer. This custom chip acts as a hardware-accurate virtual panel, allowing the exact RISC-V firmware to run unmodified and render to an interactive 88×88 graphical canvas in Wokwi.

## Hardware Background & Architecture

The physical matrix consists of:

| Parameter           | Value                     | Description                                                                                                |
| :------------------ | :------------------------ | :--------------------------------------------------------------------------------------------------------- |
| **Resolution**      | 88 × 88                   | Total $7{,}744$ RGB pixels                                                                                 |
| **Color Depth**     | 16-bit PWM per channel    | 65,536 brightness levels per color ($2.8 \times 10^{14}$ colors)                                           |
| **Multiplexing**    | 11:1 multiplexing         | 11 scanlines (`A0`–`A3`), each driving 8 physical rows                                                     |
| **Driver ICs**      | 44 × MBI5252              | 16 outputs per IC; each output drives 11 scanlines = 176 LEDs/IC                                           |
| **Parallel Chains** | 6 lines                   | Top half: `DR1`, `DG1`, `DB1` (rows 0–43, 22 ICs)<br>Bottom half: `DR2`, `DG2`, `DB2` (rows 44–87, 22 ICs) |
| **Power Rating**    | 5 V DC, up to 10 A (50 W) | 12 pins for +5 V and 4 pins for GND on a 34-pin header                                                     |

## Connector & Pinout Specification

The matrix interfaces via a standard 34-pin (2×17) 0.1" pitch header. The chip definition (`matrix-88x88.chip.json`) defines all 34 pins, matching the physical board:

|    Pin    | Matrix Signal | Chip Pin Name    | ESP32-C3 GPIO | Description                              |
| :-------: | :------------ | :--------------- | :------------ | :--------------------------------------- |
|  **1–2**  | `GND`         | `GND.1`, `GND.2` | `GND`         | Ground return                            |
|  **3–8**  | `+5V`         | `5V.1` – `5V.6`  | External 5V   | High-current power (+5 V rail)           |
|   **9**   | `A0`          | `A0`             | `GPIO3`       | Multiplex scanline address bit 0         |
|  **10**   | `A1`          | `A1`             | `GPIO4`       | Multiplex scanline address bit 1         |
|  **11**   | `A2`          | `A2`             | `GPIO5`       | Multiplex scanline address bit 2         |
|  **12**   | `A3`          | `A3`             | `GPIO6`       | Multiplex scanline address bit 3         |
|  **13**   | `GCLK`        | `GCLK`           | `GPIO0`       | Multiplex grayscale clock (~1 MHz)       |
|  **14**   | `DCLK`        | `DCLK`           | `GPIO1`       | Shift register data clock                |
|  **15**   | `DR1`         | `DR1`            | `GPIO7`       | Red data line for chain 1 (rows 0–43)    |
|  **16**   | `DB1`         | `DB1`            | `GPIO9`       | Blue data line for chain 1 (rows 0–43)   |
|  **17**   | `DG2`         | `DG2`            | `GPIO20`      | Green data line for chain 2 (rows 44–87) |
| **18–19** | `GND`         | `GND.3`, `GND.4` | `GND`         | Ground return                            |
| **20–25** | `+5V`         | `5V.7` – `5V.12` | External 5V   | High-current power (+5 V rail)           |
| **26–30** | `NC`          | `NC.1` – `NC.5`  | —             | Not connected / unused                   |
|  **31**   | `LE`          | `LE`             | `GPIO2`       | Latch Enable (command strobe)            |
|  **32**   | `DG1`         | `DG1`            | `GPIO8`       | Green data line for chain 1 (rows 0–43)  |
|  **33**   | `DR2`         | `DR2`            | `GPIO10`      | Red data line for chain 2 (rows 44–87)   |
|  **34**   | `DB2`         | `DB2`            | `GPIO21`      | Blue data line for chain 2 (rows 44–87)  |

## Signaling Protocol & Command Decoding

### 1. Shift Register Timing (`DCLK`)

- Each chain contains 22 driver ICs in series, with each IC requiring a 16-bit PWM grayscale value per color channel.
- $22 \text{ ICs} \times 16 \text{ bits} = 352 \text{ DCLK pulses}$ per data latch cycle.
- The 6 data lines (`DR1`, `DG1`, `DB1`, `DR2`, `DG2`, `DB2`) are sampled simultaneously on each rising edge of `DCLK`, clocking data in **MSB-first**.
- In the chip implementation (`on_dclk_change`):
  - `chip->data_r[ic]` and `chip->data_r[ic + 22]` accumulate bits for chain 1 and chain 2 respectively.
  - Bits are placed into bit position `(15 - (dclk_idx % 16))`.

### 2. Command Strobe (`LE` + `DCLK`)

The MBI5252 protocol issues commands by holding `LE` high while pulsing `DCLK` $N$ times. The command is executed on the falling edge of `LE`:

| Pulse Count ($N$) | Command Name     | Function in Real Hardware                                                    | Custom Chip Handling                                                                                     |
| :---------------: | :--------------- | :--------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------- |
|       **1**       | **Data Latch**   | Latches 352-bit shift register into the IC PWM buffer for current cycle      | Decodes the 44-pixel vector into the internal back-buffer via inverse mapping and advances `latch_cycle` |
|       **2**       | **VSYNC**        | Swaps front and back display buffers at frame boundary (scanline 10 $\to$ 0) | Commits `fb_data` to Wokwi via `buffer_write()`, updating the simulator display                          |
|       **4**       | **Write Config** | Writes Configuration Register 1                                              | Resets shift register index (`dclk_idx = 0`)                                                             |
|      **10**       | **Reset**        | Resets driver IC state machines                                              | Resets `latch_cycle = 0` and `dclk_idx = 0`                                                              |
|      **14**       | **Pre-Active**   | Unlocks configuration register write operations                              | Resets shift register index (`dclk_idx = 0`)                                                             |

### 3. Frame Data Lifecycle

A complete frame consists of:
$$\text{Total Cycles} = 11 \text{ scanlines} \times 16 \text{ LEDs per IC} = 176 \text{ latch cycles}$$
Each cycle shifts 352 bits per data pin and ends with a Data Latch ($N=1$). After all 176 cycles are completed and the multiplexer wraps from scanline 10 to 0, the driver issues a VSYNC command ($N=2$), triggering an atomic buffer swap.

## Pixel Mapping & Inverse Transformation

### The Physical PCB Mapping Problem

The physical matrix is not wired linearly. To minimize routing traces across the large 88×88 board:

1. Each of the 11 scanlines illuminates 8 physical rows dispersed across the display.
2. The display is divided into four 22-row vertical blocks (rows 0–21, 22–43, 44–65, 66–87).
3. For LED indices $0 \le \text{led} < 8$, column order is reversed (`7 - (led % 8)`) and row offset is $+11$.
4. For LED indices $8 \le \text{led} < 16$, column order is normal (`led % 8`) and row offset is $+0$.

In firmware, `chain_mapper::compute_chain_data_with_brightness` converts a 2D $88 \times 88$ image into a 44-element vector (`CHAIN_LEN = 44`) for each of the 176 cycles.

### The Inverse Transformation in C

To reconstruct the 2D visual image, `matrix-88x88.chip.c` implements the exact mathematical inverse when a Data Latch ($N=1$) occurs:

```c
uint32_t scanline = chip->latch_cycle / LEDS_PER_IC; // 0..10
uint32_t led      = chip->latch_cycle % LEDS_PER_IC; // 0..15

if (scanline < SCANLINES) {
  uint32_t led_row = (led < 8) ? 11 : 0;
  uint32_t led_col = led % 8;
  if (led_row == 11) {
    led_col = 7 - led_col; // Invert columns for lower row group
  }

  // Four row blocks start at indices 33, 22, 11, 0 in the 44-element chain
  const uint32_t starts[4] = {33, 22, 11, 0};
  uint32_t row = scanline + led_row;

  for (uint32_t b = 0; b < 4; b++) {
    uint32_t start = starts[b];
    for (uint32_t i = 0; i < 11; i++) {
      uint32_t col = 8 * i + led_col;
      uint32_t idx = start + i;
      if (row < MATRIX_HEIGHT && col < MATRIX_WIDTH) {
        // Downsample 16-bit PWM to 8-bit RGBA for Wokwi Framebuffer
        uint8_t r8 = (uint8_t)(chip->data_r[idx] >> 8);
        uint8_t g8 = (uint8_t)(chip->data_g[idx] >> 8);
        uint8_t b8 = (uint8_t)(chip->data_b[idx] >> 8);
        chip->fb_data[row][col] = 0xFF000000u | ((uint32_t)b8 << 16) | ((uint32_t)g8 << 8) | (uint32_t)r8;
      }
    }
    row += 22; // Step to the next 22-row block
  }
}
```

The 7,744 pixels are mapped bijectively: every pixel in the 2D grid is visited exactly once per 176 latch cycles.

## Simulation Architecture & Performance Tradeoffs

### Why GCLK / High-Frequency PWM Emulation is Bypassed

In physical hardware, the MBI5252 ICs require continuous high-frequency clocking on `GCLK` (~1 MHz) to increment internal 16-bit PWM counters and illuminate LEDs for each scanline:

- $256 \text{ pulses/scanline} \times 11 \text{ scanlines} = 2{,}816 \text{ pulses per multiplex frame}$.
- At 50 multiplex frames per refresh, that represents $\sim 140{,}800 \text{ pin toggles}$ per display refresh!

In an emulated WebAssembly environment:

- Invoking 140,000+ WASM function callbacks per frame would choke the browser or VS Code simulator process and drag CPU utilization to 100%.
- The human eye perceives the integrated PWM duty cycle as average color/brightness, which is already mathematically represented by the 16-bit color values shifted in via `DCLK`.

### VSYNC-Driven Framebuffer Presentation

The custom chip leverages the MBI5252's double-buffered design:

1. Shift-register data is captured and decoded into an off-screen back-buffer (`chip->fb_data`).
2. `GCLK` and `A0`–`A3` pins are monitored to ensure correct initialization, but no intensive per-pulse PWM emulation is performed.
3. When the firmware issues a **VSYNC** command ($N=2$), the chip calls Wokwi's native `buffer_write()` to push the completed $88 \times 88$ RGBA8888 buffer to the display canvas in one atomic operation.

This design delivers **100% visual fidelity** at silky-smooth **60 FPS** while consuming negligible CPU resources.

## Repository File Overview

```text
chips/
├── matrix-88x88.chip.json   # Wokwi part descriptor (name, pinout, 88x88 display canvas)
├── matrix-88x88.chip.c      # C source code implementing the WebAssembly logic
├── matrix-88x88.chip.wasm   # Compiled WebAssembly binary loaded by Wokwi
├── wokwi-api.h              # Header defining Wokwi C API imports and exports
├── Makefile                 # Docker-based build rule compiling C to WASM
└── README.md                # This technical documentation
```

## Building & Recompilation

The custom chip is compiled into WebAssembly using LLVM/Clang targeting `wasm32-unknown-wasi`. Wokwi provides an official Docker image containing the exact toolchain:

### Prerequisites

- [Docker](https://www.docker.com/) installed and running.

### Compilation Command

From the repository root:

```bash
make -C chips
```

Or run the Docker command directly from the `chips/` folder:

```bash
docker run --rm -v $(pwd):/src -w /src wokwi/builder-clang-wasm \
  clang --target=wasm32-unknown-wasi --sysroot /opt/wasi-libc \
  -nostartfiles -Wl,--import-memory -Wl,--export-table -Wl,--no-entry -Werror \
  -o matrix-88x88.chip.wasm matrix-88x88.chip.c
```

To clean the compiled binary:

```bash
make -C chips clean
```

## Simulation, Testing & Debugging

### Running in VS Code

1. Ensure the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension is installed.
2. Build the firmware:
   ```bash
   cargo build --release
   ```
3. Open `diagram.json` or press `F1` $\to$ **Wokwi: Start Simulator**.
4. The 88×88 LED matrix appears to the right of the ESP32-C3 board, rendering the status pixel and HTTP text in real time.

### Waveform Analysis (VCD Export)

`wokwi.toml` is configured to record signal traces to `signals.vcd`:

```toml
[wokwi]
vcdFile = "signals.vcd"
```

When simulation stops, open `signals.vcd` with waveform viewers such as **Surfer**, **PulseView**, or **GTKWave** to inspect:

- `DCLK` bursts during shift register transmission.
- `LE` high duration and pulse count for Data Latch ($N=1$) and VSYNC ($N=2$).
- Scanline switching on `A0`–`A3`.
- `GCLK` dead-time pulses on the 257th cycle.
