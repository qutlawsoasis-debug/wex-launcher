use std::path::PathBuf;

// Re-generate TypeScript definitions for Wex IPC
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = if let Some(arg) = std::env::args().nth(1) {
        PathBuf::from(arg)
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        let from_cwd = cwd.join("ui/src/lib/ipc/generated");
        if from_cwd.parent().map(|p| p.exists()).unwrap_or(false) {
            from_cwd
        } else {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest_dir.join("../../ui/src/lib/ipc/generated")
        }
    };
    println!("Generating TypeScript definitions in: {}", target.display());
    wex_ipc::write_bindings_to_dir(&target)?;
    println!("TypeScript IPC bindings successfully generated!");
    Ok(())
}
