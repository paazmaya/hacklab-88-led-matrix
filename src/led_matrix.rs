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
//! ## Data flow
//! Per frame:
//! 1. [`chain_mapper::compute_chain_data`] translates the linear 88x88
//!    bitmap into the 44 pixels-per-cycle ordering the chain hardware expects.
//! 2. [`bit_stream::chain_data_bits`] converts the chain data to MSB-first
//!    bit patterns for the shift register.
//! 3. [`LedMatrix::write_chain`] toggles GPIO pins in lockstep with DCLK.
//!
//! Configuration is sent once at init via [`bit_stream::config_bits`].
//!
//! The pure logic (frame buffer, chain mapping, bit stream generation) lives
//! in its own modules so it can be unit-tested on the host. This module owns
//! the GPIO pins and orchestrates the protocol.

use esp_hal::delay::Delay;
use esp_hal::gpio::Output;

use esp32_led_matrix::bit_stream::{self, ChainBit, PWM_BITS};
use esp32_led_matrix::chain_mapper::{self, CHAIN_LEN, SCANLINES};
use esp32_led_matrix::frame_buffer::FrameBuffer;

/// Configuration1 register value, per the wiki:
///   - scanline count = 11
///   - GCLK multiplier enabled
///   - 16-bit PWM (not 13-bit)
///   - current gain = 5 (room lighting)
/// The wiki recommends `0x0A45`. The Teensy reference design uses `0x0A4B`
/// (current gain 11). We use the wiki value.
const CONFIG_REGISTER_1: u16 = 0x0A45;

/// GCLK pulses per scanline (per the wiki: 256 regular pulses + 1 dead-time
/// pulse = 257 total).
const GCLK_PULSES_PER_SCANLINE: u32 = 256;

/// Dead time on the 257th GCLK pulse (the wiki says longer delays are
/// *required* there — MBI5252 datasheet parameters `tdth` and `tdtl` are
/// minimums in the low-microsecond range, so 5 µs on each phase gives a
/// 10 µs period, ~10× the normal pulse width).
const GCLK_DEAD_TIME_US: u32 = 5;

/// Commands sent via LE + DCLK pulses.
#[repr(u8)]
#[derive(Clone, Copy)]
enum Command {
    /// Swap display buffers (front <-> back). Must be issued at the
    /// scanline 10 -> 0 transition.
    Vsync = 2,
    /// Reset the display.
    Reset = 10,
    /// Pre-Active — enables writes to Configuration1.
    PreActive = 14,
}

/// LED Matrix Driver
pub struct LedMatrix {
    // GPIO pins — named individually because `Output<'static>` is not
    // trivially array-able. Helpers below hide the repetition.
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

    /// Pixel data + text rendering. Pure logic, no GPIO.
    buffer: FrameBuffer,

