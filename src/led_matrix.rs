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

use anyhow::{Context, Result};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_sys::{
    gpio_config_t, gpio_set_direction, gpio_set_level, GPIO_INTR_DISABLE, GPIO_MODE_OUTPUT,
    GPIO_PULLDOWN_DISABLE, GPIO_PULLUP_DISABLE,
};
use log::{debug, trace};
use std::time::{Duration, Instant};

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
    gclk: Gpio4<Output>,
    dclk: Gpio5<Output>,
    le: Gpio18<Output>,
    a0: Gpio19<Output>,
    a1: Gpio21<Output>,
    a2: Gpio22<Output>,
    a3: Gpio23<Output>,
    dr1: Gpio25<Output>,
    dg1: Gpio26<Output>,
    db1: Gpio27<Output>,
    dr2: Gpio32<Output>,
    dg2: Gpio33<Output>,
    db2: Gpio13<Output>, // Using GPIO13 (GPIO34-39 are INPUT ONLY!)

    // Frame buffer: [scanline][pixel][color]
    // Each pixel has R, G, B values (16-bit each)
    frame_buffer: [[[u16; 3]; MATRIX_WIDTH]; MATRIX_HEIGHT],

    // Current scanline being displayed
    current_scanline: usize,

    // Font for text rendering
    font: Font,

    // Initialized flag
    initialized: bool,
}

impl LedMatrix {
    /// Create a new LED matrix driver with the specified GPIO pins
    ///
    /// # Pin Assignment
    /// - GPIO4:  GCLK - Multiplex clock (~1MHz)
    /// - GPIO5:  DCLK - Data clock
    /// - GPIO18: LE   - Latch Enable
    /// - GPIO19: A0   - Address bit 0
    /// - GPIO21: A1   - Address bit 1
    /// - GPIO22: A2   - Address bit 2
    /// - GPIO23: A3   - Address bit 3
    /// - GPIO25: DR1  - Red data chain 1
    /// - GPIO26: DG1  - Green data chain 1
    /// - GPIO27: DB1  - Blue data chain 1
    /// - GPIO32: DR2  - Red data chain 2
    /// - GPIO33: DG2  - Green data chain 2
    /// - GPIO13: DB2  - Blue data chain 2 (NOTE: GPIO34-39 are INPUT ONLY!)
    pub fn new(
        gclk: Gpio4<Output>,
        dclk: Gpio5<Output>,
        le: Gpio18<Output>,
        a0: Gpio19<Output>,
        a1: Gpio21<Output>,
        a2: Gpio22<Output>,
        a3: Gpio23<Output>,
        dr1: Gpio25<Output>,
        dg1: Gpio26<Output>,
        db1: Gpio27<Output>,
        dr2: Gpio32<Output>,
        dg2: Gpio33<Output>,
        db2: Gpio13<Output>,
    ) -> Result<Self> {
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
            current_scanline: 0,
            font: Font::new(),
            initialized: false,
        };

