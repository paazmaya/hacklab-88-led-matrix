//! LED Matrix Driver for 88x88 RGB Display
//!
//! This driver implements the control protocol for the LED matrix display
//! based on the [Helsinki Hacklab documentation][wiki].
//!
//! [wiki]: https://wiki.helsinki.hacklab.fi/Ledimatriisin_ohjaaminen
//!
//! ## Control Signals
//! - GCLK: Multiplex clock (~1 MHz, 256 pulses per scanline, plus a 257th
//!   pulse with longer high/low "dead time" before the next scanline)
//! - DCLK: Data clock for shift register
//! - LE: Latch Enable (combined with DCLK for commands — see "Commands" below)
//! - A0-A3: Scanline address (0-10)
//! - DR1,DG1,DB1: RGB data chain 1
//! - DR2,DG2,DB2: RGB data chain 2
//!
//! ## Display Architecture
//! - 88x88 pixels, RGB
//! - 6 parallel shift register chains (R1,G1,B1,R2,G2,B2) each holding 22
//!   driver ICs in series
//! - 11:1 multiplexing (11 scanlines, 8 physical rows per scanline)
//! - 16-bit PWM per color channel
//! - Double buffering with VSYNC
//!
//! ## Commands (sent via LE + N DCLK pulses)
//! Raising LE and pulsing DCLK N times issues a command. The number of
//! pulses is the command code:
//!
//! | N  | Command                                |
//! |----|----------------------------------------|
//! | 1  | Data Latch (strobe shift register)     |
//! | 2  | VSYNC (swap display buffers)           |
//! | 4  | Write Configuration1 register          |
//! | 10 | Reset                                  |
//! | 14 | Pre-Active (enable configuration write)|
//!
//! When LE is low, DCLK shifts the data lines into the shift register chain.
//!
//! ## Data flow (per the wiki's reference implementation)
//! For each (scanline, led-in-ic) pair:
//! 1. [`get_chain_data`] translates the linear 88x88 bitmap into the 44
//!    pixels-per-cycle ordering the chain hardware expects.
//! 2. [`write_chain`] shifts 22 × 16 = 352 DCLKs, with MSB first, broadcasting
//!    R1/G1/B1/R2/G2/B2 across all six chains in parallel. On the final
//!    DCLK of the last IC, LE is raised to issue a Data Latch command, then
//!    dropped.
//!
//! Configuration is sent once at init: Pre-Active → 22 × 16 config bits with
//! the last 4 DCLKs having LE high (combined shift + WriteConfig command)
//! → Reset → first frame.

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

/// GCLK pulses per scanline (per the wiki: 256 regular pulses + 1 dead-time
/// pulse = 257 total).
const GCLK_PULSES_PER_SCANLINE: u32 = 256;

/// Dead time on the 257th GCLK pulse (the wiki says longer delays are
/// *required* there — MBI5252 datasheet parameters `tdth` and `tdtl` are
/// minimums in the low-microsecond range, so 5 µs on each phase gives a
/// 10 µs period, ~10× the normal pulse width).
const GCLK_DEAD_TIME_US: u32 = 5;

