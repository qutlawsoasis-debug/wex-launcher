//! System Information & Hardware Telemetry (SPEC: wex-system-info).
//!
//! Provides real hardware metrics without mock/fictitious values:
//! - Total and available RAM;
//! - CPU model name and core/thread count;
//! - GPU model, driver version, and integrated/discrete classification;
//! - Windows OS name, release/display version, build and UBR;
//! - Disk capacity and free space for Wex working directory;
//! - Java runtimes for Minecraft 1.20.1 (Java 17) and 1.21.1 (Java 21);
//! - Human-readable warnings (< 8 GB RAM, < 5 GB disk, integrated GPU);
//! - Sanitized support text omitting Windows user account names.

use serde::{Deserialize, Serialize};
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

pub const NOT_DETERMINED: &str = "не удалось определить";

/// Complete system telemetry and hardware specifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemInfo {
    /// Total physical memory in Megabytes.
    pub total_memory_mb: u32,
    /// Available physical memory in Megabytes.
    pub free_memory_mb: u32,
    /// Human-friendly memory summary (e.g. "16.0 ГБ (свободно 8.2 ГБ)").
    pub memory_display: String,
    /// Operating system identifier (e.g. "windows").
    pub os_name: String,
    /// Full OS version (e.g. "Windows 10 Pro 22H2 (сборка 19045.6466)").
    pub os_version: String,
    /// Launcher application version (e.g. "1.0.0").
    pub launcher_version: String,
    /// Wex root data directory path (sanitized for public display).
    pub data_dir: String,
    /// Processor commercial name (e.g. "AMD Ryzen 5 3600 6-Core Processor").
    pub cpu_name: String,
    /// Number of logical processor cores.
    pub cpu_cores: u32,
    /// Graphics card commercial model (e.g. "NVIDIA GeForce GTX 1660 SUPER").
    pub gpu_name: String,
    /// Graphics driver version string (e.g. "32.0.16.1742").
    pub gpu_driver: String,
    /// Whether the graphics adapter is integrated (Intel HD/UHD, AMD Vega/APU).
    pub is_integrated_gpu: bool,
    /// Available disk space on the Wex partition in Gigabytes.
    pub disk_free_gb: f64,
    /// Total disk capacity on the Wex partition in Gigabytes.
    pub disk_total_gb: f64,
    /// Disk summary (e.g. "4.1 ГБ свободно из 460.5 ГБ").
    pub disk_display: String,
    /// Installed or detected Java runtime for Minecraft 1.20.1 (Java 17).
    pub java_17: String,
    /// Installed or detected Java runtime for Minecraft 1.21.1 (Java 21).
    pub java_21: String,
    /// Active warnings in simple human terms.
    pub warnings: Vec<String>,
    /// Sanitized multi-line text suitable for copying to tech support.
    pub support_text: String,
}

// ---------------------------------------------------------------------------
// Path Sanitization (Omitting Windows Usernames)
// ---------------------------------------------------------------------------

/// Sanitizes file system paths by replacing the user directory or Windows username
/// with a generic `<user>` placeholder to protect user privacy.
pub fn sanitize_user_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let mut result = path.to_string();

    // 1. If explicit USERNAME or USER env var is present, replace it in user paths
    for var in &["USERNAME", "USER"] {
        if let Ok(user) = std::env::var(var) {
            let trimmed = user.trim();
            if !trimmed.is_empty() && trimmed.len() > 1 {
                // Replace \Users\Username\ and /Users/Username/
                let win_user_prefix = format!("\\Users\\{}\\", trimmed);
                let win_user_target = "\\Users\\<user>\\";
                result = replace_case_insensitive(&result, &win_user_prefix, win_user_target);

                let unix_user_prefix = format!("/Users/{}/", trimmed);
                let unix_user_target = "/Users/<user>/";
                result = replace_case_insensitive(&result, &unix_user_prefix, unix_user_target);

                let home_user_prefix = format!("/home/{}/", trimmed);
                let home_user_target = "/home/<user>/";
                result = replace_case_insensitive(&result, &home_user_prefix, home_user_target);
            }
        }
    }

    // 2. Generic regex-like matching for any Drive:\Users\Something\ or /Users/Something/
    result = sanitize_generic_user_dir(&result);

    result
}

