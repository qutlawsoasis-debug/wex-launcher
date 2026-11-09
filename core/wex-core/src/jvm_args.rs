//! JVM Arguments Optimization & Process Priority Manager (SPEC 2.8, wex-mc-performance).
//!
//! Provides:
//! - Tuned GC profiles (G1 Low-Pause, Generational ZGC for Java 21, Default G1);
//! - Dynamic RAM auto-allocation based on system physical memory;
//! - Process priority elevation (`ABOVE_NORMAL_PRIORITY_CLASS`) for optimal frame pacing and low stutter;
//! - Modular integration into Wex launcher pipeline with minimal invasive touches.

use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::io::RawHandle;

/// Supported Garbage Collector profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GcProfile {
    /// Default standard G1GC without aggressive flags.
    #[default]
    G1Default,
    /// G1GC tuned for ultra-low pause times (< 20ms) and steady 1% low FPS.
    G1Tuned,
    /// Generational ZGC for Java 21+ with concurrent marking and minimal STW pauses.
    GenerationalZgc,
}

impl GcProfile {
    pub fn parse_from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "g1_tuned" | "g1-tuned" | "tuned_g1" | "tuned" => GcProfile::G1Tuned,
            "zgc" | "gen_zgc" | "generational_zgc" | "generational-zgc" => {
                GcProfile::GenerationalZgc
            }
            _ => GcProfile::G1Default,
        }
    }

    /// Return the JVM flags corresponding to this GC profile for the target Java version.
    pub fn to_jvm_flags(&self, java_version: u32) -> Vec<String> {
        match self {
            GcProfile::G1Default => {
                vec![
                    "-XX:+UseG1GC".to_string(),
                    "-XX:+UnlockExperimentalVMOptions".to_string(),
                ]
            }
            GcProfile::G1Tuned => {
                vec![
                    "-XX:+UseG1GC".to_string(),
                    "-XX:+UnlockExperimentalVMOptions".to_string(),
                    "-XX:MaxGCPauseMillis=20".to_string(),
                    "-XX:+ParallelRefProcEnabled".to_string(),
                    "-XX:G1NewSizePercent=20".to_string(),
                    "-XX:G1ReservePercent=15".to_string(),
                    "-XX:G1HeapWastePercent=5".to_string(),
                    "-XX:G1MixedGCCountTarget=4".to_string(),
                    "-XX:InitiatingHeapOccupancyPercent=15".to_string(),
                    "-XX:G1MixedGCLiveThresholdPercent=90".to_string(),
                    "-XX:G1RSetUpdatingPauseTimePercent=5".to_string(),
                    "-XX:SurvivorRatio=32".to_string(),
                    "-XX:+PerfDisableSharedMem".to_string(),
                    "-XX:MaxTenuringThreshold=1".to_string(),
                ]
            }
            GcProfile::GenerationalZgc => {
                if java_version >= 21 {
                    vec![
                        "-XX:+UnlockExperimentalVMOptions".to_string(),
                        "-XX:+UseZGC".to_string(),
                        "-XX:+ZGenerational".to_string(),
                        "-XX:+AlwaysPreTouch".to_string(),
                        "-XX:+PerfDisableSharedMem".to_string(),
                    ]
                } else {
                    // Fall back to G1Tuned if Java version does not support Generational ZGC (< 21)
                    tracing::warn!(
                        "Generational ZGC requested on Java {}, falling back to Tuned G1",
                        java_version
                    );
                    GcProfile::G1Tuned.to_jvm_flags(java_version)
                }
            }
        }
    }
}

/// Process CPU execution priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessPriority {
    Normal,
    #[default]
    AboveNormal,
    High,
}

impl ProcessPriority {
    pub fn parse_from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "high" => ProcessPriority::High,
            "normal" => ProcessPriority::Normal,
            _ => ProcessPriority::AboveNormal,
        }
    }

    #[cfg(windows)]
    pub fn to_windows_class(&self) -> u32 {
        use windows_sys::Win32::System::Threading::{
            ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        };
        match self {
            ProcessPriority::Normal => NORMAL_PRIORITY_CLASS,
            ProcessPriority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            ProcessPriority::High => HIGH_PRIORITY_CLASS,
        }
    }
}

/// Calculate optimal RAM allocation in Megabytes based on total physical system memory.
///
/// Guidelines:
/// - <= 4 GB: 2048 MB (leave OS room)
/// - <= 8 GB: 3072 MB
/// - <= 16 GB: 4096 MB (or 6144 MB for ZGC)
/// - >= 32 GB: 8192 MB (or 10240 MB for ZGC)
///
/// Capped at 8192 MB for G1 to avoid oversized Eden/Survivor GC pauses.
pub fn calculate_optimal_ram_mb(total_system_ram_mb: u64, is_zgc: bool) -> u32 {
    if total_system_ram_mb <= 4096 {
        2048
    } else if total_system_ram_mb <= 8192 {
        3072
    } else if total_system_ram_mb <= 12288 {
        4096
    } else if total_system_ram_mb <= 16384 {
        if is_zgc {
            6144
        } else {
            4096
        }
    } else if total_system_ram_mb <= 24576 {
        if is_zgc {
            8192
        } else {
            6144
        }
    } else {
        if is_zgc {
            10240
        } else {
            8192
        }
    }
}

