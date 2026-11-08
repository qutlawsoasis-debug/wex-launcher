use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("Неизвестный пресет графики: «{0}»")]
    UnknownPreset(String),
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetTier {
    Low,
    Medium,
    High,
    Ultra,
}

impl PresetTier {
    pub fn as_key(&self) -> &'static str {
        match self {
            PresetTier::Low => "Low",
            PresetTier::Medium => "Medium",
            PresetTier::High => "High",
            PresetTier::Ultra => "Ultra",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PresetTier::Low => "Слабый ПК",
            PresetTier::Medium => "Средний",
            PresetTier::High => "Высокий",
            PresetTier::Ultra => "Ультра",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphicsPreset {
    pub key: String,
    pub name: String,
    pub hardware: String,
    pub render_distance: u32,
    pub simulation_distance: u32,
    pub graphics_mode: String, // "0" (fast) or "1" (fancy)
    pub particles: String,     // "0" (all), "1" (decreased), "2" (minimal)
    pub clouds: String,        // "\"false\"", "\"fast\"", "\"true\""
    pub entity_distance_scaling: f64,
    pub biome_blend_radius: u32,
    pub mipmap_levels: u32,
    pub ao: u32, // 0, 1, 2
    pub ram_mb: u32,
    pub jvm_args: Vec<String>,
    pub shader_pack: String,
    pub shader_packs_fallback: Vec<String>,
    pub shader_options: HashMap<String, String>,
    pub resource_packs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetApplyResult {
    pub preset_key: String,
    pub chosen_shader: String,
    pub final_ram_mb: u32,
    pub jvm_args: Vec<String>,
    pub written_options: HashMap<String, String>,
    pub warning: Option<String>,
}

pub fn get_default_presets() -> HashMap<String, GraphicsPreset> {
    let mut map = HashMap::new();

    let mut low_opts = HashMap::new();
    low_opts.insert("profile".to_string(), "LOW".to_string());

    map.insert(
        "Low".to_string(),
        GraphicsPreset {
            key: "Low".to_string(),
            name: "Слабый ПК".to_string(),
            hardware: "GTX 1050/1650, RX 550/560, Vega, Intel HD".to_string(),
            render_distance: 6,
            simulation_distance: 5,
            graphics_mode: "0".to_string(),
            particles: "1".to_string(),
            clouds: "\"false\"".to_string(),
            entity_distance_scaling: 0.75,
            biome_blend_radius: 1,
            mipmap_levels: 2,
            ao: 1,
            ram_mb: 6144,
            jvm_args: vec![
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=16M".to_string(),
            ],
            shader_pack: "ComplementaryReimagined_r5.9.3.zip".to_string(),
            shader_packs_fallback: vec![
                "ComplementaryReimagined_r5.9.3.zip".to_string(),
                "Builders-QOL.zip".to_string(),
            ],
            shader_options: low_opts,
            resource_packs: vec![
                "Sodium_Russian_Translation.zip".to_string(),
                "AnimatedDoors-RP-2.0.0.zip".to_string(),
                "unobtrusive-grass-1.0.1.zip".to_string(),
                "Drigo-3D-Lanterns-x-Punchy.zip".to_string(),
                "Modded-Swords-x-Punchy-1.20.1-v1.3.2.zip".to_string(),
                "FreshAnimations_v1.10.4.zip".to_string(),
                "Universal Bushy Leaves.zip".to_string(),
            ],
        },
    );

    map.insert(
        "Medium".to_string(),
        GraphicsPreset {
            key: "Medium".to_string(),
            name: "Средний".to_string(),
            hardware: "GTX 1660, RTX 2060, RX 6600".to_string(),
            render_distance: 8,
            simulation_distance: 6,
            graphics_mode: "1".to_string(),
            particles: "1".to_string(),
            clouds: "\"true\"".to_string(),
            entity_distance_scaling: 1.0,
            biome_blend_radius: 3,
            mipmap_levels: 3,
            ao: 2,
            ram_mb: 8192,
            jvm_args: vec![
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=16M".to_string(),
            ],
            shader_pack: "Derivative.zip".to_string(),
            shader_packs_fallback: vec![
                "Derivative.zip".to_string(),
                "ComplementaryReimagined_r5.9.3.zip".to_string(),
            ],
            shader_options: HashMap::new(),
            resource_packs: vec![
                "Sodium_Russian_Translation.zip".to_string(),
                "AnimatedDoors-RP-2.0.0.zip".to_string(),
                "unobtrusive-grass-1.0.1.zip".to_string(),
                "Drigo-3D-Lanterns-x-Punchy.zip".to_string(),
                "Modded-Swords-x-Punchy-1.20.1-v1.3.2.zip".to_string(),
                "FreshAnimations_v1.10.4.zip".to_string(),
                "Universal Bushy Leaves.zip".to_string(),
                "SPBR.zip".to_string(),
            ],
        },
    );

    map.insert(
        "High".to_string(),
        GraphicsPreset {
            key: "High".to_string(),
            name: "Высокий".to_string(),
            hardware: "RTX 30".to_string(),
            render_distance: 8,
            simulation_distance: 6,
            graphics_mode: "1".to_string(),
            particles: "0".to_string(),
            clouds: "\"true\"".to_string(),
            entity_distance_scaling: 1.25,
            biome_blend_radius: 5,
            mipmap_levels: 4,
            ao: 2,
            ram_mb: 8192,
            jvm_args: vec![
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=16M".to_string(),
            ],
            shader_pack: "Noble_v1.9.9.zip".to_string(),
            shader_packs_fallback: vec![
                "Noble_v1.9.9.zip".to_string(),
                "Derivative.zip".to_string(),
            ],
            shader_options: HashMap::new(),
            resource_packs: vec![
                "Sodium_Russian_Translation.zip".to_string(),
                "AnimatedDoors-RP-2.0.0.zip".to_string(),
                "unobtrusive-grass-1.0.1.zip".to_string(),
                "Drigo-3D-Lanterns-x-Punchy.zip".to_string(),
                "Modded-Swords-x-Punchy-1.20.1-v1.3.2.zip".to_string(),
                "FreshAnimations_v1.10.4.zip".to_string(),
                "Universal Bushy Leaves.zip".to_string(),
                "SPBR.zip".to_string(),
                "Classic-3D.zip".to_string(),
                "Even-Better-Enchants.zip".to_string(),
            ],
        },
    );

    map.insert(
        "Ultra".to_string(),
        GraphicsPreset {
            key: "Ultra".to_string(),
            name: "Ультра".to_string(),
            hardware: "RTX 40 и 50".to_string(),
            render_distance: 10,
            simulation_distance: 8,
            graphics_mode: "1".to_string(),
            particles: "0".to_string(),
            clouds: "\"true\"".to_string(),
            entity_distance_scaling: 1.5,
            biome_blend_radius: 7,
            mipmap_levels: 4,
            ao: 2,
            ram_mb: 10240,
            jvm_args: vec![
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+UseG1GC".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ],
            shader_pack: "Bliss_v2.1.2.zip".to_string(),
            shader_packs_fallback: vec![
                "Bliss_v2.1.2.zip".to_string(),
                "Sildurs-Vibrant-Shaders-Extreme-VL.zip".to_string(),
                "ComplementaryUnbound_r5.9.3.zip".to_string(),
            ],
            shader_options: HashMap::new(),
            resource_packs: vec![
                "Sodium_Russian_Translation.zip".to_string(),
                "AnimatedDoors-RP-2.0.0.zip".to_string(),
                "unobtrusive-grass-1.0.1.zip".to_string(),
                "Drigo-3D-Lanterns-x-Punchy.zip".to_string(),
                "Modded-Swords-x-Punchy-1.20.1-v1.3.2.zip".to_string(),
                "FreshAnimations_v1.10.4.zip".to_string(),
                "Universal Bushy Leaves.zip".to_string(),
                "SPBR.zip".to_string(),
                "Mels-3D-Items.zip".to_string(),
                "Even-Better-Enchants.zip".to_string(),
            ],
        },
    );

    map
}

pub fn apply_preset(
    game_dir: &Path,
    preset_key: &str,
    pool_dirs: &[PathBuf],
) -> Result<PresetApplyResult, PresetError> {
    let presets = get_default_presets();
    let preset = presets
        .get(preset_key)
        .ok_or_else(|| PresetError::UnknownPreset(preset_key.to_string()))?;

    fs::create_dir_all(game_dir)?;

    // 1. Synchronize shader from pool
    let shaderpacks_dir = game_dir.join("shaderpacks");
    fs::create_dir_all(&shaderpacks_dir)?;

    let mut candidate_shaders = vec![preset.shader_pack.clone()];
    candidate_shaders.extend(preset.shader_packs_fallback.clone());

    let mut chosen_shader = "OFF".to_string();
    let mut warning = None;

    // Search for shader candidate in shaderpacks/ or pool_dirs
    for candidate in &candidate_shaders {
        let in_shaderpacks = shaderpacks_dir.join(candidate);
        if in_shaderpacks.is_file() {
            chosen_shader = candidate.clone();
            break;
        }

        // Search in pool dirs
        let mut found_in_pool = None;
        for pool in pool_dirs {
            let pool_file = pool.join(candidate);
            if pool_file.is_file() {
                found_in_pool = Some(pool_file);
                break;
            }
        }

        if let Some(src) = found_in_pool {
            let _ = fs::copy(&src, &in_shaderpacks);
            chosen_shader = candidate.clone();
            break;
        }
    }

    if chosen_shader == "OFF" {
        warning = Some(format!(
            "Шейдерпак «{}» не найден в shaderpacks_pool. Шейдеры отключены.",
            preset.shader_pack
        ));
    }

    // 2. Configure Iris properties & optionsshaders.txt
    let config_dir = game_dir.join("config");
    fs::create_dir_all(&config_dir)?;

    let enable_shaders = chosen_shader != "OFF";
    let iris_content = format!(
        "enableShaders={}\nshaderPack={}\n",
        enable_shaders, chosen_shader
    );
    fs::write(config_dir.join("iris.properties"), iris_content)?;

    let opt_shaders_content = format!(
        "currentShaderPacks={}\nshaderPack={}\n",
        chosen_shader, chosen_shader
    );
    fs::write(game_dir.join("optionsshaders.txt"), opt_shaders_content)?;

    // 2.1 Write shaderOptions to <chosen>.txt
    if enable_shaders && !preset.shader_options.is_empty() {
        let txt_path = shaderpacks_dir.join(format!("{}.txt", chosen_shader));
        let mut lines = if txt_path.is_file() {
            fs::read_to_string(&txt_path)
                .unwrap_or_default()
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        for (k, v) in &preset.shader_options {
            let mut found = false;
            for line in &mut lines {
                if let Some(idx) = line.find('=') {
                    let key = line[..idx].trim();
                    if key.eq_ignore_ascii_case(k) {
                        *line = format!("{}={}", k, v);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                lines.push(format!("{}={}", k, v));
            }
        }
        let _ = fs::write(txt_path, lines.join("\n") + "\n");
    }

    // 3. Update options.txt preserving existing options
    let options_path = game_dir.join("options.txt");
    let mut options_lines = if options_path.is_file() {
        fs::read_to_string(&options_path)
            .unwrap_or_default()
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![
            "version:3465".to_string(),
            "lang:ru_ru".to_string(),
            "guiScale:2".to_string(),
            "fullscreen:true".to_string(),
        ]
    };

    let mut target_values = HashMap::new();
    target_values.insert(
        "renderDistance".to_string(),
        preset.render_distance.to_string(),
    );
    target_values.insert(
        "simulationDistance".to_string(),
        preset.simulation_distance.to_string(),
    );
    target_values.insert("graphicsMode".to_string(), preset.graphics_mode.clone());
    target_values.insert("particles".to_string(), preset.particles.clone());
    target_values.insert("clouds".to_string(), preset.clouds.clone());
    target_values.insert(
        "entityDistanceScaling".to_string(),
        format!("{:.2}", preset.entity_distance_scaling),
    );
    target_values.insert(
        "biomeBlendRadius".to_string(),
        preset.biome_blend_radius.to_string(),
    );
    target_values.insert("mipmapLevels".to_string(), preset.mipmap_levels.to_string());
    target_values.insert("ao".to_string(), preset.ao.to_string());

    let mut applied_keys = std::collections::HashSet::new();

    for line in &mut options_lines {
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_string();
            if let Some(new_val) = target_values.get(&key) {
                *line = format!("{}:{}", key, new_val);
                applied_keys.insert(key);
            }
        }
    }

    for (k, v) in &target_values {
        if !applied_keys.contains(k) {
            options_lines.push(format!("{}:{}", k, v));
        }
    }

    let tmp_options = game_dir.join("options.txt.tmp");
    fs::write(&tmp_options, options_lines.join("\n") + "\n")?;
    fs::rename(tmp_options, &options_path)?;

    Ok(PresetApplyResult {
        preset_key: preset_key.to_string(),
        chosen_shader,
        final_ram_mb: preset.ram_mb,
        jvm_args: preset.jvm_args.clone(),
        written_options: target_values,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_apply_preset_preserves_unrelated_options_txt_lines() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path();

        let initial_options = "\
version:3465
lang:ru_ru
soundCategory_master:0.8
customKey_aura_zoom:key.keyboard.c
renderDistance:12
fov:90
";
        fs::write(game_dir.join("options.txt"), initial_options).unwrap();

        let res = apply_preset(game_dir, "Low", &[]).unwrap();
        assert_eq!(res.preset_key, "Low");
        assert_eq!(res.written_options["renderDistance"], "6");
        assert_eq!(res.written_options["graphicsMode"], "0");

        let updated_text = fs::read_to_string(game_dir.join("options.txt")).unwrap();

        // Must preserve unrelated lines
        assert!(updated_text.contains("lang:ru_ru"));
        assert!(updated_text.contains("soundCategory_master:0.8"));
        assert!(updated_text.contains("customKey_aura_zoom:key.keyboard.c"));
        assert!(updated_text.contains("fov:90"));

        // Must update renderDistance to 6
        assert!(updated_text.contains("renderDistance:6"));
    }

    #[test]
    fn test_apply_preset_copies_shader_from_pool() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path().join("game");
        let pool_dir = dir.path().join("pool");
        fs::create_dir_all(&pool_dir).unwrap();

        // Create dummy shader in pool
        let shader_name = "ComplementaryReimagined_r5.9.3.zip";
        fs::write(pool_dir.join(shader_name), b"dummy-zip-content").unwrap();

        let res = apply_preset(&game_dir, "Low", &[pool_dir]).unwrap();
        assert_eq!(res.chosen_shader, shader_name);

        // Check shader was copied into game_dir/shaderpacks/
        assert!(game_dir.join("shaderpacks").join(shader_name).is_file());

        // Check iris.properties
        let iris = fs::read_to_string(game_dir.join("config").join("iris.properties")).unwrap();
        assert!(iris.contains("enableShaders=true"));
        assert!(iris.contains(&format!("shaderPack={}", shader_name)));

        // Check shaderOptions (profile=LOW)
        let shader_txt = fs::read_to_string(
            game_dir
                .join("shaderpacks")
                .join(format!("{}.txt", shader_name)),
        )
        .unwrap();
        assert!(shader_txt.contains("profile=LOW"));
    }

    #[test]
    fn test_apply_preset_fallback_when_pool_empty() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path();

        let res = apply_preset(game_dir, "Ultra", &[]).unwrap();
        assert_eq!(res.chosen_shader, "OFF");
        assert!(res.warning.is_some());

        let iris = fs::read_to_string(game_dir.join("config").join("iris.properties")).unwrap();
        assert!(iris.contains("enableShaders=false"));
        assert!(iris.contains("shaderPack=OFF"));
    }
}
