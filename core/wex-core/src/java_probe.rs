//! Which Java runs the game when the player set their own.
//!
//! One custom Java path is shared by both Minecraft versions, but 1.21.1 needs
//! Java 21 and 1.20.1 needs Java 17: a custom Java 17 would crash 1.21.1 at start.
//! The custom binary is asked for its version (`java -version`, cached per path,
//! modification time and size) and the bundled Mojang runtime is used when it is
//! too old or does not answer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// Java major version Minecraft `version` needs (the Mojang runtime it ships with).
pub fn required_java_major(version: &str) -> u32 {
    match crate::java::get_java_component_for_version(version) {
        "java-runtime-delta" => 21,
        "java-runtime-gamma" => 17,
        "java-runtime-alpha" => 16,
        _ => 8,
    }
}

/// Major version from `java -version` output: `"17.0.21"` → 17, `"1.8.0_392"` → 8,
/// `"21" 2023-09-19` → 21.
pub fn parse_java_major(output: &str) -> Option<u32> {
    let line = output.lines().find(|l| l.contains(" version \""))?;
    let start = line.find(" version \"")? + " version \"".len();
    let ver = &line[start..];
    let ver = &ver[..ver.find('"')?];
    let mut parts = ver.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

type CacheKey = (PathBuf, Option<SystemTime>, u64);

fn cache() -> &'static Mutex<HashMap<CacheKey, u32>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, u32>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `javaw` writes nothing for `-version`; ask the console `java` next to it.
fn console_java(java: &Path) -> PathBuf {
    let stem = java.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem.eq_ignore_ascii_case("javaw") {
        let sibling = java.with_file_name(if cfg!(windows) { "java.exe" } else { "java" });
        if sibling.exists() {
            return sibling;
        }
    }
    java.to_path_buf()
}

/// Major version of the Java at `java`, or `None` when it does not run or answer
/// within 20 seconds. Blocking; a version is cached until the file changes, a
/// failure (timeout, crash) is not, so the next launch asks again.
pub fn probe_java_major(java: &Path) -> Option<u32> {
    let meta = std::fs::metadata(java).ok()?;
    let key = (java.to_path_buf(), meta.modified().ok(), meta.len());
    if let Some(hit) = cache().lock().ok().and_then(|c| c.get(&key).copied()) {
        return Some(hit);
    }
    let major = run_version(&console_java(java))?;
    if let Ok(mut c) = cache().lock() {
        c.insert(key, major);
    }
    Some(major)
}

fn run_version(java: &Path) -> Option<u32> {
    let mut cmd = Command::new(java);
    cmd.arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    parse_java_major(&text)
}

/// The Java to launch Minecraft `version` with: the custom one when it exists and
/// is new enough, otherwise `mojang`. The second value explains a fallback.
pub fn choose_java(
    custom: Option<&Path>,
    mojang: PathBuf,
    version: &str,
    probe: impl FnOnce(&Path) -> Option<u32>,
) -> (PathBuf, Option<String>) {
    let Some(custom) = custom else {
        return (mojang, None);
    };
    if !custom.exists() {
        return (
            mojang,
            Some(format!(
                "Своя Java не найдена ({}), используется встроенная Java Mojang",
                custom.display()
            )),
        );
    }
    let need = required_java_major(version);
    match probe(custom) {
        Some(major) if major >= need => (custom.to_path_buf(), None),
        Some(major) => (
            mojang,
            Some(format!(
                "Своя Java {} ({}) слишком старая для Minecraft {}: нужна Java {}+, используется встроенная Java Mojang",
                major,
                custom.display(),
                version,
                need
            )),
        ),
        None => (
            mojang,
            Some(format!(
                "Не удалось узнать версию своей Java ({}), используется встроенная Java Mojang",
                custom.display()
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_need_their_java() {
        assert_eq!(required_java_major("1.20.1"), 17);
        assert_eq!(required_java_major("1.21.1"), 21);
    }

    #[test]
    fn java_version_output_is_parsed() {
        let temurin17 = "openjdk version \"17.0.21.1\" 2025-10-21\nOpenJDK Runtime Environment Temurin-17.0.21.1+2 (build 17.0.21.1+2)\n";
        assert_eq!(parse_java_major(temurin17), Some(17));
        assert_eq!(
            parse_java_major("java version \"1.8.0_392\"\nJava(TM) SE Runtime"),
            Some(8)
        );
        assert_eq!(
            parse_java_major("openjdk version \"21\" 2023-09-19\n"),
            Some(21)
        );
        assert_eq!(
            parse_java_major(
                "Picked up JAVA_TOOL_OPTIONS: -Dx=1\nopenjdk version \"21.0.12\" 2026-01-20"
            ),
            Some(21)
        );
        assert_eq!(parse_java_major("Error: could not open jvm.cfg"), None);
    }

    #[test]
    fn old_custom_java_falls_back_to_mojang_for_1_21_1() {
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("java.exe");
        std::fs::write(&custom, b"").unwrap();
        let mojang = tmp.path().join("mojang").join("java.exe");

        let (java, why) = choose_java(Some(&custom), mojang.clone(), "1.21.1", |_| Some(17));
        assert_eq!(java, mojang);
        assert!(why.unwrap().contains("Java 21"));

        let (java, why) = choose_java(Some(&custom), mojang.clone(), "1.20.1", |_| Some(17));
        assert_eq!(java, custom);
        assert!(why.is_none());

        let (java, why) = choose_java(Some(&custom), mojang.clone(), "1.21.1", |_| Some(21));
        assert_eq!(java, custom);
        assert!(why.is_none());

        let (java, why) = choose_java(Some(&custom), mojang.clone(), "1.20.1", |_| None);
        assert_eq!(java, mojang);
        assert!(why.is_some());

        let missing = tmp.path().join("nope.exe");
        let (java, why) = choose_java(Some(&missing), mojang.clone(), "1.20.1", |_| {
            panic!("a missing file is not probed")
        });
        assert_eq!(java, mojang);
        assert!(why.is_some());

        let (java, why) = choose_java(None, mojang.clone(), "1.21.1", |_| unreachable!());
        assert_eq!(java, mojang);
        assert!(why.is_none());
    }

    #[test]
    fn a_file_that_is_not_java_has_no_version() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp
            .path()
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        std::fs::write(&fake, b"not a program").unwrap();
        assert_eq!(probe_java_major(&fake), None);
        let meta = std::fs::metadata(&fake).unwrap();
        let key = (fake.clone(), meta.modified().ok(), meta.len());
        assert!(
            !cache().lock().unwrap().contains_key(&key),
            "a failed probe is not cached"
        );
        assert_eq!(probe_java_major(&tmp.path().join("missing")), None);
    }

    /// With a real Java (`WEX_TEST_JAVA`; skipped when unset) the probe reads its version.
    #[test]
    fn real_java_is_probed() {
        let Ok(java) = std::env::var("WEX_TEST_JAVA") else {
            return;
        };
        let major = probe_java_major(Path::new(&java)).expect("version of WEX_TEST_JAVA");
        assert!(major >= 8, "{major}");
        assert_eq!(probe_java_major(Path::new(&java)), Some(major), "cached");
    }
}
