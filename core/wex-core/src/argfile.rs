//! Text the Java launcher reads: the `@argfile` and the `@path` argument itself.
//!
//! On Windows `java.exe` (checked with Temurin 17.0.21 and Microsoft 21.0.12) reads
//! an `@argfile` in the active ANSI code page, not UTF-8, and gets its own command
//! line in that code page too. A UTF-8 argfile with a Cyrillic user folder
//! (`C:\Users\Иван\...`) turns the class path into mojibake and the game fails with
//! `ClassNotFoundException`; the same file in CP1251 works. Characters outside the
//! code page cannot reach Java at all (they become `?`, through the command line
//! and `JDK_JAVA_OPTIONS` alike), so path arguments with such characters are
//! replaced by their 8.3 short names, which are plain ASCII.
//!
//! On other systems the launcher uses the locale (UTF-8 in practice), so the file
//! stays UTF-8 and arguments are passed unchanged.

use crate::error::{Result, WexError};

/// Bytes of an argfile body for the Java launcher on this system.
pub fn encode_for_java(text: &str) -> Vec<u8> {
    imp::encode(text)
}

/// `arg` in a form the Java launcher can read back exactly: unchanged when the
/// system code page can hold it, otherwise with its path parts shortened to 8.3
/// names.
///
/// A path that still cannot be represented (8.3 names turned off on the drive)
/// is an error naming the folder: Java would otherwise fail later with a bare
/// `ClassNotFoundException`. Other arguments keep going with a warning and lose
/// the characters the code page lacks.
pub fn java_safe_arg(arg: &str) -> Result<String> {
    if imp::representable(arg) {
        return Ok(arg.to_string());
    }
    let (prefix, value) = split_prefix(arg);
    let mut fixed = Vec::new();
    for part in value.split(';') {
        if imp::representable(part) {
            fixed.push(part.to_string());
        } else if let Some(short) = imp::short_path(part) {
            fixed.push(short);
        } else if std::path::Path::new(part).is_absolute() {
            return Err(unreadable_path(part));
        } else {
            tracing::warn!(
                "a Java argument has characters outside the system code page; Java will see '?' instead of them"
            );
            fixed.push(part.to_string());
        }
    }
    Ok(format!("{}{}", prefix, fixed.join(";")))
}

fn unreadable_path(path: &str) -> WexError {
    let p = std::path::Path::new(path);
    // Name the first folder Java cannot read, not a file deep inside it.
    let folder = p
        .ancestors()
        .filter(|a| !imp::representable(&a.to_string_lossy()))
        .last()
        .unwrap_or(p);
    WexError::LaunchError(format!(
        "Java не сможет прочитать путь «{}»: в нём есть символы, которых нет в кодировке Windows, а короткие имена (8.3) на этом диске выключены. Перенесите папку данных Wex в путь только из латинских букв и цифр (например, C:\\Wex) и запустите игру снова.",
        folder.display()
    ))
}

/// True when the Java launcher on this system reads `text` back unchanged.
pub fn is_representable(text: &str) -> bool {
    imp::representable(text)
}