fn replace_case_insensitive(text: &str, target: &str, replacement: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_target = target.to_lowercase();
    let mut out = String::new();
    let mut last_idx = 0;

    while let Some(pos) = lower_text[last_idx..].find(&lower_target) {
        let abs_pos = last_idx + pos;
        out.push_str(&text[last_idx..abs_pos]);
        out.push_str(replacement);
        last_idx = abs_pos + target.len();
    }
    out.push_str(&text[last_idx..]);
    out
}

fn sanitize_generic_user_dir(path: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = path.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for [A-Za-z]:\Users\ or [A-Za-z]:/Users/ or \Users\ or /Users/
        let remaining: String = chars[i..].iter().collect();
        let lower_rem = remaining.to_lowercase();

        let marker_len = if lower_rem.starts_with(":\\users\\") || lower_rem.starts_with(":/users/")
        {
            Some(8)
        } else if lower_rem.starts_with("\\users\\") || lower_rem.starts_with("/users/") {
            Some(7)
        } else if lower_rem.starts_with("/home/") {
            Some(6)
        } else {
            None
        };

        if let Some(mlen) = marker_len {
            // Write drive letter or root if any
            out.push_str(&remaining[..mlen]);
            i += mlen;

            // Find next slash terminating username
            let mut username_len = 0;
            while i + username_len < len
                && chars[i + username_len] != '\\'
                && chars[i + username_len] != '/'
            {
                username_len += 1;
            }

            if username_len > 0 {
                out.push_str("<user>");
                i += username_len;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Hardware Classifications & Warning Thresholds
// ---------------------------------------------------------------------------

/// Evaluates system hardware specs against human-friendly performance warning thresholds.
pub fn evaluate_system_warnings(
    total_ram_mb: u64,
    disk_free_gb: f64,
    is_integrated_gpu: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // 1. Low RAM (< 8 GB = 8192 MB)
    if (1..8192).contains(&total_ram_mb) {
        let gb = total_ram_mb as f64 / 1024.0;
        warnings.push(format!(
            "Мало оперативной памяти ({:.1} ГБ). Для стабильной работы игры и модов рекомендуется от 8 ГБ.",
            gb
        ));
    }

    // 2. Low disk free space (< 5.0 GB)
    if (0.0..5.0).contains(&disk_free_gb) {
        warnings.push(format!(
            "Мало свободного места на диске ({:.1} ГБ). Рекомендуется освободить хотя бы 5 ГБ для миров и обновлений.",
            disk_free_gb
        ));
    }

    // 3. Integrated graphics adapter
    if is_integrated_gpu {
        warnings.push(
            "Используется встроенная видеокарта. Возможна низкая частота кадров в игре; рекомендуется дискретный графический процессор.".to_string(),
        );
    }

    warnings
}

/// Determines whether a GPU adapter name corresponds to an integrated graphics chipset.
pub fn is_integrated_graphics(gpu_name: &str) -> bool {
    let lower = gpu_name.to_lowercase();
    if lower == NOT_DETERMINED || lower.is_empty() {
        return false;
    }

    // Dedicated GPUs always override integrated false positives
    let is_dedicated = lower.contains("geforce")
        || lower.contains("gtx")
        || lower.contains("rtx")
        || lower.contains("quadro")
        || lower.contains("radeon rx")
        || lower.contains("radeon pro")
        || lower.contains("arc a")
        || lower.contains("arc(tm) a")
        || (lower.contains("arc") && lower.contains(" a"))
        || lower.contains("arc pro")
        || lower.contains("arc(tm) pro");

    if is_dedicated {
        return false;
    }

    // Intel integrated
    if lower.contains("intel")
        && (lower.contains("hd graphics")
            || lower.contains("uhd graphics")
            || lower.contains("iris")
            || lower.contains("graphics"))
    {
        return true;
    }

    // AMD APU / integrated
    if (lower.contains("amd") || lower.contains("radeon"))
        && (lower.contains("vega")
            || lower.contains("tm graphics")
            || lower.contains("graphics")
            || lower.contains("apu"))
    {
        return true;
    }

    // Generic / virtual / Apple integrated
    if lower.contains("microsoft basic display adapter")
        || lower.contains("standard vga")
        || lower.contains("apple m")
    {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// Support Report Text Formatting
// ---------------------------------------------------------------------------

/// Formats a complete, sanitized system report string for copying to technical support.
pub fn format_support_text(info: &SystemInfo) -> String {
    let mut s = String::new();
    s.push_str("=== О системе (Wex Support) ===\n");
    s.push_str(&format!("ОС: {}\n", info.os_version));
    s.push_str(&format!(
        "Процессор: {} ({} ядер)\n",
        info.cpu_name, info.cpu_cores
    ));
    s.push_str(&format!("ОЗУ: {}\n", info.memory_display));
    s.push_str(&format!(
        "Видеокарта: {} (драйвер: {})\n",
        info.gpu_name, info.gpu_driver
    ));
    s.push_str(&format!(
        "Тип видеокарты: {}\n",
        if info.is_integrated_gpu {
            "Встроенная"
        } else {
            "Дискретная"
        }
    ));
    s.push_str(&format!("Диск (Wex): {}\n", info.disk_display));
    s.push_str(&format!("Папка данных: {}\n", info.data_dir));
    s.push_str(&format!("Java для 1.20.1: {}\n", info.java_17));
    s.push_str(&format!("Java для 1.21.1: {}\n", info.java_21));
    s.push_str(&format!("Версия лаунчера: {}\n", info.launcher_version));

    s.push_str("\nПредупреждения:\n");
    if info.warnings.is_empty() {
        s.push_str("- Нет\n");
    } else {
        for w in &info.warnings {
            s.push_str(&format!("- {}\n", w));
        }
    }
    s.push_str("================================");
    s
}

// ---------------------------------------------------------------------------
// Native Windows & OS Query Implementations
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod native_win {
    use super::*;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hkey: isize,
            lpsubkey: *const u16,
            uloptions: u32,
            samdesired: u32,
            phkresult: *mut isize,
        ) -> i32;
        fn RegQueryValueExW(
            hkey: isize,
            lpvaluename: *const u16,
            lpreserved: *mut u32,
            lptype: *mut u32,
            lpdata: *mut u8,
            lpcbdata: *mut u32,
        ) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            lpdirectoryname: *const u16,
            lpfreebytesavailabletocaler: *mut u64,
            lptotalnumberofbytes: *mut u64,
            lptotalnumberoffreebytes: *mut u64,
        ) -> i32;
    }

    const HKEY_LOCAL_MACHINE: isize = -2147483646; // 0x80000002
    const KEY_READ: u32 = 0x20019;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn read_reg_string(subkey: &str, value_name: &str) -> Option<String> {
        let subkey_w = to_wide(subkey);
        let val_w = to_wide(value_name);
        let mut hkey: isize = 0;

        unsafe {
            let res = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                subkey_w.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            );
            if res != 0 {
                return None;
            }

            let mut val_type: u32 = 0;
            let mut byte_count: u32 = 0;

            // First call to get size
            let res = RegQueryValueExW(
                hkey,
                val_w.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                std::ptr::null_mut(),
                &mut byte_count,
            );
            if res != 0 || byte_count == 0 {
                RegCloseKey(hkey);
                return None;
            }

            let mut buf = vec![0u8; (byte_count + 2) as usize];
            let res = RegQueryValueExW(
                hkey,
                val_w.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                buf.as_mut_ptr(),
                &mut byte_count,
            );
            RegCloseKey(hkey);

            if res != 0 {
                return None;
            }

            if val_type == 1 || val_type == 2 {
                // REG_SZ or REG_EXPAND_SZ
                let u16_slice = std::slice::from_raw_parts(
                    buf.as_ptr() as *const u16,
                    (byte_count as usize) / 2,
                );
                let string = String::from_utf16_lossy(u16_slice);
                Some(string.trim_matches('\0').trim().to_string())
            } else {
                None
            }
        }
    }

    pub fn read_reg_u32(subkey: &str, value_name: &str) -> Option<u32> {
        let subkey_w = to_wide(subkey);
        let val_w = to_wide(value_name);
        let mut hkey: isize = 0;

        unsafe {
            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                subkey_w.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            ) != 0
            {
                return None;
            }

            let mut val_type: u32 = 0;
            let mut val: u32 = 0;
            let mut byte_count: u32 = 4;

            let res = RegQueryValueExW(
                hkey,
                val_w.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                &mut val as *mut u32 as *mut u8,
                &mut byte_count,
            );
            RegCloseKey(hkey);

            if res == 0 && val_type == 4 {
                Some(val)
            } else {
                None
            }
        }
    }

    pub fn query_disk_space(dir: &Path) -> Option<(f64, f64)> {
        let dir_str = dir.to_string_lossy();
        let dir_w = to_wide(&dir_str);
        let mut free_bytes: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free: u64 = 0;

        unsafe {
            let res = GetDiskFreeSpaceExW(
                dir_w.as_ptr(),
                &mut free_bytes,
                &mut total_bytes,
                &mut total_free,
            );
            if res != 0 && total_bytes > 0 {
                let free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                Some((free_gb, total_gb))
            } else {
                None
            }
        }
    }
}

