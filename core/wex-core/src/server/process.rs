//! Running a dedicated server: start, stop, status, players, log and console.
//!
//! The server runs as a hidden child process (`java … KnotServer nogui`) in its
//! own folder. On Windows it is put in a job object, so it never outlives Wex.
//! Stop sends `stop` to the console (the server saves the world) and kills the
//! process only if it has not exited in time.

use super::install::{server_jar_path, FABRIC_PROFILE_FILE};
use super::{ServerStore, RECORD_FILE};
use crate::config::WexPaths;
use crate::error::{Result, WexError};
use crate::fabric::{resolve_fabric_libraries, FabricProfile};
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{oneshot, watch};

const LOG_CAPACITY: usize = 2000;
const LINE_MAX: usize = 4000;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    /// Exited on its own with an error code.
    Crashed,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerLogLine {
    pub seq: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSnapshot {
    pub status: ServerStatus,
    pub exit_code: Option<i32>,
    pub players: Vec<String>,
    pub lines: Vec<ServerLogLine>,
    /// Pass back as `since` to get only new lines.
    pub next_seq: u64,
    pub started_at: String,
    pub pid: u32,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerEvent {
    Joined(String),
    Left(String),
}

fn valid_log_name(name: &str) -> bool {
    (1..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Player join/leave from a server log line. Vanilla writes
/// `[time] [Server thread/INFO]: Steve joined the game`; Fabric's log layout is
/// `[time] [Server thread/INFO] (Minecraft) Steve joined the game`. Chat lines
/// (`<Steve> … joined the game`) do not match.
pub fn parse_player_event(line: &str) -> Option<PlayerEvent> {
    let line = line.trim_end();
    let (rest, joined) = match line.strip_suffix(" joined the game") {
        Some(r) => (r, true),
        None => (line.strip_suffix(" left the game")?, false),
    };
    let (prefix, name) = rest.rsplit_once(' ').unwrap_or(("", rest));
    if !(prefix.ends_with("]:") || prefix.ends_with(')')) || !valid_log_name(name) {
        return None;
    }
    Some(if joined {
        PlayerEvent::Joined(name.to_string())
    } else {
        PlayerEvent::Left(name.to_string())
    })
}

pub fn is_ready_line(line: &str) -> bool {
    line.contains("Done (") && line.contains("For help")
}

fn problem_for_line(line: &str) -> Option<&'static str> {
    if line.contains("FAILED TO BIND TO PORT") {
        Some("Порт сервера занят другой программой. Укажите другой порт в настройках.")
    } else if line.contains("You need to agree to the EULA") {
        Some("Сервер не запустится без принятия EULA Minecraft.")
    } else if line.contains("Incompatible mods found") || line.contains("Mod resolution failed") {
        Some("Fabric не смог загрузить моды сервера. Подробности в консоли.")
    } else if line.contains("OutOfMemoryError") {
        Some("Серверу не хватило памяти. Увеличьте память в настройках.")
    } else {
        None
    }
}

/// JVM and program arguments for the server.
pub fn build_server_args(
    memory_mb: u32,
    server_jar: &Path,
    classpath: &[PathBuf],
    main_class: &str,
) -> Vec<String> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let cp = classpath
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(sep);
    vec![
        format!("-Xms{}M", memory_mb.min(1024)),
        format!("-Xmx{memory_mb}M"),
        "-Dfile.encoding=UTF-8".into(),
        "-Dsun.stdout.encoding=UTF-8".into(),
        "-Dsun.stderr.encoding=UTF-8".into(),
        format!("-Dfabric.gameJarPath={}", server_jar.to_string_lossy()),
        "-cp".into(),
        cp,
        main_class.into(),
        "nogui".into(),
    ]
}

struct Shared {
    status: Mutex<ServerStatus>,
    exit_code: Mutex<Option<i32>>,
    players: Mutex<BTreeSet<String>>,
    log: Mutex<VecDeque<ServerLogLine>>,
    seq: AtomicU64,
    stop_requested: AtomicBool,
    problem: Mutex<Option<String>>,
}

impl Shared {
    fn push(&self, mut text: String) {
        if text.len() > LINE_MAX {
            let mut cut = LINE_MAX;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
        }
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut log = self.log.lock().unwrap();
        if log.len() >= LOG_CAPACITY {
            log.pop_front();
        }
        log.push_back(ServerLogLine { seq, text });
    }

    fn handle_line(&self, line: String) {
        if is_ready_line(&line) {
            let mut s = self.status.lock().unwrap();
            if *s == ServerStatus::Starting {
                *s = ServerStatus::Running;
            }
        }
        if let Some(p) = problem_for_line(&line) {
            *self.problem.lock().unwrap() = Some(p.to_string());
        }
        match parse_player_event(&line) {
            Some(PlayerEvent::Joined(n)) => {
                self.players.lock().unwrap().insert(n);
            }
            Some(PlayerEvent::Left(n)) => {
                self.players.lock().unwrap().remove(&n);
            }
            None => {}
        }
        self.push(line);
    }
}

fn spawn_reader<R: AsyncRead + Unpin + Send + 'static>(reader: R, shared: Arc<Shared>) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf)
                        .trim_end_matches(['\r', '\n'])
                        .to_string();
                    shared.handle_line(line);
                }
            }
        }
    });
}

