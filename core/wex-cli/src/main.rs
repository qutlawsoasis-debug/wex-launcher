use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
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
    /// Import settings, skins, and servers from old launcher directory (.aura)
    Import {
        /// Directory of the old launcher (.aura)
        dir: PathBuf,
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
            println!(
                "Installation of Minecraft {} environment finished successfully!",
                version
            );
        }
        Commands::Launch {
            version,
            nick,
            xmx,
            jvm_args,
        } => {
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

            let mut process =
                prepare_and_launch(&paths, &config, &version, &client, events, 8, &jvm_args)
                    .await?;

            println!(
                "Game launched with PID {}. Waiting for game exit...",
                process.pid
            );
            tokio::select! {
                res = process.wait() => {
                    let exit_code = res?;
                    println!("Game process exited with code {:?}", exit_code);
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nReceived Ctrl+C, terminating game process and child tree (PID {})...", process.pid);
                    let _ = process.kill();
                    println!("Game process cleanly terminated.");
                }
            }
        }
        Commands::Import { dir } => {
            println!("Importing data from old launcher directory: {:?}", dir);
            let old_data = wex_extras::import::import_from_dir(&dir)?;
            println!("Found old profile for nickname: '{}', RAM: {} MB, version: '{}'", old_data.nickname, old_data.ram_mb, old_data.selected_version);

            config.nickname = old_data.nickname.clone();
            config.max_memory_mb = old_data.ram_mb;
            config.selected_version = old_data.selected_version.clone();
            config.save(&config_path)?;
            println!("Configuration updated and saved to {:?}", config_path);

            if !old_data.skins.is_empty() {
                let target_skins = paths.root.join("skins");
                std::fs::create_dir_all(&target_skins)?;
                let mut copied = 0;
                for s in &old_data.skins {
                    let dest = target_skins.join(&s.file_name);
                    if std::fs::copy(&s.path, &dest).is_ok() {
                        copied += 1;
                    }
                }
                println!("Imported {} skin(s) into {:?}", copied, target_skins);
            }

            if !old_data.servers.is_empty() {
                let servers_dat_path = paths.root.join("servers.dat");
                if let Some(parent) = servers_dat_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let pack_entries: Vec<wex_net::PackServerEntry> = old_data
                    .servers
                    .iter()
                    .map(|s| wex_net::PackServerEntry {
                        id: None,
                        name: s.name.clone(),
                        ip: s.ip.clone(),
                        icon: s.icon.clone(),
                    })
                    .collect();

                let mut root = wex_net::load_servers_dat(&servers_dat_path).unwrap_or_else(|_| {
                    wex_net::NbtTag::Compound(vec![(
                        "servers".to_string(),
                        wex_net::NbtTag::List {
                            tag_id: 10,
                            elements: Vec::new(),
                        },
                    )])
                });

                wex_net::merge_pack_servers(&mut root, &pack_entries);
                if wex_net::save_servers_dat(&servers_dat_path, &root).is_ok() {
                    println!("Imported {} server(s) into {:?}", pack_entries.len(), servers_dat_path);
                }
            }

            println!("Old launcher data import finished successfully!");
        }
    }

    Ok(())
}
