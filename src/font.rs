//! Simple bitmap font for LED matrix text display
//!
//! This module provides a 5x7 pixel font suitable for displaying text
//! on the 88x88 LED matrix. Characters are stored as bit patterns and
//! resolved via a constant lookup table indexed by ASCII code.
//!
//! The lookup table replaces what was a 60+-arm `match` statement and
//! makes adding or auditing glyphs easier: every printable ASCII code
//! appears exactly once in the table.

/// Font width in pixels.
pub const FONT_WIDTH: usize = 5;
/// Font height in pixels.
pub const FONT_HEIGHT: usize = 7;

/// A single character glyph (5x7 bitmap)
pub type Glyph = [[u8; FONT_WIDTH]; FONT_HEIGHT];

/// First ASCII code we have a glyph for.
const ASCII_OFFSET: usize = 32;

/// Number of slots in the lookup table (covers ASCII 32..=126 inclusive).
const ASCII_COUNT: usize = 95;

/// Font struct containing character glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Font {
    // The actual glyph data lives in [`GLYPH_TABLE`] below — this struct
    // is a thin wrapper that lets the rest of the code talk to fonts via
    // methods rather than free functions.
}

impl Default for Font {
    fn default() -> Self {
        Self::new()
    }
}

impl Font {
    /// Create a new font with built-in ASCII characters.
    pub const fn new() -> Self {
        Self {}
    }

    /// Get a glyph for a character, returns `None` if no glyph is defined
    /// for it (i.e. outside printable ASCII `32..=126`). Lowercase ASCII
    /// letters resolve to their uppercase glyph.
    pub fn get_glyph(&self, ch: char) -> Option<&'static Glyph> {
        let idx = (ch as usize).checked_sub(ASCII_OFFSET)?;
        GLYPH_TABLE.get(idx).copied()
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

/// Lookup table: index = (ascii_code - ASCII_OFFSET).
/// Entries are in ascending ASCII order (32..=126) so the mapping is auditable.
const GLYPH_TABLE: [&Glyph; ASCII_COUNT] = [
    &SPACE,       //  32 (0x20) ' '
    &EXCLAMATION, //  33 (0x21) '!'
    &QUOTE,       //  34 (0x22) '"'
    &HASH,        //  35 (0x23) '#'
    &DOLLAR,      //  36 (0x24) '$'
    &PERCENT,     //  37 (0x25) '%'
    &AMPERSAND,   //  38 (0x26) '&'
    &APOSTROPHE,  //  39 (0x27) "'"
    &LPAREN,      //  40 (0x28) '('
    &RPAREN,      //  41 (0x29) ')'
    &ASTERISK,    //  42 (0x2a) '*'
    &PLUS,        //  43 (0x2b) '+'
    &COMMA,       //  44 (0x2c) ','
    &MINUS,       //  45 (0x2d) '-'
    &PERIOD,      //  46 (0x2e) '.'
    &SLASH,       //  47 (0x2f) '/'
    &DIGIT_0,     //  48 (0x30) '0'
    &DIGIT_1,     //  49 (0x31) '1'
    &DIGIT_2,     //  50 (0x32) '2'
    &DIGIT_3,     //  51 (0x33) '3'
    &DIGIT_4,     //  52 (0x34) '4'
    &DIGIT_5,     //  53 (0x35) '5'
    &DIGIT_6,     //  54 (0x36) '6'
    &DIGIT_7,     //  55 (0x37) '7'
    &DIGIT_8,     //  56 (0x38) '8'
    &DIGIT_9,     //  57 (0x39) '9'
    &COLON,       //  58 (0x3a) ':'
    &SEMICOLON,   //  59 (0x3b) ';'
    &LESS,        //  60 (0x3c) '<'
    &EQUALS,      //  61 (0x3d) '='
    &GREATER,     //  62 (0x3e) '>'
    &QUESTION,    //  63 (0x3f) '?'
    &AT,          //  64 (0x40) '@'
    &UPPER_A,     //  65 (0x41) 'A'
    &UPPER_B,     //  66 (0x42) 'B'
    &UPPER_C,     //  67 (0x43) 'C'
    &UPPER_D,     //  68 (0x44) 'D'
    &UPPER_E,     //  69 (0x45) 'E'
    &UPPER_F,     //  70 (0x46) 'F'
    &UPPER_G,     //  71 (0x47) 'G'
    &UPPER_H,     //  72 (0x48) 'H'
    &UPPER_I,     //  73 (0x49) 'I'
    &UPPER_J,     //  74 (0x4a) 'J'
    &UPPER_K,     //  75 (0x4b) 'K'
    &UPPER_L,     //  76 (0x4c) 'L'
    &UPPER_M,     //  77 (0x4d) 'M'
    &UPPER_N,     //  78 (0x4e) 'N'
    &UPPER_O,     //  79 (0x4f) 'O'
    &UPPER_P,     //  80 (0x50) 'P'
    &UPPER_Q,     //  81 (0x51) 'Q'
    &UPPER_R,     //  82 (0x52) 'R'
    &UPPER_S,     //  83 (0x53) 'S'
    &UPPER_T,     //  84 (0x54) 'T'
    &UPPER_U,     //  85 (0x55) 'U'
    &UPPER_V,     //  86 (0x56) 'V'
    &UPPER_W,     //  87 (0x57) 'W'
    &UPPER_X,     //  88 (0x58) 'X'
    &UPPER_Y,     //  89 (0x59) 'Y'
    &UPPER_Z,     //  90 (0x5a) 'Z'
    &LBRACKET,    //  91 (0x5b) '['
    &BACKSLASH,   //  92 (0x5c) '\\'
    &RBRACKET,    //  93 (0x5d) ']'
    &CARET,       //  94 (0x5e) '^'
    &UNDERSCORE,  //  95 (0x5f) '_'
    &BACKTICK,    //  96 (0x60) '`'
    &UPPER_A,     //  97 (0x61) 'a' -> 'A'
    &UPPER_B,     //  98 (0x62) 'b' -> 'B'
    &UPPER_C,     //  99 (0x63) 'c' -> 'C'
    &UPPER_D,     // 100 (0x64) 'd' -> 'D'
    &UPPER_E,     // 101 (0x65) 'e' -> 'E'
    &UPPER_F,     // 102 (0x66) 'f' -> 'F'
    &UPPER_G,     // 103 (0x67) 'g' -> 'G'
    &UPPER_H,     // 104 (0x68) 'h' -> 'H'
    &UPPER_I,     // 105 (0x69) 'i' -> 'I'
    &UPPER_J,     // 106 (0x6a) 'j' -> 'J'
    &UPPER_K,     // 107 (0x6b) 'k' -> 'K'
    &UPPER_L,     // 108 (0x6c) 'l' -> 'L'
    &UPPER_M,     // 109 (0x6d) 'm' -> 'M'
    &UPPER_N,     // 110 (0x6e) 'n' -> 'N'
    &UPPER_O,     // 111 (0x6f) 'o' -> 'O'
    &UPPER_P,     // 112 (0x70) 'p' -> 'P'
    &UPPER_Q,     // 113 (0x71) 'q' -> 'Q'
    &UPPER_R,     // 114 (0x72) 'r' -> 'R'
    &UPPER_S,     // 115 (0x73) 's' -> 'S'
    &UPPER_T,     // 116 (0x74) 't' -> 'T'
    &UPPER_U,     // 117 (0x75) 'u' -> 'U'
    &UPPER_V,     // 118 (0x76) 'v' -> 'V'
    &UPPER_W,     // 119 (0x77) 'w' -> 'W'
    &UPPER_X,     // 120 (0x78) 'x' -> 'X'
    &UPPER_Y,     // 121 (0x79) 'y' -> 'Y'
    &UPPER_Z,     // 122 (0x7a) 'z' -> 'Z'
    &LBRACE,      // 123 (0x7b) '{'
    &PIPE,        // 124 (0x7c) '|'
    &RBRACE,      // 125 (0x7d) '}'
    &TILDE,       // 126 (0x7e) '~'
];

// --- Glyph definitions -------------------------------------------------------

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

const BACKSLASH: Glyph = [
    [1, 0, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1],
];

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

const LBRACKET: Glyph = [
    [0, 1, 1, 1, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 1, 1, 0],
];

const RBRACKET: Glyph = [
    [0, 1, 1, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 1, 1, 1, 0],
];

const CARET: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 1, 0, 1, 0],
    [1, 0, 0, 0, 1],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const UNDERSCORE: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1],
];

