use crate::auth::offline_uuid;
use crate::config::{LauncherConfig, WexPaths};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use crate::fabric::{fetch_fabric_profile, install_fabric_libraries, resolve_fabric_libraries};
use crate::java::ensure_mojang_java;
use crate::minecraft::{
    fetch_version_package, install_assets, install_minecraft_client, install_vanilla_libraries,
    resolve_vanilla_libraries,
};
use crate::pack::sync_aura_pack;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

pub struct LaunchOptions<'a> {
    pub paths: &'a WexPaths,
    pub config: &'a LauncherConfig,
    pub version: &'a str,
    pub java_binary: &'a Path,
    pub client_jar: &'a Path,
    pub vanilla_libraries: &'a [PathBuf],
    pub fabric_libraries: &'a [PathBuf],
    pub asset_index_id: &'a str,
    pub extra_jvm_args: &'a [String],
    pub quick_play_multiplayer: Option<&'a str>,
    pub prewarm_signal: Option<&'a Path>,
    pub prewarm_parent_pid: Option<u32>,
}

#[cfg(windows)]
pub struct WindowsJobObject {
    job_handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for WindowsJobObject {}
#[cfg(windows)]
unsafe impl Sync for WindowsJobObject {}

#[cfg(windows)]
impl WindowsJobObject {
    pub fn new() -> Result<Self> {
        use std::mem::size_of;
        use std::ptr;
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let job = CreateJobObjectW(ptr::null(), ptr::null());
            if job.is_null() {
                return Err(WexError::LaunchError("CreateJobObjectW failed".to_string()));
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let success = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if success == 0 {
                windows_sys::Win32::Foundation::CloseHandle(job);
                return Err(WexError::LaunchError(
                    "SetInformationJobObject failed".to_string(),
                ));
            }

            Ok(Self { job_handle: job })
        }
    }

    /// # Safety
    /// `process_handle` must be a valid process handle.
    pub unsafe fn assign_process(
        &self,
        process_handle: windows_sys::Win32::Foundation::HANDLE,
    ) -> Result<()> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

        let res = AssignProcessToJobObject(self.job_handle, process_handle);
        if res == 0 {
            return Err(WexError::LaunchError(
                "AssignProcessToJobObject failed".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for WindowsJobObject {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            if !self.job_handle.is_null() {
                CloseHandle(self.job_handle);
            }
        }
    }
}

pub struct GameProcess {
    pub pid: u32,
    pub log_path: PathBuf,
    pub child: tokio::process::Child,
    #[cfg(windows)]
    pub _job_object: Option<WindowsJobObject>,
}

unsafe impl Send for GameProcess {}
unsafe impl Sync for GameProcess {}

impl GameProcess {
    pub async fn wait(&mut self) -> Result<Option<i32>> {
        let status = self.child.wait().await?;
        Ok(status.code())
    }

    pub fn kill(&mut self) -> Result<()> {
        let _ = self.child.start_kill();
        #[cfg(windows)]
        {
            if self.pid > 0 {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &self.pid.to_string(), "/T"])
                    .output();
            }
        }
        Ok(())
    }
}

fn get_artifact_base_name(file_name: &str) -> String {
    let name = file_name.strip_suffix(".jar").unwrap_or(file_name);
    let parts: Vec<&str> = name.split('-').collect();
    let mut base_parts = Vec::new();
    for p in parts {
        if p.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            break;
        }
        base_parts.push(p);
    }
    if base_parts.is_empty() {
        name.to_lowercase()
    } else {
        base_parts.join("-").to_lowercase()
    }
}

pub fn build_classpath(
    client_jar: &Path,
    vanilla_libs: &[PathBuf],
    fabric_libs: &[PathBuf],
) -> String {
    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let mut entries = Vec::new();
    let mut fabric_artifacts = HashSet::new();

    // Fabric libraries first (priority over older vanilla dependencies)
    for lib in fabric_libs {
        if lib.exists() {
            if let Some(file_name) = lib.file_name().and_then(|n| n.to_str()) {
                fabric_artifacts.insert(get_artifact_base_name(file_name));
            }
            entries.push(lib.display().to_string());
        }
    }

    // Vanilla libraries (skip ASM or other conflicting libraries already supplied by Fabric)
    for lib in vanilla_libs {
        if lib.exists() {
            if let Some(file_name) = lib.file_name().and_then(|n| n.to_str()) {
                let base = get_artifact_base_name(file_name);
                if fabric_artifacts.contains(&base) {
                    continue;
                }
            }
            entries.push(lib.display().to_string());
        }
    }

    // Minecraft client jar
    if client_jar.exists() {
        entries.push(client_jar.display().to_string());
    }

    entries.join(separator)
}

pub fn format_arg_for_file(arg: &str) -> String {
    if arg.contains(' ') || arg.contains('\t') || arg.contains('"') {
        let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        arg.to_string()
    }
}

