//! WiFi connectivity module
//!
//! Handles WiFi connection using the ESP-IDF WiFi stack.

use anyhow::{Context, Result};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::{EspNetif, EspNetifStack, NetifConfiguration, NetifStackMode};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use log::{debug, info};

/// Connect to WiFi and return the WiFi instance
pub fn connect_wifi(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    password: &str,
) -> Result<EspWifi<'static>> {
    info!("Initializing WiFi...");

    let netif_stack = EspNetifStack::new()?;
    let netif = EspNetif::new_with_conf(&NetifConfiguration {
        ip_configuration: esp_idf_svc::ipv4::Configuration::Dhcp,
        ..Default::default()
    })?;

    let mut wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;

    let configuration = Configuration::Client(ClientConfiguration {
        ssid: ssid.as_bytes().try_into().context("Invalid SSID")?,
        password: password.as_bytes().try_into().context("Invalid password")?,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    wifi.set_configuration(&configuration)?;
    wifi.start()?;
    info!("WiFi started, connecting to {}...", ssid);

    wifi.connect()?;
    info!("WiFi connected!");

    // Wait for IP assignment
    let mut retries = 30;
    while retries > 0 {
        if let Some(ip_info) = wifi.sta_netif().get_ip_info() {
            info!("Got IP: {}", ip_info.ip);
            break;
        }
        retries -= 1;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    if retries == 0 {
        anyhow::bail!("Failed to get IP address");
    }

    Ok(wifi)
}

/// Get the IP address of the ESP32
pub fn get_ip_address(wifi: &EspWifi) -> Option<String> {
    wifi.sta_netif()
        .get_ip_info()
        .map(|info| info.ip.to_string())
}
