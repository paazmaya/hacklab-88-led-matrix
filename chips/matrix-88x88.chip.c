/**
 * Wokwi Custom Chip: Helsinki Hacklab 88x88 RGB LED Matrix
 *
 * This chip models the physical 88x88 RGB LED panel used at Helsinki Hacklab:
 * - Resolution: 88x88 RGB pixels (7,744 pixels = 23,232 individual LED dies)
 * - Drivers: 44x Macroblock MBI5252 (or compatible) 16-channel constant-current
 *   LED sink drivers with internal 16-bit PWM counters and double-buffered SRAM.
 * - Architecture: 6 parallel serial data chains (R1, G1, B1, R2, G2, B2) split
 *   into two 22-IC chains (Chain 1 for rows 0..43, Chain 2 for rows 44..87).
 * - Multiplexing: 11:1 multiplexing across 11 scanlines (A0..A3), each scanline
 *   driving 8 physical rows.
 * - Connector: 34-pin header (12x 5V, 4x GND, 5x NC, 13 logic pins).
 *
 * Simulation Strategy & Design Choices:
 * In physical hardware, the MBI5252 ICs require continuous GCLK clocking (~1 MHz,
 * 256 pulses per scanline) to drive 16-bit PWM counters. Simulating 140,000+ pin
 * events per refresh frame in WebAssembly would severely degrade simulator
 * performance. Instead, this chip takes advantage of the MBI5252 double-buffer
 * architecture:
 * 1. DCLK shifts in 16-bit PWM color values MSB-first across all 6 data lines.
 * 2. Data Latch (LE high for 1 DCLK pulse) decodes the 44-pixel chain vector
 *    into an internal 88x88 RGBA back-buffer using the inverse of the PCB routing.
 * 3. VSYNC (LE high for 2 DCLK pulses) commits the completed frame to Wokwi's
 *    framebuffer in a single atomic call, providing 100% visual fidelity at 60 FPS.
 *
 * See chips/README.md for complete technical documentation.
 */

#include "wokwi-api.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Display dimensions */
#define MATRIX_WIDTH 88
#define MATRIX_HEIGHT 88

/* Driver IC chain configuration */
#define ICS_PER_CHAIN 22   /* 22 ICs per parallel chain (top or bottom half) */
#define PWM_BITS 16        /* 16-bit PWM color depth per channel */
#define CHAIN_LEN 44       /* Total ICs across both chains: 22 (top) + 22 (bottom) */
#define SCANLINES 11       /* 11:1 multiplexing scanlines */
#define LEDS_PER_IC 16     /* 16 output channels per driver IC */
#define TOTAL_CYCLES (SCANLINES * LEDS_PER_IC)   /* 11 * 16 = 176 latch cycles per frame */
#define DCLKS_PER_CYCLE (ICS_PER_CHAIN * PWM_BITS) /* 22 * 16 = 352 DCLKs per cycle */

typedef struct {
  /* 13 control pins connected to ESP32-C3 */
  pin_t pin_gclk;
  pin_t pin_dclk;
  pin_t pin_le;
  pin_t pin_a0;
  pin_t pin_a1;
  pin_t pin_a2;
  pin_t pin_a3;
  pin_t pin_dr1;
  pin_t pin_dg1;
  pin_t pin_db1;
  pin_t pin_dr2;
  pin_t pin_dg2;
  pin_t pin_db2;

  /* Wokwi native framebuffer handle and dimensions */
  buffer_t fb;
  uint32_t fb_width;
  uint32_t fb_height;

  /* Internal RGBA8888 back-buffer updated during data latch cycles */
  uint32_t fb_data[MATRIX_HEIGHT][MATRIX_WIDTH];

  /* 16-bit PWM shift registers for the 6 parallel chains (44 ICs total: 22 top, 22 bottom) */
  uint16_t data_r[CHAIN_LEN];
  uint16_t data_g[CHAIN_LEN];
  uint16_t data_b[CHAIN_LEN];
  uint32_t dclk_idx;     /* Current bit index within the 352-bit shift cycle */
  uint32_t latch_cycle;  /* Current cycle (0..175 = 11 scanlines * 16 leds) */

  /* Command decoding: count DCLK rising edges while LE is high */
  uint32_t le_pulse_count;
  bool le_high;
} matrix_chip_t;

