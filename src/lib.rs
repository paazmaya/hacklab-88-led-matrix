//! LED Matrix Controller Library
//!
//! This library exports testable components of the LED matrix controller.
//! `font`, `frame_buffer`, `chain_mapper`, `bit_stream`, and `http_request`
//! are pure Rust and can be tested on any platform. `led_matrix` and
//! `http_server` tie the pure logic to GPIO / network and are only compiled
//! when the `esp32` feature is enabled.

#![no_std]

pub mod bit_stream;
pub mod chain_mapper;
pub mod font;
pub mod frame_buffer;
pub mod http_request;

pub const MATRIX_WIDTH: usize = 88;
pub const MATRIX_HEIGHT: usize = 88;