pub fn build_launch_args(opts: &LaunchOptions) -> Vec<String> {
    let mut args = Vec::new();

    // Memory
    args.push(format!("-Xmx{}M", opts.config.max_memory_mb));

    // Standard Aura / Fabric JVM arguments
    args.push("-XX:+UnlockExperimentalVMOptions".to_string());
    args.push("-DFabricMcEmu= net.minecraft.client.main.Main ".to_string());

    let natives_dir = opts.paths.instance_natives_dir(opts.version);
    args.push(format!("-Djava.library.path={}", natives_dir.display()));
    args.push(format!("-Djna.tmpdir={}", natives_dir.display()));

    if cfg!(target_os = "windows") {
        args.push("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".to_string());
    }

    // Config JVM arguments
    for arg in &opts.config.jvm_args {
        args.push(arg.clone());
    }
    for arg in opts.extra_jvm_args {
        args.push(arg.clone());
    }

    if let Some(signal) = opts.prewarm_signal {
        args.push(format!("-Dwex.prewarm.signal={}", signal.display()));
        if let Some(parent_pid) = opts.prewarm_parent_pid {
            args.push(format!("-Dwex.prewarm.parentPid={}", parent_pid));
        }
        if !args.iter().any(|a| a.contains("wex.qa.background")) {
            args.push("-Dwex.qa.background=true".to_string());
        }
        if !args.iter().any(|a| a.contains("aura.client.hiddenWindow")) {
            args.push("-Daura.client.hiddenWindow=true".to_string());
        }
    } else if std::env::var("WEX_QA_BACKGROUND")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
        || opts
            .extra_jvm_args
            .iter()
            .any(|a| a.contains("wex.qa.background"))
        || opts
            .config
            .jvm_args
            .iter()
            .any(|a| a.contains("wex.qa.background"))
    {
        if !args.iter().any(|a| a.contains("wex.qa.background")) {
            args.push("-Dwex.qa.background=true".to_string());
        }
        if !args.iter().any(|a| a.contains("aura.client.hiddenWindow")) {
            args.push("-Daura.client.hiddenWindow=true".to_string());
        }
    }

    // Classpath
    let cp = build_classpath(
        opts.client_jar,
        opts.vanilla_libraries,
        opts.fabric_libraries,
    );
    args.push("-cp".to_string());
    args.push(cp);

    // Main class
    args.push("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string());

    // Game arguments
    let instance_dir = opts.paths.instance_dir(opts.version);
    let player_uuid = offline_uuid(&opts.config.nickname);

    args.push("--username".to_string());
    args.push(opts.config.nickname.clone());
    args.push("--version".to_string());
    args.push(format!("fabric-loader-0.19.5-{}", opts.version));
    args.push("--gameDir".to_string());
    args.push(instance_dir.display().to_string());
    args.push("--assetsDir".to_string());
    args.push(opts.paths.assets_dir.display().to_string());
    args.push("--assetIndex".to_string());
    args.push(opts.asset_index_id.to_string());
    args.push("--uuid".to_string());
    args.push(player_uuid.to_string());
    args.push("--accessToken".to_string());
    args.push("0".to_string());
    args.push("--userType".to_string());
    args.push("legacy".to_string()); // Minor 1: legacy for offline account
    args.push("--versionType".to_string());
    args.push("release".to_string());

    // Minor 3: QuickPlay Multiplayer if specified
    if let Some(server) = opts.quick_play_multiplayer {
        args.push("--quickPlayMultiplayer".to_string());
        args.push(server.to_string());
    }

    args
}

pub fn create_args_file(instance_dir: &Path, args: &[String]) -> Result<PathBuf> {
    let args_file = instance_dir.join("argsFile.txt");
    let mut content = String::new();
    for arg in args {
        content.push_str(&format_arg_for_file(arg));
        content.push('\n');
    }
    fs::write(&args_file, content)?;
    Ok(args_file)
}

pub fn build_launch_command(opts: &LaunchOptions) -> Result<Command> {
    let instance_dir = opts.paths.instance_dir(opts.version);
    let args = build_launch_args(opts);
    let args_file = create_args_file(&instance_dir, &args)?;

    let mut cmd = Command::new(opts.java_binary);
    // Use @argsFile to avoid the 32767 character command-line limit on Windows
    cmd.arg(format!("@{}", args_file.display()));

    cmd.current_dir(&instance_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    Ok(cmd)
}

pub async fn launch_game_process(
    opts: LaunchOptions<'_>,
    events: Arc<dyn EventHandler>,
) -> Result<GameProcess> {
    let instance_dir = opts.paths.instance_dir(opts.version);
    let logs_dir = instance_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;

    let log_path = logs_dir.join("latest.log");
    let log_file = File::create(&log_path)?;

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Launching Minecraft {} as '{}'...",
            opts.version, opts.config.nickname
        ),
    });

    let mut command = build_launch_command(&opts)?;
    let mut child = command.spawn().map_err(|e| {
        WexError::LaunchError(format!(
            "Failed to spawn Java process {}: {}",
            opts.java_binary.display(),
            e
        ))
    })?;

    let pid = child.id().unwrap_or(0);
    events.on_event(&Event::GameLaunched { pid });

    #[cfg(windows)]
    let job_object = match WindowsJobObject::new() {
        Ok(job) => {
            if let Some(raw_handle) = child.raw_handle() {
                unsafe {
                    let _ = job.assign_process(raw_handle as _);
                }
            }
            Some(job)
        }
        Err(e) => {
            tracing::warn!("Failed to create Job Object: {}", e);
            None
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let shared_log_file = Arc::new(std::sync::Mutex::new(log_file));

    // Handle stdout
    if let Some(stdout) = stdout {
        let events_clone = events.clone();
        let log_file_clone = shared_log_file.clone();
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(mut f) = log_file_clone.lock() {
                    let _ = writeln!(f, "{}", line);
                }
                events_clone.on_event(&Event::GameOutput { line });
            }
        });
    }

    // Handle stderr
    if let Some(stderr) = stderr {
        let events_clone = events.clone();
        let log_file_clone = shared_log_file;
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(mut f) = log_file_clone.lock() {
                    let _ = writeln!(f, "[STDERR] {}", line);
                }
                events_clone.on_event(&Event::GameOutput { line });
            }
        });
    }

    Ok(GameProcess {
        pid,
        log_path,
        child,
        #[cfg(windows)]
        _job_object: job_object,
    })
}