/// Commands sent via LE + DCLK pulses
#[repr(u8)]
enum Command {
    /// Strobe shift register data (raised LE during the last DCLK of the
    /// chain shift — see [`LedMatrix::write_chain`]). Kept as an enum variant
    /// for documentation; the actual strobe pulse is inlined in `write_chain`
    /// so the LE high-time matches the last DCLK exactly without an extra
    /// pulse cycle.
    #[allow(dead_code)]
    DataLatch = 1,
    /// Swap display buffers (front <-> back). Must be issued at the
    /// scanline 10 -> 0 transition.
    Vsync = 2,
    /// Write Configuration1 register. Sent as part of [`LedMatrix::send_config`]
    /// by holding LE high during the last 4 DCLKs of the config shift.
    WriteConfig = 4,
    /// Reset the display.
    Reset = 10,
    /// Pre-Active — enables writes to Configuration1.
    PreActive = 14,
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
    /// Create a new LED matrix driver with the specified GPIO pins.
    ///
    /// # Pin Order (ESP32-C3 SuperMini)
    /// The constructor takes 13 [`Output`] pins in this fixed order, which
    /// matches the wiring diagram in `README.md`:
    ///
    /// | # | Argument | LED-matrix signal | ESP32-C3 GPIO | Notes              |
    /// |---|----------|-------------------|---------------|--------------------|
    /// | 1 | `gclk`   | GCLK              | GPIO0         | multiplex clock    |
    /// | 2 | `dclk`   | DCLK              | GPIO1         | data clock         |
    /// | 3 | `le`     | LE                | GPIO2         | latch enable       |
    /// | 4 | `a0`     | A0                | GPIO3         | address bit 0      |
    /// | 5 | `a1`     | A1                | GPIO4         | address bit 1      |
    /// | 6 | `a2`     | A2                | GPIO5         | address bit 2      |
    /// | 7 | `a3`     | A3                | GPIO6         | address bit 3      |
    /// | 8 | `dr1`    | DR1               | GPIO7         | red   data chain 1 |
    /// | 9 | `dg1`    | DG1               | GPIO8         | green data chain 1 (boot) |
    /// |10 | `db1`    | DB1               | GPIO9         | blue  data chain 1 (boot) |
    /// |11 | `dr2`    | DR2               | GPIO10        | red   data chain 2 |
    /// |12 | `dg2`    | DG2               | GPIO20        | green data chain 2 (UART RXD) |
    /// |13 | `db2`    | DB2               | GPIO21        | blue  data chain 2 (UART TXD) |
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

        // Configuration1 register value, per the wiki:
        //   - scanline count = 11
        //   - GCLK multiplier enabled
        //   - 16-bit PWM (not 13-bit)
        //   - current gain = 5 (room lighting)
        // The wiki recommends `0x0A45`. The Teensy reference design uses
        // `0x0A4B` (current gain 11). We use the wiki value.
        const CONFIG_REGISTER_1: u16 = 0x0A45;
        self.send_config(CONFIG_REGISTER_1);

        // Reset the display after configuration. The wiki says config must
        // be re-sent after Reset to take effect, so reset comes *after*
        // send_config here. (Some reference designs do it the other way
        // around — the Hacklab panel reportedly tolerates both.)
        self.send_command(Command::Reset);
        delay.delay_millis(10);

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

