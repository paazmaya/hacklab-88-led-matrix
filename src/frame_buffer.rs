//! LED Matrix Frame Buffer
//!
//! Pure data type that owns the 88x88 RGB pixel array plus text rendering
//! logic. No GPIO dependencies, so it can be unit-tested on the host.

use crate::font::Font;
use crate::{MATRIX_HEIGHT, MATRIX_WIDTH};

/// RGB color for a single pixel (16-bit per channel).
pub type Pixel = [u16; 3];

/// 88x88 RGB frame buffer.
///
/// `pixels[row][col]` is `[r, g, b]` with each channel stored as `u16` to
/// match the LED panel's 16-bit PWM depth. Indexing is `pixels[y][x]`
/// (row-major), matching the natural "row, column" coordinate system used
/// by the rest of the driver.
pub struct FrameBuffer {
    pixels: [[Pixel; MATRIX_WIDTH]; MATRIX_HEIGHT],
    font: Font,
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuffer {
    /// Create a fresh, blank frame buffer with the built-in font.
    pub const fn new() -> Self {
        Self {
            pixels: [[[0u16; 3]; MATRIX_WIDTH]; MATRIX_HEIGHT],
            font: Font::new(),
        }
    }

    /// Reset every pixel to black (`[0, 0, 0]`).
    pub fn clear(&mut self) {
        for row in self.pixels.iter_mut() {
            for px in row.iter_mut() {
                *px = [0, 0, 0];
            }
        }
    }

    /// Set a single pixel's RGB color. Out-of-bounds writes are silently
    /// dropped to mirror the behaviour of the original driver.
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u16, g: u16, b: u16) {
        if x < MATRIX_WIDTH && y < MATRIX_HEIGHT {
            self.pixels[y][x] = [r, g, b];
        }
    }

    /// Read a pixel's RGB color. Returns black for out-of-bounds reads.
    pub fn get_pixel(&self, x: usize, y: usize) -> Pixel {
        if x < MATRIX_WIDTH && y < MATRIX_HEIGHT {
            self.pixels[y][x]
        } else {
            [0, 0, 0]
        }
    }

    /// Render `text` to the buffer, clearing it first.
    ///
    /// Glyphs are drawn left-to-right starting at `x = 4` and centred
    /// vertically using full white (`0xFFFF, 0xFFFF, 0xFFFF`).
    pub fn display_text(&mut self, text: &str) {
        self.clear();
        if text.is_empty() {
            return;
        }

        let start_y = ((MATRIX_HEIGHT - self.font.height()) / 2) as i32;
        self.draw_text_at(text, 4, start_y, 0xFFFF, 0xFFFF, 0xFFFF);
    }

