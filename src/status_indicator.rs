//! Status indicator for Wi-Fi and system state.
//!
//! Provides a 1-pixel visual status signal positioned by default at the
//! upper-right corner of the 88x88 matrix (`x = 87, y = 0`).
//!
//! ### Signaling Conventions
//!
//! | State | Color | Cadence | Description |
//! |---|---|---|---|
//! | **Connecting** | Amber / Yellow | 1 Hz (500 ms on / 500 ms off) | Attempting to associate with Wi-Fi AP |
//! | **Waiting DHCP** | Cyan / Blue | 2 Hz (250 ms on / 250 ms off) | Associated with AP; waiting for DHCP lease |
//! | **Connected / Ready** | Dim Green | Heartbeat (double pulse every 3 s) | Fully connected, IP assigned, HTTP server ready |
//! | **Error / Lost** | Red | 4 Hz (125 ms on / 125 ms off) | Connection failed or lost |

use crate::frame_buffer::{FrameBuffer, Pixel};
use crate::{MATRIX_HEIGHT, MATRIX_WIDTH};

/// Default column for the status indicator pixel (upper-right: column 87).
pub const STATUS_PIXEL_X: usize = MATRIX_WIDTH - 1;

/// Default row for the status indicator pixel (upper-right: row 0).
pub const STATUS_PIXEL_Y: usize = 0;

/// Color when indicator pixel is unlit.
pub const COLOR_OFF: Pixel = [0, 0, 0];

/// Amber / Yellow color for Wi-Fi connecting phase.
pub const COLOR_CONNECTING: Pixel = [0x2000, 0x1000, 0x0000];

/// Cyan / Light Blue color for DHCP address negotiation.
pub const COLOR_WAITING_DHCP: Pixel = [0x0000, 0x1000, 0x2000];

/// Soft Dim Green color for connected / healthy state.
pub const COLOR_CONNECTED: Pixel = [0x0000, 0x1800, 0x0000];

/// Red color for connection error or lost connection.
pub const COLOR_ERROR: Pixel = [0x2000, 0x0000, 0x0000];

/// Total cycle period for `Connecting` state: 1 Hz (1000 ms).
pub const PERIOD_CONNECTING_MS: u64 = 1000;
/// On-duration for `Connecting` state: 500 ms.
pub const ON_CONNECTING_MS: u64 = 500;

/// Total cycle period for `WaitingDhcp` state: 2 Hz (500 ms).
pub const PERIOD_WAITING_DHCP_MS: u64 = 500;
/// On-duration for `WaitingDhcp` state: 250 ms.
pub const ON_WAITING_DHCP_MS: u64 = 250;

/// Total cycle period for `Connected` state: 3000 ms heartbeat.
pub const PERIOD_CONNECTED_MS: u64 = 3000;
/// First pulse of heartbeat: 0..100 ms.
pub const HEARTBEAT_PULSE_1_END_MS: u64 = 100;
/// Start of second pulse: 250 ms.
pub const HEARTBEAT_PULSE_2_START_MS: u64 = 250;
/// End of second pulse: 350 ms.
pub const HEARTBEAT_PULSE_2_END_MS: u64 = 350;

/// Total cycle period for `Error` state: 4 Hz (250 ms).
pub const PERIOD_ERROR_MS: u64 = 250;
/// On-duration for `Error` state: 125 ms.
pub const ON_ERROR_MS: u64 = 125;

/// System / Wi-Fi status states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusState {
    /// Initializing or attempting to associate with Wi-Fi AP.
    Connecting,
    /// Associated with AP; waiting for DHCP IPv4 lease.
    WaitingDhcp,
    /// Connected to Wi-Fi with valid IPv4 address; HTTP server active.
    Connected,
    /// Connection failed or lost.
    Error,
}

/// Returns the base RGB color associated with a [`StatusState`].
#[inline]
pub const fn state_color(state: StatusState) -> Pixel {
    match state {
        StatusState::Connecting => COLOR_CONNECTING,
        StatusState::WaitingDhcp => COLOR_WAITING_DHCP,
        StatusState::Connected => COLOR_CONNECTED,
        StatusState::Error => COLOR_ERROR,
    }
}

/// Computes whether the status indicator pixel is ON at `time_ms` for the given state.
pub fn is_pixel_on(state: StatusState, time_ms: u64) -> bool {
    match state {
        StatusState::Connecting => {
            let phase = time_ms % PERIOD_CONNECTING_MS;
            phase < ON_CONNECTING_MS
        }
        StatusState::WaitingDhcp => {
            let phase = time_ms % PERIOD_WAITING_DHCP_MS;
            phase < ON_WAITING_DHCP_MS
        }
        StatusState::Connected => {
            let phase = time_ms % PERIOD_CONNECTED_MS;
            phase < HEARTBEAT_PULSE_1_END_MS
                || (HEARTBEAT_PULSE_2_START_MS..HEARTBEAT_PULSE_2_END_MS).contains(&phase)
        }
        StatusState::Error => {
            let phase = time_ms % PERIOD_ERROR_MS;
            phase < ON_ERROR_MS
        }
    }
}

