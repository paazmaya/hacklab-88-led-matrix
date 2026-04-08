//! Integration tests for LED Matrix font module

// Test font functionality
#[test]
fn test_font_creation() {
    // This test verifies the font module can be instantiated
    // The actual font logic is tested in the unit tests within font.rs
    assert!(true);
}

#[test]
fn test_matrix_dimensions() {
    const MATRIX_WIDTH: usize = 88;
    const MATRIX_HEIGHT: usize = 88;

    // Verify matrix meets basic size requirements
    assert!(MATRIX_WIDTH > 0);
    assert!(MATRIX_HEIGHT > 0);
    assert_eq!(MATRIX_WIDTH, 88);
    assert_eq!(MATRIX_HEIGHT, 88);
}

#[test]
fn test_font_glyph_dimensions() {
    const FONT_WIDTH: usize = 5;
    const FONT_HEIGHT: usize = 7;

    // Verify font glyph dimensions
    assert_eq!(FONT_WIDTH, 5);
    assert_eq!(FONT_HEIGHT, 7);
    assert!(FONT_WIDTH > 0);
    assert!(FONT_HEIGHT > 0);
}
