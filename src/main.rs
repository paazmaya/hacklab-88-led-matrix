//! ESP32-C3 LED Matrix Controller
//!
//! This application controls an 88x88 RGB LED matrix display via a web interface.
//! It connects to WiFi and serves an HTTP server where users can input text
//! to display on the LED matrix.
//!
//! Built with pure Rust using esp-hal (no ESP-IDF required).
//!
//! ## Pin Configuration (ESP32-C3 SuperMini)
//! GPIO0–GPIO6:  GCLK, DCLK, LE, A0, A1, A2, A3 (control signals)
//! GPIO7–GPIO10: DR1, DG1, DB1, DR2 (RGB data chain 1 + first half of chain 2)
//! GPIO20, GPIO21: DG2, DB2 (RGB data chain 2 second half; share pins with UART)
//!
//! All 13 pins line up with the wiring diagram in README.md.
//! GPIO8/GPIO9 are boot-strapping pins — the LED matrix's pull-ups keep them
//! HIGH at boot, so the chip enters normal boot mode. GPIO20/GPIO21 are the
//! USB-serial UART pins; if the serial monitor prints while the matrix is
//! refreshing you may see faint noise on DG2/DB2.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use log::info;

mod http_server;
mod led_matrix;
mod wifi;

use crate::led_matrix::LedMatrix;

/// LED Matrix dimensions
pub const MATRIX_WIDTH: usize = 88;
pub const MATRIX_HEIGHT: usize = 88;

/// WiFi credentials - can be configured via environment variables at compile time
/// (e.g. `WIFI_SSID="MyRouter" WIFI_PASSWORD="secret" cargo release-esp32`)
/// Defaults to "Wokwi-GUEST" and empty password for immediate out-of-the-box Wokwi simulation.
pub const WIFI_SSID: &str = match option_env!("WIFI_SSID") {
    Some(val) => val,
    None => "Wokwi-GUEST",
};

pub const WIFI_PASSWORD: &str = match option_env!("WIFI_PASSWORD") {
    Some(val) => val,
    None => "",
};

/// Global display command buffer
pub static DISPLAY_COMMAND: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    Option<esp32_led_matrix::http_request::DisplayCommand>,
> = embassy_sync::mutex::Mutex::new(None);

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // Initialize ESP32 with default clock configuration
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Start esp-rtos runtime (RISC-V / ESP32-C3 requires timer + software interrupt)
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let sw_intr =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_intr.software_interrupt0);

    // Initialize logging
    esp_println::logger::init_logger_from_env();
    info!("=== ESP32 LED Matrix Controller ===");
    info!("Pure Rust build with esp-hal");

    // Initialize LED matrix GPIO pins.
    //
    // Pin map matches the wiring diagram in README.md — the user wires the
    // LED matrix signals to these specific ESP32-C3 GPIOs. GPIO8/GPIO9 are
    // boot-strapping pins (the matrix's pull-ups keep them HIGH at boot, so
    // normal boot mode is preserved) and GPIO20/GPIO21 are the UART pins
    // (serial logging may be visible as faint noise on DG2/DB2).
    let mut led_matrix = LedMatrix::new(
        Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default()), // GCLK  — multiplex clock
        Output::new(peripherals.GPIO1, Level::Low, OutputConfig::default()), // DCLK  — data clock
        Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default()), // LE    — latch enable
        Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default()), // A0    — address bit 0
        Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()), // A1    — address bit 1
        Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()), // A2    — address bit 2
        Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default()), // A3    — address bit 3
        Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default()), // DR1   — red   data chain 1
        Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default()), // DG1   — green data chain 1 (boot)
        Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()), // DB1   — blue  data chain 1 (boot)
        Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default()), // DR2   — red   data chain 2
        Output::new(peripherals.GPIO20, Level::Low, OutputConfig::default()), // DG2   — green data chain 2 (UART RXD)
        Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default()), // DB2   — blue  data chain 2 (UART TXD)
    );

    // Initialize WiFi and start network task
    info!("Initializing WiFi...");
    let wifi_stack = wifi::init_wifi_inline(spawner, peripherals.WIFI);

    // Spawn the HTTP server task, handing it a reference to the network stack
    spawner
        .spawn(http_server::http_server_task(wifi_stack))
        .ok();

    info!("=== System Starting ===");
    info!("Connecting to WiFi SSID: {}", WIFI_SSID);

    let mut status_indicator = esp32_led_matrix::status_indicator::StatusIndicator::new(
        esp32_led_matrix::status_indicator::StatusState::Connecting,
    );
    let mut logged_ip = false;

    // Main display refresh loop
    loop {
        // Update Wi-Fi connection and network status
        let current_status = wifi::get_status(wifi_stack);
        status_indicator.set_state(current_status);

        if current_status == esp32_led_matrix::status_indicator::StatusState::Connected
            && !logged_ip
        {
            if let Some(ip) = wifi::get_ip_address(wifi_stack) {
                info!("WiFi connected! Open http://{}/ to control the display", ip);
                logged_ip = true;
            }
        } else if current_status != esp32_led_matrix::status_indicator::StatusState::Connected {
            logged_ip = false;
        }

        // Check for new display command
        {
            let mut lock = DISPLAY_COMMAND.lock().await;
            if let Some(cmd) = lock.take() {
                if let Some(brightness) = cmd.brightness {
                    led_matrix.set_brightness(brightness);
                }
                if cmd.clear {
                    led_matrix.clear();
                }
                if !cmd.text.is_empty() {
                    let start_x = cmd.x.unwrap_or(4);
                    let start_y = cmd.y.unwrap_or(((MATRIX_HEIGHT - 7) / 2) as i32);
                    led_matrix.draw_text_at(
                        &cmd.text,
                        start_x,
                        start_y,
                        cmd.color[0],
                        cmd.color[1],
                        cmd.color[2],
                    );
                }
            }
        }

        // Overlay status indicator in upper-right corner (x = 87, y = 0).
        // Marks the matrix dirty only if the status pixel color changed.
        let now_ms = embassy_time::Instant::now().as_millis();
        led_matrix.apply_status(&status_indicator, now_ms);

        // Multiplexed LED matrix refresh (shifts data if dirty, then runs high-duty multiplexing)
        led_matrix.refresh();

        // Adaptive delay: when there is active text, 1 ms keeps refresh smooth and bright.
        // When only the status indicator or blank screen is active, relax sleep to save energy.
        let sleep_ms = if led_matrix.has_content() {
            1
        } else if led_matrix.is_blank() {
            50
        } else {
            20
        };
        Timer::after(Duration::from_millis(sleep_ms)).await;
    }
}