/// Computes the pixel color at `time_ms` for a given [`StatusState`].
/// Returns `COLOR_OFF` during the off phases of blinking/pulsing.
pub fn compute_pixel_color(state: StatusState, time_ms: u64) -> Pixel {
    if is_pixel_on(state, time_ms) {
        state_color(state)
    } else {
        COLOR_OFF
    }
}

/// Status indicator manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusIndicator {
    state: StatusState,
}

impl Default for StatusIndicator {
    fn default() -> Self {
        Self::new(StatusState::Connecting)
    }
}

impl StatusIndicator {
    /// Create a new status indicator with the specified initial state.
    pub const fn new(state: StatusState) -> Self {
        Self { state }
    }

    /// Update the current status state.
    pub fn set_state(&mut self, state: StatusState) {
        self.state = state;
    }

    /// Read the current status state.
    pub fn state(&self) -> StatusState {
        self.state
    }

    /// Check if the indicator pixel is in its ON phase at `time_ms`.
    pub fn is_on(&self, time_ms: u64) -> bool {
        is_pixel_on(self.state, time_ms)
    }

    /// Get the current pixel color at `time_ms`.
    pub fn pixel_color(&self, time_ms: u64) -> Pixel {
        compute_pixel_color(self.state, time_ms)
    }

    /// Apply the status indicator pixel to the default upper-right corner (`x = 87, y = 0`).
    pub fn apply(&self, buffer: &mut FrameBuffer, time_ms: u64) {
        self.apply_to(buffer, time_ms, STATUS_PIXEL_X, STATUS_PIXEL_Y);
    }