/**
 * DCLK pin change handler (Data Clock).
 *
 * On rising edge:
 * 1. If LE is held high, counts DCLK pulses to determine the command code.
 * 2. If shifting data (dclk_idx < 352), clocks 1 bit into the 16-bit shift register
 *    for each of the 6 parallel data lines (DR1, DG1, DB1, DR2, DG2, DB2).
 *    Bits arrive MSB-first.
 *
 * Note on Data Latch timing: On the 352nd DCLK of a data cycle, the firmware sets
 * LE high while simultaneously clocking the final LSB of IC 21. This dual-purpose
 * cycle clocks the bit and registers pulse count = 1 for the Data Latch command.
 */
static void on_dclk_change(void *user_data, pin_t pin, uint32_t value) {
  matrix_chip_t *chip = (matrix_chip_t *)user_data;
  if (value == HIGH) {
    if (chip->le_high) {
      chip->le_pulse_count++;
    }

    // Shift in 1 bit (MSB-first) on each of the 6 parallel chains.
    if (chip->dclk_idx < DCLKS_PER_CYCLE) {
      uint32_t ic = chip->dclk_idx / PWM_BITS;
      uint32_t bit_pos = chip->dclk_idx % PWM_BITS;
      uint32_t bit_idx = (PWM_BITS - 1) - bit_pos;

      if (bit_pos == 0) {
        chip->data_r[ic] = 0;
        chip->data_g[ic] = 0;
        chip->data_b[ic] = 0;
        chip->data_r[ic + ICS_PER_CHAIN] = 0;
        chip->data_g[ic + ICS_PER_CHAIN] = 0;
        chip->data_b[ic + ICS_PER_CHAIN] = 0;
      }

      chip->data_r[ic] |= (uint16_t)(pin_read(chip->pin_dr1) & 1) << bit_idx;
      chip->data_g[ic] |= (uint16_t)(pin_read(chip->pin_dg1) & 1) << bit_idx;
      chip->data_b[ic] |= (uint16_t)(pin_read(chip->pin_db1) & 1) << bit_idx;
      chip->data_r[ic + ICS_PER_CHAIN] |= (uint16_t)(pin_read(chip->pin_dr2) & 1) << bit_idx;
      chip->data_g[ic + ICS_PER_CHAIN] |= (uint16_t)(pin_read(chip->pin_dg2) & 1) << bit_idx;
      chip->data_b[ic + ICS_PER_CHAIN] |= (uint16_t)(pin_read(chip->pin_db2) & 1) << bit_idx;
      chip->dclk_idx++;
    }
  }
}

/**
 * LE pin change handler (Latch Enable).
 *
 * Rising edge:
 *   Marks LE high and resets the command pulse counter.
 *
 * Falling edge:
 *   Decodes the command based on how many DCLK rising edges were counted while
 *   LE was high:
 *   - cmd == 1 (Data Latch):
 *       Decodes the current 44-pixel chain vector into fb_data[row][col] using
 *       the inverse of the PCB serpentine routing (inverse of compute_chain_data).
 *       Advances latch_cycle (0..175) and resets dclk_idx.
 *   - cmd == 2 (VSYNC):
 *       Triggered at the scanline 10 -> 0 frame boundary. Flushes the completed
 *       88x88 RGBA back-buffer to Wokwi's framebuffer via buffer_write().
 *       Resets latch_cycle and dclk_idx.
 *   - cmd == 10 (Reset):
 *       Resets shift counters and latch cycle to 0.
 *   - Other commands (PreActive = 14, WriteConfig = 4):
 *       Resets shift bit index dclk_idx to 0.
 */