pub struct ServerProcess {
    pub id: String,
    pub pid: u32,
    pub started_at: String,
    shared: Arc<Shared>,
    stdin: tokio::sync::Mutex<Option<ChildStdin>>,
    kill_tx: Mutex<Option<oneshot::Sender<()>>>,
    exited: watch::Receiver<bool>,
    #[cfg(windows)]
    _job: Option<crate::launcher::WindowsJobObject>,
}

unsafe impl Send for ServerProcess {}
unsafe impl Sync for ServerProcess {}

/// A console command as typed by the player: one line, no control characters.
pub fn clean_command(command: &str) -> Result<String> {
    let c = command.trim().trim_start_matches('/').trim();
    if c.is_empty() || c.len() > 256 || c.chars().any(|ch| ch.is_control()) {
        return Err(WexError::Server(
            "Команда должна быть одной строкой до 256 символов".into(),
        ));
    }
    Ok(c.to_string())
}

impl ServerProcess {
    pub fn status(&self) -> ServerStatus {
        *self.shared.status.lock().unwrap()
    }

    pub fn has_exited(&self) -> bool {
        *self.exited.borrow()
    }

    pub fn players(&self) -> Vec<String> {
        self.shared
            .players
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    pub fn snapshot(&self, since: u64) -> ServerSnapshot {
        let lines = self
            .shared
            .log
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.seq > since)
            .cloned()
            .collect();
        ServerSnapshot {
            status: self.status(),
            exit_code: *self.shared.exit_code.lock().unwrap(),
            players: self.players(),
            lines,
            next_seq: self.shared.seq.load(Ordering::SeqCst),
            started_at: self.started_at.clone(),
            pid: self.pid,
            problem: self.shared.problem.lock().unwrap().clone(),
        }
    }

    pub async fn send_command(&self, command: &str) -> Result<()> {
        let c = clean_command(command)?;
        if self.has_exited() {
            return Err(WexError::Server("Сервер не запущен".into()));
        }
        let mut stdin = self.stdin.lock().await;
        let pipe = stdin
            .as_mut()
            .ok_or_else(|| WexError::Server("Консоль сервера недоступна".into()))?;
        pipe.write_all(format!("{c}\n").as_bytes()).await?;
        pipe.flush().await?;
        self.shared.push(format!("> {c}"));
        Ok(())
    }

