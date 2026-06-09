//! Simple bitmap font for LED matrix text display
//!
//! This module provides a 5x7 pixel font suitable for displaying text
//! on the 88x88 LED matrix. Characters are stored as bit patterns.

/// Font dimensions
const FONT_WIDTH: usize = 5;
const FONT_HEIGHT: usize = 7;

/// A single character glyph (5x7 bitmap)
pub type Glyph = [[u8; FONT_WIDTH]; FONT_HEIGHT];

/// Font struct containing character glyphs
pub struct Font {
    // Glyphs stored as a simple array indexed by ASCII code
    // Only printable ASCII (32-126) are stored
}

impl Font {
    /// Create a new font with built-in ASCII characters
    pub const fn new() -> Self {
        Self {}
    }

    /// Get a glyph for a character, returns None if not found
    pub fn get_glyph(&self, ch: char) -> Option<&'static Glyph> {
        // Convert to ASCII and get glyph
        let code = ch as u8;

        // Only handle printable ASCII (32-126)
        if !(32..=126).contains(&code) {
            return None;
        }

        // Get the glyph
        Some(match code {
            b' ' => &SPACE,
            b'!' => &EXCLAMATION,
            b'"' => &QUOTE,
            b'#' => &HASH,
            b'$' => &DOLLAR,
            b'%' => &PERCENT,
            b'&' => &AMPERSAND,
            b'\'' => &APOSTROPHE,
            b'(' => &LPAREN,
            b')' => &RPAREN,
            b'*' => &ASTERISK,
            b'+' => &PLUS,
            b',' => &COMMA,
            b'-' => &MINUS,
            b'.' => &PERIOD,
            b'/' => &SLASH,
            b'0' => &DIGIT_0,
            b'1' => &DIGIT_1,
            b'2' => &DIGIT_2,
            b'3' => &DIGIT_3,
            b'4' => &DIGIT_4,
            b'5' => &DIGIT_5,
            b'6' => &DIGIT_6,
            b'7' => &DIGIT_7,
            b'8' => &DIGIT_8,
            b'9' => &DIGIT_9,
            b':' => &COLON,
            b';' => &SEMICOLON,
            b'<' => &LESS,
            b'=' => &EQUALS,
            b'>' => &GREATER,
            b'?' => &QUESTION,
            b'@' => &AT,
            b'A' => &UPPER_A,
            b'B' => &UPPER_B,
            b'C' => &UPPER_C,
            b'D' => &UPPER_D,
            b'E' => &UPPER_E,
            b'F' => &UPPER_F,
            b'G' => &UPPER_G,
            b'H' => &UPPER_H,
            b'I' => &UPPER_I,
            b'J' => &UPPER_J,
            b'K' => &UPPER_K,
            b'L' => &UPPER_L,
            b'M' => &UPPER_M,
            b'N' => &UPPER_N,
            b'O' => &UPPER_O,
            b'P' => &UPPER_P,
            b'Q' => &UPPER_Q,
            b'R' => &UPPER_R,
            b'S' => &UPPER_S,
            b'T' => &UPPER_T,
            b'U' => &UPPER_U,
            b'V' => &UPPER_V,
            b'W' => &UPPER_W,
            b'X' => &UPPER_X,
            b'Y' => &UPPER_Y,
            b'Z' => &UPPER_Z,
            // Lowercase uses same glyphs as uppercase
            b'a'..=b'z' => {
                let upper = code - 32; // Convert to uppercase
                return self.get_glyph(upper as char);
            }
            _ => return None,
        })
    }

    /// Get font width in pixels
    pub const fn width(&self) -> usize {
        FONT_WIDTH
    }

    /// Get font height in pixels
    pub const fn height(&self) -> usize {
        FONT_HEIGHT
    }
}

// Define glyphs as constants
const SPACE: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const EXCLAMATION: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
];

const QUOTE: Glyph = [
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const HASH: Glyph = [
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1],
    [0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1],
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
];

const DOLLAR: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 1, 0, 0],
    [1, 0, 1, 0, 0],
    [0, 1, 1, 1, 0],
    [0, 0, 1, 0, 1],
    [0, 0, 1, 0, 1],
    [0, 1, 1, 1, 0],
];