static void on_le_change(void *user_data, pin_t pin, uint32_t value) {
  matrix_chip_t *chip = (matrix_chip_t *)user_data;
  if (value == HIGH) {
    chip->le_high = true;
    chip->le_pulse_count = 0;
  } else {
    chip->le_high = false;
    uint32_t cmd = chip->le_pulse_count;

    if (cmd == 1) {
      // Data Latch: decode the 44-pixel chain vector for the current (scanline, led)
      // cycle using the exact inverse of chain_mapper::compute_chain_data.
      uint32_t scanline = chip->latch_cycle / LEDS_PER_IC;
      uint32_t led = chip->latch_cycle % LEDS_PER_IC;

      if (scanline < SCANLINES) {
        uint32_t led_row = (led < 8) ? 11 : 0;
        uint32_t led_col = led % 8;
        if (led_row == 11) {
          led_col = 7 - led_col;
        }

        const uint32_t starts[4] = {33, 22, 11, 0};
        uint32_t row = scanline + led_row;

        for (uint32_t b = 0; b < 4; b++) {
          uint32_t start = starts[b];
          for (uint32_t i = 0; i < 11; i++) {
            uint32_t col = 8 * i + led_col;
            uint32_t idx = start + i;
            if (row < MATRIX_HEIGHT && col < MATRIX_WIDTH) {
              uint8_t r8 = (uint8_t)(chip->data_r[idx] >> 8);
              uint8_t g8 = (uint8_t)(chip->data_g[idx] >> 8);
              uint8_t b8 = (uint8_t)(chip->data_b[idx] >> 8);
              chip->fb_data[row][col] = 0xFF000000u | ((uint32_t)b8 << 16) | ((uint32_t)g8 << 8) | (uint32_t)r8;
            }
          }
          row += 22;
        }
      }

      chip->latch_cycle = (chip->latch_cycle + 1) % TOTAL_CYCLES;
      chip->dclk_idx = 0;
    } else if (cmd == 2) {
      // VSYNC: Buffer swap / update display
      chip->latch_cycle = 0;
      chip->dclk_idx = 0;
      if (chip->fb) {
        buffer_write(chip->fb, 0, chip->fb_data, sizeof(chip->fb_data));
      }
    } else {
      // Reset (10), PreActive (14), WriteConfig (4), or other commands reset shift counters
      chip->dclk_idx = 0;
      if (cmd == 10) {
        chip->latch_cycle = 0;
      }
    }
  }
}

/**
 * Chip entry point called by Wokwi upon simulator startup.
 *
 * Allocates the chip state, initializes the Wokwi Framebuffer (88x88 RGBA),
 * defines all 13 control pins as INPUT, and attaches edge watchers to DCLK and LE.
 */
void chip_init(void) {
  matrix_chip_t *chip = (matrix_chip_t *)calloc(1, sizeof(matrix_chip_t));

  chip->fb = framebuffer_init(&chip->fb_width, &chip->fb_height);

  chip->pin_gclk = pin_init("GCLK", INPUT);
  chip->pin_dclk = pin_init("DCLK", INPUT);
  chip->pin_le = pin_init("LE", INPUT);
  chip->pin_a0 = pin_init("A0", INPUT);
  chip->pin_a1 = pin_init("A1", INPUT);
  chip->pin_a2 = pin_init("A2", INPUT);
  chip->pin_a3 = pin_init("A3", INPUT);
  chip->pin_dr1 = pin_init("DR1", INPUT);
  chip->pin_dg1 = pin_init("DG1", INPUT);
  chip->pin_db1 = pin_init("DB1", INPUT);
  chip->pin_dr2 = pin_init("DR2", INPUT);
  chip->pin_dg2 = pin_init("DG2", INPUT);
  chip->pin_db2 = pin_init("DB2", INPUT);

  pin_watch_config_t dclk_watch = {
    .user_data = chip,
    .edge = BOTH,
    .pin_change = on_dclk_change,
  };
  pin_watch(chip->pin_dclk, &dclk_watch);

  pin_watch_config_t le_watch = {
    .user_data = chip,
    .edge = BOTH,
    .pin_change = on_le_change,
  };
  pin_watch(chip->pin_le, &le_watch);
}
