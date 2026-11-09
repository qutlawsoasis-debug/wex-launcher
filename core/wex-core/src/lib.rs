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
pub mod system_info;

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
    apply_current_process_affinity, build_classpath, build_launch_command,
    install_full_environment, is_fast_path_ready, launch_game_process, mark_fast_path_verified,
    prepare_and_launch, prepare_and_launch_with_options, prepare_and_prewarm, prewarm_jvm,
    resolve_fast_path_environment, set_current_process_priority_low,
    set_current_process_priority_normal, GameProcess, LaunchOptions,
};
pub use pack::{sync_aura_pack, sync_aura_pack_cancellable};
pub use reqwest;
pub use system_info::{
    collect_system_info, evaluate_system_warnings, format_support_text, is_integrated_graphics,
    sanitize_user_path, SystemInfo, NOT_DETERMINED,
};
