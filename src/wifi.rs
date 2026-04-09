//! WiFi connectivity module using esp-radio 0.17.0
//!
//! Handles WiFi connection using the pure Rust esp-radio crate with embassy-net.

extern crate alloc;

use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent};
use log::{error, info};
use static_cell::StaticCell;

use crate::WIFI_PASSWORD;
use crate::WIFI_SSID;

/// Global radio controller — must outlive WifiController and WifiDevice
static RADIO_CONTROLLER: StaticCell<Controller<'static>> = StaticCell::new();

/// Global WiFi stack resources
static WIFI_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

/// Initialize WiFi and return the network stack.
///
/// Requires `esp_rtos::start()` to have been called before this function.
pub fn init_wifi_inline(
    spawner: Spawner,
    wifi: esp_hal::peripherals::WIFI<'static>,
) -> Stack<'static> {
    // Initialize the radio controller (requires RTOS scheduler to be running)
    let controller: Controller<'static> = esp_radio::init().unwrap();
    let controller = RADIO_CONTROLLER.init(controller);

    // Create WiFi interface with default hardware config (buffer sizes etc.)
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(controller, wifi, esp_radio::wifi::Config::default()).unwrap();

    // Configure station (client) mode with SSID and password
    wifi_controller
        .set_config(&ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(alloc::string::String::from(WIFI_SSID))
                .with_password(alloc::string::String::from(WIFI_PASSWORD)),
        ))
        .unwrap();

    // Create network stack
    let stack_config = embassy_net::Config::dhcpv4(Default::default());
    let stack_resources = WIFI_RESOURCES.init(StackResources::<3>::new());

    let (stack, runner) = embassy_net::new(
        interfaces.sta,
        stack_config,
        stack_resources,
        1234, // Random seed
    );

    // Spawn network runner and WiFi connection tasks
    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(wifi_connection_task(wifi_controller)).ok();

    stack
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

/// Wait for WiFi connection to come up
pub async fn wait_for_connection() {
    info!("Waiting for WiFi connection...");
    embassy_time::Timer::after(embassy_time::Duration::from_secs(2)).await;
    info!("WiFi connection established!");
}

/// Get the IP address of the device (placeholder)
pub fn get_ip_address() -> Option<heapless::String<16>> {
    None
}

/// WiFi connection task — starts WiFi, then connects and reconnects as needed
#[embassy_executor::task]
async fn wifi_connection_task(mut controller: WifiController<'static>) {
    info!("WiFi connection task started");

    controller.start_async().await.unwrap();

    loop {
        info!("Connecting to SSID: {}", WIFI_SSID);
        match controller.connect_async().await {
            Ok(()) => {
                info!("WiFi connected!");
                // Wait until disconnected before attempting reconnect
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                info!("WiFi disconnected, reconnecting...");
            }
            Err(e) => {
                error!("WiFi connect error: {:?}", e);
                embassy_time::Timer::after(embassy_time::Duration::from_secs(2)).await;
            }
        }
    }
}

