//! `server.properties` reading and merging.
//!
//! The file is a Java properties file. Wex only rewrites the keys it manages
//! and keeps every other line (comments, keys the player or a mod added) as is,
//! so editing the settings in Wex never resets the rest of the server config.
//! Values are written with `\uXXXX` escapes for non-ASCII text: both the UTF-8
//! and the ISO-8859-1 readers of the dedicated server decode them the same way.

use super::{Difficulty, GameMode, ServerSettings};

/// Escapes a value for a Java properties file.
pub fn escape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for (i, ch) in value.chars().enumerate() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ' ' if i == 0 => out.push_str("\\ "),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
        }
    }
    out
}

/// Reverses [`escape_value`] (and the escapes the server itself writes).
pub fn unescape_value(raw: &str) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u16; 2];
            units.extend_from_slice(c.encode_utf16(&mut buf));
            continue;
        }
        match chars.next() {
            Some('n') => units.push('\n' as u16),
            Some('r') => units.push('\r' as u16),
            Some('t') => units.push('\t' as u16),
            Some('u') => {
                let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                match u16::from_str_radix(&hex, 16) {
                    Ok(unit) if hex.len() == 4 => units.push(unit),
                    _ => units.extend(format!("u{hex}").encode_utf16()),
                }
            }
            Some(other) => {
                let mut buf = [0u16; 2];
                units.extend_from_slice(other.encode_utf16(&mut buf));
            }
            None => {}
        }
    }
    String::from_utf16_lossy(&units)
}

fn split_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return None;
    }
    let idx = trimmed.find(['=', ':'])?;
    Some((trimmed[..idx].trim(), trimmed[idx + 1..].trim_start()))
}

/// Value of `key`, unescaped, if the file has it.
pub fn read_property(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter_map(split_line)
        .filter(|(k, _)| *k == key)
        .last()
        .map(|(_, v)| unescape_value(v))
}

/// Rewrites `updates` in `existing`, keeping all other lines; missing keys are appended.
pub fn merge_properties(existing: &str, updates: &[(&'static str, String)]) -> String {
    let mut written = vec![false; updates.len()];
    let mut out = String::with_capacity(existing.len() + 256);
    for line in existing.lines() {
        if let Some((key, _)) = split_line(line) {
            if let Some(pos) = updates.iter().position(|(k, _)| *k == key) {
                if !written[pos] {
                    written[pos] = true;
                    out.push_str(&format!("{key}={}\n", escape_value(&updates[pos].1)));
                }
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    for (i, (key, value)) in updates.iter().enumerate() {
        if !written[i] {
            out.push_str(&format!("{key}={}\n", escape_value(value)));
        }
    }
    out
}

fn gamemode(g: GameMode) -> &'static str {
    match g {
        GameMode::Survival => "survival",
        GameMode::Creative => "creative",
        GameMode::Adventure => "adventure",
    }
}

fn difficulty(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Peaceful => "peaceful",
        Difficulty::Easy => "easy",
        Difficulty::Normal => "normal",
        Difficulty::Hard => "hard",
    }
}

/// The keys Wex manages, from the player's settings.
pub fn managed_properties(s: &ServerSettings) -> Vec<(&'static str, String)> {
    let b = |v: bool| if v { "true" } else { "false" }.to_string();
    vec![
        ("motd", s.name.clone()),
        ("max-players", s.max_players.to_string()),
        ("gamemode", gamemode(s.gamemode).into()),
        ("difficulty", difficulty(s.difficulty).into()),
        ("white-list", b(s.whitelist)),
        ("enforce-whitelist", b(s.whitelist)),
        ("online-mode", b(s.online_mode)),
        ("pvp", b(s.pvp)),
        ("server-port", s.port.to_string()),
        ("server-ip", s.bind_ip.clone().unwrap_or_default()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_roundtrip_keeps_cyrillic_and_specials() {
        for v in [
            "Сервер друзей",
            "a\\b",
            " leading",
            "x=y:z",
            "emoji 🎮",
            "line\nbreak",
        ] {
            assert_eq!(unescape_value(&escape_value(v)), v, "{v}");
        }
        assert_eq!(escape_value("Мир"), "\\u041c\\u0438\\u0440");
        assert!(escape_value("🎮").contains("\\ud83c\\udfae"));
    }

    #[test]
    fn merge_rewrites_managed_keys_and_keeps_others() {
        let existing = "#Minecraft server properties\n#date\nmotd=Old\nlevel-seed=123\nspawn-protection = 16\nmax-players=20\n";
        let out = merge_properties(
            existing,
            &[
                ("motd", "Новый".into()),
                ("max-players", "8".into()),
                ("pvp", "false".into()),
            ],
        );
        assert!(out.starts_with("#Minecraft server properties\n#date\n"));
        assert!(out.contains("level-seed=123\n"));
        assert!(out.contains("spawn-protection = 16\n"));
        assert!(out.contains("max-players=8\n"));
        assert!(!out.contains("max-players=20"));
        assert!(out.ends_with("pvp=false\n"));
        assert_eq!(read_property(&out, "motd").as_deref(), Some("Новый"));
        assert_eq!(read_property(&out, "level-seed").as_deref(), Some("123"));
        assert_eq!(
            read_property(&out, "spawn-protection").as_deref(),
            Some("16")
        );
    }

    #[test]
    fn duplicate_keys_collapse_to_one() {
        let out = merge_properties("motd=a\nmotd=b\n", &[("motd", "c".into())]);
        assert_eq!(out, "motd=c\n");
    }

    #[test]
    fn managed_properties_follow_settings() {
        let mut s = ServerSettings {
            whitelist: true,
            online_mode: false,
            gamemode: GameMode::Creative,
            difficulty: Difficulty::Hard,
            port: 25570,
            ..ServerSettings::default()
        };
        s.bind_ip = Some("127.0.0.1".into());
        let props = managed_properties(&s);
        let get = |k: &str| props.iter().find(|(key, _)| *key == k).unwrap().1.clone();
        assert_eq!(get("white-list"), "true");
        assert_eq!(get("enforce-whitelist"), "true");
        assert_eq!(get("online-mode"), "false");
        assert_eq!(get("gamemode"), "creative");
        assert_eq!(get("difficulty"), "hard");
        assert_eq!(get("server-port"), "25570");
        assert_eq!(get("server-ip"), "127.0.0.1");
    }
}
