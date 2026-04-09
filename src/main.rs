//! ESP32-C3 LED Matrix Controller
//!
//! This application controls an 88x88 RGB LED matrix display via a web interface.
//! It connects to WiFi and serves an HTTP server where users can input text
//! to display on the LED matrix.
//!
//! Built with pure Rust using esp-hal (no ESP-IDF required).
//!
//! ## Pin Configuration (ESP32-C3)
//! GPIO4-GPIO10: GCLK, DCLK, LE, A0-A3 (control signals)
//! GPIO0-GPIO3, GPIO20-GPIO21: DR1, DG1, DB1, DR2, DG2, DB2 (RGB data)
//!
//! NOTE: GPIO11-GPIO17 are internal SPI flash pins on ESP32-C3 and were removed
//! from esp-hal in 1.0.0-rc.1 (#4202). Hardware must be wired to the pins above.
//! Original design targeted ESP32-S2 where GPIO11-GPIO16 were regular GPIOs.

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

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // Initialize ESP32 with default clock configuration
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Start esp-rtos runtime (RISC-V / ESP32-C3 requires timer + software interrupt)
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let sw_intr = esp_hal::interrupt::software::SoftwareInterruptControl::new(
        peripherals.SW_INTERRUPT,
    );
    esp_rtos::start(timg0.timer0, sw_intr.software_interrupt0);

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
        Output::new(peripherals.GPIO0,  Level::Low, OutputConfig::default()), // DR1 (was GPIO11)
        Output::new(peripherals.GPIO1,  Level::Low, OutputConfig::default()), // DG1 (was GPIO12)
        Output::new(peripherals.GPIO2,  Level::Low, OutputConfig::default()), // DB1 (was GPIO13)
        Output::new(peripherals.GPIO3,  Level::Low, OutputConfig::default()), // DR2 (was GPIO14)
        Output::new(peripherals.GPIO20, Level::Low, OutputConfig::default()), // DG2 (was GPIO15)
        Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default()), // DB2 (was GPIO16)
    );

    // Initialize WiFi and start network task
    info!("Initializing WiFi...");
    let _wifi_stack = wifi::init_wifi_inline(
        spawner,
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