pub async fn install_full_environment(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<(PathBuf, PathBuf, Vec<PathBuf>, Vec<PathBuf>, String)> {
    paths.ensure_dirs()?;
    paths.ensure_instance_dirs(version)?;

    let ver_cache_dir = paths.versions_dir.join(version);
    let _ = fs::create_dir_all(&ver_cache_dir);
    let pkg_cache_file = ver_cache_dir.join(format!("{}.json", version));
    let fabric_cache_file = ver_cache_dir.join(format!("fabric-{}.json", version));

    // 1. Minecraft Piston package (with offline fallback cache)
    events.on_event(&Event::StageStarted {
        stage: format!("Minecraft {} Environment", version),
        step: 1,
        total_steps: 4,
    });
    let pkg = match fetch_version_package(client, version).await {
        Ok(p) => {
            if let Ok(json) = serde_json::to_string_pretty(&p) {
                let _ = fs::write(&pkg_cache_file, json);
            }
            p
        }
        Err(e) => {
            if pkg_cache_file.exists() {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                        "Network unavailable ({}); using cached version package for Minecraft {}",
                        e, version
                    ),
                });
                let content = fs::read_to_string(&pkg_cache_file)?;
                serde_json::from_str(&content)?
            } else {
                return Err(e);
            }
        }
    };

    let client_jar = match install_minecraft_client(paths, version, &pkg, client, events).await {
        Ok(jar) => jar,
        Err(e) => {
            let local_jar = paths.client_jar(version);
            if local_jar.exists() {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                        "Network error downloading client.jar, using cached {}",
                        local_jar.display()
                    ),
                });
                local_jar
            } else {
                return Err(e);
            }
        }
    };

    let vanilla_libs =
        match install_vanilla_libraries(paths, version, &pkg, client, events, concurrency).await {
            Ok(libs) => libs,
            Err(e) => {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                    "Network error downloading vanilla libraries ({}), resolving cached libraries",
                    e
                ),
                });
                resolve_vanilla_libraries(paths, &pkg)
            }
        };

    let _ = install_assets(paths, &pkg, client, events, concurrency).await;

    // 2. Fabric Profile & Libraries (with offline fallback cache)
    events.on_event(&Event::StageStarted {
        stage: format!("Fabric Loader 0.19.5 ({})", version),
        step: 2,
        total_steps: 4,
    });
    let fabric_profile = match fetch_fabric_profile(client, version, None).await {
        Ok(prof) => {
            if let Ok(json) = serde_json::to_string_pretty(&prof) {
                let _ = fs::write(&fabric_cache_file, json);
            }
            prof
        }
        Err(e) => {
            if fabric_cache_file.exists() {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                        "Network unavailable ({}); using cached Fabric profile for Minecraft {}",
                        e, version
                    ),
                });
                let content = fs::read_to_string(&fabric_cache_file)?;
                serde_json::from_str(&content)?
            } else {
                return Err(e);
            }
        }
    };

    let fabric_libs =
        match install_fabric_libraries(paths, &fabric_profile, client, events, concurrency).await {
            Ok(libs) => libs,
            Err(e) => {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                    "Network error downloading Fabric libraries ({}), resolving cached libraries",
                    e
                ),
                });
                resolve_fabric_libraries(paths, &fabric_profile)
            }
        };

    // 3. Java Runtime
    events.on_event(&Event::StageStarted {
        stage: format!("Mojang Java Runtime ({})", version),
        step: 3,
        total_steps: 4,
    });
    let java_binary = ensure_mojang_java(paths, version, client, events, concurrency).await?;

    // 4. Aura-Pack sync
    events.on_event(&Event::StageStarted {
        stage: format!("Aura-Pack ({})", version),
        step: 4,
        total_steps: 4,
    });
    let state_file = paths.instance_dir(version).join("pack.state.json");
    if let Err(e) = sync_aura_pack(paths, version, client, events, concurrency).await {
        if matches!(e, WexError::Cancelled) {
            return Err(e);
        }
        if state_file.exists() {
            events.on_event(&Event::Log {
                level: LogLevel::Warn,
                message: format!("Aura-Pack sync failed ({}), but local pack state exists. Continuing offline launch.", e),
            });
        } else {
            return Err(e);
        }
    }

    let asset_index_id = pkg.asset_index.id;
    let _ = mark_fast_path_verified(paths, version);
    Ok((
        java_binary,
        client_jar,
        vanilla_libs,
        fabric_libs,
        asset_index_id,
    ))
}

