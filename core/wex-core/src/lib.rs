pub mod auth;
pub mod config;
pub mod download;
pub mod error;
pub mod event;
pub mod fabric;
pub mod ipc_adapter;
pub mod java;
pub mod jvm_args;
pub mod launcher;
pub mod minecraft;
pub mod pack;
pub mod playtime;

pub use auth::offline_uuid;
pub use config::{LauncherConfig, WexPaths};
pub use download::is_disk_full_error;
pub use error::{Result, WexError};
pub use event::{
    ChannelEventHandler, ConsoleEventHandler, Event, EventHandler, LogLevel, NoopEventHandler,
    SharedEventHandler,
};
pub use ipc_adapter::WexLauncherCore;
#[cfg(windows)]
pub use jvm_args::apply_process_handle_priority;
pub use jvm_args::{
    apply_process_priority, build_optimized_jvm_args, calculate_optimal_ram_mb,
    detect_system_ram_mb, GcProfile, ProcessPriority,
};
pub use launcher::{
    build_classpath, build_launch_command, install_full_environment, launch_game_process,
    prepare_and_launch, prewarm_jvm, GameProcess, LaunchOptions,
};
pub use pack::{sync_aura_pack, sync_aura_pack_cancellable};
pub use reqwest;