    /// Initialized flag — refresh() is a no-op until init() has run.
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
            buffer: FrameBuffer::new(),
            initialized: false,
        };

        matrix.init();
        matrix
    }

    /// Render `text` to the back buffer (cleared first).
    pub fn display_text(&mut self, text: &str) {
        self.buffer.display_text(text);
    }

    /// Initialize the display with configuration.
    fn init(&mut self) {
        self.set_all_pins_low();

        // Wait for power stabilization.
        Delay::new().delay_millis(100);

        self.send_config(CONFIG_REGISTER_1);

        // Reset after config so the new register values take effect.
        // (Some reference designs do it the other way around — the Hacklab
        // panel reportedly tolerates both.)
        self.send_command(Command::Reset);
        Delay::new().delay_millis(10);

        self.initialized = true;
    }

    /// Send a command to the display via LE + DCLK.
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
    /// 1. Send the Pre-Active command (N=14, LE high for 14 DCLKs)
    /// 2. Send the 16-bit Configuration1 value
    /// 3. Send the WriteConfig command (N=4)
    ///
    /// Steps 2 and 3 are combined by holding LE high during the last 4
    /// DCLKs of the 16-bit shift. The 16-bit value is broadcast 22 times
    /// (352 DCLKs total) so it lands in the shift register of every IC.
    fn send_config(&mut self, config: u16) {
        self.send_command(Command::PreActive);
        for bit in bit_stream::config_bits(config) {
            self.shift_one_bit(bit);
        }
        self.le.set_low();
    }

    /// Refresh the display — must be called continuously.
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

        // Phase 1: send image data for all scanlines. Scope the
        // immutable borrow of `self.buffer` so it ends before we start
        // toggling GPIO in `write_chain` (which needs `&mut self`).
        let mut data = [[0u16; 3]; CHAIN_LEN];
        for scanline in 0..SCANLINES {
            for led in 0..PWM_BITS {
                {
                    let pixels = self.buffer.as_pixels();
                    chain_mapper::compute_chain_data(scanline, led, pixels, &mut data);
                }
                self.write_chain(&data);
            }
        }

        // Phase 2: multiplex one frame.
        self.multiplex_frame();
    }

    /// Run one full multiplex cycle — 11 scanlines, each with 256 GCLK
    /// pulses plus a 257th dead-time pulse.
    fn multiplex_frame(&mut self) {
        let delay = Delay::new();
        for scanline in 0..SCANLINES {
            self.set_scanline(scanline);
            self.pulse_gclk_n(GCLK_PULSES_PER_SCANLINE);

            // VSYNC must be issued at the scanline-10 -> 0 wrap so the
            // display swaps buffers exactly at the frame boundary.
            if scanline == SCANLINES - 1 {
                self.send_command(Command::Vsync);
            }

            // 257th GCLK pulse: longer high/low phase than the regular
            // 256. MBI5252 datasheet's tdth/tdtl are microsecond minimums.
            self.gclk.set_high();
            delay.delay_micros(GCLK_DEAD_TIME_US);
            self.gclk.set_low();
            delay.delay_micros(GCLK_DEAD_TIME_US);
        }
    }

    /// Shift 22 × 16 = 352 DCLKs for one `(scanline, led)` cycle.
    ///
    /// Bits shift MSB-first. On the very last DCLK of the very last IC, LE
    /// is raised to issue the Data Latch command (N=1). LE is dropped
    /// immediately after the loops finish.
    fn write_chain(&mut self, data: &[[u16; 3]; CHAIN_LEN]) {
        // Drop LE before clocking — the latch at the end of the previous
        // cycle, if any, would have left it high.
        self.le.set_low();

        for bit in bit_stream::chain_data_bits(data) {
            self.shift_one_bit(bit);
        }

        self.le.set_low();
    }

    /// Apply one [`ChainBit`] (data lines + optional LE) and pulse DCLK.
    #[inline]
    fn shift_one_bit(&mut self, bit: ChainBit) {
        Self::set_data_pin(&mut self.dr1, bit.r1);
        Self::set_data_pin(&mut self.dg1, bit.g1);
        Self::set_data_pin(&mut self.db1, bit.b1);
        Self::set_data_pin(&mut self.dr2, bit.r2);
        Self::set_data_pin(&mut self.dg2, bit.g2);
        Self::set_data_pin(&mut self.db2, bit.b2);
        if bit.le_high {
            self.le.set_high();
        }
        self.pulse_dclk();
    }

    /// Set a single data pin high or low.
    #[inline]
    fn set_data_pin(pin: &mut Output<'static>, high: bool) {
        if high {
            pin.set_high();
        } else {
            pin.set_low();
        }
    }

    /// Generate a single DCLK pulse.
    #[inline(always)]
    fn pulse_dclk(&mut self) {
        self.dclk.set_high();
        // Minimal delay — ESP32 is fast enough.
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        self.dclk.set_low();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    /// Generate GCLK pulses.
    #[inline]
    fn pulse_gclk_n(&mut self, count: u32) {
        for _ in 0..count {
            self.gclk.set_high();
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            self.gclk.set_low();
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Set scanline address (0-10) on the four A pins.
    fn set_scanline(&mut self, scanline: usize) {
        let addr = scanline as u8;
        // Iterate over the 4-bit mask. Each iteration handles one pin.
        let mut remaining = addr;
        for pin in [&mut self.a0, &mut self.a1, &mut self.a2, &mut self.a3] {
            let high = remaining & 1 != 0;
            Self::set_data_pin(pin, high);
            remaining >>= 1;
        }
    }

    /// Drive all output pins low.
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