/// Prewarms the Java Virtual Machine by executing a minimal JVM probe.
/// This warms up the OS disk cache for JVM binaries (`jvm.dll`, runtime libraries)
/// and verifies JVM responsiveness before full client execution.
pub async fn prewarm_jvm(java_binary: &Path) -> Result<std::time::Duration> {
    if !java_binary.exists() {
        return Err(WexError::LaunchError(format!(
            "Java binary not found for prewarm: {:?}",
            java_binary
        )));
    }

    let start = std::time::Instant::now();
    let mut cmd = tokio::process::Command::new(java_binary);
    cmd.args(["-Xms16m", "-Xmx32m", "-version"]);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| {
        WexError::LaunchError(format!("Не удалось запустить процесс прогрева JVM: {}", e))
    })?;

    let status = child.wait().await.map_err(|e| {
        WexError::LaunchError(format!("Ошибка ожидания процесса прогрева JVM: {}", e))
    })?;

    if !status.success() {
        return Err(WexError::LaunchError(format!(
            "Процесс прогрева JVM завершился с кодом {:?}",
            status.code()
        )));
    }

    Ok(start.elapsed())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FastPathState {
    pub version: String,
    pub verified: bool,
    pub client_jar_size: u64,
    pub timestamp_utc: i64,
}

pub fn fast_path_state_path(paths: &WexPaths, version: &str) -> PathBuf {
    paths.instance_dir(version).join("fast_launch.state.json")
}

pub fn mark_fast_path_verified(paths: &WexPaths, version: &str) -> Result<()> {
    let client_jar = paths.client_jar(version);
    let size = if let Ok(meta) = fs::metadata(&client_jar) {
        meta.len()
    } else {
        0
    };
    let state = FastPathState {
        version: version.to_string(),
        verified: true,
        client_jar_size: size,
        timestamp_utc: chrono::Utc::now().timestamp(),
    };
    let inst_dir = paths.instance_dir(version);
    let _ = fs::create_dir_all(&inst_dir);
    if let Ok(content) = serde_json::to_string_pretty(&state) {
        let _ = fs::write(fast_path_state_path(paths, version), content);
    }
    Ok(())
}

pub fn is_fast_path_ready(paths: &WexPaths, version: &str) -> bool {
    let client_jar = paths.client_jar(version);
    if !client_jar.exists() {
        return false;
    }

    let state_file = fast_path_state_path(paths, version);
    if !state_file.exists() {
        return false;
    }

    let pkg_cache_file = paths
        .versions_dir
        .join(version)
        .join(format!("{}.json", version));
    if !pkg_cache_file.exists() {
        return false;
    }

    let fabric_cache_file = paths
        .versions_dir
        .join(version)
        .join(format!("fabric-{}.json", version));
    if !fabric_cache_file.exists() {
        return false;
    }

    let pack_state_file = paths.instance_dir(version).join("pack.state.json");
    if !pack_state_file.exists() {
        return false;
    }

    // Check Mojang Java binary exists
    let java_dir = if version == "1.20.1" {
        "java-runtime-gamma"
    } else {
        "java-runtime-delta"
    };
    let mojang_java = paths
        .runtime_dir
        .join(java_dir)
        .join(if cfg!(target_os = "windows") {
            "bin/javaw.exe"
        } else {
            "bin/java"
        });
    if !mojang_java.exists() {
        return false;
    }

    true
}

pub type FastPathEnvironment = (PathBuf, PathBuf, Vec<PathBuf>, Vec<PathBuf>, String);

