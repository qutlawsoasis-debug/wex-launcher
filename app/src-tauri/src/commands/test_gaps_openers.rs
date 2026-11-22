//! Windows process doubles live beside a private copy of libtest in tempdir.
//! Rust resolves that executable directory before Windows/parent PATH, so the
//! real Explorer is never launched. No process-wide environment is mutated.
use super::*;

fn isolated_opener_case(name: &str, valid: bool) -> bool {
    if std::env::var("WEX_TEST_OPENER_CHILD").as_deref() == Ok(name) {
        return true;
    }
    let temp = tempdir().unwrap();
    let runner = temp.path().join("test-runner.exe");
    fs::copy(std::env::current_exe().unwrap(), &runner).unwrap();
    let opener = temp.path().join("explorer.exe");
    if valid {
        let source = temp.path().join("opener.rs");
        fs::write(
            &source,
            r#"
#![windows_subsystem = "windows"]
use std::io::Write;
fn main() {
    let marker = std::env::var_os("WEX_TEST_OPENER_MARKER").unwrap();
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(marker).unwrap();
    writeln!(file, "{}", std::env::args().skip(1).collect::<Vec<_>>().join("|")).unwrap();
}
"#,
        )
        .unwrap();
        let output =
            std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .args(["--edition=2021", "--crate-name", "opener_fixture"])
                .arg(&source)
                .arg("-o")
                .arg(&opener)
                .output()
                .unwrap();
        assert!(
            output.status.success(),
            "fixture compiler: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        // An existing but invalid executable makes spawn fail; resolution cannot
        // fall through to the real Explorer after finding this file.
        fs::write(&opener, b"invalid executable fixture").unwrap();
    }
    let output = std::process::Command::new(&runner)
        .arg(format!("commands::test_gaps::openers::{name}"))
        .args(["--exact", "--test-threads=1"])
        .env("WEX_TEST_OPENER_CHILD", name)
        .env("WEX_TEST_OPENER_MARKER", temp.path().join("opened.txt"))
        .env("APPDATA", temp.path().join("profile"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "opener child: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

/// Lowercased, slash-separated, long-form (8.3 and verbatim prefix resolved) path so the
/// comparison does not depend on how the temp directory is spelled.
fn normalize_opened(raw: &str) -> String {
    let resolved = fs::canonicalize(raw)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| raw.to_owned());
    resolved
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .trim_end_matches('/')
        .to_lowercase()
}

/// Waits until the double has logged one more call and checks that it opened
/// `path`. The marker is append-only and calls are consumed in order, so no
/// truncation can race with a still-running double. The deadline is only a
/// safety net for a double that never starts.
async fn assert_opened(path: &std::path::Path) {
    static CONSUMED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let marker = std::path::PathBuf::from(std::env::var_os("WEX_TEST_OPENER_MARKER").unwrap());
    let expected = normalize_opened(&path.to_string_lossy());
    let index = CONSUMED.fetch_add(1, Ordering::SeqCst);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let text = fs::read_to_string(&marker).unwrap_or_default();
        // A line is complete only once its newline is written.
        let lines: Vec<&str> = text.split_terminator('\n').collect();
        if let Some(line) = lines.get(index).filter(|_| text.ends_with('\n')) {
            assert_eq!(
                normalize_opened(line.trim().trim_start_matches("/select,")),
                expected,
                "opener call #{index} received {line}"
            );
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "opener did not receive {expected} (call #{index})"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn folder_and_log_commands_success_invoke_only_process_double() {
    if !isolated_opener_case(
        "folder_and_log_commands_success_invoke_only_process_double",
        true,
    ) {
        return;
    }
    let f = Fixture::new().await;
    settings::open_game_data_dir(f.play()).await.unwrap();
    assert_opened(&f.paths.root).await;
    settings::open_data_dir(f.play()).await.unwrap();
    assert_opened(&f.paths.root).await;
    for version in ["1.20.1", "1.21.1"] {
        mods::open_mods_dir(version.into(), f.play()).await.unwrap();
        assert_opened(&f.paths.instance_dir(version).join("mods")).await;
        workshop_packs::workshop_open_packs_folder(
            "shaderpack".into(),
            Some(version.into()),
            f.play(),
        )
        .await
        .unwrap();
        assert_opened(&f.paths.instance_dir(version).join("shaderpacks")).await;
        // Minecraft's log4j file is there too; the button opens Wex's capture.
        let logs = f.paths.instance_dir(version).join("logs");
        fs::write(logs.join("latest.log"), b"log4j").unwrap();
        let log = logs.join(wex_core::GAME_OUTPUT_LOG);
        fs::write(&log, b"fixture log").unwrap();
        let opened = super::super::crash::crash_open_log_file(Some(version.into()), f.play())
            .await
            .unwrap();
        assert_eq!(std::path::Path::new(&opened), log);
        assert_opened(&log).await;
    }
    workshop::workshop_open_path(f.paths.root.to_string_lossy().into(), f.play())
        .await
        .unwrap();
    assert_opened(&f.paths.root).await;
}

#[tokio::test]
async fn folder_and_log_commands_error_propagate_process_spawn_failure() {
    if !isolated_opener_case(
        "folder_and_log_commands_error_propagate_process_spawn_failure",
        false,
    ) {
        return;
    }
    let f = Fixture::new().await;
    assert!(settings::open_game_data_dir(f.play()).await.is_err());
    assert!(settings::open_data_dir(f.play()).await.is_err());
    assert!(mods::open_mods_dir("1.21.1".into(), f.play())
        .await
        .is_err());
    assert!(workshop_packs::workshop_open_packs_folder(
        "resourcepack".into(),
        Some("1.21.1".into()),
        f.play()
    )
    .await
    .is_err());
    assert!(
        workshop::workshop_open_path(f.paths.root.to_string_lossy().into(), f.play())
            .await
            .is_err()
    );
    assert!(super::super::crash::crash_open_log_file(None, f.play())
        .await
        .is_err());
    assert!(
        !std::path::PathBuf::from(std::env::var_os("WEX_TEST_OPENER_MARKER").unwrap()).exists()
    );
}