const PERCENT: Glyph = [
    [1, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 1],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const AMPERSAND: Glyph = [
    [0, 1, 1, 0, 0],
    [1, 0, 0, 1, 0],
    [1, 0, 0, 1, 0],
    [0, 1, 1, 0, 0],
    [1, 0, 1, 0, 0],
    [1, 0, 0, 1, 0],
    [0, 1, 1, 0, 1],
];

const APOSTROPHE: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const LPAREN: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
];

const RPAREN: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
];

const ASTERISK: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 1, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const PLUS: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
];

const COMMA: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
];

const MINUS: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const PERIOD: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
];

const SLASH: Glyph = [
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
];

// Digits
const DIGIT_0: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 1, 1],
    [1, 0, 1, 0, 1],
    [1, 1, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const DIGIT_1: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 1, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 1, 1, 0],
];

const DIGIT_2: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 1, 1, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

const DIGIT_3: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 1, 1, 0],
    [0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const DIGIT_4: Glyph = [
    [0, 0, 0, 1, 0],
    [0, 0, 1, 1, 0],
    [0, 1, 0, 1, 0],
    [1, 0, 0, 1, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
];

const DIGIT_5: Glyph = [
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const DIGIT_6: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const DIGIT_7: Glyph = [
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
];

const DIGIT_8: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const DIGIT_9: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const COLON: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
];

const SEMICOLON: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
];

const LESS: Glyph = [
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 1, 0],
];

const EQUALS: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const GREATER: Glyph = [
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
];

const QUESTION: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 1, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
];

const AT: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 1, 1, 1],
    [1, 0, 1, 0, 1],
    [1, 0, 1, 1, 1],
    [1, 0, 0, 0, 0],
    [0, 1, 1, 1, 0],
];

// Uppercase letters
const UPPER_A: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
];

const UPPER_B: Glyph = [
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
];

const UPPER_C: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const UPPER_D: Glyph = [
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
];

const UPPER_E: Glyph = [
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

const UPPER_F: Glyph = [
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
];

const UPPER_G: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 0],
    [1, 0, 1, 1, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const UPPER_H: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
];

const UPPER_I: Glyph = [
    [0, 1, 1, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 1, 1, 0],
];

const UPPER_J: Glyph = [
    [0, 0, 1, 1, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [1, 0, 0, 1, 0],
    [0, 1, 1, 0, 0],
];

const UPPER_K: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 0, 0, 1, 0],
    [1, 0, 1, 0, 0],
    [1, 1, 0, 0, 0],
    [1, 0, 1, 0, 0],
    [1, 0, 0, 1, 0],
    [1, 0, 0, 0, 1],
];

const UPPER_L: Glyph = [
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

const UPPER_M: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 1, 0, 1, 1],
    [1, 0, 1, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
];

const UPPER_N: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 1, 0, 0, 1],
    [1, 0, 1, 0, 1],
    [1, 0, 0, 1, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
];

const UPPER_O: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const UPPER_P: Glyph = [
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
];

const UPPER_Q: Glyph = [
    [0, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 1, 0, 1],
    [1, 0, 0, 1, 0],
    [0, 1, 1, 0, 1],
];

const UPPER_R: Glyph = [
    [1, 1, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
    [1, 0, 1, 0, 0],
    [1, 0, 0, 1, 0],
    [1, 0, 0, 0, 1],
];

const UPPER_S: Glyph = [
    [0, 1, 1, 1, 1],
    [1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [0, 1, 1, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 0, 1],
    [1, 1, 1, 1, 0],
];

const UPPER_T: Glyph = [
    [1, 1, 1, 1, 1],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
];

const UPPER_U: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 1, 1, 0],
];

const UPPER_V: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
    [0, 0, 1, 0, 0],
];

const UPPER_W: Glyph = [
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 0, 0, 1],
    [1, 0, 1, 0, 1],
    [1, 0, 1, 0, 1],
    [1, 1, 0, 1, 1],
    [1, 0, 0, 0, 1],
];

const UPPER_X: Glyph = [
    [1, 0, 0, 0, 1],
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 1, 0],
    [0, 1, 0, 1, 0],
    [1, 0, 0, 0, 1],
];

