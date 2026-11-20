//! Safe joining of untrusted relative paths (manifests, IPC arguments) onto a trusted base.

use std::path::{Component, Path, PathBuf};

use crate::error::{Result, WexError};

/// Joins `rel` onto `base`, refusing anything that could escape `base`.
///
/// A backslash is treated as a separator. Rejected: empty input or empty segments,
/// `.`/`..` segments, absolute paths, drive prefixes, `:` (drive letters, NTFS
/// alternate data streams), NUL bytes, and any non-`Normal` path component.
pub fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let bad = |why: &str| WexError::ManifestError(format!("Unsafe path {:?}: {}", rel, why));
    if rel.is_empty() {
        return Err(bad("empty"));
    }
    if rel.contains('\0') {
        return Err(bad("NUL byte"));
    }
    if rel.contains(':') {
        return Err(bad("contains ':'"));
    }
    let normalized = rel.replace('\\', "/");
    let mut out = base.to_path_buf();
    for segment in normalized.split('/') {
        if segment.is_empty() {
            return Err(bad("empty segment"));
        }
        if segment == "." || segment == ".." {
            return Err(bad("relative segment"));
        }
        let mut comps = Path::new(segment).components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(_)), None) => out.push(segment),
            _ => return Err(bad("non-normal component")),
        }
    }
    Ok(out)
}

/// True for UNC paths (`\\server\share`, `\\?\UNC\...`, `//server/share`) that would
/// trigger an outbound SMB/NTLM authentication when opened.
pub fn is_unc_path(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('/', "\\");
    s.starts_with("\\\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_paths() {
        let base = Path::new("base");
        assert_eq!(
            safe_join(base, "mods/a.jar").unwrap(),
            base.join("mods").join("a.jar")
        );
        assert_eq!(
            safe_join(base, "config\\x.json").unwrap(),
            base.join("config").join("x.json")
        );
    }

    #[test]
    fn rejects_escapes() {
        let base = Path::new("base");
        for bad in [
            "",
            "..",
            "../x",
            "a/../../x",
            "a\\..\\x",
            "/etc/passwd",
            "\\\\srv\\share\\x",
            "C:\\x",
            "C:x",
            "a/b:stream",
            "a//b",
            "a/",
            "./a",
            "a\0b",
        ] {
            assert!(safe_join(base, bad).is_err(), "should reject {:?}", bad);
        }
    }

    #[test]
    fn detects_unc() {
        assert!(is_unc_path(Path::new("\\\\server\\share\\a.png")));
        assert!(is_unc_path(Path::new("//server/share/a.png")));
        assert!(is_unc_path(Path::new("\\\\?\\UNC\\server\\share")));
        assert!(!is_unc_path(Path::new("C:\\a\\b.png")));
    }
}
