#include "wokwi-api.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MATRIX_WIDTH 88
#define MATRIX_HEIGHT 88
#define CHAIN_LEN 44
#define SCANLINES 11

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

  uint32_t fb_data[MATRIX_HEIGHT][MATRIX_WIDTH]; // RGBA8888 for Wokwi Framebuffer

  // Shift registers for the 6 chains (44 IC outputs per chain)
  uint8_t shift_r1[CHAIN_LEN];
  uint8_t shift_g1[CHAIN_LEN];
  uint8_t shift_b1[CHAIN_LEN];
  uint8_t shift_r2[CHAIN_LEN];
  uint8_t shift_g2[CHAIN_LEN];
  uint8_t shift_b2[CHAIN_LEN];
  uint32_t shift_idx;

  uint32_t le_pulse_count;
  bool le_high;
} matrix_chip_t;

static void on_dclk_change(void *user_data, pin_t pin, uint32_t value) {
  matrix_chip_t *chip = (matrix_chip_t *)user_data;
  if (value == HIGH) {
    if (chip->le_high) {
      chip->le_pulse_count++;
      return;
    }

    // Shift in 1 bit on each of the 6 parallel chains
    uint32_t idx = chip->shift_idx % CHAIN_LEN;
    chip->shift_r1[idx] = pin_read(chip->pin_dr1);
    chip->shift_g1[idx] = pin_read(chip->pin_dg1);
    chip->shift_b1[idx] = pin_read(chip->pin_db1);
    chip->shift_r2[idx] = pin_read(chip->pin_dr2);
    chip->shift_g2[idx] = pin_read(chip->pin_dg2);
    chip->shift_b2[idx] = pin_read(chip->pin_db2);
    chip->shift_idx++;
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
      // Data Latch: copy current shift data to active scanline
      uint32_t addr = (pin_read(chip->pin_a0)) |
                      (pin_read(chip->pin_a1) << 1) |
                      (pin_read(chip->pin_a2) << 2) |
                      (pin_read(chip->pin_a3) << 3);

      if (addr < SCANLINES) {
        // Map chain data to rows (chain 1: 0..43, chain 2: 44..87)
        for (uint32_t i = 0; i < CHAIN_LEN; i++) {
          uint32_t row1 = addr * 4 + (i / 11);
          uint32_t col = (i % 11) * 8;
          uint32_t row2 = 44 + row1;

          if (row1 < 44 && col < MATRIX_WIDTH) {
            uint8_t r = chip->shift_r1[i] ? 255 : 0;
            uint8_t g = chip->shift_g1[i] ? 255 : 0;
            uint8_t b = chip->shift_b1[i] ? 255 : 0;
            chip->fb_data[row1][col] = 0xFF000000 | (b << 16) | (g << 8) | r;
          }
          if (row2 < MATRIX_HEIGHT && col < MATRIX_WIDTH) {
            uint8_t r = chip->shift_r2[i] ? 255 : 0;
            uint8_t g = chip->shift_g2[i] ? 255 : 0;
            uint8_t b = chip->shift_b2[i] ? 255 : 0;
            chip->fb_data[row2][col] = 0xFF000000 | (b << 16) | (g << 8) | r;
          }
        }
      }
      chip->shift_idx = 0;
    } else if (cmd == 2) {
      // VSYNC: Buffer swap / update display
      if (chip->fb) {
        buffer_write(chip->fb, 0, chip->fb_data, sizeof(chip->fb_data));
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
