#include "wokwi-api.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MATRIX_WIDTH 88
#define MATRIX_HEIGHT 88
#define ICS_PER_CHAIN 22
#define PWM_BITS 16
#define CHAIN_LEN 44
#define SCANLINES 11
#define LEDS_PER_IC 16
#define TOTAL_CYCLES (SCANLINES * LEDS_PER_IC)
#define DCLKS_PER_CYCLE (ICS_PER_CHAIN * PWM_BITS)

typedef struct {
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

  buffer_t fb;
  uint32_t fb_width;
  uint32_t fb_height;

  uint32_t fb_data[MATRIX_HEIGHT][MATRIX_WIDTH]; // RGBA8888 back-buffer for Wokwi Framebuffer

  // 16-bit PWM shift registers for the 6 parallel chains (22 ICs per chain)
  uint16_t data_r[CHAIN_LEN];
  uint16_t data_g[CHAIN_LEN];
  uint16_t data_b[CHAIN_LEN];
  uint32_t dclk_idx;
  uint32_t latch_cycle;

  uint32_t le_pulse_count;
  bool le_high;
} matrix_chip_t;

static void on_dclk_change(void *user_data, pin_t pin, uint32_t value) {
  matrix_chip_t *chip = (matrix_chip_t *)user_data;
  if (value == HIGH) {
    if (chip->le_high) {
      chip->le_pulse_count++;
    }

    // Shift in 1 bit (MSB-first) on each of the 6 parallel chains.
    // Note: on the 352nd DCLK of a data cycle, LE is held high for the Data Latch
    // strobe while simultaneously clocking in the final LSB of IC 21.
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
