fn main() {
    // Tell Cargo to re-run this build script if the WiFi credentials change
    println!("cargo:rerun-if-env-changed=WIFI_SSID");
    println!("cargo:rerun-if-env-changed=WIFI_PASSWORD");
}
