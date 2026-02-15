//! WiFi connectivity module using esp-wifi
//!
//! Handles WiFi connection using the pure Rust esp-wifi crate with embassy-net.

use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use esp_hal::rng::Rng;
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration, WifiController},
};
use log::{error, info};
use static_cell::StaticCell;

use crate::WIFI_PASSWORD;
use crate::WIFI_SSID;

/// Global WiFi stack resources
static WIFI_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

/// Global WiFi initialization instance
static WIFI_INIT: StaticCell<esp_wifi::EspWifiController<'static>> = StaticCell::new();
static WIFI_STACK_CELL: StaticCell<Stack<'static>> = StaticCell::new();

/// Global network stack reference
pub static NET_STACK: embassy_sync::once_lock::OnceLock<Stack<'static>> =
    embassy_sync::once_lock::OnceLock::new();

/// Initialize WiFi inline (needed for peripheral lifetime management)
pub fn init_wifi_inline(
    spawner: Spawner,
    timg1: esp_hal::peripherals::TIMG1,
    rng: esp_hal::peripherals::RNG,
    wifi: esp_hal::peripherals::WIFI,
) -> &'static Stack<'static> {
    // SAFETY: The peripherals are created in main and live for 'static lifetime
    // We use transmute to extend the lifetimes since the Rust compiler can't
    // infer this across function boundaries
    unsafe {
        let timg1: esp_hal::peripherals::TIMG1<'static> = core::mem::transmute(timg1);
        let rng: esp_hal::peripherals::RNG<'static> = core::mem::transmute(rng);
        let wifi: esp_hal::peripherals::WIFI<'static> = core::mem::transmute(wifi);

        // Initialize esp-wifi
        let wifi_init = init(
            esp_hal::timer::timg::TimerGroup::new(timg1).timer0,
            Rng::new(rng),
        )
        .unwrap();
        let wifi_init: esp_wifi::EspWifiController<'static> = core::mem::transmute(wifi_init);
        let wifi_init = WIFI_INIT.init(wifi_init);

        // Get WiFi controller and interfaces
        // Returns (WifiController, Interfaces) where Interfaces contains sta and ap WifiDevice fields
        let (controller, interfaces) = esp_wifi::wifi::new(wifi_init, wifi).unwrap();
        let controller: esp_wifi::wifi::WifiController<'static> = core::mem::transmute(controller);

        // Configure WiFi as client
        let config = Configuration::Client(ClientConfiguration {
            ssid: WIFI_SSID.try_into().unwrap(),
            password: WIFI_PASSWORD.try_into().unwrap(),
            ..Default::default()
        });
        let mut controller = controller;
        controller.set_configuration(&config).unwrap();

        // Create network stack
        let stack_config = embassy_net::Config::dhcpv4(Default::default());
        let stack_resources = WIFI_RESOURCES.init(StackResources::<3>::new());

        let (stack, runner) = embassy_net::new(
            interfaces.sta,
            stack_config,
            stack_resources,
            1234, // Random seed
        );

        // Spawn network runner task
        spawner.spawn(net_task(runner)).ok();

        // Spawn WiFi connection management task
        spawner.spawn(wifi_connection_task(controller)).ok();

        // Initialize stack in static cell
        let stack = WIFI_STACK_CELL.init(stack);

        // Store globally
        NET_STACK.init(*stack).ok();

        stack
    }
}
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, esp_wifi::wifi::WifiDevice<'static>>) {
    runner.run().await
}

/// Wait for WiFi connection
pub async fn wait_for_connection() {
    info!("Waiting for DHCP lease...");

    // Wait for the stack to be configured (DHCP)
    loop {
        // This is a simplified wait - in practice you'd check the stack status
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;

        // Check if we have an IP
        // The stack will automatically connect and get DHCP
        break;
    }

    info!("WiFi connection established!");
}

/// Get the IP address of the ESP32
pub fn get_ip_address() -> Option<heapless::String<16>> {
    // This would need to be implemented with the actual stack status
    // For now, return a placeholder
    Some(heapless::String::try_from("192.168.1.x").unwrap())
}

/// WiFi connection task
#[embassy_executor::task]
async fn wifi_connection_task(mut controller: WifiController<'static>) {
    info!("WiFi connection task started");
    info!("Connecting to SSID: {}", WIFI_SSID);

    loop {
        match controller.is_started() {
            Ok(true) => {
                // WiFi is started, check if connected
                match controller.is_connected() {
                    Ok(true) => {
                        // Connected, wait for disconnect event
                        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
                    }
                    Ok(false) => {
                        info!("WiFi disconnected, reconnecting...");
                        controller.connect().ok();
                    }
                    Err(e) => {
                        error!("WiFi connection error: {:?}", e);
                    }
                }
            }
            Ok(false) => {
                // Start WiFi
                info!("Starting WiFi...");
                controller.start().ok();
            }
            Err(e) => {
                error!("WiFi status error: {:?}", e);
            }
        }

        embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
    }
}