        matrix.init()?;
        Ok(matrix)
    }

    /// Initialize the display with configuration
    fn init(&mut self) -> Result<()> {
        debug!("Initializing LED matrix hardware...");

        // Reset all pins to low
        self.set_all_pins_low();

        // Wait for power stabilization
        FreeRtos::delay_ms(100);

        // Send reset command
        self.send_command(Command::Reset)?;
        FreeRtos::delay_ms(10);

        // Send pre-active command
        self.send_command(Command::PreActive)?;
        FreeRtos::delay_ms(1);

        // Configure display (enable output, set PWM mode)
        // Configuration register bits:
        // Bit 0: Output enable
        // Bit 1: 16-bit PWM mode
        // Bits 2-4: Current gain (111 = max)
        let config_value: u16 = 0b0000_0000_0001_1111; // Enable output, max current
        self.send_config(config_value)?;

        self.initialized = true;
        debug!("LED matrix initialized successfully");
        Ok(())
    }

    /// Send a command to the display via LE + DCLK
    fn send_command(&mut self, cmd: Command) -> Result<()> {
        // Pull LE high
        self.le.set_high()?;

        // Pulse DCLK N times
        for _ in 0..cmd as u8 {
            self.pulse_dclk();
        }

        // Pull LE low
        self.le.set_low()?;

        Ok(())
    }

    /// Send configuration value to the display
    fn send_config(&mut self, config: u16) -> Result<()> {
        // First send WriteConfig command
        self.le.set_high()?;
        self.pulse_dclk();
        self.pulse_dclk();
        self.pulse_dclk();
        self.pulse_dclk(); // 4 pulses = WriteConfig command
        self.le.set_low()?;

        // Now shift out the 16-bit config value
        for bit in (0..16).rev() {
            // Set data line based on bit value (using DR1 as data line)
            if (config >> bit) & 1 == 1 {
                self.dr1.set_high()?;
            } else {
                self.dr1.set_low()?;
            }
            self.pulse_dclk();
        }

        // Latch the config
        self.send_command(Command::DataLatch)?;

        Ok(())
    }

    /// Generate a single DCLK pulse
    #[inline(always)]
    fn pulse_dclk(&mut self) {
        self.dclk.set_high().ok();
        // Minimum pulse width is ~10ns, ESP32 at 240MHz can do ~4ns per cycle
        // Add small delay for reliable timing
        unsafe {
            esp_idf_sys::esp_rom_delay_us(1);
        }
        self.dclk.set_low().ok();
        unsafe {
            esp_idf_sys::esp_rom_delay_us(1);
        }
    }

    /// Generate GCLK pulses (256 per scanline)
    #[inline(always)]
    fn pulse_gclk_n(&mut self, count: u32) {
        for _ in 0..count {
            self.gclk.set_high().ok();
            // ~500ns period for ~1MHz GCLK
            unsafe {
                esp_idf_sys::esp_rom_delay_us(0);
            } // Minimal delay
            self.gclk.set_low().ok();
            unsafe {
                esp_idf_sys::esp_rom_delay_us(0);
            }
        }
    }

    /// Set scanline address (0-10)
    fn set_scanline(&mut self, scanline: usize) {
        let addr = scanline as u8;
        if addr & 0x01 != 0 {
            self.a0.set_high().ok();
        } else {
            self.a0.set_low().ok();
        }
        if addr & 0x02 != 0 {
            self.a1.set_high().ok();
        } else {
            self.a1.set_low().ok();
        }
        if addr & 0x04 != 0 {
            self.a2.set_high().ok();
        } else {
            self.a2.set_low().ok();
        }
        if addr & 0x08 != 0 {
            self.a3.set_high().ok();
        } else {
            self.a3.set_low().ok();
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

    /// Fill a rectangular area with a color
    pub fn fill_rect(
        &mut self,
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
        r: u16,
        g: u16,
        b: u16,
    ) {
        for y in y1..=y2.min(MATRIX_HEIGHT - 1) {
            for x in x1..=x2.min(MATRIX_WIDTH - 1) {
                self.set_pixel(x, y, r, g, b);
            }
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
        let mut x = 4; // Small margin from left

        for ch in text.chars() {
            if x >= MATRIX_WIDTH - self.font.width() {
                break; // Text too long, truncate
            }
            self.draw_char(ch, x, start_y, 0xFFFF, 0xFFFF, 0xFFFF); // White text
            x += self.font.width() + 1; // Add spacing
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
    ///
    /// This function:
    /// 1. Sends image data for all 11 scanlines
    /// 2. Generates GCLK pulses for multiplexing
    /// 3. Triggers VSYNC to swap buffers
    pub fn refresh(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Send image data for all scanlines
        for scanline in 0..SCANLINES {
            // Send data for this scanline
            self.send_scanline_data(scanline)?;
        }

        // Now perform the multiplexed display cycle
        for scanline in 0..SCANLINES {
            // Set scanline address
            self.set_scanline(scanline);

            // Generate 256 GCLK pulses for this scanline
            self.pulse_gclk_n(256);

            // When transitioning from scanline 10 to 0, do VSYNC
            if scanline == SCANLINES - 1 {
                // Send VSYNC command
                self.send_command(Command::Vsync)?;
            }

            // Generate 257th GCLK pulse with dead time
            FreeRtos::delay_ms(1); // Dead time
            self.gclk.set_high()?;
            FreeRtos::delay_ms(1);
            self.gclk.set_low()?;
        }

        self.current_scanline = (self.current_scanline + 1) % SCANLINES;
        Ok(())
    }

    /// Send data for one scanline to all chains
    fn send_scanline_data(&mut self, scanline: usize) -> Result<()> {
        // Each scanline covers 8 rows of the display (88 / 11 = 8 pixels per chain)
        // Chain 1 (R1,G1,B1) handles rows 0-43
        // Chain 2 (R2,G2,B2) handles rows 44-87

        let row1 = scanline * 8; // First row for chain 1
        let row2 = scanline * 8 + 44; // First row for chain 2

        // Send data for all ICs in each chain (22 ICs per chain)
        // Each IC controls 16 LEDs
        for ic in 0..ICS_PER_CHAIN {
            // Send 16-bit data for each color
            for bit in (0..PWM_BITS).rev() {
                // Calculate pixel positions for this IC
                let pixel_base = ic * LEDS_PER_IC;

                // Prepare 6 data bits for both chains
                for led in 0..LEDS_PER_IC {
                    let col = pixel_base + led;
                    if col >= MATRIX_WIDTH {
                        continue;
                    }

                    // Chain 1 data
                    let r1 = if self.frame_buffer[row1][col][0] & (1 << bit) != 0 {
                        1
                    } else {
                        0
                    };
                    let g1 = if self.frame_buffer[row1][col][1] & (1 << bit) != 0 {
                        1
                    } else {
                        0
                    };
                    let b1 = if self.frame_buffer[row1][col][2] & (1 << bit) != 0 {
                        1
                    } else {
                        0
                    };

                    // Chain 2 data
                    let r2 = if row2 < MATRIX_HEIGHT
                        && self.frame_buffer[row2][col][0] & (1 << bit) != 0
                    {
                        1
                    } else {
                        0
                    };
                    let g2 = if row2 < MATRIX_HEIGHT
                        && self.frame_buffer[row2][col][1] & (1 << bit) != 0
                    {
                        1
                    } else {
                        0
                    };
                    let b2 = if row2 < MATRIX_HEIGHT
                        && self.frame_buffer[row2][col][2] & (1 << bit) != 0
                    {
                        1
                    } else {
                        0
                    };

                    // Set data lines
                    if r1 != 0 {
                        self.dr1.set_high()?;
                    } else {
                        self.dr1.set_low()?;
                    }
                    if g1 != 0 {
                        self.dg1.set_high()?;
                    } else {
                        self.dg1.set_low()?;
                    }
                    if b1 != 0 {
                        self.db1.set_high()?;
                    } else {
                        self.db1.set_low()?;
                    }
                    if r2 != 0 {
                        self.dr2.set_high()?;
                    } else {
                        self.dr2.set_low()?;
                    }
                    if g2 != 0 {
                        self.dg2.set_high()?;
                    } else {
                        self.dg2.set_low()?;
                    }
                    if b2 != 0 {
                        self.db2.set_high()?;
                    } else {
                        self.db2.set_low()?;
                    }

                    // On the very last bit of the very last IC, set LE for latch
                    if ic == ICS_PER_CHAIN - 1 && led == LEDS_PER_IC - 1 && bit == 0 {
                        self.le.set_high()?;
                    }

                    // Pulse DCLK
                    self.pulse_dclk();

                    // Clear LE if it was set
                    self.le.set_low()?;
                }
            }
        }

        Ok(())
    }

    /// Set all output pins to low
    fn set_all_pins_low(&mut self) {
        self.gclk.set_low().ok();
        self.dclk.set_low().ok();
        self.le.set_low().ok();
        self.a0.set_low().ok();
        self.a1.set_low().ok();
        self.a2.set_low().ok();
        self.a3.set_low().ok();
        self.dr1.set_low().ok();
        self.dg1.set_low().ok();
        self.db1.set_low().ok();
        self.dr2.set_low().ok();
        self.dg2.set_low().ok();
        self.db2.set_low().ok();
    }
}

/// Drop implementation - ensure display is cleared
impl Drop for LedMatrix {
    fn drop(&mut self) {
        self.clear();
        self.set_all_pins_low();
    }
}
