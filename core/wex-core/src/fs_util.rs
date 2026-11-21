//! Small file-system helpers shared by the installers.

use crate::error::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes `data` to `path` through a temporary file in the same folder and a rename,
/// so a crash or a full disk never leaves a truncated file at `path`: readers see
/// either the old content or the new one.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = temp_sibling(path);
    let written = (|| -> std::io::Result<()> {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(crate::download::map_io_err(e, path));
    }
    Ok(())
}

fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{}.{}.tmp", name, std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_atomic_replaces_content_and_leaves_no_temp_file() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("1.21.1.json");
        fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let names: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
    }

    #[test]
    fn write_atomic_failure_keeps_the_old_file() {
        let tmp = tempdir().unwrap();
        // The target is a directory: the rename fails, nothing is clobbered.
        let path = tmp.path().join("dir");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), b"x").unwrap();
        assert!(write_atomic(&path, b"new").is_err());
        assert!(path.join("keep").exists());
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }
}