const UPPER_Y: Glyph = [
    [1, 0, 0, 0, 1],
    [0, 1, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
];

const UPPER_Z: Glyph = [
    [1, 1, 1, 1, 1],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

// Default trait implementation
#[cfg_attr(not(test), allow(dead_code))]
impl Default for Font {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_new() {
        let font = Font::new();
        assert_eq!(font.width(), 5);
        assert_eq!(font.height(), 7);
    }

    #[test]
    fn test_font_default() {
        let font = Font::default();
        assert_eq!(font.width(), 5);
        assert_eq!(font.height(), 7);
    }

    #[test]
    fn test_font_dimensions() {
        let font = Font::new();
        assert_eq!(font.width(), FONT_WIDTH);
        assert_eq!(font.height(), FONT_HEIGHT);
    }

    #[test]
    fn test_space_glyph() {
        let font = Font::new();
        let glyph = font.get_glyph(' ');
        assert!(glyph.is_some());
        let glyph = glyph.unwrap();
        // All zeros for space
        for row in glyph.iter() {
            for &pixel in row.iter() {
                assert_eq!(pixel, 0);
            }
        }
    }

    #[test]
    fn test_digit_glyphs() {
        let font = Font::new();

        // Test all digits 0-9 exist
        for ch in '0'..='9' {
            assert!(font.get_glyph(ch).is_some(), "Digit {} should exist", ch);
        }
    }

    #[test]
    fn test_uppercase_letters() {
        let font = Font::new();

        // Test all uppercase letters A-Z exist
        for ch in 'A'..='Z' {
            assert!(font.get_glyph(ch).is_some(), "Letter {} should exist", ch);
        }
    }

    #[test]
    fn test_lowercase_letters() {
        let font = Font::new();

        // Test all lowercase letters a-z exist and map to uppercase
        for ch in 'a'..='z' {
            assert!(
                font.get_glyph(ch).is_some(),
                "Lowercase letter {} should exist",
                ch
            );

            // Verify lowercase maps to same glyph as uppercase
            let lower = font.get_glyph(ch).unwrap();
            let upper = font.get_glyph(ch.to_uppercase().next().unwrap()).unwrap();
            assert_eq!(
                lower as *const _, upper as *const _,
                "Lowercase {} should map to same glyph as uppercase",
                ch
            );
        }
    }

    #[test]
    fn test_special_characters() {
        let font = Font::new();

        let special_chars = "!\"#$%&'()*+,-./:;<=>?@";
        for ch in special_chars.chars() {
            assert!(
                font.get_glyph(ch).is_some(),
                "Special character '{}' should exist",
                ch
            );
        }
    }

    #[test]
    fn test_invalid_characters() {
        let font = Font::new();

        // Test characters outside printable ASCII range return None
        assert!(font.get_glyph('\x00').is_none()); // Null character
        assert!(font.get_glyph('\x01').is_none()); // SOH
        assert!(font.get_glyph('\n').is_none()); // Newline
        assert!(font.get_glyph('\t').is_none()); // Tab
    }

    #[test]
    fn test_boundary_characters() {
        let font = Font::new();

        // Test boundary ASCII characters
        assert!(font.get_glyph(' ').is_some()); // First printable (32)
        assert!(font.get_glyph('Z').is_some()); // Far boundary (uppercase letter)
        assert!(font.get_glyph('\x1f').is_none()); // Just before first printable
        assert!(font.get_glyph('\x7f').is_none()); // DEL character (just after printable range)
    }

    #[test]
    fn test_glyph_dimensions() {
        let font = Font::new();

        // Test that returned glyphs have correct dimensions
        let glyph = font.get_glyph('A').unwrap();
        assert_eq!(glyph.len(), FONT_HEIGHT);
        for row in glyph.iter() {
            assert_eq!(row.len(), FONT_WIDTH);
        }
    }

    #[test]
    fn test_exclamation_mark() {
        let font = Font::new();
        let exclamation = font.get_glyph('!').unwrap();

        // Verify the pattern matches what's defined
        assert_eq!(exclamation[0][2], 1); // Top pixel
        assert_eq!(exclamation[1][2], 1);
        assert_eq!(exclamation[5][2], 0); // Gap in the middle
        assert_eq!(exclamation[6][2], 1); // Dot at bottom
    }

    #[test]
    fn test_glyph_immutability() {
        let font = Font::new();

        // Get the same glyph twice and verify they're references to the same data
        let glyph1 = font.get_glyph('X').unwrap();
        let glyph2 = font.get_glyph('X').unwrap();
        assert_eq!(glyph1 as *const _, glyph2 as *const _);
    }
}
