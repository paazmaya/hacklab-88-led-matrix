//! ESP32-S2 LED Matrix Controller
//!
//! This application controls an 88x88 RGB LED matrix display via a web interface.
//! It connects to WiFi and serves an HTTP server where users can input text
//! to display on the LED matrix.
//!
//! Built with pure Rust using esp-hal (no ESP-IDF required).
//!
//! ## Pin Configuration (ESP32-S2)
//! Uses GPIO 4-16 which are available on ESP32-S2 (avoiding USB pins 18-20)

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use log::info;

mod font;
mod http_server;
mod led_matrix;
mod wifi;

use crate::led_matrix::LedMatrix;

/// LED Matrix dimensions
pub const MATRIX_WIDTH: usize = 88;
pub const MATRIX_HEIGHT: usize = 88;

/// WiFi credentials - MODIFY THESE FOR YOUR NETWORK
const WIFI_SSID: &str = "YOUR_WIFI_SSID";
const WIFI_PASSWORD: &str = "YOUR_WIFI_PASSWORD";

/// Global display text buffer
static DISPLAY_TEXT: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    heapless::String<32>,
> = embassy_sync::mutex::Mutex::new(heapless::String::new());

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // Initialize ESP32 with default clock configuration
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Initialize logging
    esp_println::logger::init_logger_from_env();
    info!("=== ESP32 LED Matrix Controller ===");
    info!("Pure Rust build with esp-hal");

    // Initialize LED matrix GPIO pins (ESP32-S2 compatible)
    let mut led_matrix = LedMatrix::new(
        Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()), // GCLK
        Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()), // DCLK
        Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default()), // LE
        Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default()), // A0
        Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default()), // A1
        Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()), // A2
        Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default()), // A3
        Output::new(peripherals.GPIO11, Level::Low, OutputConfig::default()), // DR1
        Output::new(peripherals.GPIO12, Level::Low, OutputConfig::default()), // DG1
        Output::new(peripherals.GPIO13, Level::Low, OutputConfig::default()), // DB1
        Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default()), // DR2
        Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default()), // DG2
        Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default()), // DB2
    );

    // Initialize WiFi and start network task
    info!("Initializing WiFi...");
    let _wifi_stack = wifi::init_wifi_inline(
        spawner,
        peripherals.TIMG1,
        peripherals.RNG,
        peripherals.WIFI,
    );

    // Wait for WiFi connection
    info!("Waiting for WiFi connection...");
    wifi::wait_for_connection().await;
    info!("WiFi connected!");

    // Get and display IP address
    if let Some(ip) = wifi::get_ip_address() {
        info!("IP Address: {}", ip);
    }

    // Spawn the HTTP server task
    spawner.spawn(http_server::http_server_task()).ok();

    info!("=== System Ready ===");
    info!("Open http://<ESP32_IP>/ in your browser to control the display");

    // Main display refresh loop
    loop {
        // Get current display text
        let text = DISPLAY_TEXT.lock().await.clone();

        // Update display
        led_matrix.display_text(&text);
        led_matrix.refresh();

        // Small delay to prevent watchdog
        Timer::after(Duration::from_millis(1)).await;
    }
}