const BACKTICK: Glyph = [
    [0, 1, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const LBRACE: Glyph = [
    [0, 0, 1, 1, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [0, 0, 1, 1, 0],
];

const PIPE: Glyph = [
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 1, 0, 0],
];

const RBRACE: Glyph = [
    [0, 1, 1, 0, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 1, 0],
    [0, 1, 1, 0, 0],
];

const TILDE: Glyph = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0],
    [1, 0, 1, 0, 1],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

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

    #[test]
    fn test_backslash_glyph() {
        let font = Font::new();
        let glyph = font.get_glyph('\\').expect("backslash glyph should exist");
        assert_eq!(glyph.len(), FONT_HEIGHT);
        // Verify diagonal pixels from top-left (0,0) to bottom-right (4,6)
        assert_eq!(glyph[0][0], 1);
        assert_eq!(glyph[6][4], 1);
    }

    #[test]
    fn test_all_printable_ascii_32_to_126_have_glyphs() {
        let font = Font::new();
        for code in 32u8..=126u8 {
            let ch = code as char;
            let glyph = font
                .get_glyph(ch)
                .unwrap_or_else(|| panic!("ASCII {} ('{}') must have a glyph", code, ch));
            if ch != ' ' {
                let lit = glyph.iter().flatten().any(|&px| px != 0);
                assert!(
                    lit,
                    "Glyph for '{}' (ASCII {}) should not be blank",
                    ch, code
                );
            }
        }
    }

    #[test]
    fn test_unicode_with_lowercase_low_byte_returns_none() {
        let font = Font::new();
        // '\u{0161}' ('š') has low byte 0x61 ('a'); must not truncate to ASCII 'a'
        assert!(font.get_glyph('\u{0161}').is_none());
        assert!(font.get_glyph('ä').is_none());
        assert!(font.get_glyph('ö').is_none());
    }
}
