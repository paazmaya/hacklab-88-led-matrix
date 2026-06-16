//! Chain pixel mapping for the Hacklab LED panel.
//!
//! The 88x88 RGB matrix is split into two physical shift-register chains,
//! each holding 22 driver ICs in series. For every `(scanline, led)`
//! cycle, the driver must emit a 44-element vector:
//!
//! - `data[0..10]` and `data[11..21]` are chain 1's two row groups
//! - `data[22..32]` and `data[33..43]` are chain 2's two row groups
//!
//! The translation between the linear 88x88 bitmap and this 44-element
//! ordering is non-obvious — it's a direct port of the `getChainData`
//! function from the wiki's reference implementation. See
//! [`compute_chain_data`] for the mapping.
//!
//! This module is pure data transformation; it has no GPIO dependencies
//! and is fully unit-testable on the host.

use crate::frame_buffer::Pixel;

/// 11 scanlines (multiplexing factor).
pub const SCANLINES: usize = 11;

/// Number of ICs per chain.
pub const ICS_PER_CHAIN: usize = 22;

/// LEDs driven by each IC.
pub const LEDS_PER_IC: usize = 16;

/// Length of the per-cycle chain vector.
pub const CHAIN_LEN: usize = 44;

/// Translate the 88x88 frame buffer into the 44-pixel ordering the chain
/// hardware expects for one `(scanline, led)` cycle.
///
/// Per call: fills `data[0..10]` and `data[11..21]` with chain 1's two row
/// groups, and `data[22..32]` and `data[33..43]` with chain 2's two row
/// groups for this `(scanline, led)`. The frame buffer is indexed as
/// `pixels[y][x]`.
pub fn compute_chain_data(
    scanline: usize,
    led: usize,
    pixels: &[[Pixel; 88]; 88],
    data: &mut [[u16; 3]; CHAIN_LEN],
) {
    // led 0..7 picks one row-group of the scanline; led 8..15 picks the
    // other. ledColumn is the per-group column offset, reversed for the
    // first group to match the physical wiring.
    let led_row: usize = if led < 8 { 11 } else { 0 };
    let mut led_column: usize = led % 8;
    let mut row: usize = scanline + led_row;
    if led_row == 11 {
        led_column = 7 - led_column;
    }

    // Four row blocks, each contributing 11 columns of one row. The start
    // index descends (33, 22, 11, 0) because the loop writes the highest
    // row block first.
    const STARTS: [usize; 4] = [33, 22, 11, 0];
    for &start in &STARTS {
        for i in 0..11usize {
            let col = 8 * i + led_column;
            data[start + i] = pixels[row][col];
        }
        row += 22;
    }
}

/// Build a complete frame's worth of (scanline, led) → chain-data pairs.
///
/// Returns 11 × 16 = 176 cycles, each with its associated 44-pixel vector.
/// The caller typically streams these directly to the shift registers.
pub fn compute_full_frame(
    pixels: &[[Pixel; 88]; 88],
) -> [[[u16; 3]; CHAIN_LEN]; SCANLINES * LEDS_PER_IC] {
    let mut out = [[[0u16; 3]; CHAIN_LEN]; SCANLINES * LEDS_PER_IC];
    for scanline in 0..SCANLINES {
        for led in 0..LEDS_PER_IC {
            let idx = scanline * LEDS_PER_IC + led;
            compute_chain_data(scanline, led, pixels, &mut out[idx]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a frame buffer where every pixel has the same color — easy
    /// to assert against.
    fn uniform_pixels(r: u16, g: u16, b: u16) -> [[Pixel; 88]; 88] {
        [[r, g, b]; 88].map(|row| [row; 88])
    }

    #[test]
    fn uniform_buffer_round_trips_through_mapper() {
        let px = uniform_pixels(7, 11, 13);
        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 0, &px, &mut data);

        // Every chain pixel should be the same as the source uniform color.
        for entry in data.iter() {
            assert_eq!(*entry, [7, 11, 13]);
        }
    }

    #[test]
    fn full_frame_has_expected_cycle_count() {
        let px = uniform_pixels(1, 2, 3);
        let frame = compute_full_frame(&px);
        // SCANLINES * LEDS_PER_IC = 11 * 16 = 176 cycles
        assert_eq!(frame.len(), SCANLINES * LEDS_PER_IC);
        for cycle in frame.iter() {
            for entry in cycle.iter() {
                assert_eq!(*entry, [1, 2, 3]);
            }
        }
    }

    #[test]
    fn single_pixel_is_found_at_expected_chain_position() {
        let mut px = uniform_pixels(0, 0, 0);
        // Paint a single pixel red at (col=2, row=33). With the mapping,
        // led=0 picks led_row=11 → first row block at scanline+11. For
        // scanline=0 that is row=11. led_column = 0. The four row
        // blocks start at rows scanline+11, scanline+33, scanline+55,
        // scanline+77 — none of which is row 33.
        //
        // Instead pick a coordinate we can reason about: row = scanline,
        // col = 8*0 + 0 = 0 for (scanline=0, led=0).
        px[0][0] = [42, 43, 44];

        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 0, &px, &mut data);
        // led_row=11 (led<8), so we walk from row=scanline+11 onwards.
        // scanline=0 → row=11. The first row block reads row 11, the
        // second row 33, the third row 55, the fourth row 77. So the
        // pixel at (col=0,row=0) is *not* hit at scanline=0/led=0.
        assert!(data.iter().all(|c| *c == [0, 0, 0]));

        // Now try (scanline=0, led=8): led_row=0 → row=0. led_column = 0
        // (since led%8 = 0). The first row block reads row 0, starting at
        // data[33]. data[33] = pixels[0][0] = [42, 43, 44].
        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 8, &px, &mut data);
        assert_eq!(data[33], [42, 43, 44]);
    }

    #[test]
    fn led_below_eight_inverts_column_order() {
        let mut px = uniform_pixels(0, 0, 0);
        // For (scanline=0, led=0):
        //   - led_row=11 (led<8)
        //   - led_column starts at led%8=0, then is flipped to 7-0=7
        //   - row = scanline+11 = 11
        //   - First row block starts at data[33], reading pixels[11][7]
        px[11][7] = [9, 9, 9];
        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 0, &px, &mut data);
        assert_eq!(data[33], [9, 9, 9]);

        // For (scanline=0, led=7):
        //   - led_row=11, led_column=7, flipped to 7-7=0
        //   - First row block reads pixels[11][0]
        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 7, &px, &mut data);
        // data[33] = pixels[11][0] which is still zero (only col 7 was set)
        assert_eq!(data[33], [0, 0, 0]);
    }

    #[test]
    fn led_eight_or_above_does_not_invert_columns() {
        let mut px = uniform_pixels(0, 0, 0);
        // For (scanline=0, led=8):
        //   - led_row=0 (led>=8)
        //   - led_column=8%8=0 — NOT flipped
        //   - First row block reads pixels[0][0]
        px[0][0] = [1, 2, 3];
        let mut data = [[0u16; 3]; CHAIN_LEN];
        compute_chain_data(0, 8, &px, &mut data);
        assert_eq!(data[33], [1, 2, 3]);
    }
}
