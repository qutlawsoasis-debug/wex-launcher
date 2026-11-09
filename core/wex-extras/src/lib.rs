pub mod crash;
pub mod discord;
pub mod import;
pub mod music;
pub mod presets;
pub mod skins;

pub use crash::{analyze_crash, analyze_log_text, CrashDiagnosis, CrashKind};
pub use discord::{Activity, DiscordRpcClient, DEFAULT_DISCORD_APP_ID};
pub use image;
pub use import::{
    import_from_dir, ImportedCape, ImportedServer, ImportedSkin, ImportedWardrobeItem,
    ImportedWorld, OldLauncherData,
};
pub use music::{AudioTrack, MediaSessionState, MusicError, MusicManager, MusicSettings};
pub use presets::{
    apply_preset, get_default_presets, GraphicsPreset, PresetApplyResult, PresetTier,
};
pub use skins::{
    convert_64x32_to_64x64, detect_skin_model, generate_csl_config, validate_skin_bytes, SkinError,
    SkinHistoryManager, SkinModel, SkinValidationResult, WardrobeItem,
};