/// Collects true system specs, hardware metadata, and environment details.
pub fn collect_system_info(wex_data_dir: &Path, launcher_version: &str) -> SystemInfo {
    // 1. RAM via GlobalMemoryStatusEx
    let (total_memory_mb, free_memory_mb) = {
        #[cfg(windows)]
        {
            use std::mem::zeroed;
            #[repr(C)]
            struct MemoryStatusEx {
                length: u32,
                memory_load: u32,
                total_phys: u64,
                avail_phys: u64,
                total_page_file: u64,
                avail_page_file: u64,
                total_virtual: u64,
                avail_virtual: u64,
                avail_extended_virtual: u64,
            }
            extern "system" {
                fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
            }
            unsafe {
                let mut status: MemoryStatusEx = zeroed();
                status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
                if GlobalMemoryStatusEx(&mut status) != 0 {
                    (
                        (status.total_phys / (1024 * 1024)) as u32,
                        (status.avail_phys / (1024 * 1024)) as u32,
                    )
                } else {
                    (0, 0)
                }
            }
        }
        #[cfg(not(windows))]
        {
            (0, 0)
        }
    };

    let memory_display = if total_memory_mb > 0 {
        format!(
            "{:.1} ГБ (свободно {:.1} ГБ)",
            total_memory_mb as f64 / 1024.0,
            free_memory_mb as f64 / 1024.0
        )
    } else {
        NOT_DETERMINED.to_string()
    };

    // 2. CPU model and cores
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(0);

    let cpu_name = {
        #[cfg(windows)]
        {
            native_win::read_reg_string(
                r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "ProcessorNameString",
            )
            .unwrap_or_else(|| NOT_DETERMINED.to_string())
        }
        #[cfg(not(windows))]
        {
            NOT_DETERMINED.to_string()
        }
    };

    // 3. GPU model and driver
    let (gpu_name, gpu_driver) = {
        #[cfg(windows)]
        {
            let mut name = None;
            let mut driver = None;

            for index in &["0000", "0001", "0002"] {
                let key = format!(
                    r"SYSTEM\CurrentControlSet\Control\Class\{{4d36e968-e325-11ce-bfc1-08002be10318}}\{}",
                    index
                );
                if let Some(desc) = native_win::read_reg_string(&key, "DriverDesc") {
                    let drv = native_win::read_reg_string(&key, "DriverVersion")
                        .unwrap_or_else(|| NOT_DETERMINED.to_string());

                    // Prefer dedicated GPUs if multiple adapters exist
                    let is_primary = !is_integrated_graphics(&desc) || name.is_none();
                    if is_primary {
                        name = Some(desc);
                        driver = Some(drv);
                    }
                }
            }

            (
                name.unwrap_or_else(|| NOT_DETERMINED.to_string()),
                driver.unwrap_or_else(|| NOT_DETERMINED.to_string()),
            )
        }
        #[cfg(not(windows))]
        {
            (NOT_DETERMINED.to_string(), NOT_DETERMINED.to_string())
        }
    };

    let is_integrated = is_integrated_graphics(&gpu_name);

    // 4. Windows version
    let os_version = {
        #[cfg(windows)]
        {
            let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
            let product = native_win::read_reg_string(key, "ProductName")
                .unwrap_or_else(|| "Windows".to_string());
            let display_version = native_win::read_reg_string(key, "DisplayVersion")
                .or_else(|| native_win::read_reg_string(key, "ReleaseId"))
                .unwrap_or_default();
            let build = native_win::read_reg_string(key, "CurrentBuildNumber")
                .or_else(|| native_win::read_reg_string(key, "CurrentBuild"))
                .unwrap_or_default();
            let ubr = native_win::read_reg_u32(key, "UBR");

            // Fix Windows 11 naming quirk where ProductName still says "Windows 10"
            let build_num: u32 = build.parse().unwrap_or(0);
            let product_clean = if build_num >= 22000 && product.contains("Windows 10") {
                product.replace("Windows 10", "Windows 11")
            } else {
                product
            };

            let mut ver_str = product_clean;
            if !display_version.is_empty() {
                ver_str.push_str(&format!(" {}", display_version));
            }
            if !build.is_empty() {
                if let Some(ubr_val) = ubr {
                    ver_str.push_str(&format!(" (сборка {}.{})", build, ubr_val));
                } else {
                    ver_str.push_str(&format!(" (сборка {})", build));
                }
            }
            ver_str
        }
        #[cfg(not(windows))]
        {
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
        }
    };

    // 5. Disk capacity and free space
    let (disk_free_gb, disk_total_gb) = {
        #[cfg(windows)]
        {
            native_win::query_disk_space(wex_data_dir).unwrap_or((0.0, 0.0))
        }
        #[cfg(not(windows))]
        {
            (0.0, 0.0)
        }
    };

    let disk_display = if disk_total_gb > 0.0 {
        format!("{:.1} ГБ свободно из {:.1} ГБ", disk_free_gb, disk_total_gb)
    } else {
        NOT_DETERMINED.to_string()
    };

    // 6. Java runtimes for 1.20.1 and 1.21.1
    let runtime_dir = wex_data_dir.join("runtime");
    let (j17_path, j21_path) = detect_java_paths_internal(&runtime_dir);

    let java_17 = j17_path
        .map(|p| sanitize_user_path(&p))
        .unwrap_or_else(|| "не найдена (будет скачана автоматически)".to_string());
    let java_21 = j21_path
        .map(|p| sanitize_user_path(&p))
        .unwrap_or_else(|| "не найдена (будет скачана автоматически)".to_string());

    // 7. Human-friendly warnings
    let warnings = evaluate_system_warnings(total_memory_mb as u64, disk_free_gb, is_integrated);

    // 8. Sanitized data directory
    let data_dir_sanitized = sanitize_user_path(&wex_data_dir.to_string_lossy());

    let mut info = SystemInfo {
        total_memory_mb,
        free_memory_mb,
        memory_display,
        os_name: std::env::consts::OS.to_string(),
        os_version,
        launcher_version: launcher_version.to_string(),
        data_dir: data_dir_sanitized,
        cpu_name,
        cpu_cores,
        gpu_name,
        gpu_driver,
        is_integrated_gpu: is_integrated,
        disk_free_gb,
        disk_total_gb,
        disk_display,
        java_17,
        java_21,
        warnings,
        support_text: String::new(),
    };

    info.support_text = format_support_text(&info);
    info
}

