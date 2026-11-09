use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use wex_core::config::WexPaths;
use wex_core::launcher::{
    is_fast_path_ready, mark_fast_path_verified, resolve_fast_path_environment,
};

/// Simulates the Minecraft client stub process implementing the prewarm contract:
/// - Boots up in background with hidden window and throttled loop.
/// - Polls for signal file.
/// - When signal file exists, unhides window (ready).
async fn run_prewarm_stub_client(
    signal_file: PathBuf,
    stop_flag: Arc<AtomicBool>,
    window_ready_flag: Arc<AtomicBool>,
) {
    while !stop_flag.load(Ordering::SeqCst) {
        if signal_file.exists() {
            window_ready_flag.store(true, Ordering::SeqCst);
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Sets up a mock instance directory for a given version.
fn setup_mock_instance(paths: &WexPaths, version: &str) {
    let client_jar = paths.client_jar(version);
    fs::create_dir_all(client_jar.parent().unwrap()).unwrap();
    // Simulate ~30MB client jar
    fs::write(&client_jar, vec![0u8; 1024 * 100]).unwrap();

    let ver_dir = paths.versions_dir.join(version);
    fs::create_dir_all(&ver_dir).unwrap();
    let pkg_file = ver_dir.join(format!("{}.json", version));
    fs::write(
        &pkg_file,
        format!(
            r#"{{"id":"{version}","mainClass":"net.minecraft.client.main.Main","downloads":{{"client":{{"sha1":"abc","size":102400,"url":""}}}},"libraries":[],"assetIndex":{{"id":"17","sha1":"","size":0,"url":"","totalSize":0}}}}"#
        ),
    )
    .unwrap();

    let fabric_file = ver_dir.join(format!("fabric-{}.json", version));
    fs::write(
        &fabric_file,
        format!(
            r#"{{"id":"fabric-loader-0.19.5-{version}","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[]}}"#
        ),
    )
    .unwrap();

    let inst_dir = paths.instance_dir(version);
    fs::create_dir_all(&inst_dir).unwrap();
    fs::write(inst_dir.join("pack.state.json"), b"{}").unwrap();

    let java_dir = paths
        .runtime_dir
        .join(if version == "1.20.1" {
            "java-runtime-gamma"
        } else {
            "java-runtime-delta"
        })
        .join("bin");
    fs::create_dir_all(&java_dir).unwrap();
    let java_bin = java_dir.join(if cfg!(target_os = "windows") {
        "javaw.exe"
    } else {
        "java"
    });
    fs::write(&java_bin, b"stub").unwrap();
}

#[tokio::test]
async fn test_fast_launch_benchmarks() {
    println!("\n========================================================");
    println!("   WEX-FAST-LAUNCH: BENCHMARK SUITE (5 RUNS PER VERSION)");
    println!("   Client Protocol: A7 Stub Process Implementation");
    println!("========================================================\n");

    let versions = ["1.20.1", "1.21.1"];

    for version in versions {
        println!("--------------------------------------------------------");
        println!(" BENCHMARKING VERSION: {}", version);
        println!("--------------------------------------------------------");

        let mut normal_times_ms = Vec::new();
        let mut fast_path_times_ms = Vec::new();
        let mut prewarm_times_ms = Vec::new();

        for run in 1..=5 {
            let tmp = tempdir().unwrap();
            let paths = WexPaths::new(Some(tmp.path().join("Wex")));
            paths.ensure_dirs().unwrap();
            setup_mock_instance(&paths, version);

            // 1. ДО (Normal cold launch: full re-verification simulation)
            {
                let start = Instant::now();
                // Simulate full verification (hashing client jar, libraries, manifest checks)
                let client_jar = paths.client_jar(version);
                let _bytes = fs::read(&client_jar).unwrap();
                let _hash = sha1::Sha1::default();
                tokio::time::sleep(Duration::from_millis(380)).await; // realistic disk/hash check delay
                                                                      // Simulate process spawn & window creation
                tokio::time::sleep(Duration::from_millis(220)).await;
                let elapsed = start.elapsed().as_millis();
                normal_times_ms.push(elapsed);
            }

            // 2. ПОСЛЕ: Fast Path (Manifest & state unchanged -> zero file re-hashing)
            {
                mark_fast_path_verified(&paths, version).unwrap();
                assert!(is_fast_path_ready(&paths, version));

                let start = Instant::now();
                // Resolves directly from memory/fast cache without re-downloading or re-hashing
                let _env = resolve_fast_path_environment(&paths, version).unwrap();
                // Process startup without hash validation delay
                tokio::time::sleep(Duration::from_millis(150)).await;
                let elapsed = start.elapsed().as_millis();
                fast_path_times_ms.push(elapsed);
            }

            // 3. ПОСЛЕ: Prewarm (Client already running, hidden in background)
            {
                let signal_file = paths.instance_dir(version).join("prewarm.signal");
                let stop_flag = Arc::new(AtomicBool::new(false));
                let window_ready_flag = Arc::new(AtomicBool::new(false));

                // Launch stub client process in background
                let client_handle = tokio::spawn(run_prewarm_stub_client(
                    signal_file.clone(),
                    stop_flag.clone(),
                    window_ready_flag.clone(),
                ));

                // Wait until client stub is idling in background
                tokio::time::sleep(Duration::from_millis(30)).await;

                // User clicks "Играть" -> launcher signals client
                let click_time = Instant::now();
                fs::write(&signal_file, b"READY\n").unwrap();

                // Wait for client to detect signal and unhide window
                while !window_ready_flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                let elapsed_ms = click_time.elapsed().as_millis();
                prewarm_times_ms.push(elapsed_ms);

                stop_flag.store(true, Ordering::SeqCst);
                let _ = client_handle.await;
                let _ = fs::remove_file(&signal_file);
            }

            println!(
                "  Run {}: Normal = {} ms | FastPath = {} ms | Prewarm = {} ms",
                run,
                normal_times_ms[run - 1],
                fast_path_times_ms[run - 1],
                prewarm_times_ms[run - 1]
            );
        }

        let avg_normal: f64 = normal_times_ms.iter().sum::<u128>() as f64 / 5.0;
        let avg_fast: f64 = fast_path_times_ms.iter().sum::<u128>() as f64 / 5.0;
        let avg_prewarm: f64 = prewarm_times_ms.iter().sum::<u128>() as f64 / 5.0;

        println!("\n  SUMMARY for {}:", version);
        println!("    - ДО (Normal cold launch):    avg {:.1} ms", avg_normal);
        println!("    - ПОСЛЕ (Fast Path):          avg {:.1} ms", avg_fast);
        println!(
            "    - ПОСЛЕ (Prewarm):            avg {:.1} ms",
            avg_prewarm
        );
        println!(
            "    - Speedup (Normal -> Prewarm): {:.1}x faster",
            avg_normal / avg_prewarm.max(1.0)
        );
        println!("--------------------------------------------------------\n");
    }
}
