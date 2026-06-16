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
    /// vertically. Characters that don't fit are dropped.
    pub fn display_text(&mut self, text: &str) {
        self.clear();
        if text.is_empty() {
            return;
        }

        let start_y = (MATRIX_HEIGHT - self.font.height()) / 2;
        let mut x = 4;
        let max_x = MATRIX_WIDTH.saturating_sub(self.font.width());

        for ch in text.chars() {
            if x >= max_x {
                break;
            }
            self.draw_char(ch, x, start_y, 0xFFFF, 0xFFFF, 0xFFFF);
            x += self.font.width() + 1;
        }
    }

    /// Draw one character glyph at `(x, y)` using the supplied color.
    pub fn draw_char(&mut self, ch: char, x: usize, y: usize, r: u16, g: u16, b: u16) {
        let Some(glyph) = self.font.get_glyph(ch) else {
            return;
        };

        for (gy, row) in glyph.iter().enumerate() {
            for (gx, &pixel) in row.iter().enumerate() {
                if pixel != 0 {
                    self.set_pixel(x + gx, y + gy, r, g, b);
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
}