    /// Draw `text` starting at `(start_x, start_y)` with the supplied color.
    ///
    /// - Supports `\n` to advance to the next line: advances `y` by glyph height
    ///   plus 1-pixel line spacing, and resets `x` to `start_x`.
    /// - Advances `x` by glyph width plus 1-pixel character spacing for each glyph.
    /// - Coordinates are signed (`i32`), allowing text to be partially or
    ///   completely clipped outside the 88x88 matrix without underflow or panics.
    pub fn draw_text_at(&mut self, text: &str, start_x: i32, start_y: i32, r: u16, g: u16, b: u16) {
        let mut cur_x = start_x;
        let mut cur_y = start_y;
        let char_advance = (self.font.width() + 1) as i32;
        let line_height = (self.font.height() + 1) as i32;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = start_x;
                cur_y += line_height;
                continue;
            }
            self.draw_char(ch, cur_x, cur_y, r, g, b);
            cur_x += char_advance;
        }
    }

    /// Draw one character glyph at `(x, y)` using the supplied color.
    ///
    /// Accepts signed `(x, y)` coordinates and clips gracefully: pixels outside
    /// `0..MATRIX_WIDTH` and `0..MATRIX_HEIGHT` are dropped.
    pub fn draw_char(&mut self, ch: char, x: i32, y: i32, r: u16, g: u16, b: u16) {
        let Some(glyph) = self.font.get_glyph(ch) else {
            return;
        };

        for (gy, row) in glyph.iter().enumerate() {
            let py = y + gy as i32;
            if py < 0 || py >= MATRIX_HEIGHT as i32 {
                continue;
            }
            for (gx, &pixel) in row.iter().enumerate() {
                let px = x + gx as i32;
                if pixel != 0 && px >= 0 && px < MATRIX_WIDTH as i32 {
                    self.pixels[py as usize][px as usize] = [r, g, b];
                }
            }
        }
    }

    /// Borrow the raw 88x88 RGB array.
    ///
    /// Required by [`crate::chain_mapper::compute_chain_data`] when running
    /// in `no_std` mode — there is no `AsRef` blanket that would let it
    /// dereference through to the inner field.
    pub fn as_pixels(&self) -> &[[Pixel; MATRIX_WIDTH]; MATRIX_HEIGHT] {
        &self.pixels
    }

    /// Mutable access to the raw pixel array.
    pub fn as_pixels_mut(&mut self) -> &mut [[Pixel; MATRIX_WIDTH]; MATRIX_HEIGHT] {
        &mut self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_blank() {
        let fb = FrameBuffer::new();
        for row in fb.as_pixels().iter() {
            for px in row.iter() {
                assert_eq!(*px, [0, 0, 0]);
            }
        }
    }

    #[test]
    fn default_matches_new() {
        let fb = FrameBuffer::default();
        assert_eq!(fb.get_pixel(0, 0), [0, 0, 0]);
    }

    #[test]
    fn set_and_get_pixel() {
        let mut fb = FrameBuffer::new();
        fb.set_pixel(10, 20, 100, 200, 300);
        assert_eq!(fb.get_pixel(10, 20), [100, 200, 300]);
    }

    #[test]
    fn out_of_bounds_set_is_noop() {
        let mut fb = FrameBuffer::new();
        fb.set_pixel(MATRIX_WIDTH, 0, 1, 2, 3);
        fb.set_pixel(0, MATRIX_HEIGHT, 1, 2, 3);
        // buffer still zeroed
        assert_eq!(fb.get_pixel(0, 0), [0, 0, 0]);
    }

    #[test]
    fn out_of_bounds_get_returns_black() {
        let fb = FrameBuffer::new();
        assert_eq!(fb.get_pixel(MATRIX_WIDTH, 0), [0, 0, 0]);
        assert_eq!(fb.get_pixel(0, MATRIX_HEIGHT), [0, 0, 0]);
    }

    #[test]
    fn clear_resets_all_pixels() {
        let mut fb = FrameBuffer::new();
        fb.set_pixel(1, 1, 50, 60, 70);
        fb.set_pixel(87, 87, 80, 90, 100);
        fb.clear();
        assert_eq!(fb.get_pixel(1, 1), [0, 0, 0]);
        assert_eq!(fb.get_pixel(87, 87), [0, 0, 0]);
    }

    #[test]
    fn empty_text_clears_buffer() {
        let mut fb = FrameBuffer::new();
        fb.set_pixel(5, 5, 1, 2, 3);
        fb.display_text("");
        assert_eq!(fb.get_pixel(5, 5), [0, 0, 0]);
    }

    #[test]
    fn draw_char_writes_only_set_pixels() {
        let mut fb = FrameBuffer::new();
        fb.draw_char('!', 0, 0, 0xFFFF, 0xFFFF, 0xFFFF);
        // The '!' glyph has at least one set pixel
        let mut any_set = false;
        for row in fb.as_pixels().iter() {
            for px in row.iter() {
                if *px != [0, 0, 0] {
                    any_set = true;
                    break;
                }
            }
        }
        assert!(any_set, "expected some pixels lit after drawing '!'");
    }

    #[test]
    fn draw_char_clips_negative_and_out_of_bounds() {
        let mut fb = FrameBuffer::new();
        // Negative coordinates must not panic and must clip cleanly
        fb.draw_char('A', -2, -3, 0x1111, 0x2222, 0x3333);
        // Completely off-screen
        fb.draw_char('A', -20, -20, 0x1111, 0x2222, 0x3333);
        fb.draw_char('A', 200, 200, 0x1111, 0x2222, 0x3333);
    }

    #[test]
    fn draw_text_at_renders_custom_color_and_positions() {
        let mut fb = FrameBuffer::new();
        fb.draw_text_at("A", 10, 20, 0xAAAA, 0xBBBB, 0xCCCC);

        // Top-left pixel of glyph 'A' (gx=0, gy=0 is 0 in standard 5x7 font, but let's check lit pixels)
        let mut lit_count = 0;
        for y in 20..27 {
            for x in 10..15 {
                let px = fb.get_pixel(x, y);
                if px != [0, 0, 0] {
                    assert_eq!(px, [0xAAAA, 0xBBBB, 0xCCCC]);
                    lit_count += 1;
                }
            }
        }
        assert!(lit_count > 0, "expected 'A' glyph to have lit pixels");
    }

    #[test]
    fn draw_text_at_multiline_support() {
        let mut fb = FrameBuffer::new();
        fb.draw_text_at("A\nB", 0, 0, 0xFFFF, 0xFFFF, 0xFFFF);

        // Line 1 ('A') starts at y = 0..7
        let mut line1_lit = false;
        for y in 0..7 {
            for x in 0..5 {
                if fb.get_pixel(x, y) != [0, 0, 0] {
                    line1_lit = true;
                }
            }
        }

        // Line 2 ('B') starts at y = 8..15
        let mut line2_lit = false;
        for y in 8..15 {
            for x in 0..5 {
                if fb.get_pixel(x, y) != [0, 0, 0] {
                    line2_lit = true;
                }
            }
        }

        assert!(line1_lit, "line 1 should be lit");
        assert!(line2_lit, "line 2 should be lit");
    }

    #[test]
    fn draw_text_at_negative_coordinates_gracefully_clips() {
        let mut fb = FrameBuffer::new();
        // Negative x and y partially off-screen
        fb.draw_text_at("A", -2, -2, 0x1111, 0x2222, 0x3333);

        // Verify that pixels within 0..5 and 0..7 are colored
        let mut in_bounds_lit = false;
        for y in 0..5 {
            for x in 0..3 {
                if fb.get_pixel(x, y) == [0x1111, 0x2222, 0x3333] {
                    in_bounds_lit = true;
                }
            }
        }
        assert!(in_bounds_lit, "clipped glyph should have visible pixels");
    }

    #[test]
    fn draw_char_backslash_renders() {
        let mut fb = FrameBuffer::new();
        fb.draw_char('\\', 10, 10, 0x5555, 0x6666, 0x7777);
        assert_eq!(fb.get_pixel(10, 10), [0x5555, 0x6666, 0x7777]);
        assert_eq!(fb.get_pixel(14, 16), [0x5555, 0x6666, 0x7777]);
    }

    #[test]
    fn draw_char_unsupported_character_returns_early() {
        let mut fb = FrameBuffer::new();
        // A character not in the font table should return early without modifying buffer
        fb.draw_char('\u{1000}', 0, 0, 0x5555, 0x6666, 0x7777);
        for row in fb.as_pixels().iter() {
            for px in row.iter() {
                assert_eq!(*px, [0, 0, 0]);
            }
        }
    }

    #[test]
    fn as_pixels_mut_allows_direct_modification() {
        let mut fb = FrameBuffer::new();
        let pixels = fb.as_pixels_mut();
        pixels[5][10] = [0x1234, 0x5678, 0x9ABC];
        assert_eq!(fb.get_pixel(10, 5), [0x1234, 0x5678, 0x9ABC]);
    }
}
