use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use crate::auth::offline_uuid;
use crate::config::{LauncherConfig, WexPaths};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use crate::fabric::{fetch_fabric_profile, install_fabric_libraries};
use crate::java::ensure_mojang_java;
use crate::minecraft::{
    fetch_version_package, install_assets, install_minecraft_client, install_vanilla_libraries,
};
use crate::pack::sync_aura_pack;

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
}

pub struct GameProcess {
    pub pid: u32,
    pub log_path: PathBuf,
    pub child: tokio::process::Child,
}

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

pub fn build_classpath(client_jar: &Path, vanilla_libs: &[PathBuf], fabric_libs: &[PathBuf]) -> String {
    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
    let mut entries = Vec::new();

    // Fabric libraries first
    for lib in fabric_libs {
        if lib.exists() {
            entries.push(lib.display().to_string());
        }
    }

    // Vanilla libraries
    for lib in vanilla_libs {
        if lib.exists() {
            entries.push(lib.display().to_string());
        }
    }

    // Minecraft client jar
    if client_jar.exists() {
        entries.push(client_jar.display().to_string());
    }

    entries.join(separator)
}

pub fn build_launch_command(opts: &LaunchOptions) -> Command {
    let mut cmd = Command::new(opts.java_binary);

    // Memory
    cmd.arg(format!("-Xmx{}M", opts.config.max_memory_mb));

    // Standard Aura / Fabric JVM arguments
    cmd.arg("-XX:+UnlockExperimentalVMOptions");
    cmd.arg("-DFabricMcEmu= net.minecraft.client.main.Main ");

    let natives_dir = opts.paths.instance_natives_dir(opts.version);
    cmd.arg(format!("-Djava.library.path={}", natives_dir.display()));
    cmd.arg(format!("-Djna.tmpdir={}", natives_dir.display()));

    if cfg!(target_os = "windows") {
        cmd.arg("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump");
    }

    // Config JVM arguments
    for arg in &opts.config.jvm_args {
        cmd.arg(arg);
    }
    for arg in opts.extra_jvm_args {
        cmd.arg(arg);
    }

    // Classpath
    let cp = build_classpath(opts.client_jar, opts.vanilla_libraries, opts.fabric_libraries);
    cmd.arg("-cp").arg(cp);

    // Main class
    cmd.arg("net.fabricmc.loader.impl.launch.knot.KnotClient");

    // Game arguments
    let instance_dir = opts.paths.instance_dir(opts.version);
    let player_uuid = offline_uuid(&opts.config.nickname);

    cmd.arg("--username").arg(&opts.config.nickname);
    cmd.arg("--version").arg(format!("fabric-loader-0.19.5-{}", opts.version));
    cmd.arg("--gameDir").arg(instance_dir.display().to_string());
    cmd.arg("--assetsDir").arg(opts.paths.assets_dir.display().to_string());
    cmd.arg("--assetIndex").arg(opts.asset_index_id);
    cmd.arg("--uuid").arg(player_uuid.to_string());
    cmd.arg("--accessToken").arg("0");
    cmd.arg("--userType").arg("mojang");
    cmd.arg("--versionType").arg("release");

    cmd.current_dir(&instance_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    cmd
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
        message: format!("Launching Minecraft {} as '{}'...", opts.version, opts.config.nickname),
    });

    let mut command = build_launch_command(&opts);
    let mut child = command.spawn().map_err(|e| {
        WexError::LaunchError(format!("Failed to spawn Java process {}: {}", opts.java_binary.display(), e))
    })?;

    let pid = child.id().unwrap_or(0);
    events.on_event(&Event::GameLaunched { pid });

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

    // 1. Minecraft Piston package
    events.on_event(&Event::StageStarted {
        stage: format!("Minecraft {} Environment", version),
        step: 1,
        total_steps: 4,
    });
    let pkg = fetch_version_package(client, version).await?;
    let client_jar = install_minecraft_client(paths, version, &pkg, client, events).await?;
    let vanilla_libs = install_vanilla_libraries(paths, version, &pkg, client, events, concurrency).await?;
    install_assets(paths, &pkg, client, events, concurrency).await?;

    // 2. Fabric Profile & Libraries
    events.on_event(&Event::StageStarted {
        stage: format!("Fabric Loader 0.19.5 ({})", version),
        step: 2,
        total_steps: 4,
    });
    let fabric_profile = fetch_fabric_profile(client, version, None).await?;
    let fabric_libs = install_fabric_libraries(paths, &fabric_profile, client, events, concurrency).await?;

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
    sync_aura_pack(paths, version, client, events, concurrency).await?;

    let asset_index_id = pkg.asset_index.id;
    Ok((java_binary, client_jar, vanilla_libs, fabric_libs, asset_index_id))
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
    let (mojang_java, client_jar, vanilla_libs, fabric_libs, asset_index_id) =
        install_full_environment(paths, version, client, events.as_ref(), concurrency).await?;

    let java_binary = if let Some(custom) = &config.custom_java_path {
        if custom.exists() {
            custom.clone()
        } else {
            mojang_java
        }
    } else {
        mojang_java
    };

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

        let cp = build_classpath(&client_jar, &[lib1.clone()], &[lib2.clone()]);
        assert!(cp.contains("lib2.jar"));
        assert!(cp.contains("lib1.jar"));
        assert!(cp.contains("client.jar"));
    }
}
