//! ESP32 LED Matrix Controller
//!
//! This application controls an 88x88 RGB LED matrix display via a web interface.
//! It connects to WiFi and serves an HTTP server where users can input text
//! to display on the LED matrix.

use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStackMode};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, Configuration, EspWifi};
use esp_idf_sys::{self as sys};
use log::{error, info, warn};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

mod font;
mod http_server;
mod led_matrix;
mod wifi;

use http_server::start_http_server;
use led_matrix::LedMatrix;

/// LED Matrix dimensions
pub const MATRIX_WIDTH: usize = 88;
pub const MATRIX_HEIGHT: usize = 88;

/// WiFi credentials - MODIFY THESE FOR YOUR NETWORK
const WIFI_SSID: &str = "YOUR_WIFI_SSID";
const WIFI_PASSWORD: &str = "YOUR_WIFI_PASSWORD";

/// Shared display text buffer
static DISPLAY_TEXT: Mutex<String> = Mutex::new(String::new());

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_sys::link_patches();

    // Initialize logging
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("=== ESP32 LED Matrix Controller ===");
    info!("Starting initialization...");

    // Get peripherals
    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Initialize WiFi
    info!("Connecting to WiFi: {}", WIFI_SSID);
    let _wifi = wifi::connect_wifi(peripherals.modem, sysloop, nvs, WIFI_SSID, WIFI_PASSWORD)?;
    info!("WiFi connected successfully!");

    // Get IP address
    let netif = EspNetif::new_with_conf(&NetifConfiguration {
        ip_configuration: esp_idf_svc::ipv4::Configuration::Dhcp,
        ..Default::default()
    })?;

    // Initialize LED matrix
    // NOTE: GPIO34-39 are INPUT ONLY on ESP32, so we use GPIO13 for DB2
    info!("Initializing LED matrix driver...");
    let led_matrix = Arc::new(Mutex::new(LedMatrix::new(
        peripherals.pins.gpio4,  // GCLK  - Multiplex clock
        peripherals.pins.gpio5,  // DCLK  - Data clock
        peripherals.pins.gpio18, // LE    - Latch Enable
        peripherals.pins.gpio19, // A0    - Address bit 0
        peripherals.pins.gpio21, // A1    - Address bit 1
        peripherals.pins.gpio22, // A2    - Address bit 2
        peripherals.pins.gpio23, // A3    - Address bit 3
        peripherals.pins.gpio25, // DR1   - Red data chain 1
        peripherals.pins.gpio26, // DG1   - Green data chain 1
        peripherals.pins.gpio27, // DB1   - Blue data chain 1
        peripherals.pins.gpio32, // DR2   - Red data chain 2
        peripherals.pins.gpio33, // DG2   - Green data chain 2
        peripherals.pins.gpio13, // DB2   - Blue data chain 2 (NOT gpio34!)
    )?));

    // Start display refresh task
    let matrix_clone = led_matrix.clone();
    std::thread::spawn(move || {
        info!("Display refresh task started");
        loop {
            if let Ok(mut matrix) = matrix_clone.lock() {
                // Get current display text
                if let Ok(text) = DISPLAY_TEXT.lock() {
                    matrix.display_text(&text);
                }
                // Refresh the display
                if let Err(e) = matrix.refresh() {
                    error!("Display refresh error: {:?}", e);
                }
            }
            FreeRtos::delay_ms(10);
        }
    });

    // Start HTTP server
    info!("Starting HTTP server...");
    let server = start_http_server(led_matrix.clone())?;

    info!("=== System Ready ===");
    info!("Open http://<ESP32_IP>/ in your browser to control the display");

    // Keep main thread alive
    loop {
        FreeRtos::delay_ms(1000);
    }
}
