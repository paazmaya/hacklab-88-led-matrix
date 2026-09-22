fn main() {
    // Register to prevent unexpected_cfgs compiler warnings
    println!("cargo:rustc-check-cfg=cfg(tarpaulin_include)");

    // Tell Cargo to re-run this build script if the WiFi credentials change
    println!("cargo:rerun-if-env-changed=WIFI_SSID");
    println!("cargo:rerun-if-env-changed=WIFI_PASSWORD");
}
