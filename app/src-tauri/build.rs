#[path = "src/ipc_commands.rs"]
mod ipc_commands;

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(ipc_commands::IPC_COMMANDS)),
    )
    .expect("failed to build Tauri permissions");
    // tauri-build links its Windows resources to binaries only. MockRuntime
    // unit tests also need the Common Controls v6 manifest from resource.lib.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!(
            "cargo:rustc-link-search=native={}",
            std::env::var("OUT_DIR").unwrap()
        );
    }
}
