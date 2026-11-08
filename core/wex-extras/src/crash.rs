use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrashKind {
    OutOfMemory,
    MissingFabricDependency {
        mod_name: String,
        required_dep: String,
    },
    IncompatibleMod {
        mod_name: String,
    },
    MixinConflict {
        mod_names: Vec<String>,
        target_class: Option<String>,
    },
    InvalidJavaVersion {
        required_java: String,
        current_java: Option<String>,
    },
    CorruptedFile {
        details: String,
    },
    GraphicsDriverOrShader {
        details: String,
    },
    Unknown {
        exit_code: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashDiagnosis {
    pub kind: CrashKind,
    pub title: String,
    pub user_advice: String,
    pub technical_summary: String,
}

const GPU_MARKERS: &[&str] = &[
    "nvoglv64.dll",
    "atio6axx.dll",
    "amdocl",
    "ig75icd64.dll",
    "ig9icd64.dll",
    "igxelpicd64.dll",
    "GLFW error",
    "GL_OUT_OF_MEMORY",
    "CompileShader",
    "net.irisshaders",
    "me.jellysquid.mods.sodium",
];

const CORRUPTION_MARKERS: &[&str] = &[
    "SessionLock",
    "already locked",
    "RegionFile",
    "Failed to read chunk",
    "ZipException",
    "invalid CEN header",
    "zip END header not found",
    "level.dat",
];

pub fn analyze_crash(game_dir: &Path, exit_code: Option<i32>) -> CrashDiagnosis {
    let combined_log = collect_recent_logs(game_dir, exit_code);
    analyze_log_text(exit_code, &combined_log)
}

pub fn analyze_log_text(exit_code: Option<i32>, log_text: &str) -> CrashDiagnosis {
    let snippet = build_snippet(exit_code, log_text);

    // 1. Missing Fabric Dependency
    // e.g.: Mod 'iris' (1.7.0) requires 'sodium' (0.5.8), but only 0.5.2 is present!
    // or: Mod 'indium' requires 'sodium'
    let dep_re = Regex::new(
        r"(?i)Mod '(?P<mod>[^']+)'(?: \([^)]+\))? requires (?:version )?'(?P<dep>[^']+)'",
    )
    .expect("valid regex");

    if let Some(caps) = dep_re.captures(log_text) {
        let mod_name = caps["mod"].to_string();
        let dep = caps["dep"].to_string();
        return CrashDiagnosis {
            kind: CrashKind::MissingFabricDependency {
                mod_name: mod_name.clone(),
                required_dep: dep.clone(),
            },
            title: format!("Отсутствует зависимость для «{}»", mod_name),
            user_advice: format!(
                "Мод «{}» требует обязательной установки «{}». Установите недостающую библиотеку или удалите мод во вкладке Мастерская.",
                mod_name, dep
            ),
            technical_summary: snippet,
        };
    }

    // 2. Incompatible Mods (Fabric Loader FormattedException)
    // e.g.: Incompatible mods found!
    //       net.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!
    //       - Mod 'foo' is incompatible with mod 'bar'
    let incomp_re = Regex::new(
        r"(?i)(?:- Mod '(?P<mod>[^']+)' is incompatible with|Incompatible mod: '(?P<mod2>[^']+)')",
    )
    .expect("valid regex");

    if let Some(caps) = incomp_re.captures(log_text) {
        let mod_name = caps
            .name("mod")
            .or_else(|| caps.name("mod2"))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "Неизвестный мод".to_string());

        return CrashDiagnosis {
            kind: CrashKind::IncompatibleMod {
                mod_name: mod_name.clone(),
            },
            title: format!("Несовместимый мод: «{}»", mod_name),
            user_advice: format!(
                "Обнаружен несовместимый мод «{}». Удалите или обновите его во вкладке Мастерская.",
                mod_name
            ),
            technical_summary: snippet,
        };
    }

    if log_text.contains("Incompatible mods found") || log_text.contains("Mod resolution failed") {
        return CrashDiagnosis {
            kind: CrashKind::IncompatibleMod {
                mod_name: "Несовместимый набор модов".to_string(),
            },
            title: "Конфликт модов при запуске".to_string(),
            user_advice: "Проверьте добавленные вручную сторонние моды во вкладке Мастерская или обновите сборку."
                .to_string(),
            technical_summary: snippet,
        };
    }

    // 3. Mixin Conflict
    // e.g.: MixinApplyError: Mixin [aura-client.mixins.json:GameRendererMixin] from phase [DEFAULT] in config [...] FAILED during APPLY
    // e.g.: Critical injection failure: ... from mod aura-client
    let mixin_re = Regex::new(
        r"(?i)(?:Mixin \[(?P<config>[^:]+):(?P<target>[^\]]+)\]|from mod (?P<mod>[a-zA-Z0-9_-]+)|handler\$[a-z0-9]+\$(?P<mod2>[a-zA-Z0-9_-]+)\$)",
    ).expect("valid regex");

    let is_mixin = log_text.contains("MixinApplyError")
        || log_text.contains("MixinTransformerError")
        || log_text.contains("Mixin prepare failed")
        || log_text.contains("Mixin apply failed")
        || log_text.contains("Critical injection failure");

    if is_mixin {
        let mut mods = Vec::new();
        let mut target = None;

        for caps in mixin_re.captures_iter(log_text) {
            if let Some(m) = caps.name("mod").or_else(|| caps.name("mod2")) {
                let m_str = m.as_str().to_string();
                if !mods.contains(&m_str) {
                    mods.push(m_str);
                }
            }
            if let Some(cfg) = caps.name("config") {
                let cfg_name = cfg
                    .as_str()
                    .trim_end_matches(".mixins.json")
                    .trim_end_matches(".json")
                    .to_string();
                if !mods.contains(&cfg_name) {
                    mods.push(cfg_name);
                }
            }
            if target.is_none() {
                if let Some(t) = caps.name("target") {
                    target = Some(t.as_str().to_string());
                }
            }
        }

        let mod_desc = if !mods.is_empty() {
            mods.join(", ")
        } else {
            "сторонний мод".to_string()
        };

        return CrashDiagnosis {
            kind: CrashKind::MixinConflict {
                mod_names: mods,
                target_class: target,
            },
            title: format!("Конфликт Mixin (мод: {})", mod_desc),
            user_advice: format!(
                "Сбой внедрения кода (Mixin) в моде «{}». Проверьте совместимость мода с текущей версией Minecraft или отключите его.",
                mod_desc
            ),
            technical_summary: snippet,
        };
    }

    // 4. Invalid Java version
    // e.g.: java.lang.UnsupportedClassVersionError: ... (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0
    let java_re = Regex::new(
        r"(?i)UnsupportedClassVersionError:.*\(class file version (?P<req>[0-9.]+)\).*only recognizes class file versions up to (?P<cur>[0-9.]+)",
    ).expect("valid regex");

    if let Some(caps) = java_re.captures(log_text) {
        let req_raw = caps["req"].split('.').next().unwrap_or("0");
        let cur_raw = caps["cur"].split('.').next().unwrap_or("0");

        let req_java = class_ver_to_java(req_raw.parse().unwrap_or(0));
        let cur_java = class_ver_to_java(cur_raw.parse().unwrap_or(0));

        return CrashDiagnosis {
            kind: CrashKind::InvalidJavaVersion {
                required_java: req_java.clone(),
                current_java: Some(cur_java),
            },
            title: "Несовместимая версия Java".to_string(),
            user_advice: format!(
                "Игра или установленный мод требуют {} (ваша текущая версия устарела). Выберите нужную версию Java в Настройках лаунчера.",
                req_java
            ),
            technical_summary: snippet,
        };
    }

    if log_text.contains("UnsupportedClassVersionError")
        || log_text.contains("has been compiled by a more recent version of the Java Runtime")
    {
        return CrashDiagnosis {
            kind: CrashKind::InvalidJavaVersion {
                required_java: "Java 21".to_string(),
                current_java: None,
            },
            title: "Несовместимая версия Java".to_string(),
            user_advice: "Для работы сборки требуется более новая версия Java (рекомендуется Java 21). Переключите среду в Настройках лаунчера.".to_string(),
            technical_summary: snippet,
        };
    }

    // 5. Out of Memory (OOM)
    let is_oom_text = log_text.contains("OutOfMemoryError")
        || log_text.contains("Out of Memory Error")
        || log_text.contains("There is insufficient memory for the Java Runtime Environment")
        || log_text.contains("Java heap space");

    if is_oom_text || exit_code == Some(-805306369) {
        return CrashDiagnosis {
            kind: CrashKind::OutOfMemory,
            title: "Нехватка оперативной памяти (ОЗУ)".to_string(),
            user_advice: "Увеличьте выделенный объём памяти в Настройках лаунчера (рекомендуется 6–8 ГБ) или закройте фоновые приложения.".to_string(),
            technical_summary: snippet,
        };
    }

    // 6. Graphics Driver / Shaders / OpenGL
    let is_gpu = GPU_MARKERS
        .iter()
        .any(|marker| log_text.to_lowercase().contains(&marker.to_lowercase()))
        || exit_code == Some(-1073741819)
        || exit_code == Some(-1073740791);

    if is_gpu {
        return CrashDiagnosis {
            kind: CrashKind::GraphicsDriverOrShader {
                details: "GPU/Shader driver crash".to_string(),
            },
            title: "Сбой видеодрайвера или шейдеров".to_string(),
            user_advice: "Отключите шейдеры во вкладке Мастерская, обновите драйвер видеокарты или переключите графический профиль на «Слабый ПК».".to_string(),
            technical_summary: snippet,
        };
    }

    // 7. Corrupted File / World corruption
    let is_corrupt = CORRUPTION_MARKERS
        .iter()
        .any(|marker| log_text.to_lowercase().contains(&marker.to_lowercase()));

    if is_corrupt {
        return CrashDiagnosis {
            kind: CrashKind::CorruptedFile {
                details: "Corrupted chunk, zip or lock file".to_string(),
            },
            title: "Ошибка чтения файлов игры или мира".to_string(),
            user_advice: "Обнаружено повреждение файлов мира или сборки. Если ошибка возникает при загрузке мира — восстановите его из резервной копии во вкладке Мастерская.".to_string(),
            technical_summary: snippet,
        };
    }

    // 8. Unknown / Generic crash
    let code_str = exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "неизвестно".to_string());
    CrashDiagnosis {
        kind: CrashKind::Unknown { exit_code },
        title: format!("Игра завершилась с ошибкой (код {})", code_str),
        user_advice: "Нажмите «Показать лог», чтобы просмотреть технические детали, или обратитесь за поддержкой к разработчикам.".to_string(),
        technical_summary: snippet,
    }
}

