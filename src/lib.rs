//! LED Matrix Controller Library
//!
//! This library exports the testable core components of the 88x88 RGB LED matrix controller.
//! The architecture decouples hardware-independent rendering and protocol generation from
//! hardware I/O:
//!
//! - [`frame_buffer`]: In-memory 88x88 RGB bitmap storage with 16-bit PWM depth per channel,
//!   pixel manipulation, bounds clipping, and text rendering.
//! - [`font`]: Built-in 5x7 ASCII bitmap font providing glyph data and character rendering.
//! - [`chain_mapper`]: Pure mapping function translating the linear 88x88 frame buffer into the
//!   44-pixel-per-cycle hardware ordering required by the dual shift-register chains and 11:1 multiplexing.
//! - [`bit_stream`]: Generator transforming 44-pixel chain vectors and configuration registers into
//!   exact 352-bit DCLK/LE pulse sequences for MBI5252 LED driver ICs.
//! - [`http_request`]: Lightweight, allocation-free HTTP request parser that extracts text,
//!   coordinates, hex colors, clear flags, and brightness adjustments from query strings.
//! - [`status_indicator`]: Visual system and Wi-Fi connection state machine mapped to a single
//!   corner status pixel with configurable cadence, color, and heartbeat patterns.
//!
//! When the `esp32` Cargo feature is enabled, hardware-dependent modules (`led_matrix`, `http_server`,
//! and `wifi`) bind these pure data structures to ESP32-C3 GPIO pins, the hardware radio, and async
//! Embassy tasks.

#![no_std]

pub mod bit_stream;
pub mod chain_mapper;
pub mod font;
pub mod frame_buffer;
pub mod http_request;
pub mod status_indicator;

/// Horizontal resolution of the LED matrix in pixels.
pub const MATRIX_WIDTH: usize = 88;

/// Vertical resolution of the LED matrix in pixels.
pub const MATRIX_HEIGHT: usize = 88;

/// Default display brightness percentage (5..=100%).
pub const DEFAULT_BRIGHTNESS: u8 = 50;
