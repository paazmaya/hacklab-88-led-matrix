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

    // Initialize LED matrix GPIO pins.
    //
    // Pin map matches the wiring diagram in README.md — the user wires the
    // LED matrix signals to these specific ESP32-C3 GPIOs. GPIO8/GPIO9 are
    // boot-strapping pins (the matrix's pull-ups keep them HIGH at boot, so
    // normal boot mode is preserved) and GPIO20/GPIO21 are the UART pins
    // (serial logging may be visible as faint noise on DG2/DB2).
    let mut led_matrix = LedMatrix::new(
        Output::new(peripherals.GPIO0,  Level::Low, OutputConfig::default()), // GCLK  — multiplex clock
        Output::new(peripherals.GPIO1,  Level::Low, OutputConfig::default()), // DCLK  — data clock
        Output::new(peripherals.GPIO2,  Level::Low, OutputConfig::default()), // LE    — latch enable
        Output::new(peripherals.GPIO3,  Level::Low, OutputConfig::default()), // A0    — address bit 0
        Output::new(peripherals.GPIO4,  Level::Low, OutputConfig::default()), // A1    — address bit 1
        Output::new(peripherals.GPIO5,  Level::Low, OutputConfig::default()), // A2    — address bit 2
        Output::new(peripherals.GPIO6,  Level::Low, OutputConfig::default()), // A3    — address bit 3
        Output::new(peripherals.GPIO7,  Level::Low, OutputConfig::default()), // DR1   — red   data chain 1
        Output::new(peripherals.GPIO8,  Level::Low, OutputConfig::default()), // DG1   — green data chain 1 (boot)
        Output::new(peripherals.GPIO9,  Level::Low, OutputConfig::default()), // DB1   — blue  data chain 1 (boot)
        Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default()), // DR2   — red   data chain 2
        Output::new(peripherals.GPIO20, Level::Low, OutputConfig::default()), // DG2   — green data chain 2 (UART RXD)
        Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default()), // DB2   — blue  data chain 2 (UART TXD)
    );

    // Initialize WiFi and start network task
    info!("Initializing WiFi...");
    let wifi_stack = wifi::init_wifi_inline(
        spawner,
        peripherals.WIFI,
    );

    // Wait for WiFi connection (link up + DHCP lease)
    info!("Waiting for WiFi connection...");
    wifi::wait_for_connection(wifi_stack).await;
    info!("WiFi connected!");

    // Get and display IP address
    if let Some(ip) = wifi::get_ip_address(wifi_stack) {
        info!("IP Address: http://{}/", ip);
    } else {
        info!("WiFi ready, but no IP address yet");
    }

    // Spawn the HTTP server task, handing it a reference to the network stack
    spawner.spawn(http_server::http_server_task(wifi_stack)).ok();

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
