//! LED Matrix Driver for 88x88 RGB Display
//!
//! This driver implements the control protocol for the LED matrix display
//! based on the Helsinki Hacklab documentation.
//!
//! ## Control Signals
//! - GCLK: Multiplex clock (~1 MHz, 256 pulses per scanline)
//! - DCLK: Data clock for shift register
//! - LE: Latch Enable (combined with DCLK for commands)
//! - A0-A3: Scanline address (0-10)
//! - DR1,DG1,DB1: RGB data chain 1
//! - DR2,DG2,DB2: RGB data chain 2
//!
//! ## Display Architecture
//! - 88x88 pixels, RGB
//! - 6 parallel shift register chains (R1,G1,B1,R2,G2,B2)
//! - 11:1 multiplexing (11 scanlines)
//! - 16-bit PWM per color channel
//! - Double buffering with VSYNC

use esp_hal::delay::Delay;
use esp_hal::gpio::Output;

use crate::font::Font;
use crate::{MATRIX_HEIGHT, MATRIX_WIDTH};

/// Number of scanlines (multiplexing factor)
const SCANLINES: usize = 11;

/// Number of ICs per chain
const ICS_PER_CHAIN: usize = 22;

/// LEDs per IC
const LEDS_PER_IC: usize = 16;

/// PWM bit depth
const PWM_BITS: usize = 16;

/// Commands sent via LE + DCLK pulses
#[repr(u8)]
enum Command {
    DataLatch = 1,   // Strobe shift register data
    Vsync = 2,       // Swap display buffers
    WriteConfig = 4, // Write configuration register
    Reset = 10,      // Reset display
    PreActive = 14,  // Enable configuration write
}

/// LED Matrix Driver
pub struct LedMatrix {
    // GPIO pins
    gclk: Output<'static>,
    dclk: Output<'static>,
    le: Output<'static>,
    a0: Output<'static>,
    a1: Output<'static>,
    a2: Output<'static>,
    a3: Output<'static>,
    dr1: Output<'static>,
    dg1: Output<'static>,
    db1: Output<'static>,
    dr2: Output<'static>,
    dg2: Output<'static>,
    db2: Output<'static>,

    // Frame buffer: [row][col][color]
    frame_buffer: [[[u16; 3]; MATRIX_WIDTH]; MATRIX_HEIGHT],

    // Font for text rendering
    font: Font,

    // Initialized flag
    initialized: bool,
}

impl LedMatrix {
    /// Create a new LED matrix driver with the specified GPIO pins
    ///
    /// # Pin Assignment (ESP32-S2 Compatible)
    /// - GPIO4:  GCLK - Multiplex clock (~1MHz)
    /// - GPIO5:  DCLK - Data clock
    /// - GPIO6:  LE   - Latch Enable
    /// - GPIO7:  A0   - Address bit 0
    /// - GPIO8:  A1   - Address bit 1
    /// - GPIO9:  A2   - Address bit 2
    /// - GPIO10: A3   - Address bit 3
    /// - GPIO11: DR1  - Red data chain 1
    /// - GPIO12: DG1  - Green data chain 1
    /// - GPIO13: DB1  - Blue data chain 1
    /// - GPIO14: DR2  - Red data chain 2
    /// - GPIO15: DG2  - Green data chain 2
    /// - GPIO16: DB2  - Blue data chain 2
    pub fn new(
        gclk: Output<'static>,
        dclk: Output<'static>,
        le: Output<'static>,
        a0: Output<'static>,
        a1: Output<'static>,
        a2: Output<'static>,
        a3: Output<'static>,
        dr1: Output<'static>,
        dg1: Output<'static>,
        db1: Output<'static>,
        dr2: Output<'static>,
        dg2: Output<'static>,
        db2: Output<'static>,
    ) -> Self {
        let mut matrix = Self {
            gclk,
            dclk,
            le,
            a0,
            a1,
            a2,
            a3,
            dr1,
            dg1,
            db1,
            dr2,
            dg2,
            db2,
            frame_buffer: [[[0u16; 3]; MATRIX_WIDTH]; MATRIX_HEIGHT],
            font: Font::new(),
            initialized: false,
        };

        matrix.init();
        matrix
    }

    /// Initialize the display with configuration
    fn init(&mut self) {
        // Reset all pins to low
        self.set_all_pins_low();

        // Wait for power stabilization
        let delay = Delay::new();
        delay.delay_millis(100);

        // Send reset command
        self.send_command(Command::Reset);
        delay.delay_millis(10);

        // Send pre-active command
        self.send_command(Command::PreActive);
        delay.delay_millis(1);

        // Configure display (enable output, set PWM mode)
        let config_value: u16 = 0b0000_0000_0001_1111;
        self.send_config(config_value);

        self.initialized = true;
    }

    /// Send a command to the display via LE + DCLK
    fn send_command(&mut self, cmd: Command) {
        self.le.set_high();

        for _ in 0..cmd as u8 {
            self.pulse_dclk();
        }

        self.le.set_low();
    }

    /// Send configuration value to the display
    fn send_config(&mut self, config: u16) {
        // Send WriteConfig command
        self.le.set_high();
        for _ in 0..4 {
            self.pulse_dclk();
        }
        self.le.set_low();

        // Shift out the 16-bit config value
        for bit in (0..16).rev() {
            if (config >> bit) & 1 == 1 {
                self.dr1.set_high();
            } else {
                self.dr1.set_low();
            }
            self.pulse_dclk();
        }

        // Latch the config
        self.send_command(Command::DataLatch);
    }