    /// Apply the status indicator pixel to custom coordinates `(x, y)`.
    pub fn apply_to(&self, buffer: &mut FrameBuffer, time_ms: u64, x: usize, y: usize) {
        if x < MATRIX_WIDTH && y < MATRIX_HEIGHT {
            let color = self.pixel_color(time_ms);
            buffer.set_pixel(x, y, color[0], color[1], color[2]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_connecting() {
        let indicator = StatusIndicator::default();
        assert_eq!(indicator.state(), StatusState::Connecting);
    }

    #[test]
    fn state_transitions_update_correctly() {
        let mut indicator = StatusIndicator::new(StatusState::Connecting);
        assert_eq!(indicator.state(), StatusState::Connecting);

        indicator.set_state(StatusState::WaitingDhcp);
        assert_eq!(indicator.state(), StatusState::WaitingDhcp);

        indicator.set_state(StatusState::Connected);
        assert_eq!(indicator.state(), StatusState::Connected);

        indicator.set_state(StatusState::Error);
        assert_eq!(indicator.state(), StatusState::Error);
    }

    #[test]
    fn state_colors_match_specifications() {
        assert_eq!(state_color(StatusState::Connecting), COLOR_CONNECTING);
        assert_eq!(state_color(StatusState::WaitingDhcp), COLOR_WAITING_DHCP);
        assert_eq!(state_color(StatusState::Connected), COLOR_CONNECTED);
        assert_eq!(state_color(StatusState::Error), COLOR_ERROR);
    }

    #[test]
    fn connecting_state_blinks_at_1hz_500ms_duty() {
        let indicator = StatusIndicator::new(StatusState::Connecting);

        // First cycle: 0..500 ms ON, 500..1000 ms OFF
        assert!(indicator.is_on(0));
        assert_eq!(indicator.pixel_color(0), COLOR_CONNECTING);

        assert!(indicator.is_on(250));
        assert_eq!(indicator.pixel_color(250), COLOR_CONNECTING);

        assert!(indicator.is_on(499));
        assert_eq!(indicator.pixel_color(499), COLOR_CONNECTING);

        assert!(!indicator.is_on(500));
        assert_eq!(indicator.pixel_color(500), COLOR_OFF);

        assert!(!indicator.is_on(750));
        assert_eq!(indicator.pixel_color(750), COLOR_OFF);

        assert!(!indicator.is_on(999));
        assert_eq!(indicator.pixel_color(999), COLOR_OFF);

        // Next cycle
        assert!(indicator.is_on(1000));
        assert_eq!(indicator.pixel_color(1000), COLOR_CONNECTING);

        assert!(!indicator.is_on(1500));
        assert_eq!(indicator.pixel_color(1500), COLOR_OFF);
    }

    #[test]
    fn waiting_dhcp_blinks_at_2hz_250ms_duty() {
        let indicator = StatusIndicator::new(StatusState::WaitingDhcp);

        // 0..250 ms ON, 250..500 ms OFF
        assert!(indicator.is_on(0));
        assert_eq!(indicator.pixel_color(0), COLOR_WAITING_DHCP);

        assert!(indicator.is_on(150));
        assert_eq!(indicator.pixel_color(150), COLOR_WAITING_DHCP);

        assert!(indicator.is_on(249));
        assert_eq!(indicator.pixel_color(249), COLOR_WAITING_DHCP);

        assert!(!indicator.is_on(250));
        assert_eq!(indicator.pixel_color(250), COLOR_OFF);

        assert!(!indicator.is_on(499));
        assert_eq!(indicator.pixel_color(499), COLOR_OFF);

        // Cycle repeats at 500 ms
        assert!(indicator.is_on(500));
        assert_eq!(indicator.pixel_color(500), COLOR_WAITING_DHCP);
    }

    #[test]
    fn connected_state_pulses_heartbeat() {
        let indicator = StatusIndicator::new(StatusState::Connected);

        // Pulse 1: 0..100 ms
        assert!(indicator.is_on(0));
        assert_eq!(indicator.pixel_color(0), COLOR_CONNECTED);
        assert!(indicator.is_on(99));
        assert_eq!(indicator.pixel_color(99), COLOR_CONNECTED);

        // Inter-pulse pause: 100..250 ms
        assert!(!indicator.is_on(100));
        assert_eq!(indicator.pixel_color(100), COLOR_OFF);
        assert!(!indicator.is_on(200));
        assert_eq!(indicator.pixel_color(200), COLOR_OFF);

        // Pulse 2: 250..350 ms
        assert!(indicator.is_on(250));
        assert_eq!(indicator.pixel_color(250), COLOR_CONNECTED);
        assert!(indicator.is_on(349));
        assert_eq!(indicator.pixel_color(349), COLOR_CONNECTED);

        // Rest of cycle: 350..3000 ms
        assert!(!indicator.is_on(350));
        assert_eq!(indicator.pixel_color(350), COLOR_OFF);
        assert!(!indicator.is_on(1500));
        assert_eq!(indicator.pixel_color(1500), COLOR_OFF);
        assert!(!indicator.is_on(2999));
        assert_eq!(indicator.pixel_color(2999), COLOR_OFF);

        // Next 3s cycle starts
        assert!(indicator.is_on(3000));
        assert_eq!(indicator.pixel_color(3000), COLOR_CONNECTED);
    }

    #[test]
    fn error_state_blinks_at_4hz_125ms_duty() {
        let indicator = StatusIndicator::new(StatusState::Error);

        // 0..125 ms ON, 125..250 ms OFF
        assert!(indicator.is_on(0));
        assert_eq!(indicator.pixel_color(0), COLOR_ERROR);

        assert!(indicator.is_on(124));
        assert_eq!(indicator.pixel_color(124), COLOR_ERROR);

        assert!(!indicator.is_on(125));
        assert_eq!(indicator.pixel_color(125), COLOR_OFF);

        assert!(!indicator.is_on(249));
        assert_eq!(indicator.pixel_color(249), COLOR_OFF);

        // Next 250 ms cycle
        assert!(indicator.is_on(250));
        assert_eq!(indicator.pixel_color(250), COLOR_ERROR);
    }

    #[test]
    fn apply_sets_upper_right_pixel_accurately() {
        let mut fb = FrameBuffer::new();
        let indicator = StatusIndicator::new(StatusState::Connecting);

        // At time 0 (ON phase)
        indicator.apply(&mut fb, 0);
        assert_eq!(
            fb.get_pixel(STATUS_PIXEL_X, STATUS_PIXEL_Y),
            COLOR_CONNECTING
        );
        // Surrounding pixels should remain black
        assert_eq!(fb.get_pixel(STATUS_PIXEL_X - 1, STATUS_PIXEL_Y), [0, 0, 0]);
        assert_eq!(fb.get_pixel(STATUS_PIXEL_X, STATUS_PIXEL_Y + 1), [0, 0, 0]);

        // At time 500 (OFF phase)
        indicator.apply(&mut fb, 500);
        assert_eq!(fb.get_pixel(STATUS_PIXEL_X, STATUS_PIXEL_Y), COLOR_OFF);
    }

    #[test]
    fn apply_to_respects_custom_coords_and_bounds() {
        let mut fb = FrameBuffer::new();
        let indicator = StatusIndicator::new(StatusState::Error);

        // Valid custom coordinates
        indicator.apply_to(&mut fb, 0, 10, 20);
        assert_eq!(fb.get_pixel(10, 20), COLOR_ERROR);

        // Out of bounds coordinate does not panic
        indicator.apply_to(&mut fb, 0, 100, 200);
    }
}
