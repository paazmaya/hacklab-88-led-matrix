//! Integration tests that exercise the public modules together.
//!
//! The individual modules (`font`, `frame_buffer`, `chain_mapper`,
//! `bit_stream`, `http_request`) have their own unit tests. These tests
//! live in `tests/` to verify that the pieces fit together correctly —
//! e.g. a request parsed by `http_request` can be drawn into a
//! `FrameBuffer` and then mapped to a bit stream for the chain.

use esp32_led_matrix::bit_stream::{ChainBit, chain_data_bits};
use esp32_led_matrix::chain_mapper::{
    CHAIN_LEN, ICS_PER_CHAIN, LEDS_PER_IC, SCANLINES, compute_chain_data, compute_full_frame,
};
use esp32_led_matrix::font::Font;
use esp32_led_matrix::frame_buffer::FrameBuffer;
use esp32_led_matrix::http_request::{MAX_MESSAGE_LEN, NOT_FOUND_RESPONSE, dispatch, html_page};
use esp32_led_matrix::{MATRIX_HEIGHT, MATRIX_WIDTH};

#[test]
fn public_constants_hold_invariants() {
    assert_eq!(MATRIX_WIDTH, 88);
    assert_eq!(MATRIX_HEIGHT, 88);
    assert!(MATRIX_WIDTH > 0 && MATRIX_HEIGHT > 0);
    assert!(SCANLINES > 0 && LEDS_PER_IC > 0 && ICS_PER_CHAIN > 0);
    assert_eq!(CHAIN_LEN, 44);
    // CHAIN_LEN must equal 2 * ICS_PER_CHAIN because each chain holds two
    // pixel groups per cycle (top/bottom row groups).
    assert_eq!(CHAIN_LEN, 2 * ICS_PER_CHAIN);
}

#[test]
fn frame_buffer_round_trip_matches_constant_pixel_value() {
    let mut fb = FrameBuffer::new();
    fb.set_pixel(0, 0, 11, 22, 33);
    fb.set_pixel(87, 87, 44, 55, 66);
    assert_eq!(fb.get_pixel(0, 0), [11, 22, 33]);
    assert_eq!(fb.get_pixel(87, 87), [44, 55, 66]);
    // Unknown coordinates are black.
    assert_eq!(fb.get_pixel(MATRIX_WIDTH, 0), [0, 0, 0]);
    assert_eq!(fb.get_pixel(0, MATRIX_HEIGHT), [0, 0, 0]);
}

#[test]
fn http_to_frame_to_chain_pipeline_produces_expected_pixel() {
    // 1. Simulate a text update request from a client.
    let request = b"GET /text?msg=hi HTTP/1.1\r\nHost: example\r\n\r\n";
    let response = dispatch(request);
    assert!(response.body.starts_with(b"HTTP/1.1 200"));
    let text = response
        .display_text
        .as_ref()
        .expect("text update should set display_text");
    assert_eq!(text.as_str(), "hi");

    // 2. Render the decoded text into the frame buffer.
    let mut fb = FrameBuffer::new();
    fb.display_text(text.as_str());

    // 3. The buffer must contain at least one non-black pixel because the
    //    'h'/'i' glyphs paint something.
    let mut lit = 0usize;
    for row in fb.as_pixels().iter() {
        for px in row.iter() {
            if *px != [0, 0, 0] {
                lit += 1;
            }
        }
    }
    assert!(lit > 0, "rendered text should light at least one pixel");

    // 4. Run the chain mapper for every (scanline, led) cycle — none of
    //    these calls may panic, and the output array must have the
    //    expected length.
    let frame = compute_full_frame(fb.as_pixels());
    assert_eq!(frame.len(), SCANLINES * LEDS_PER_IC);
    for cycle in frame.iter() {
        assert_eq!(cycle.len(), CHAIN_LEN);
    }
}

#[test]
fn http_clear_request_produces_empty_display_text() {
    let response = dispatch(b"GET /clear HTTP/1.1");
    assert!(response.body.starts_with(b"HTTP/1.1 200"));
    let text = response
        .display_text
        .as_ref()
        .expect("clear should set display_text");
    assert_eq!(text.as_str(), "");
}