#[allow(clippy::type_complexity)]
pub fn resolve_fast_path_environment(
    paths: &WexPaths,
    version: &str,
) -> Result<FastPathEnvironment> {
    let pkg_cache_file = paths
        .versions_dir
        .join(version)
        .join(format!("{}.json", version));
    let content = fs::read_to_string(&pkg_cache_file)?;
    let pkg: crate::minecraft::VersionPackage = serde_json::from_str(&content)?;

    let fabric_cache_file = paths
        .versions_dir
        .join(version)
        .join(format!("fabric-{}.json", version));
    let fcontent = fs::read_to_string(&fabric_cache_file)?;
    let fabric_profile: crate::fabric::FabricProfile = serde_json::from_str(&fcontent)?;

    let vanilla_libs = resolve_vanilla_libraries(paths, &pkg);
    let fabric_libs = resolve_fabric_libraries(paths, &fabric_profile);

    let client_jar = paths.client_jar(version);
    let java_dir = if version == "1.20.1" {
        "java-runtime-gamma"
    } else {
        "java-runtime-delta"
    };
    let mojang_java = paths
        .runtime_dir
        .join(java_dir)
        .join(if cfg!(target_os = "windows") {
            "bin/javaw.exe"
        } else {
            "bin/java"
        });

    let asset_index_id = pkg.asset_index.id;

    Ok((
        mojang_java,
        client_jar,
        vanilla_libs,
        fabric_libs,
        asset_index_id,
    ))
}