fn class_ver_to_java(class_ver: u32) -> String {
    match class_ver {
        65 => "Java 21 (class 65.0)".to_string(),
        61 => "Java 17 (class 61.0)".to_string(),
        60 => "Java 16 (class 60.0)".to_string(),
        55 => "Java 11 (class 55.0)".to_string(),
        52 => "Java 8 (class 52.0)".to_string(),
        v if v >= 45 => format!("Java {} (class {}.0)", v - 44, v),
        _ => "более новую версию Java".to_string(),
    }
}

fn build_snippet(exit_code: Option<i32>, log_text: &str) -> String {
    let mut out = String::new();
    if let Some(code) = exit_code {
        out.push_str(&format!("ExitCode: {}\n", code));
    }
    let lines: Vec<&str> = log_text.lines().collect();
    if lines.is_empty() {
        return out;
    }

    let tail_count = 80.min(lines.len());
    let tail = &lines[lines.len() - tail_count..];
    out.push_str(&tail.join("\n"));
    if out.len() > 3000 {
        out.truncate(3000);
        out.push_str("\n... [обрезано]");
    }
    out
}

fn collect_recent_logs(game_dir: &Path, exit_code: Option<i32>) -> String {
    let mut combined = String::new();
    if let Some(code) = exit_code {
        combined.push_str(&format!("ExitCode: {}\n", code));
    }

    // 1. crash-reports/*.txt (latest)
    let crash_dir = game_dir.join("crash-reports");
    if crash_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "txt")
                        .unwrap_or(false)
                })
                .collect();
            files.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            if let Some(latest) = files.last() {
                if let Ok(text) = fs::read_to_string(latest.path()) {
                    combined.push_str("\n--- CrashReport ---\n");
                    combined.push_str(&text);
                }
            }
        }
    }

    // 2. hs_err_pid*.log
    if let Ok(entries) = fs::read_dir(game_dir) {
        let mut hs_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("hs_err_pid"))
            .collect();
        hs_files.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        if let Some(latest) = hs_files.last() {
            if let Ok(text) = fs::read_to_string(latest.path()) {
                combined.push_str("\n--- NativeCrash (hs_err) ---\n");
                combined.push_str(&text);
            }
        }
    }

    // 3. logs/latest.log
    let latest_log = game_dir.join("logs").join("latest.log");
    if latest_log.is_file() {
        if let Ok(text) = fs::read_to_string(&latest_log) {
            combined.push_str("\n--- latest.log ---\n");
            combined.push_str(&text);
        }
    }

    combined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_out_of_memory_error() {
        let log = r#"
[23:01:05] [Render thread/INFO]: Starting Minecraft...
[23:01:10] [Render thread/ERROR]: Uncaught exception in thread "Render thread"
java.lang.OutOfMemoryError: Java heap space
	at net.minecraft.world.chunk.Chunk.<init>(Chunk.java:120)
"#;
        let diag = analyze_log_text(Some(1), log);
        assert!(matches!(diag.kind, CrashKind::OutOfMemory));
        assert!(diag.title.contains("ОЗУ"));
    }

    #[test]
    fn test_diagnose_exit_code_805306369() {
        let diag = analyze_log_text(Some(-805306369), "");
        assert!(matches!(diag.kind, CrashKind::OutOfMemory));
    }

    #[test]
    fn test_diagnose_missing_fabric_dependency() {
        let log = r#"
net.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!
A potential solution has been determined, this may resolve your issue:
	 - Install sodium, version 0.5.8 or later.
More details:
	 - Mod 'iris' (1.7.0) requires 'sodium' (0.5.8), but only 0.5.2 is present!
"#;
        let diag = analyze_log_text(Some(1), log);
        match diag.kind {
            CrashKind::MissingFabricDependency {
                mod_name,
                required_dep,
            } => {
                assert_eq!(mod_name, "iris");
                assert_eq!(required_dep, "sodium");
            }
            other => panic!("Unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn test_diagnose_incompatible_mod() {
        let log = r#"
Incompatible mods found!
net.fabricmc.loader.impl.FormattedException: Incompatible mods found!
	- Mod 'optifine' is incompatible with mod 'sodium'
"#;
        let diag = analyze_log_text(Some(1), log);
        match diag.kind {
            CrashKind::IncompatibleMod { mod_name } => {
                assert_eq!(mod_name, "optifine");
            }
            other => panic!("Unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn test_diagnose_mixin_conflict() {
        let log = r#"
---- Minecraft Crash Report ----
Description: Initializing game
org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: An unexpected critical error was encountered
	at knot//net.minecraft.client.render.GameRenderer.handler$zle000$aura_client$aura_afterRender(GameRenderer.java:3125)
Caused by: org.spongepowered.asm.mixin.throwables.MixinApplyError: Mixin [aura-client.mixins.json:GameRendererMixin] from phase [DEFAULT] FAILED during APPLY
"#;
        let diag = analyze_log_text(Some(1), log);
        match diag.kind {
            CrashKind::MixinConflict { mod_names, .. } => {
                assert!(
                    mod_names.contains(&"aura-client".to_string())
                        || mod_names.contains(&"aura_client".to_string())
                );
            }
            other => panic!("Unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn test_diagnose_invalid_java_version() {
        let log = r#"
Exception in thread "main" java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0
	at java.lang.ClassLoader.defineClass1(Native Method)
"#;
        let diag = analyze_log_text(Some(1), log);
        match diag.kind {
            CrashKind::InvalidJavaVersion { required_java, .. } => {
                assert!(required_java.contains("Java 21"));
            }
            other => panic!("Unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn test_diagnose_gpu_crash() {
        let log = r#"
# A fatal error has been detected by the Java Runtime Environment:
#
#  EXCEPTION_ACCESS_VIOLATION (0xc0000005) at pc=0x00007ff812345678, pid=1234, tid=5678
#
# JRE version: OpenJDK Runtime Environment (21.0.3)
# Java VM: OpenJDK 64-Bit Server VM (21.0.3, mixed mode, tiered, compressed oops)
# Problematic frame:
# C  [nvoglv64.dll+0x9abcde]
"#;
        let diag = analyze_log_text(Some(-1073741819), log);
        assert!(matches!(
            diag.kind,
            CrashKind::GraphicsDriverOrShader { .. }
        ));
        assert!(diag.title.contains("видеодрайвера"));
    }

    #[test]
    fn test_diagnose_world_corruption() {
        let log = r#"
[22:00:10] [Server thread/ERROR]: Encountered an unexpected exception
net.minecraft.world.level.storage.SessionLock$CannotLockException: C:\game\saves\NewWorld\session.lock: already locked
"#;
        let diag = analyze_log_text(Some(1), log);
        assert!(matches!(diag.kind, CrashKind::CorruptedFile { .. }));
        assert!(diag.title.contains("файлов"));
    }
}
