//! Bit-stream generation for the LED matrix shift register protocol.
//!
//! The driver talks to the panel by toggling 6 RGB data lines and a
//! latch-enable (LE) line in sync with a DCLK clock. The exact bit
//! sequence depends on whether we're streaming pixel data or the
//! configuration register, so this module produces two helpers:
//!
//! - [`chain_data_bits`] — 22 ICs × 16 bits of RGB pixel data, with LE
//!   raised for the very last DCLK (data latch strobe).
//! - [`config_bits`] — 22 ICs × 16 bits of the same register value
//!   broadcast to all six chains, with LE raised for the last 4 DCLKs
//!   (write-config command).
//!
//! Extracting this from the GPIO-talking code means the bit patterns can
//! be unit-tested on the host without any ESP32 hardware.

use crate::chain_mapper::{CHAIN_LEN, ICS_PER_CHAIN};

/// PWM bit depth per color channel.
pub const PWM_BITS: usize = 16;

/// Number of DCLKs in a configuration write (22 ICs × 16 bits).
pub const CONFIG_TOTAL_DCLKS: usize = PWM_BITS * ICS_PER_CHAIN;

/// Pulse count of the WriteConfig command (LE held high for the last
/// `CONFIG_WRITE_PULSES` DCLKs of the config shift).
pub const CONFIG_WRITE_PULSES: usize = 4;

/// One DCLK cycle worth of pin states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainBit {
    pub r1: bool,
    pub g1: bool,
    pub b1: bool,
    pub r2: bool,
    pub g2: bool,
    pub b2: bool,
    pub le_high: bool,
}

impl ChainBit {
    /// Construct a bit where all 6 data lines carry the same value and
    /// LE is in the requested state. Used for config-register writes.
    pub const fn broadcast(value: bool, le_high: bool) -> Self {
        Self {
            r1: value,
            g1: value,
            b1: value,
            r2: value,
            g2: value,
            b2: value,
            le_high,
        }
    }
}

/// Yield the 22 × 16 = 352 DCLKs of bit-pattern for sending one
/// `(scanline, led)` cycle. Bits shift MSB-first. The very last DCLK
/// raises LE so it becomes the data-latch strobe.
pub fn chain_data_bits(data: &[[u16; 3]; CHAIN_LEN]) -> [ChainBit; ICS_PER_CHAIN * PWM_BITS] {
    let mut out = [ChainBit::broadcast(false, false); ICS_PER_CHAIN * PWM_BITS];
    for ic in 0..ICS_PER_CHAIN {
        let p1 = data[ic];
        let p2 = data[ic + ICS_PER_CHAIN];
        for (bit_pos, bit_idx) in (0..PWM_BITS).rev().enumerate() {
            let le_high = ic == ICS_PER_CHAIN - 1 && bit_idx == 0;
            out[ic * PWM_BITS + bit_pos] = ChainBit {
                r1: bit(p1[0], bit_idx),
                g1: bit(p1[1], bit_idx),
                b1: bit(p1[2], bit_idx),
                r2: bit(p2[0], bit_idx),
                g2: bit(p2[1], bit_idx),
                b2: bit(p2[2], bit_idx),
                le_high,
            };
        }
    }
    out
}

/// Yield the 22 × 16 = 352 DCLKs of bit-pattern for sending the
/// configuration register. The same bit is broadcast to all six chains.
/// The last 4 DCLKs have LE high (WriteConfig command).
pub fn config_bits(config: u16) -> [ChainBit; CONFIG_TOTAL_DCLKS] {
    let mut out = [ChainBit::broadcast(false, false); CONFIG_TOTAL_DCLKS];
    let le_threshold = CONFIG_TOTAL_DCLKS - CONFIG_WRITE_PULSES;
    for (i, slot) in out.iter_mut().enumerate() {
        let bit_idx = PWM_BITS - 1 - (i % PWM_BITS);
        let bit_set = (config >> bit_idx) & 1 != 0;
        *slot = ChainBit::broadcast(bit_set, i >= le_threshold);
    }
    out
}

#[inline]
const fn bit(value: u16, idx: usize) -> bool {
    (value >> idx) & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_mapper::LEDS_PER_IC;

    const CHAIN_DATA_LEN: usize = ICS_PER_CHAIN * PWM_BITS;

    fn uniform_data(value: u16) -> [[u16; 3]; CHAIN_LEN] {
        [[value, value, value]; CHAIN_LEN]
    }

    #[test]
    fn chain_data_has_expected_length() {
        let data = uniform_data(0);
        let bits = chain_data_bits(&data);
        assert_eq!(bits.len(), CHAIN_DATA_LEN);
    }

    #[test]
    fn chain_data_raises_le_only_on_last_bit() {
        let data = uniform_data(0);
        let bits = chain_data_bits(&data);
        for b in &bits[..bits.len() - 1] {
            assert!(!b.le_high, "LE should only be high on the final DCLK");
        }
        assert!(bits.last().unwrap().le_high, "last bit must have LE high");
    }

    #[test]
    fn chain_data_is_msb_first() {
        let mut data = [[0u16; 3]; CHAIN_LEN];
        data[0] = [0x8001, 0, 0];
        data[ICS_PER_CHAIN] = [0, 0, 0];
        let bits = chain_data_bits(&data);
        assert!(bits[0].r1);
        assert!(bits[PWM_BITS - 1].r1);
        assert!(!bits[8].r1);
    }

    #[test]
    fn config_bits_broadcast_same_value() {
        let bits = config_bits(0xFFFF);
        for b in bits.iter() {
            assert!(b.r1 && b.g1 && b.b1 && b.r2 && b.g2 && b.b2);
        }
    }

    #[test]
    fn config_bits_le_high_on_last_four_only() {
        let bits = config_bits(0x0000);
        let le_threshold = CONFIG_TOTAL_DCLKS - CONFIG_WRITE_PULSES;
        for (i, b) in bits.iter().enumerate() {
            assert_eq!(b.le_high, i >= le_threshold, "LE state wrong at DCLK {}", i);
        }
    }

    #[test]
    fn leds_per_ic_constant_matches_chain_data_block_size() {
        assert_eq!(LEDS_PER_IC, PWM_BITS);
    }
}
