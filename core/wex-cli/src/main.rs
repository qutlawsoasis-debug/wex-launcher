use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use clap::{Parser, Subcommand};
use wex_core::{
    config::{LauncherConfig, WexPaths},
    event::ConsoleEventHandler,
    launcher::{install_full_environment, prepare_and_launch},
    pack::sync_aura_pack,
    reqwest,
};

#[derive(Parser)]
#[command(name = "wex-cli")]
#[command(about = "Command line interface for Wex launcher", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Custom data directory (default: %APPDATA%\Wex)
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Synchronize Aura-Pack files for the specified version
    Sync {
        /// Minecraft version (1.20.1 or 1.21.1)
        version: String,
    },
    /// Install Minecraft, Fabric, Java, and Aura-Pack
    Install {
        /// Minecraft version (1.20.1 or 1.21.1)
        version: String,
    },
    /// Launch the Minecraft instance
    Launch {
        /// Minecraft version (1.20.1 or 1.21.1)
        version: String,

        /// Player nickname (default: from config or "Player")
        #[arg(long)]
        nick: Option<String>,

        /// Maximum RAM in megabytes (e.g. 4096)
        #[arg(long)]
        xmx: Option<u32>,

        /// Extra JVM arguments
        #[arg(long = "jvm-arg")]
        jvm_args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = WexPaths::new(cli.data_dir);
    paths.ensure_dirs()?;

    let config_path = paths.config_file.clone();
    let mut config = LauncherConfig::load_or_default(&config_path);

    let client = reqwest::Client::builder()
        .user_agent("WexLauncher/0.1.0")
        .build()?;
    let events = Arc::new(ConsoleEventHandler);

    match cli.command {
        Commands::Sync { version } => {
            println!("Synchronizing Aura-Pack for Minecraft {}...", version);
            let res = sync_aura_pack(&paths, &version, &client, events.as_ref(), 8).await?;
            println!(
                "Sync complete! Pack version: {}, updated files: {}, deleted mods: {}",
                res.pack_version, res.files_downloaded, res.mods_deleted
            );
        }
        Commands::Install { version } => {
            println!("Installing full environment for Minecraft {}...", version);
            let _ = install_full_environment(&paths, &version, &client, events.as_ref(), 8).await?;
            println!("Installation of Minecraft {} environment finished successfully!", version);
        }
        Commands::Launch { version, nick, xmx, jvm_args } => {
            if let Some(n) = nick {
                config.nickname = n;
            }
            if let Some(m) = xmx {
                config.max_memory_mb = m;
            }
            config.selected_version = version.clone();
            let _ = config.save(&config_path);

            println!(
                "Preparing and launching Minecraft {} (user: '{}', max memory: {} MB)...",
                version, config.nickname, config.max_memory_mb
            );

            let mut process = prepare_and_launch(
                &paths,
                &config,
                &version,
                &client,
                events,
                8,
                &jvm_args,
            )
            .await?;

            println!("Game launched with PID {}. Waiting for game exit...", process.pid);
            let exit_code = process.wait().await?;
            println!("Game process exited with code {:?}", exit_code);
        }
    }

    Ok(())
}