    async fn wait_exit(&self, timeout: Duration) -> bool {
        let mut rx = self.exited.clone();
        let done = tokio::time::timeout(timeout, rx.wait_for(|e| *e)).await;
        done.is_ok()
    }

    /// Graceful stop: `stop` saves the world; killed only after `timeout`.
    pub async fn stop(&self, timeout: Duration) -> Result<()> {
        if self.has_exited() {
            return Ok(());
        }
        self.shared.stop_requested.store(true, Ordering::SeqCst);
        *self.shared.status.lock().unwrap() = ServerStatus::Stopping;
        let sent = {
            let mut stdin = self.stdin.lock().await;
            match stdin.as_mut() {
                Some(pipe) => pipe.write_all(b"stop\n").await.is_ok() && pipe.flush().await.is_ok(),
                None => false,
            }
        };
        if sent && self.wait_exit(timeout).await {
            return Ok(());
        }
        self.kill().await;
        Ok(())
    }

    /// Immediate kill (the world may lose the last minutes).
    pub async fn kill(&self) {
        self.shared.stop_requested.store(true, Ordering::SeqCst);
        if let Some(tx) = self.kill_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        self.wait_exit(Duration::from_secs(10)).await;
    }
}

/// Checks everything and starts the server.
pub async fn start_server(
    paths: &WexPaths,
    store: &ServerStore,
    id: &str,
) -> Result<ServerProcess> {
    let rec = store.load(id)?;
    let dir = store.server_dir(id)?;
    if rec.installed.is_none() {
        return Err(WexError::Server("Сначала установите сервер".into()));
    }
    if !store.eula_accepted(&rec) {
        return Err(WexError::Server("Сначала примите EULA Minecraft".into()));
    }
    let jar = server_jar_path(paths, &rec.version);
    let profile_text = std::fs::read_to_string(dir.join(FABRIC_PROFILE_FILE)).map_err(|_| {
        WexError::Server("Установка сервера не завершена, установите его заново".into())
    })?;
    let profile: FabricProfile = serde_json::from_str(&profile_text)?;
    let classpath = resolve_fabric_libraries(paths, &profile);
    if !jar.is_file() || classpath.iter().any(|p| !p.is_file()) {
        return Err(WexError::Server(
            "Не хватает файлов сервера, установите его заново".into(),
        ));
    }
    let component = crate::java::get_java_component_for_version(&rec.version);
    let java = crate::java::get_java_binary_path(&paths.runtime_dir, component, false);
    if !java.is_file() {
        return Err(WexError::Server(
            "Не найдена Java для сервера, установите его заново".into(),
        ));
    }
    if super::net::port_in_use(rec.settings.port) {
        return Err(WexError::Server(format!(
            "Порт {} уже занят. Остановите другой сервер или выберите другой порт.",
            rec.settings.port
        )));
    }
    store.write_properties(&rec)?;
    debug_assert!(dir.join(RECORD_FILE).is_file());

    let mut cmd = Command::new(&java);
    cmd.args(build_server_args(
        rec.settings.memory_mb,
        &jar,
        &classpath,
        &profile.main_class,
    ))
    .current_dir(&dir);
    spawn_supervised(id, cmd)
}