pub async fn prepare_and_launch(
    paths: &WexPaths,
    config: &LauncherConfig,
    version: &str,
    client: &reqwest::Client,
    events: Arc<dyn EventHandler>,
    concurrency: usize,
    extra_jvm_args: &[String],
) -> Result<GameProcess> {
    prepare_and_launch_with_options(
        paths,
        config,
        version,
        client,
        events,
        concurrency,
        extra_jvm_args,
        None,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_and_prewarm(
    paths: &WexPaths,
    config: &LauncherConfig,
    version: &str,
    client: &reqwest::Client,
    events: Arc<dyn EventHandler>,
    concurrency: usize,
    extra_jvm_args: &[String],
    signal_file: &Path,
    parent_pid: u32,
) -> Result<GameProcess> {
    prepare_and_launch_with_options(
        paths,
        config,
        version,
        client,
        events,
        concurrency,
        extra_jvm_args,
        Some(signal_file),
        Some(parent_pid),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_and_launch_with_options(
    paths: &WexPaths,
    config: &LauncherConfig,
    version: &str,
    client: &reqwest::Client,
    events: Arc<dyn EventHandler>,
    concurrency: usize,
    extra_jvm_args: &[String],
    prewarm_signal: Option<&Path>,
    prewarm_parent_pid: Option<u32>,
) -> Result<GameProcess> {
    let (mojang_java, client_jar, vanilla_libs, fabric_libs, asset_index_id) = if is_fast_path_ready(
        paths, version,
    ) {
        events.on_event(&Event::Log {
                level: LogLevel::Info,
                message: format!(
                    "[FAST-LAUNCH] Использован быстрый путь запуска для Minecraft {}: проверка неизменённых файлов пропущена",
                    version
                ),
            });
        match resolve_fast_path_environment(paths, version) {
            Ok(env) => env,
            Err(e) => {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                        "[FAST-LAUNCH] Ошибка быстрого пути ({}), возврат к полной проверке",
                        e
                    ),
                });
                install_full_environment(paths, version, client, events.as_ref(), concurrency)
                    .await?
            }
        }
    } else {
        install_full_environment(paths, version, client, events.as_ref(), concurrency).await?
    };

    let java_binary = if let Some(custom) = &config.custom_java_path {
        if custom.exists() {
            custom.clone()
        } else {
            mojang_java
        }
    } else {
        mojang_java
    };

    if config.fast_start_jvm {
        events.on_event(&Event::Log {
            level: LogLevel::Info,
            message: "[PREWARM] Быстрый запуск активен: предварительный прогрев JVM и верификация runtime среды".to_string(),
        });
        match prewarm_jvm(&java_binary).await {
            Ok(duration) => {
                events.on_event(&Event::Log {
                    level: LogLevel::Info,
                    message: format!(
                        "[PREWARM] JVM успешно прогрета за {} мс",
                        duration.as_millis()
                    ),
                });
            }
            Err(e) => {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!("[PREWARM] Предупреждение прогрева JVM: {}", e),
                });
            }
        }
    }

    let launch_opts = LaunchOptions {
        paths,
        config,
        version,
        java_binary: &java_binary,
        client_jar: &client_jar,
        vanilla_libraries: &vanilla_libs,
        fabric_libraries: &fabric_libs,
        asset_index_id: &asset_index_id,
        extra_jvm_args,
        quick_play_multiplayer: config.quick_play_multiplayer.as_deref(),
        prewarm_signal,
        prewarm_parent_pid,
    };

    launch_game_process(launch_opts, events).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_classpath_builder() {
        let tmp = tempdir().unwrap();
        let client_jar = tmp.path().join("client.jar");
        let lib1 = tmp.path().join("lib1.jar");
        let lib2 = tmp.path().join("lib2.jar");

        File::create(&client_jar).unwrap();
        File::create(&lib1).unwrap();
        File::create(&lib2).unwrap();

        let cp = build_classpath(
            &client_jar,
            std::slice::from_ref(&lib1),
            std::slice::from_ref(&lib2),
        );
        assert!(cp.contains("lib2.jar"));
        assert!(cp.contains("lib1.jar"));
        assert!(cp.contains("client.jar"));
    }

    #[test]
    fn test_args_file_generation_and_formatting() {
        let tmp = tempdir().unwrap();
        let instance_dir = tmp.path().join("instances").join("1.20.1");
        fs::create_dir_all(&instance_dir).unwrap();

        let raw_args = vec![
            "-Xmx4096M".to_string(),
            "-Djava.library.path=C:\\Wex\\natives".to_string(),
            "-cp".to_string(),
            "C:\\Program Files\\Wex\\lib 1.jar;C:\\Program Files\\Wex\\lib 2.jar".to_string(),
            "--username".to_string(),
            "Player One".to_string(),
        ];

        let args_file = create_args_file(&instance_dir, &raw_args).unwrap();
        assert!(args_file.exists());
        let content = fs::read_to_string(&args_file).unwrap();
        // Check that paths with spaces are properly quoted and escaped
        assert!(content.contains(
            "\"C:\\\\Program Files\\\\Wex\\\\lib 1.jar;C:\\\\Program Files\\\\Wex\\\\lib 2.jar\""
        ));
        assert!(content.contains("\"Player One\""));
        assert!(content.contains("-Xmx4096M"));
    }

    #[test]
    fn test_classpath_deduplication_asm_conflict() {
        let tmp = tempdir().unwrap();
        let client_jar = tmp.path().join("client.jar");
        let fabric_asm = tmp.path().join("asm-9.7.jar");
        let fabric_asm_tree = tmp.path().join("asm-tree-9.7.jar");
        let vanilla_asm = tmp.path().join("asm-9.3.jar");
        let vanilla_asm_tree = tmp.path().join("asm-tree-9.3.jar");
        let vanilla_guava = tmp.path().join("guava-32.1.2-jre.jar");

        File::create(&client_jar).unwrap();
        File::create(&fabric_asm).unwrap();
        File::create(&fabric_asm_tree).unwrap();
        File::create(&vanilla_asm).unwrap();
        File::create(&vanilla_asm_tree).unwrap();
        File::create(&vanilla_guava).unwrap();

        let fabric_libs = vec![fabric_asm.clone(), fabric_asm_tree.clone()];
        let vanilla_libs = vec![
            vanilla_asm.clone(),
            vanilla_asm_tree.clone(),
            vanilla_guava.clone(),
        ];

        let cp = build_classpath(&client_jar, &vanilla_libs, &fabric_libs);
        // Fabric ASM 9.7 must be present
        assert!(cp.contains("asm-9.7.jar"));
        assert!(cp.contains("asm-tree-9.7.jar"));
        // Vanilla ASM 9.3 must be eliminated to prevent JVM bytecode mismatch
        assert!(!cp.contains("asm-9.3.jar"));
        assert!(!cp.contains("asm-tree-9.3.jar"));
        // Non-conflicting vanilla library must remain
        assert!(cp.contains("guava-32.1.2-jre.jar"));
    }

    #[test]
    fn test_offline_fallback_resolution() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();

        let version = "1.20.1";
        let ver_dir = paths.versions_dir.join(version);
        fs::create_dir_all(&ver_dir).unwrap();

        // Create cached version package
        let pkg_cache_file = ver_dir.join(format!("{}.json", version));
        let mock_pkg_json = r#"{
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": "test_sha1",
                    "size": 1234,
                    "url": "https://piston-data.mojang.com/v1/objects/test/client.jar"
                }
            },
            "libraries": [
                {
                    "name": "com.mojang:logging:1.1.1",
                    "downloads": {
                        "artifact": {
                            "path": "com/mojang/logging/1.1.1/logging-1.1.1.jar",
                            "sha1": "abc",
                            "size": 500,
                            "url": "https://libraries.minecraft.net/com/mojang/logging/1.1.1/logging-1.1.1.jar"
                        }
                    }
                }
            ],
            "assetIndex": {
                "id": "5",
                "sha1": "idx_sha1",
                "size": 100,
                "url": "https://piston-meta.mojang.com/v1/packages/idx.json",
                "totalSize": 200
            }
        }"#;
        fs::write(&pkg_cache_file, mock_pkg_json).unwrap();

        // Create cached Fabric profile
        let fabric_cache_file = ver_dir.join(format!("fabric-{}.json", version));
        let mock_fabric_json = r#"{
            "id": "fabric-loader-0.19.5-1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                {
                    "name": "net.fabricmc:sponge-mixin:0.15.3+mixin.0.8.7"
                }
            ]
        }"#;
        fs::write(&fabric_cache_file, mock_fabric_json).unwrap();

        // Verify offline reading of cached JSONs without network
        let pkg: crate::minecraft::VersionPackage =
            serde_json::from_str(&fs::read_to_string(&pkg_cache_file).unwrap()).unwrap();
        let fabric_prof: crate::fabric::FabricProfile =
            serde_json::from_str(&fs::read_to_string(&fabric_cache_file).unwrap()).unwrap();

        let v_libs = resolve_vanilla_libraries(&paths, &pkg);
        let f_libs = resolve_fabric_libraries(&paths, &fabric_prof);

        assert_eq!(v_libs.len(), 1);
        assert!(v_libs[0].ends_with("logging-1.1.1.jar"));

        assert_eq!(f_libs.len(), 1);
        assert!(f_libs[0].ends_with("sponge-mixin-0.15.3+mixin.0.8.7.jar"));
    }

    #[test]
    fn test_wex_qa_background_arg_injection() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let config = LauncherConfig::default();

        let java_bin = PathBuf::from("java.exe");
        let client_jar = PathBuf::from("client.jar");
        let vanilla_libs = Vec::new();
        let fabric_libs = Vec::new();

        std::env::set_var("WEX_QA_BACKGROUND", "true");
        let launch_opts = LaunchOptions {
            paths: &paths,
            config: &config,
            version: "1.20.1",
            java_binary: &java_bin,
            client_jar: &client_jar,
            vanilla_libraries: &vanilla_libs,
            fabric_libraries: &fabric_libs,
            asset_index_id: "5",
            extra_jvm_args: &[],
            quick_play_multiplayer: None,
            prewarm_signal: None,
            prewarm_parent_pid: None,
        };

        let args = build_launch_args(&launch_opts);
        std::env::remove_var("WEX_QA_BACKGROUND");

        assert!(args.iter().any(|a| a == "-Dwex.qa.background=true"));
        assert!(args.iter().any(|a| a == "-Daura.client.hiddenWindow=true"));
    }

    #[test]
    fn test_quick_play_multiplayer_argument_injection() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let config = LauncherConfig::default();

        let java_bin = PathBuf::from("java.exe");
        let client_jar = PathBuf::from("client.jar");
        let vanilla_libs = Vec::new();
        let fabric_libs = Vec::new();

        let launch_opts = LaunchOptions {
            paths: &paths,
            config: &config,
            version: "1.21.1",
            java_binary: &java_bin,
            client_jar: &client_jar,
            vanilla_libraries: &vanilla_libs,
            fabric_libraries: &fabric_libs,
            asset_index_id: "6",
            extra_jvm_args: &[],
            quick_play_multiplayer: Some("192.168.1.42:54321"),
            prewarm_signal: None,
            prewarm_parent_pid: None,
        };

        let args = build_launch_args(&launch_opts);
        let qp_pos = args.iter().position(|a| a == "--quickPlayMultiplayer");
        assert!(
            qp_pos.is_some(),
            "--quickPlayMultiplayer must be present in launch args"
        );
        assert_eq!(args[qp_pos.unwrap() + 1], "192.168.1.42:54321");
    }

    #[tokio::test]
    async fn test_install_full_environment_offline_success() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        let version = "1.20.1";
        paths.ensure_instance_dirs(version).unwrap();

        let ver_dir = paths.versions_dir.join(version);
        fs::create_dir_all(&ver_dir).unwrap();

        // 1. Cached version package
        let pkg_cache_file = ver_dir.join(format!("{}.json", version));
        let mock_pkg_json = r#"{
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": "test_sha1",
                    "size": 100,
                    "url": "https://piston-data.mojang.com/v1/objects/test/client.jar"
                }
            },
            "libraries": [],
            "assetIndex": {
                "id": "5",
                "sha1": "idx_sha1",
                "size": 100,
                "url": "https://piston-meta.mojang.com/v1/packages/idx.json",
                "totalSize": 200
            }
        }"#;
        fs::write(&pkg_cache_file, mock_pkg_json).unwrap();

        // 2. Installed client.jar
        let client_jar = paths.client_jar(version);
        fs::create_dir_all(client_jar.parent().unwrap()).unwrap();
        fs::write(&client_jar, b"mock client jar binary").unwrap();

        // 3. Cached Fabric profile
        let fabric_cache_file = ver_dir.join(format!("fabric-{}.json", version));
        let mock_fabric_json = r#"{
            "id": "fabric-loader-0.19.5-1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": []
        }"#;
        fs::write(&fabric_cache_file, mock_fabric_json).unwrap();

        // 4. Installed Mojang Java Runtime
        let comp = crate::java::get_java_component_for_version(version);
        let comp_dir = paths.runtime_dir.join(comp);
        let bin_dir = comp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(comp_dir.join("release"), "JAVA_VERSION=17\n").unwrap();
        let javaw = bin_dir.join("javaw.exe");
        fs::write(&javaw, b"dummy java").unwrap();

        // 5. Installed pack state
        let state_file = paths.instance_dir(version).join("pack.state.json");
        let pack_state = crate::pack::PackState {
            pack_version: "20261108.offline".to_string(),
            minecraft: version.to_string(),
            fabric_loader: "0.19.5".to_string(),
            installed_at_utc: chrono::Utc::now().to_rfc3339(),
            installed_mods: vec![],
        };
        fs::write(&state_file, serde_json::to_string(&pack_state).unwrap()).unwrap();

        // Unreachable client (offline)
        let offline_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();
        let events = crate::event::NoopEventHandler;

        let res = install_full_environment(&paths, version, &offline_client, &events, 2).await;
        assert!(
            res.is_ok(),
            "Offline environment installation must succeed when cached/installed: {:?}",
            res
        );
        let (java_bin, jar, _, _, asset_id) = res.unwrap();
        assert_eq!(jar, client_jar);
        assert_eq!(asset_id, "5");
        assert!(java_bin.exists());
    }

    #[tokio::test]
    async fn test_prewarm_jvm_nonexistent() {
        let fake_java = PathBuf::from("nonexistent_java_path_12345.exe");
        let res = prewarm_jvm(&fake_java).await;
        assert!(
            res.is_err(),
            "Must report error for nonexistent java binary"
        );
    }

    #[test]
    fn test_launcher_config_fast_start_jvm_toggle() {
        let mut cfg = LauncherConfig::default();
        assert!(
            !cfg.fast_start_jvm,
            "fast_start_jvm must be disabled by default"
        );
        cfg.fast_start_jvm = true;
        assert!(cfg.fast_start_jvm);
    }

    #[test]
    fn test_prewarm_arg_injection() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let config = LauncherConfig::default();

        let java_bin = PathBuf::from("java.exe");
        let client_jar = PathBuf::from("client.jar");
        let vanilla_libs = Vec::new();
        let fabric_libs = Vec::new();
        let signal_path = tmp.path().join("prewarm.signal");

        let launch_opts = LaunchOptions {
            paths: &paths,
            config: &config,
            version: "1.21.1",
            java_binary: &java_bin,
            client_jar: &client_jar,
            vanilla_libraries: &vanilla_libs,
            fabric_libraries: &fabric_libs,
            asset_index_id: "6",
            extra_jvm_args: &[],
            quick_play_multiplayer: None,
            prewarm_signal: Some(&signal_path),
            prewarm_parent_pid: Some(99999),
        };

        let args = build_launch_args(&launch_opts);
        assert!(args
            .iter()
            .any(|a| a.starts_with("-Dwex.prewarm.signal=") && a.contains("prewarm.signal")));
        assert!(args.iter().any(|a| a == "-Dwex.prewarm.parentPid=99999"));
        assert!(args.iter().any(|a| a == "-Dwex.qa.background=true"));
        assert!(args.iter().any(|a| a == "-Daura.client.hiddenWindow=true"));
    }

    #[test]
    fn test_fast_path_state_lifecycle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let version = "1.21.1";

        // Initially not ready
        assert!(!is_fast_path_ready(&paths, version));

        // Create mock files
        let client_jar = paths.client_jar(version);
        fs::create_dir_all(client_jar.parent().unwrap()).unwrap();
        fs::write(&client_jar, b"PK0304").unwrap();

        let ver_dir = paths.versions_dir.join(version);
        fs::create_dir_all(&ver_dir).unwrap();
        let pkg_file = ver_dir.join(format!("{}.json", version));
        fs::write(
            &pkg_file,
            r#"{"id":"1.21.1","mainClass":"net.minecraft.client.main.Main","downloads":{"client":{"sha1":"","size":0,"url":""}},"libraries":[],"assetIndex":{"id":"17","sha1":"","size":0,"url":"","totalSize":0}}"#,
        )
        .unwrap();

        let fabric_file = ver_dir.join(format!("fabric-{}.json", version));
        fs::write(
            &fabric_file,
            r#"{"id":"fabric-loader-0.19.5-1.21.1","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[]}"#,
        )
        .unwrap();

        let inst_dir = paths.instance_dir(version);
        fs::create_dir_all(&inst_dir).unwrap();
        fs::write(inst_dir.join("pack.state.json"), b"{}").unwrap();

        let java_dir = paths.runtime_dir.join("java-runtime-delta").join("bin");
        fs::create_dir_all(&java_dir).unwrap();
        let java_bin = java_dir.join(if cfg!(target_os = "windows") {
            "javaw.exe"
        } else {
            "java"
        });
        fs::write(&java_bin, b"stub").unwrap();

        // Mark verified
        mark_fast_path_verified(&paths, version).unwrap();

        // Now fast path must be ready
        assert!(is_fast_path_ready(&paths, version));

        // Resolve fast path
        let (java, jar, v_libs, f_libs, asset_idx) =
            resolve_fast_path_environment(&paths, version).unwrap();
        assert_eq!(jar, client_jar);
        assert_eq!(java, java_bin);
        assert_eq!(asset_idx, "17");
        assert!(v_libs.is_empty());
        assert!(f_libs.is_empty());
    }
}