#[test]
fn http_root_request_returns_html_but_no_display_change() {
    let response = dispatch(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(response.body.starts_with(b"HTTP/1.1 200"));
    assert!(response.display_text.is_none());
}

#[test]
fn http_unknown_path_returns_404_and_no_display_change() {
    let response = dispatch(b"GET /admin HTTP/1.1\r\n\r\n");
    assert_eq!(response.body, NOT_FOUND_RESPONSE);
    assert!(response.display_text.is_none());
}

#[test]
fn http_text_request_truncates_to_max_message_length() {
    // Build a request whose msg= value is longer than the cap.
    let long_payload = "A".repeat(MAX_MESSAGE_LEN + 10);
    let request = format!("GET /text?msg={long_payload} HTTP/1.1\r\n\r\n");
    let response = dispatch(request.as_bytes());
    let text = response.display_text.expect("should set text");
    assert_eq!(text.len(), MAX_MESSAGE_LEN);
    assert!(text.chars().all(|c| c == 'A'));
}

#[test]
fn http_text_request_with_url_encoded_message_decodes_correctly() {
    let response = dispatch(b"GET /text?msg=hello+world%21 HTTP/1.1\r\n\r\n");
    let text = response.display_text.expect("should set text");
    assert_eq!(text.as_str(), "hello world!");
}

#[test]
fn html_page_is_returned_for_root_request() {
    // The body of a root response starts with the HTTP headers; the
    // parser only returns the headers for GET / — the network layer is
    // expected to append the page. Verify the page helper provides a
    // non-empty payload.
    assert!(!html_page().is_empty());
}

#[test]
fn frame_buffer_clears_previous_pixels_when_displaying_new_text() {
    let mut fb = FrameBuffer::new();
    fb.set_pixel(50, 50, 1, 2, 3);
    fb.display_text("A");
    // No lit pixel should remain at (50, 50) — the buffer is cleared
    // before redrawing.
    assert_eq!(fb.get_pixel(50, 50), [0, 0, 0]);
}

#[test]
fn frame_buffer_then_chain_mapper_round_trip_uniform_color() {
    // A uniform frame buffer should map to a chain vector where every
    // entry is the same color.
    let mut fb = FrameBuffer::new();
    // Paint a non-black uniform color.
    for y in 0..MATRIX_HEIGHT {
        for x in 0..MATRIX_WIDTH {
            fb.set_pixel(x, y, 0x1111, 0x2222, 0x3333);
        }
    }
    let mut data = [[0u16; 3]; CHAIN_LEN];
    compute_chain_data(0, 0, fb.as_pixels(), &mut data);
    for entry in data.iter() {
        assert_eq!(*entry, [0x1111, 0x2222, 0x3333]);
    }
}

#[test]
fn chain_data_bits_length_matches_constants() {
    // ICS_PER_CHAIN pixels of 16 bits = ICS_PER_CHAIN * 16 DCLKs per
    // (scanline, led) cycle.
    let data = [[0u16; 3]; CHAIN_LEN];
    let bits = chain_data_bits(&data);
    assert_eq!(bits.len(), ICS_PER_CHAIN * 16);
}

#[test]
fn chain_data_bits_last_bit_is_le_high_for_full_frame() {
    // For any cycle, the last bit out of chain_data_bits must raise LE
    // so the ICs latch the values.
    let data = [[0u16; 3]; CHAIN_LEN];
    let bits = chain_data_bits(&data);
    assert!(matches!(bits.last(), Some(ChainBit { le_high: true, .. })));
    for b in &bits[..bits.len() - 1] {
        assert!(!b.le_high);
    }
}

#[test]
fn full_pipeline_text_to_uniform_chain_data() {
    // Render "AA" into a frame buffer. After pixel-mapping through the
    // chain mapper, no cycle should be all-zero (i.e. the lit pixels
    // contributed to at least one chain-data slot).
    let mut fb = FrameBuffer::new();
    fb.display_text("AA");
    let frame = compute_full_frame(fb.as_pixels());
    let mut cycles_with_signal = 0usize;
    for cycle in frame.iter() {
        if cycle.iter().any(|p| *p != [0, 0, 0]) {
            cycles_with_signal += 1;
        }
    }
    assert!(
        cycles_with_signal > 0,
        "rendered text should show up in at least one chain-data cycle"
    );
}

#[test]
fn font_dimensions_match_expectations() {
    let font = Font::new();
    assert_eq!(font.width(), 5);
    assert_eq!(font.height(), 7);
}

#[test]
fn font_handles_lowercase_via_uppercase_fallback() {
    let font = Font::new();
    let upper_a = font.get_glyph('A').expect("uppercase A has glyph");
    let lower_a = font.get_glyph('a').expect("lowercase a maps to A");
    // Lowercase should resolve to the exact same glyph as uppercase.
    assert_eq!(upper_a, lower_a);
}