    /// Send the Configuration1 register to all driver ICs.
    ///
    /// Per the wiki (`Ledimatriisin ohjaaminen > Ohjainpiirin konfigurointi`):
    ///
    /// 1. Send the Pre-Active command (N=14, LE high for 14 DCLKs)
    /// 2. Send the 16-bit Configuration1 value
    /// 3. Send the WriteConfig command (N=4)
    ///
    /// Steps 2 and 3 can be combined by holding LE high during the last 4
    /// DCLKs of the 16-bit shift. The wiki also notes that the register
    /// "pitää lähettää jokaiseen ketjuun ja jokaiselle 22 piirille erikseen"
    /// — must reach each of the 22 ICs in every chain. We satisfy this by
    /// broadcasting the same 16-bit value 22 times (352 DCLKs total) so the
    /// value lands in the shift register of every IC, with the WriteConfig
    /// command issued on the final 4 DCLKs.
    fn send_config(&mut self, config: u16) {
        // Step 1: Pre-Active enables writes to Configuration1.
        self.send_command(Command::PreActive);

        // Step 2+3: 352 DCLKs, MSB first, with the same bit on all 6 data
        // lines. The last 4 DCLKs have LE high, which is the WriteConfig
        // command combined with the trailing bits of the last config word.
        let total_dclks: usize = PWM_BITS * ICS_PER_CHAIN; // 352
        let cmd_pulses: usize = Command::WriteConfig as usize; // 4
        let le_threshold = total_dclks - cmd_pulses;

        for i in 0..total_dclks {
            // Raise LE for the last `cmd_pulses` DCLKs — this delivers the
            // WriteConfig command while still clocking the trailing bits.
            if i >= le_threshold {
                self.le.set_high();
            }

            // Bit index cycles 15..0 for each of the 22 config words.
            let bit_idx = PWM_BITS - 1 - (i % PWM_BITS);
            let bit_set = (config >> bit_idx) & 1 != 0;

            // Broadcast to all 6 data lines (R1, G1, B1, R2, G2, B2).
            if bit_set {
                self.dr1.set_high();
                self.dg1.set_high();
                self.db1.set_high();
                self.dr2.set_high();
                self.dg2.set_high();
                self.db2.set_high();
            } else {
                self.dr1.set_low();
                self.dg1.set_low();
                self.db1.set_low();
                self.dr2.set_low();
                self.dg2.set_low();
                self.db2.set_low();
            }

            self.pulse_dclk();
        }

        // Drop LE after the WriteConfig command.
        self.le.set_low();
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
    ///
    /// Two phases, per the wiki:
    /// 1. Shift one full frame of image data into the display's back buffer
    ///    (the display keeps showing the previous frame while we do this).
    /// 2. Run a complete multiplex cycle: 256 GCLK pulses per scanline,
    ///    advancing the address every scanline. At the scanline-10 -> 0
    ///    wrap-around, issue VSYNC so the display swaps to the back buffer
    ///    we just filled.
    pub fn refresh(&mut self) {
        if !self.initialized {
            return;
        }

        // Phase 1: send image data for all scanlines.
        for scanline in 0..SCANLINES {
            self.send_scanline_data(scanline);
        }

        // Phase 2: multiplex one frame. The display shows the new image after
        // the VSYNC at the end of this loop.
        let delay = Delay::new();
        for scanline in 0..SCANLINES {
            self.set_scanline(scanline);
            self.pulse_gclk_n(GCLK_PULSES_PER_SCANLINE);

            // VSYNC must be issued just before the scanline 10 -> 0 wrap,
            // so the display swaps buffers exactly at the frame boundary.
            if scanline == SCANLINES - 1 {
                self.send_command(Command::Vsync);
            }

            // 257th GCLK pulse: longer high/low phase than the regular 256
            // pulses. The MBI5252 datasheet's `tdth`/`tdtl` are minimums in
            // the low-microsecond range, so we hold each phase for
            // `GCLK_DEAD_TIME_US`.
            self.gclk.set_high();
            delay.delay_micros(GCLK_DEAD_TIME_US);
            self.gclk.set_low();
            delay.delay_micros(GCLK_DEAD_TIME_US);
        }
    }

    /// Send the data for one scanline into the back buffer.
    ///
    /// Per the wiki's reference pseudocode, the outer loop is `scanline` then
    /// `led` (which LED output channel of each IC we're programming), and
    /// for each `(scanline, led)` pair we shift 22 × 16 = 352 DCLKs. The
    /// complex pixel-to-bit mapping (44 pixels per cycle, 22 per chain) is
    /// handled by [`get_chain_data`], and the actual GPIO toggling is in
    /// [`write_chain`].
    fn send_scanline_data(&mut self, scanline: usize) {
        let mut data: [[u16; 3]; 44] = [[0u16; 3]; 44];
        for led in 0..LEDS_PER_IC {
            self.get_chain_data(scanline, led, &mut data);
            self.write_chain(&data);
        }
    }

    /// Translate the linear 88x88 frame buffer into the 44-pixel ordering
    /// the chain hardware expects for one `(scanline, led)` cycle.
    ///
    /// Direct port of the `getChainData` function from the wiki's reference
    /// implementation (originally in the Teensy code by Depili). The mapping
    /// is non-obvious — the wiki's own comment notes that the PCB layout
    /// inside the panel chose an arbitrary ordering and "syntyneet
    /// epäloogisuudet on jätetty softamiesten päänsäryksi" (the resulting
    /// illogicalities have been left for the software people to deal with).
    ///
    /// Per call: fills `data[0..21]` with the 22 chain-1 pixels and
    /// `data[22..43]` with the 22 chain-2 pixels for this `(scanline, led)`.
    /// The chain data layout is:
    ///
    /// - `data[0..10]` and `data[11..21]` are chain 1's two row groups
    /// - `data[22..32]` and `data[33..43]` are chain 2's two row groups
    fn get_chain_data(&self, scanline: usize, led: usize, data: &mut [[u16; 3]; 44]) {
        // led 0..7 picks one row-group of the scanline; led 8..15 picks the
        // other. ledColumn is the per-group column offset, reversed for the
        // first group to match the physical wiring.
        let led_row: usize = if led < 8 { 11 } else { 0 };
        let mut led_column: usize = led % 8;
        let mut row: usize = scanline + led_row;
        if led_row == 11 {
            led_column = 7 - led_column;
        }

        // Four row blocks, each contributing 11 columns of one row. The
        // start index descends (33, 22, 11, 0) because the loop writes the
        // highest row block first.
        const STARTS: [usize; 4] = [33, 22, 11, 0];
        for &start in &STARTS {
            for i in 0..11usize {
                let col = 8 * i + led_column;
                data[start + i] = self.frame_buffer[row][col];
            }
            row += 22;
        }
    }

    /// Shift 22 × 16 = 352 DCLKs for one `(scanline, led)` cycle.
    ///
    /// Bits shift MSB-first. On the very last DCLK of the very last IC, LE
    /// is raised to issue the Data Latch command (N=1: 1 DCLK with LE
    /// high). LE is dropped immediately after the loops finish.
    fn write_chain(&mut self, data: &[[u16; 3]; 44]) {
        // Make sure LE is low before we start clocking data in — the latch
        // at the end of the *previous* cycle, if any, would have left it
        // high.
        self.le.set_low();

        for ic in 0..ICS_PER_CHAIN {
            let p1 = &data[ic]; // chain 1 pixel for this IC slot
            let p2 = &data[ic + ICS_PER_CHAIN]; // chain 2 pixel for this IC slot

            for bit_idx in (0..PWM_BITS).rev() {
                // Last bit of the last IC: raise LE so this DCLK becomes the
                // Data Latch strobe (N=1).
                if ic == ICS_PER_CHAIN - 1 && bit_idx == 0 {
                    self.le.set_high();
                }

                // Chain 1: R = p1[0], G = p1[1], B = p1[2]
                if (p1[0] >> bit_idx) & 1 != 0 {
                    self.dr1.set_high();
                } else {
                    self.dr1.set_low();
                }
                if (p1[1] >> bit_idx) & 1 != 0 {
                    self.dg1.set_high();
                } else {
                    self.dg1.set_low();
                }
                if (p1[2] >> bit_idx) & 1 != 0 {
                    self.db1.set_high();
                } else {
                    self.db1.set_low();
                }

                // Chain 2: R = p2[0], G = p2[1], B = p2[2]
                if (p2[0] >> bit_idx) & 1 != 0 {
                    self.dr2.set_high();
                } else {
                    self.dr2.set_low();
                }
                if (p2[1] >> bit_idx) & 1 != 0 {
                    self.dg2.set_high();
                } else {
                    self.dg2.set_low();
                }
                if (p2[2] >> bit_idx) & 1 != 0 {
                    self.db2.set_high();
                } else {
                    self.db2.set_low();
                }

                self.pulse_dclk();
            }
        }

        // Drop LE after the Data Latch strobe.
        self.le.set_low();
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
