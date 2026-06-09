//! WiFi connectivity module using esp-radio 0.17.0
//!
//! Handles WiFi connection using the pure Rust esp-radio crate with embassy-net.

extern crate alloc;

use core::fmt::Write;

use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent};
use log::{error, info};
use static_cell::StaticCell;

use crate::WIFI_PASSWORD;
use crate::WIFI_SSID;

/// Global radio controller — must outlive `WifiController` and `WifiDevice`.
static RADIO_CONTROLLER: StaticCell<Controller<'static>> = StaticCell::new();

/// Global WiFi stack resources (sockets, etc.).
static WIFI_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

/// Global network stack — stored here so the stack value is never dropped.
/// A `'static` reference is returned to callers (HTTP server, etc.).
static STACK: StaticCell<Stack<'static>> = StaticCell::new();

/// Initialize WiFi (radio + driver) and the embassy-net stack.
///
/// The radio controller and the network stack are stored in `'static` cells so
/// they live for the entire program lifetime. A reference to the stack is
/// returned so the HTTP server (and any other user) can accept connections
/// on it.
///
/// Requires `esp_rtos::start()` to have been called before this function so
/// the embassy executor is running and can host the spawned tasks.
pub fn init_wifi_inline(
    spawner: Spawner,
    wifi: esp_hal::peripherals::WIFI<'static>,
) -> &'static Stack<'static> {
    // Initialize the radio controller (requires RTOS scheduler to be running).
    let controller: Controller<'static> = esp_radio::init().unwrap();
    let controller = RADIO_CONTROLLER.init(controller);

    // Create WiFi interface with default hardware config (buffer sizes etc.).
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(controller, wifi, esp_radio::wifi::Config::default()).unwrap();

    // Configure station (client) mode with SSID and password.
    wifi_controller
        .set_config(&ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(alloc::string::String::from(WIFI_SSID))
                .with_password(alloc::string::String::from(WIFI_PASSWORD)),
        ))
        .unwrap();

    // Build the network stack with DHCP (IP address assigned by router).
    let stack_config = Config::dhcpv4(Default::default());
    let stack_resources = WIFI_RESOURCES.init(StackResources::<3>::new());

    let (stack, runner) = embassy_net::new(
        interfaces.sta,
        stack_config,
        stack_resources,
        1234, // Random seed
    );

    // Spawn the network runner task (drives the stack + DHCP socket)
    // and the WiFi connection task (associates and reconnects as needed).
    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(wifi_connection_task(wifi_controller)).ok();

    // Park the stack in a `'static` cell and hand out a reference.
    STACK.init(stack)
}

/// Network runner task — drives the embassy-net stack (DHCP, ARP, etc.).
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

/// Wait until WiFi is associated and a DHCP lease has been acquired.
///
/// Polls the embassy-net stack for link-up state and an IPv4 configuration.
/// Yields to the executor between polls so other tasks can make progress.
pub async fn wait_for_connection(stack: &Stack<'static>) {
    info!("Waiting for WiFi link...");
    while !stack.is_link_up() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(200)).await;
    }
    info!("Link is up, waiting for DHCP lease...");

    while stack.config_v4().is_none() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(200)).await;
    }
    info!("WiFi ready — got IPv4 configuration.");
}

/// Get the current IPv4 address as a printable `"x.x.x.x"` string.
///
/// Returns `None` if the stack doesn't yet have an IPv4 address
/// (no DHCP lease yet, or the link is down).
pub fn get_ip_address(stack: &Stack<'static>) -> Option<heapless::String<16>> {
    let config = stack.config_v4()?;
    let addr = config.address.address(); // embassy_net::Ipv4Address (smoltcp) — `address()` returns the address from a CIDR
    let mut s: heapless::String<16> = heapless::String::new();
    // `smoltcp::wire::Ipv4Address` implements `core::fmt::Display` as dotted
    // decimal. `"255.255.255.255"` is the longest possible form (15 chars +
    // NUL is irrelevant — we just need 15 to fit, 16 is a comfortable margin).
    write!(s, "{}", addr).ok()?;
    Some(s)
}

/// WiFi connection task — starts WiFi, then connects and reconnects as needed.
///
/// `WifiController::connect_async` resolves once the station is associated to
/// the AP. We then wait for a `StaDisconnected` event and try again.
#[embassy_executor::task]
async fn wifi_connection_task(mut controller: WifiController<'static>) {
    info!("WiFi connection task started");

    controller.start_async().await.unwrap();

    loop {
        info!("Connecting to SSID: {}", WIFI_SSID);
        match controller.connect_async().await {
            Ok(()) => {
                info!("WiFi connected!");
                // Wait until disconnected before attempting reconnect.
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
