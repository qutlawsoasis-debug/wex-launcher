pub mod auth;
pub mod config;
pub mod download;
pub mod error;
pub mod event;
pub mod fabric;
pub mod ipc_adapter;
pub mod java;
pub mod launcher;
pub mod minecraft;
pub mod pack;
pub mod playtime;

pub use auth::offline_uuid;
pub use config::{LauncherConfig, WexPaths};
pub use error::{Result, WexError};
pub use event::{
    ChannelEventHandler, ConsoleEventHandler, Event, EventHandler, LogLevel, NoopEventHandler,
    SharedEventHandler,
};
pub use ipc_adapter::WexLauncherCore;
pub use launcher::{
    build_classpath, build_launch_command, install_full_environment, launch_game_process,
    prepare_and_launch, GameProcess, LaunchOptions,
};
pub use pack::sync_aura_pack;
pub use reqwest;