/// Detect total physical system memory in Megabytes via OS API.
pub fn detect_system_ram_mb() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) != 0 {
                Some(status.ullTotalPhys / (1024 * 1024))
            } else {
                None
            }
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Apply process priority to a running process by its PID.
pub fn apply_process_priority(pid: u32, priority: ProcessPriority) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, SetPriorityClass, PROCESS_SET_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let success = SetPriorityClass(handle, priority.to_windows_class()) != 0;
            CloseHandle(handle);
            success
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, priority);
        true
    }
}

/// Apply process priority directly using an existing raw process handle (Windows).
#[cfg(windows)]
pub fn apply_process_handle_priority(handle: RawHandle, priority: ProcessPriority) -> bool {
    if handle.is_null() {
        return false;
    }
    use windows_sys::Win32::System::Threading::SetPriorityClass;
    unsafe { SetPriorityClass(handle as _, priority.to_windows_class()) != 0 }
}

/// Build optimized JVM arguments list combining memory, GC profile, and custom flags.
pub fn build_optimized_jvm_args(
    gc: GcProfile,
    ram_mb: u32,
    java_version: u32,
    custom_args: &[String],
) -> Vec<String> {
    let mut args = Vec::new();

    // 1. Memory sizing
    args.push(format!("-Xmx{}M", ram_mb));
    // Set Xms to half of Xmx or equal to avoid incremental heap resizes
    let xms = (ram_mb / 2).max(1024);
    args.push(format!("-Xms{}M", xms));

    // 2. GC Profile flags
    for flag in gc.to_jvm_flags(java_version) {
        if !args.contains(&flag) {
            args.push(flag);
        }
    }

    // 3. User & configuration custom args (override or add)
    for arg in custom_args {
        if !args.contains(arg) {
            args.push(arg.clone());
        }
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_profile_flags_g1_default() {
        let flags = GcProfile::G1Default.to_jvm_flags(21);
        assert!(flags.contains(&"-XX:+UseG1GC".to_string()));
        assert!(flags.contains(&"-XX:+UnlockExperimentalVMOptions".to_string()));
    }

    #[test]
    fn test_gc_profile_flags_g1_tuned() {
        let flags = GcProfile::G1Tuned.to_jvm_flags(21);
        assert!(flags.contains(&"-XX:+UseG1GC".to_string()));
        assert!(flags.contains(&"-XX:MaxGCPauseMillis=20".to_string()));
        assert!(flags.contains(&"-XX:+ParallelRefProcEnabled".to_string()));
        assert!(flags.contains(&"-XX:+PerfDisableSharedMem".to_string()));
    }

    #[test]
    fn test_gc_profile_flags_zgc_java_21() {
        let flags = GcProfile::GenerationalZgc.to_jvm_flags(21);
        assert!(flags.contains(&"-XX:+UseZGC".to_string()));
        assert!(flags.contains(&"-XX:+ZGenerational".to_string()));
        assert!(flags.contains(&"-XX:+AlwaysPreTouch".to_string()));
    }

    #[test]
    fn test_gc_profile_flags_zgc_fallback_on_older_java() {
        // Java 17 does not support Generational ZGC, should fall back to Tuned G1
        let flags = GcProfile::GenerationalZgc.to_jvm_flags(17);
        assert!(flags.contains(&"-XX:+UseG1GC".to_string()));
        assert!(!flags.contains(&"-XX:+ZGenerational".to_string()));
    }

    #[test]
    fn test_optimal_ram_calculation() {
        assert_eq!(calculate_optimal_ram_mb(4096, false), 2048);
        assert_eq!(calculate_optimal_ram_mb(8192, false), 3072);
        assert_eq!(calculate_optimal_ram_mb(16384, false), 4096);
        assert_eq!(calculate_optimal_ram_mb(16384, true), 6144);
        assert_eq!(calculate_optimal_ram_mb(32768, false), 8192);
        assert_eq!(calculate_optimal_ram_mb(32768, true), 10240);
        assert_eq!(calculate_optimal_ram_mb(65536, false), 8192);
    }

    #[test]
    fn test_process_priority_parsing() {
        assert_eq!(
            ProcessPriority::parse_from_str("high"),
            ProcessPriority::High
        );
        assert_eq!(
            ProcessPriority::parse_from_str("above_normal"),
            ProcessPriority::AboveNormal
        );
        assert_eq!(
            ProcessPriority::parse_from_str("normal"),
            ProcessPriority::Normal
        );
        assert_eq!(
            ProcessPriority::parse_from_str("other"),
            ProcessPriority::AboveNormal
        );
    }

    #[test]
    fn test_build_optimized_jvm_args() {
        let custom = vec!["-Dwex.test=1".to_string()];
        let args = build_optimized_jvm_args(GcProfile::G1Tuned, 4096, 21, &custom);
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Xms2048M".to_string()));
        assert!(args.contains(&"-XX:MaxGCPauseMillis=20".to_string()));
        assert!(args.contains(&"-Dwex.test=1".to_string()));
    }

    #[test]
    fn test_system_ram_detection() {
        let ram = detect_system_ram_mb();
        #[cfg(windows)]
        {
            assert!(ram.is_some());
            assert!(ram.unwrap() > 1024);
        }
        #[cfg(not(windows))]
        {
            assert!(ram.is_none());
        }
    }
}
