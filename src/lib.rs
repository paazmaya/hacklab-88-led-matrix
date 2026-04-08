//! LED Matrix Controller Library
//!
//! This library exports testable components of the LED matrix controller.
//! The font module is pure Rust and can be tested on any platform.

#![no_std]

pub mod font;

pub const MATRIX_WIDTH: usize = 88;
pub const MATRIX_HEIGHT: usize = 88;
