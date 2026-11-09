fn main() {
    tauri_build::build();
    // tauri-build links its Windows resources to binaries only. MockRuntime
    // unit tests also need the Common Controls v6 manifest from resource.lib.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!(
            "cargo:rustc-link-search=native={}",
            std::env::var("OUT_DIR").unwrap()
        );
    }
}
