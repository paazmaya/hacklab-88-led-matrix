fn main() {
    embuild::espidf::sysenv::output_cargo_cfgs();
    embuild::build::CfgArgs::output_propagated("ESP_IDF").unwrap();
}