    /// Generate a single DCLK pulse
    #[inline(always)]
    fn pulse_dclk(&mut self) {
        self.dclk.set_high();
        // Minimal delay - ESP32 is fast enough
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        self.dclk.set_low();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    /// Generate GCLK pulses (256 per scanline)
    #[inline(always)]
    fn pulse_gclk_n(&mut self, count: u32) {
        for _ in 0..count {
            self.gclk.set_high();
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            self.gclk.set_low();
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Set scanline address (0-10)
    fn set_scanline(&mut self, scanline: usize) {
        let addr = scanline as u8;
        if addr & 0x01 != 0 {
            self.a0.set_high();
        } else {
            self.a0.set_low();
        }
        if addr & 0x02 != 0 {
            self.a1.set_high();
        } else {
            self.a1.set_low();
        }
        if addr & 0x04 != 0 {
            self.a2.set_high();
        } else {
            self.a2.set_low();
        }
        if addr & 0x08 != 0 {
            self.a3.set_high();
        } else {
            self.a3.set_low();
        }
    }

    /// Clear the frame buffer (all LEDs off)
    pub fn clear(&mut self) {
        for row in 0..MATRIX_HEIGHT {
            for col in 0..MATRIX_WIDTH {
                self.frame_buffer[row][col] = [0, 0, 0];
            }
        }
    }

    /// Set a pixel color (RGB, 16-bit per channel)
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u16, g: u16, b: u16) {
        if x < MATRIX_WIDTH && y < MATRIX_HEIGHT {
            self.frame_buffer[y][x] = [r, g, b];
        }
    }

    /// Display text on the matrix
    pub fn display_text(&mut self, text: &str) {
        self.clear();
        if text.is_empty() {
            return;
        }

        // Render text starting from left edge, centered vertically
        let start_y = (MATRIX_HEIGHT - self.font.height()) / 2;
        let mut x = 4;

        for ch in text.chars() {
            if x >= MATRIX_WIDTH - self.font.width() {
                break;
            }
            self.draw_char(ch, x, start_y, 0xFFFF, 0xFFFF, 0xFFFF);
            x += self.font.width() + 1;
        }
    }

    /// Draw a single character at the specified position
    fn draw_char(&mut self, ch: char, x: usize, y: usize, r: u16, g: u16, b: u16) {
        if let Some(glyph) = self.font.get_glyph(ch) {
            for (gy, row) in glyph.iter().enumerate() {
                for (gx, &pixel) in row.iter().enumerate() {
                    if pixel != 0 {
                        self.set_pixel(x + gx, y + gy, r, g, b);
                    }
                }
            }
        }
    }

    /// Refresh the display - this must be called continuously
    pub fn refresh(&mut self) {
        if !self.initialized {
            return;
        }

        // Send image data for all scanlines
        for scanline in 0..SCANLINES {
            self.send_scanline_data(scanline);
        }

        // Perform the multiplexed display cycle
        for scanline in 0..SCANLINES {
            self.set_scanline(scanline);
            self.pulse_gclk_n(256);

            // VSYNC when transitioning from scanline 10 to 0
            if scanline == SCANLINES - 1 {
                self.send_command(Command::Vsync);
            }

            // 257th GCLK pulse with dead time
            self.gclk.set_high();
            // Dead time
            self.gclk.set_low();
        }
    }

    /// Send data for one scanline to all chains
    fn send_scanline_data(&mut self, scanline: usize) {
        let row1 = scanline * 8;
        let row2 = scanline * 8 + 44;

        for ic in 0..ICS_PER_CHAIN {
            for bit in (0..PWM_BITS).rev() {
                let pixel_base = ic * LEDS_PER_IC;

                for led in 0..LEDS_PER_IC {
                    let col = pixel_base + led;
                    if col >= MATRIX_WIDTH {
                        continue;
                    }

                    // Chain 1 data
                    let r1 = self.frame_buffer[row1][col][0] & (1 << bit) != 0;
                    let g1 = self.frame_buffer[row1][col][1] & (1 << bit) != 0;
                    let b1 = self.frame_buffer[row1][col][2] & (1 << bit) != 0;

                    // Chain 2 data
                    let r2 =
                        row2 < MATRIX_HEIGHT && self.frame_buffer[row2][col][0] & (1 << bit) != 0;
                    let g2 =
                        row2 < MATRIX_HEIGHT && self.frame_buffer[row2][col][1] & (1 << bit) != 0;
                    let b2 =
                        row2 < MATRIX_HEIGHT && self.frame_buffer[row2][col][2] & (1 << bit) != 0;

                    // Set data lines
                    if r1 {
                        self.dr1.set_high();
                    } else {
                        self.dr1.set_low();
                    }
                    if g1 {
                        self.dg1.set_high();
                    } else {
                        self.dg1.set_low();
                    }
                    if b1 {
                        self.db1.set_high();
                    } else {
                        self.db1.set_low();
                    }
                    if r2 {
                        self.dr2.set_high();
                    } else {
                        self.dr2.set_low();
                    }
                    if g2 {
                        self.dg2.set_high();
                    } else {
                        self.dg2.set_low();
                    }
                    if b2 {
                        self.db2.set_high();
                    } else {
                        self.db2.set_low();
                    }

                    // On the last bit of the last IC, set LE for latch
                    if ic == ICS_PER_CHAIN - 1 && led == LEDS_PER_IC - 1 && bit == 0 {
                        self.le.set_high();
                    }

                    self.pulse_dclk();
                    self.le.set_low();
                }
            }
        }
    }

    /// Set all output pins to low
    fn set_all_pins_low(&mut self) {
        self.gclk.set_low();
        self.dclk.set_low();
        self.le.set_low();
        self.a0.set_low();
        self.a1.set_low();
        self.a2.set_low();
        self.a3.set_low();
        self.dr1.set_low();
        self.dg1.set_low();
        self.db1.set_low();
        self.dr2.set_low();
        self.dg2.set_low();
        self.db2.set_low();
    }
}
