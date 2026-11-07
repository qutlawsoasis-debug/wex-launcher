use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = manifest_dir.join("../../ui/src/lib/ipc/generated");
    println!("Generating TypeScript definitions in: {}", target.display());
    wex_ipc::write_bindings_to_dir(&target)?;
    println!("TypeScript IPC bindings successfully generated!");
    Ok(())
}