/// Spawns `cmd` hidden, with piped console, and supervises it.
pub fn spawn_supervised(id: &str, mut cmd: Command) -> Result<ServerProcess> {
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd
        .spawn()
        .map_err(|e| WexError::Server(format!("Не удалось запустить Java сервера: {e}")))?;
    let pid = child.id().unwrap_or(0);

    #[cfg(windows)]
    let job = crate::launcher::WindowsJobObject::new()
        .ok()
        .inspect(|job| {
            if let Some(handle) = child.raw_handle() {
                // SAFETY: the handle belongs to the child we just spawned and is alive.
                let _ = unsafe { job.assign_process(handle as _) };
            }
        });

    let shared = Arc::new(Shared {
        status: Mutex::new(ServerStatus::Starting),
        exit_code: Mutex::new(None),
        players: Mutex::new(BTreeSet::new()),
        log: Mutex::new(VecDeque::new()),
        seq: AtomicU64::new(0),
        stop_requested: AtomicBool::new(false),
        problem: Mutex::new(None),
    });
    if let Some(out) = child.stdout.take() {
        spawn_reader(out, shared.clone());
    }
    if let Some(err) = child.stderr.take() {
        spawn_reader(err, shared.clone());
    }
    let stdin = child.stdin.take();
    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    let (exit_tx, exit_rx) = watch::channel(false);
    let waiter_shared = shared.clone();
    tokio::spawn(async move {
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = kill_rx => {
                let _ = child.start_kill();
                child.wait().await.ok()
            }
        };
        let code = status.and_then(|s| s.code());
        *waiter_shared.exit_code.lock().unwrap() = code;
        let requested = waiter_shared.stop_requested.load(Ordering::SeqCst);
        *waiter_shared.status.lock().unwrap() = if requested || code == Some(0) {
            ServerStatus::Stopped
        } else {
            ServerStatus::Crashed
        };
        waiter_shared.players.lock().unwrap().clear();
        let _ = exit_tx.send(true);
    });

    Ok(ServerProcess {
        id: id.to_string(),
        pid,
        started_at: chrono::Utc::now().to_rfc3339(),
        shared,
        stdin: tokio::sync::Mutex::new(stdin),
        kill_tx: Mutex::new(Some(kill_tx)),
        exited: exit_rx,
        #[cfg(windows)]
        _job: job,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_events_vanilla_and_fabric_layouts() {
        assert_eq!(
            parse_player_event("[12:00:01] [Server thread/INFO]: Steve joined the game"),
            Some(PlayerEvent::Joined("Steve".into()))
        );
        assert_eq!(
            parse_player_event(
                "[12:00:01] [Server thread/INFO] (Minecraft) Alex_2 left the game\r"
            ),
            Some(PlayerEvent::Left("Alex_2".into()))
        );
        assert_eq!(
            parse_player_event("[12:00:01] [Server thread/INFO]: <Steve> Bob joined the game"),
            None
        );
        assert_eq!(
            parse_player_event("[12:00] [Server thread/INFO]: [Steve: Gave] joined the game"),
            None
        );
        assert_eq!(parse_player_event("Steve joined the game"), None);
        assert_eq!(
            parse_player_event("[x]: averyveryverylongname1 joined the game"),
            None
        );
    }

    #[test]
    fn ready_and_problem_lines() {
        assert!(is_ready_line(
            "[12:00] [Server thread/INFO]: Done (3.412s)! For help, type \"help\""
        ));
        assert!(!is_ready_line("Done loading"));
        assert!(problem_for_line("**** FAILED TO BIND TO PORT!").is_some());
        assert!(problem_for_line("all good").is_none());
    }

    #[test]
    fn server_args_point_fabric_at_the_vanilla_jar() {
        let args = build_server_args(
            4096,
            Path::new("v/server.jar"),
            &[PathBuf::from("a.jar"), PathBuf::from("b.jar")],
            "net.fabricmc.loader.impl.launch.knot.KnotServer",
        );
        assert_eq!(args[0], "-Xms1024M");
        assert_eq!(args[1], "-Xmx4096M");
        assert!(args
            .iter()
            .any(|a| a.starts_with("-Dfabric.gameJarPath=") && a.ends_with("server.jar")));
        let cp = &args[args.iter().position(|a| a == "-cp").unwrap() + 1];
        assert!(cp.contains("a.jar") && cp.contains("b.jar"));
        assert_eq!(args.last().unwrap(), "nogui");
        assert_eq!(
            &args[args.len() - 2],
            "net.fabricmc.loader.impl.launch.knot.KnotServer"
        );
    }

    #[test]
    fn console_commands_are_single_lines() {
        assert_eq!(clean_command(" /say hi ").unwrap(), "say hi");
        assert!(clean_command("").is_err());
        assert!(clean_command("say a\nstop").is_err());
        assert!(clean_command(&"x".repeat(300)).is_err());
    }

    #[test]
    fn log_ring_buffer_and_players() {
        let shared = Shared {
            status: Mutex::new(ServerStatus::Starting),
            exit_code: Mutex::new(None),
            players: Mutex::new(BTreeSet::new()),
            log: Mutex::new(VecDeque::new()),
            seq: AtomicU64::new(0),
            stop_requested: AtomicBool::new(false),
            problem: Mutex::new(None),
        };
        for i in 0..(LOG_CAPACITY + 5) {
            shared.handle_line(format!("line {i}"));
        }
        assert_eq!(shared.log.lock().unwrap().len(), LOG_CAPACITY);
        assert_eq!(shared.log.lock().unwrap().front().unwrap().seq, 6);
        shared.handle_line("[t] [Server thread/INFO]: Done (1.0s)! For help, type \"help\"".into());
        assert_eq!(*shared.status.lock().unwrap(), ServerStatus::Running);
        shared.handle_line("[t] [Server thread/INFO]: Steve joined the game".into());
        shared.handle_line("[t] [Server thread/INFO]: Alex joined the game".into());
        shared.handle_line("[t] [Server thread/INFO]: Steve left the game".into());
        assert_eq!(
            shared
                .players
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["Alex"]
        );
        shared.handle_line("é".repeat(LINE_MAX));
        assert!(shared.log.lock().unwrap().back().unwrap().text.len() <= LINE_MAX);
    }

    #[tokio::test]
    async fn start_requirements_are_checked() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let store = ServerStore::new(&paths.root);
        let rec = store
            .create("1.21.1", super::super::ServerSettings::default())
            .unwrap();
        let err = start_server(&paths, &store, &rec.id).await.err().unwrap();
        assert!(err.to_string().contains("установите"));
    }

    /// The process plumbing with a stand-in console (cmd.exe reads commands
    /// from stdin), no Java needed: output lines, readiness, commands, kill.
    #[cfg(windows)]
    #[tokio::test]
    async fn supervised_process_console_and_stop() {
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/Q", "/K"]);
        let p = spawn_supervised("srv-00000000", cmd).unwrap();
        assert_eq!(p.status(), ServerStatus::Starting);
        p.send_command("echo [t] [Server thread/INFO]: Done (1s)! For help, type")
            .await
            .unwrap();
        p.send_command("echo [t] [Server thread/INFO]: Steve joined the game")
            .await
            .unwrap();
        let ok = tokio::time::timeout(Duration::from_secs(20), async {
            while p.status() != ServerStatus::Running || p.players().is_empty() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
        assert!(ok.is_ok(), "{:?}", p.snapshot(0).lines);
        assert_eq!(p.players(), vec!["Steve".to_string()]);
        let snap = p.snapshot(0);
        assert!(snap.lines.iter().any(|l| l.text.starts_with("> echo")));
        let later = p.snapshot(snap.next_seq);
        assert!(later.lines.is_empty());
        // cmd.exe ignores `stop`, so this exercises the kill fallback.
        p.stop(Duration::from_millis(500)).await.unwrap();
        assert!(p.has_exited());
        assert_eq!(p.status(), ServerStatus::Stopped);
        assert!(p.players().is_empty());
        assert!(p.send_command("say hi").await.is_err());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn exit_on_its_own_with_error_is_a_crash() {
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/Q", "/C", "exit 3"]);
        let p = spawn_supervised("srv-00000000", cmd).unwrap();
        assert!(p.wait_exit(Duration::from_secs(20)).await);
        assert_eq!(p.status(), ServerStatus::Crashed);
        assert_eq!(p.snapshot(0).exit_code, Some(3));
    }
}