fn detect_java_paths_internal(runtime_dir: &Path) -> (Option<String>, Option<String>) {
    let mut java_17 = None;
    let mut java_21 = None;

    let exe_name = if cfg!(target_os = "windows") {
        "javaw.exe"
    } else {
        "java"
    };

    // 1. Check isolated Mojang runtimes in Wex data dir
    let mojang_17 = runtime_dir
        .join("java-runtime-gamma")
        .join("bin")
        .join(exe_name);
    if mojang_17.exists() {
        java_17 = Some(mojang_17.to_string_lossy().to_string());
    }

    let mojang_21 = runtime_dir
        .join("java-runtime-delta")
        .join("bin")
        .join(exe_name);
    if mojang_21.exists() {
        java_21 = Some(mojang_21.to_string_lossy().to_string());
    }

    // 2. Check standard system locations on Windows
    #[cfg(target_os = "windows")]
    {
        let standard_roots = [
            PathBuf::from(r"C:\Program Files\Microsoft"),
            PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
            PathBuf::from(r"C:\Program Files\Java"),
        ];

        for root in &standard_roots {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let javaw = path.join("bin").join("javaw.exe");
                    if javaw.exists() {
                        if java_17.is_none() && (name.contains("17") || name.contains("gamma")) {
                            java_17 = Some(javaw.to_string_lossy().to_string());
                        }
                        if java_21.is_none() && (name.contains("21") || name.contains("delta")) {
                            java_21 = Some(javaw.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    (java_17, java_21)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_sanitization_removes_windows_username() {
        // Windows user profile path
        let input = r"C:\Users\JohnDoe\AppData\Roaming\Wex\instances\1.20.1";
        let sanitized = sanitize_user_path(input);
        assert_eq!(
            sanitized,
            r"C:\Users\<user>\AppData\Roaming\Wex\instances\1.20.1"
        );
        assert!(!sanitized.contains("JohnDoe"));

        // Forward slash variant
        let input_slash = "C:/Users/Administrator/Wex/logs/latest.log";
        let sanitized_slash = sanitize_user_path(input_slash);
        assert_eq!(sanitized_slash, "C:/Users/<user>/Wex/logs/latest.log");
        assert!(!sanitized_slash.contains("Administrator"));

        // Unix home path
        let input_unix = "/home/alexander/.wex/config.json";
        let sanitized_unix = sanitize_user_path(input_unix);
        assert_eq!(sanitized_unix, "/home/<user>/.wex/config.json");
        assert!(!sanitized_unix.contains("alexander"));

        // Path without username should remain intact
        let public_path = r"C:\Program Files\Java\jdk-21\bin\javaw.exe";
        assert_eq!(sanitize_user_path(public_path), public_path);
    }

    #[test]
    fn test_warning_threshold_ram_below_8gb() {
        // 4GB RAM (4096 MB) must produce warning
        let warnings = evaluate_system_warnings(4096, 20.0, false);
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("Мало оперативной памяти")),
            "4GB RAM must trigger memory warning"
        );

        // 7GB RAM (7168 MB) must produce warning
        let warnings_7g = evaluate_system_warnings(7168, 20.0, false);
        assert!(
            warnings_7g
                .iter()
                .any(|w| w.contains("Мало оперативной памяти")),
            "7GB RAM must trigger memory warning"
        );

        // 8GB RAM (8192 MB) and 16GB RAM must NOT trigger memory warning
        let warnings_8g = evaluate_system_warnings(8192, 20.0, false);
        assert!(
            !warnings_8g
                .iter()
                .any(|w| w.contains("Мало оперативной памяти")),
            "8GB RAM must NOT trigger memory warning"
        );

        let warnings_16g = evaluate_system_warnings(16384, 20.0, false);
        assert!(
            !warnings_16g
                .iter()
                .any(|w| w.contains("Мало оперативной памяти")),
            "16GB RAM must NOT trigger memory warning"
        );
    }

    #[test]
    fn test_warning_threshold_disk_below_5gb() {
        // 4.2 GB free disk must produce warning
        let warnings = evaluate_system_warnings(16384, 4.2, false);
        assert!(
            warnings.iter().any(|w| w.contains("Мало свободного места")),
            "4.2 GB disk must trigger disk warning"
        );

        // 4.99 GB free disk must produce warning
        let warnings_499 = evaluate_system_warnings(16384, 4.99, false);
        assert!(
            warnings_499
                .iter()
                .any(|w| w.contains("Мало свободного места")),
            "4.99 GB disk must trigger disk warning"
        );

        // 5.0 GB and 100 GB must NOT trigger disk warning
        let warnings_5g = evaluate_system_warnings(16384, 5.0, false);
        assert!(
            !warnings_5g
                .iter()
                .any(|w| w.contains("Мало свободного места")),
            "5.0 GB disk must NOT trigger disk warning"
        );

        let warnings_100g = evaluate_system_warnings(16384, 100.0, false);
        assert!(
            !warnings_100g
                .iter()
                .any(|w| w.contains("Мало свободного места")),
            "100 GB disk must NOT trigger disk warning"
        );
    }

    #[test]
    fn test_warning_threshold_integrated_gpu() {
        // Integrated GPU must produce warning
        let warnings = evaluate_system_warnings(16384, 20.0, true);
        assert!(
            warnings.iter().any(|w| w.contains("встроенная видеокарта")),
            "Integrated GPU must trigger GPU warning"
        );

        // Discrete GPU must NOT produce warning
        let warnings_discrete = evaluate_system_warnings(16384, 20.0, false);
        assert!(
            !warnings_discrete
                .iter()
                .any(|w| w.contains("встроенная видеокарта")),
            "Discrete GPU must NOT trigger GPU warning"
        );
    }

    #[test]
    fn test_integrated_graphics_detection() {
        // Integrated adapters
        assert!(is_integrated_graphics("Intel(R) HD Graphics 630"));
        assert!(is_integrated_graphics("Intel(R) UHD Graphics 770"));
        assert!(is_integrated_graphics("Intel(R) Iris(R) Xe Graphics"));
        assert!(is_integrated_graphics("AMD Radeon(TM) Vega 8 Graphics"));
        assert!(is_integrated_graphics("AMD Radeon Graphics"));
        assert!(is_integrated_graphics("Microsoft Basic Display Adapter"));
        assert!(is_integrated_graphics("Apple M1"));

        // Discrete adapters
        assert!(!is_integrated_graphics("NVIDIA GeForce GTX 1660 SUPER"));
        assert!(!is_integrated_graphics("NVIDIA GeForce RTX 3060"));
        assert!(!is_integrated_graphics("NVIDIA GeForce RTX 4090"));
        assert!(!is_integrated_graphics("AMD Radeon RX 6700 XT"));
        assert!(!is_integrated_graphics("AMD Radeon RX 7900 XTX"));
        assert!(!is_integrated_graphics("Intel(R) Arc(TM) A770 Graphics"));
    }

    #[test]
    fn test_support_text_formatting_and_sanitization() {
        let info = SystemInfo {
            total_memory_mb: 16384,
            free_memory_mb: 8192,
            memory_display: "16.0 ГБ (свободно 8.0 ГБ)".to_string(),
            os_name: "windows".to_string(),
            os_version: "Windows 11 Pro 23H2 (сборка 22631.3593)".to_string(),
            launcher_version: "1.0.0".to_string(),
            data_dir: r"C:\Users\<user>\AppData\Roaming\Wex".to_string(),
            cpu_name: "AMD Ryzen 5 3600 6-Core Processor".to_string(),
            cpu_cores: 12,
            gpu_name: "NVIDIA GeForce GTX 1660 SUPER".to_string(),
            gpu_driver: "32.0.16.1742".to_string(),
            is_integrated_gpu: false,
            disk_free_gb: 12.5,
            disk_total_gb: 460.0,
            disk_display: "12.5 ГБ свободно из 460.0 ГБ".to_string(),
            java_17:
                r"C:\Users\<user>\AppData\Roaming\Wex\runtime\java-runtime-gamma\bin\javaw.exe"
                    .to_string(),
            java_21: r"C:\Program Files\Microsoft\jdk-21.0.12.101-hotspot\bin\javaw.exe"
                .to_string(),
            warnings: vec![],
            support_text: String::new(),
        };

        let formatted = format_support_text(&info);
        assert!(formatted.contains("=== О системе (Wex Support) ==="));
        assert!(formatted.contains("ОС: Windows 11 Pro 23H2 (сборка 22631.3593)"));
        assert!(formatted.contains("Процессор: AMD Ryzen 5 3600 6-Core Processor (12 ядер)"));
        assert!(formatted.contains("ОЗУ: 16.0 ГБ (свободно 8.0 ГБ)"));
        assert!(
            formatted.contains("Видеокарта: NVIDIA GeForce GTX 1660 SUPER (драйвер: 32.0.16.1742)")
        );
        assert!(formatted.contains("Тип видеокарты: Дискретная"));
        assert!(formatted.contains("Папка данных: C:\\Users\\<user>\\AppData\\Roaming\\Wex"));
        assert!(formatted.contains("Предупреждения:\n- Нет"));
        assert!(!formatted.contains("aq03"));
        assert!(!formatted.contains("JohnDoe"));
    }

    #[test]
    fn test_collect_system_info_real_machine() {
        let temp = tempfile::tempdir().unwrap();
        let info = collect_system_info(temp.path(), "1.0.0");

        // Real PC sanity checks
        #[cfg(windows)]
        {
            assert!(info.total_memory_mb > 1024, "Real PC must have > 1GB RAM");
            assert!(info.cpu_cores > 0, "CPU cores count must be > 0");
            assert_ne!(info.os_version, NOT_DETERMINED);
            assert_ne!(info.cpu_name, NOT_DETERMINED);
            assert!(info
                .support_text
                .contains("=== О системе (Wex Support) ==="));
        }

        // Must not contain hardcoded raw user names in data_dir
        assert!(!info.data_dir.contains("C:\\Users\\aq03\\"));
    }
}