/// Split `-Dkey=`, `-javaagent:` or `@` off an argument; the rest may be a path list.
fn split_prefix(arg: &str) -> (&str, &str) {
    if let Some(rest) = arg.strip_prefix('@') {
        return (&arg[..1], rest);
    }
    if let Some(rest) = arg.strip_prefix("-javaagent:") {
        return (&arg[..arg.len() - rest.len()], rest);
    }
    if arg.starts_with('-') {
        if let Some(i) = arg.find('=') {
            return (&arg[..=i], &arg[i + 1..]);
        }
    }
    ("", arg)
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Globalization::{
        GetACP, WideCharToMultiByte, CP_ACP, WC_NO_BEST_FIT_CHARS,
    };
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    const CP_UTF8: u32 = 65001;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    /// `s` in the ANSI code page; `None` when a character has no exact byte form.
    fn ansi_exact(s: &str) -> Option<Vec<u8>> {
        if s.is_empty() {
            return Some(Vec::new());
        }
        if unsafe { GetACP() } == CP_UTF8 {
            return Some(s.as_bytes().to_vec());
        }
        let w = wide(s);
        let mut used_default = 0;
        let len = unsafe {
            WideCharToMultiByte(
                CP_ACP,
                WC_NO_BEST_FIT_CHARS,
                w.as_ptr(),
                w.len() as i32,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                &mut used_default,
            )
        };
        if len <= 0 || used_default != 0 {
            return None;
        }
        let mut out = vec![0u8; len as usize];
        let written = unsafe {
            WideCharToMultiByte(
                CP_ACP,
                WC_NO_BEST_FIT_CHARS,
                w.as_ptr(),
                w.len() as i32,
                out.as_mut_ptr(),
                len,
                std::ptr::null(),
                &mut used_default,
            )
        };
        if written != len || used_default != 0 {
            return None;
        }
        Some(out)
    }

    /// Lossy ANSI form: characters without a byte form become `?`, as Java would see them.
    fn ansi_lossy(s: &str) -> Vec<u8> {
        if let Some(b) = ansi_exact(s) {
            return b;
        }
        s.split_inclusive(|_| true)
            .flat_map(|c| ansi_exact(c).unwrap_or_else(|| vec![b'?']))
            .collect()
    }

    pub fn representable(s: &str) -> bool {
        ansi_exact(s).is_some()
    }

    pub fn encode(text: &str) -> Vec<u8> {
        ansi_lossy(text)
    }

    fn short_name(p: &Path) -> Option<PathBuf> {
        let w: Vec<u16> = p.as_os_str().encode_wide().chain(Some(0)).collect();
        let need = unsafe { GetShortPathNameW(w.as_ptr(), std::ptr::null_mut(), 0) };
        if need == 0 {
            return None;
        }
        let mut buf = vec![0u16; need as usize];
        let got = unsafe { GetShortPathNameW(w.as_ptr(), buf.as_mut_ptr(), need) };
        if got == 0 || got >= need {
            return None;
        }
        buf.truncate(got as usize);
        Some(PathBuf::from(std::ffi::OsString::from_wide(&buf)))
    }

    /// 8.3 form of an absolute path. A path that does not exist yet keeps its
    /// missing tail; the result is used only when every part became representable.
    pub fn short_path(s: &str) -> Option<String> {
        let path = Path::new(OsStr::new(s));
        if !path.is_absolute() {
            return None;
        }
        let existing = path.ancestors().find(|a| a.exists())?;
        let tail = path.strip_prefix(existing).ok()?;
        let mut short = short_name(existing)?;
        if !tail.as_os_str().is_empty() {
            short.push(tail);
        }
        let out = short.to_str()?.to_string();
        representable(&out).then_some(out)
    }

    #[cfg(test)]
    pub fn decode(bytes: &[u8]) -> String {
        use windows_sys::Win32::Globalization::MultiByteToWideChar;
        if bytes.is_empty() {
            return String::new();
        }
        let len = unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                std::ptr::null_mut(),
                0,
            )
        };
        let mut out = vec![0u16; len.max(0) as usize];
        unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                out.as_mut_ptr(),
                len,
            )
        };
        String::from_utf16_lossy(&out)
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn representable(_s: &str) -> bool {
        true
    }

    pub fn encode(text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    pub fn short_path(_s: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_are_split_off_path_values() {
        assert_eq!(split_prefix("@C:\\a\\b.txt"), ("@", "C:\\a\\b.txt"));
        assert_eq!(split_prefix("-Dx.y=C:\\a;C:\\b"), ("-Dx.y=", "C:\\a;C:\\b"));
        assert_eq!(
            split_prefix("-javaagent:C:\\a.jar"),
            ("-javaagent:", "C:\\a.jar")
        );
        assert_eq!(split_prefix("C:\\a;C:\\b"), ("", "C:\\a;C:\\b"));
        assert_eq!(split_prefix("--gameDir"), ("", "--gameDir"));
    }

    #[test]
    fn ascii_arguments_are_untouched() {
        for a in [
            "-Xmx4096M",
            "-cp",
            "C:\\Wex\\a.jar;C:\\Wex\\b.jar",
            "@C:\\x y\\f.txt",
        ] {
            assert!(is_representable(a));
            assert_eq!(java_safe_arg(a).unwrap(), a);
        }
        assert_eq!(encode_for_java("-Xmx1M\n"), b"-Xmx1M\n");
    }

    /// A Cyrillic data root (a player named Иван) must reach Java exactly.
    #[cfg(windows)]
    #[test]
    fn cyrillic_paths_reach_the_java_launcher_exactly() {
        let root = tempfile::Builder::new()
            .prefix("wex-Иван-")
            .tempdir()
            .unwrap();
        let dir = root.path().join("библиотеки");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jar"), b"x").unwrap();
        let jar = dir.join("a.jar").display().to_string();
        let missing = dir.join("logs").join("новый.log").display().to_string();
        let cp = format!("{};{}", jar, jar);
        for arg in [
            cp.clone(),
            format!("-Dwex.dir={}", dir.display()),
            format!("@{}", jar),
            missing.clone(),
        ] {
            let safe = match java_safe_arg(&arg) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("skipped: {e}");
                    return;
                }
            };
            if !is_representable(&safe) {
                eprintln!("skipped: no code page form and no 8.3 names on this volume");
                return;
            }
            let back = imp::decode(&encode_for_java(&safe));
            assert_eq!(
                back, safe,
                "argfile bytes must decode back in the ANSI code page"
            );
            let (_, value) = split_prefix(&back);
            let first = value.split(';').next().unwrap();
            let p = std::path::Path::new(first);
            if arg == missing {
                assert_eq!(
                    std::fs::canonicalize(p.parent().unwrap().parent().unwrap()).unwrap(),
                    std::fs::canonicalize(&dir).unwrap()
                );
            } else {
                assert_eq!(
                    std::fs::canonicalize(p).unwrap(),
                    std::fs::canonicalize(first_of(&arg)).unwrap()
                );
            }
        }
    }

    /// A path no 8.3 name can save stops the launch with the folder named.
    #[cfg(windows)]
    #[test]
    fn unreadable_path_fails_with_the_folder_named() {
        if is_representable("张") {
            eprintln!("skipped: the code page holds this character");
            return;
        }
        // A folder that does not exist has no short name.
        let arg = "-Dx=C:\\wex-no-such-张\\libs\\a.jar";
        let err = java_safe_arg(arg).unwrap_err().to_string();
        assert!(err.contains("C:\\wex-no-such-张"), "{err}");
        assert!(!err.contains("a.jar"), "{err}");
        assert!(err.contains("латинских"), "{err}");
        // Not a path: goes on (with a warning) instead of failing.
        assert!(java_safe_arg("-Dname=张").is_ok());
    }

    #[cfg(windows)]
    fn first_of(arg: &str) -> String {
        split_prefix(arg).1.split(';').next().unwrap().to_string()
    }

    /// End to end with a real Java (`WEX_TEST_JAVA`, else `java` on PATH; skipped
    /// when there is none): the class path and a property in a Cyrillic folder
    /// survive the trip through the argfile, and so does a folder with characters
    /// outside the code page (through its 8.3 name).
    #[cfg(windows)]
    #[test]
    fn java_reads_cyrillic_argfile_back() {
        let java = std::env::var("WEX_TEST_JAVA").unwrap_or_else(|_| "java".to_string());
        let root = tempfile::Builder::new()
            .prefix("wex-Иван-")
            .tempdir()
            .unwrap();
        for name in ["Пётр классы", "张 Łódź"] {
            let dir = root.path().join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let args = [
                "-cp".to_string(),
                dir.display().to_string(),
                format!("-Dwex.probe={}", dir.display()),
                "-XshowSettings:properties".to_string(),
                "-version".to_string(),
            ];
            let Ok(safe) = args
                .iter()
                .map(|a| java_safe_arg(a))
                .collect::<Result<Vec<_>>>()
            else {
                eprintln!("skipped {name}: no code page form and no 8.3 names on this volume");
                continue;
            };
            if !safe.iter().all(|a| is_representable(a)) {
                eprintln!("skipped {name}: no code page form and no 8.3 names on this volume");
                continue;
            }
            let mut body = String::new();
            for a in &safe {
                body.push_str(&crate::launcher::format_arg_for_file(a));
                body.push('\n');
            }
            let file = dir.join("аргументы.txt");
            std::fs::write(&file, encode_for_java(&body)).unwrap();
            let out = match std::process::Command::new(&java)
                .arg(java_safe_arg(&format!("@{}", file.display())).unwrap())
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("skipped: no Java at {java}: {e}");
                    return;
                }
            };
            let text = imp::decode(&out.stderr);
            assert!(out.status.success(), "{text}");
            let want = std::fs::canonicalize(&dir).unwrap();
            for key in ["wex.probe = ", "java.class.path = "] {
                let line = text
                    .lines()
                    .find_map(|l| l.trim().strip_prefix(key))
                    .unwrap_or_else(|| panic!("{key} missing in:\n{text}"));
                assert_eq!(
                    std::fs::canonicalize(line.trim()).unwrap(),
                    want,
                    "{key}{line}"
                );
            }
        }
    }
}
