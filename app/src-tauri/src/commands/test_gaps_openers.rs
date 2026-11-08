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

async fn assert_opened(path: &std::path::Path) {
    let marker = std::path::PathBuf::from(std::env::var_os("WEX_TEST_OPENER_MARKER").unwrap());
    let expected = path.to_string_lossy().replace('\\', "/");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if fs::read_to_string(&marker)
            .unwrap_or_default()
            .replace('\\', "/")
            .contains(&expected)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "opener did not receive {}",
            expected
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    // The short-lived GUI-subsystem double exits immediately after its write.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    fs::write(marker, b"").unwrap();
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
        let log = f.paths.instance_dir(version).join("logs/latest.log");
        fs::write(&log, b"fixture log").unwrap();
        let opened = super::super::crash::crash_open_log_file(Some(version.into()), f.play())
            .await
            .unwrap();
        assert_eq!(std::path::Path::new(&opened), log);
        assert_opened(&log).await;
    }
    workshop::workshop_open_path(f.paths.root.to_string_lossy().into())
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
        workshop::workshop_open_path(f.paths.root.to_string_lossy().into())
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
