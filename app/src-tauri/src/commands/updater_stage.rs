//! Own the staging directory for its whole lifetime and verify the saved bytes at use.
use super::{build_installer_plan, verify_artifact_signature, InstallerExecutionPlan};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct StagedUpdate {
    pub installer_path: PathBuf,
    pub version: String,
    pub temp_dir: PathBuf,
    directory: Arc<tempfile::TempDir>,
    artifact_path: PathBuf,
    signature: String,
    trusted_key: String,
    installer_hash: [u8; 32],
}

impl StagedUpdate {
    /// An asynchronous installer may still need its package after the launcher exits.
    pub fn preserve_for_installer(self) {
        match Arc::try_unwrap(self.directory) {
            Ok(directory) => {
                let _ = directory.keep();
            }
            Err(directory) => {
                std::mem::forget(directory);
            }
        }
    }

    pub fn new(
        bytes: &[u8],
        signature: &str,
        trusted_key: &str,
        version: &str,
        extension: &str,
    ) -> Result<Self, String> {
        super::validate_update_version(version)?;
        verify_artifact_signature(bytes, signature, trusted_key)?;
        if !matches!(extension, "exe" | "msi" | "zip") {
            return Err("Unsupported update artifact".into());
        }
        // No version, URL basename or other IPC text is ever part of the directory name.
        let directory = Arc::new(
            tempfile::Builder::new()
                .prefix("wex-update-")
                .tempdir()
                .map_err(|e| e.to_string())?,
        );
        let temp_dir = directory.path().to_path_buf();
        let artifact_path = temp_dir.join(format!("artifact.{extension}"));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&artifact_path)
            .and_then(|mut f| f.write_all(bytes))
            .map_err(|e| e.to_string())?;
        let (installer_path, installer_hash) = if extension == "zip" {
            save_signed_zip_installer(bytes, &temp_dir)?
        } else {
            (artifact_path.clone(), Sha256::digest(bytes).into())
        };
        let stage = Self {
            installer_path,
            version: version.into(),
            temp_dir,
            directory,
            artifact_path,
            signature: signature.into(),
            trusted_key: trusted_key.into(),
            installer_hash,
        };
        stage.checked_path(&stage.installer_path)?;
        Ok(stage)
    }

    fn checked_path(&self, path: &Path) -> Result<(), String> {
        if self.temp_dir != self.directory.path() {
            return Err("Update directory mismatch".into());
        }
        let root = self
            .directory
            .path()
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let relative = path
            .strip_prefix(self.directory.path())
            .map_err(|_| "Update path escapes staging directory")?;
        let mut current = self.directory.path().to_path_buf();
        reject_link(&current)?;
        for component in relative.components() {
            if !matches!(component, std::path::Component::Normal(_)) {
                return Err("Invalid update path".into());
            }
            current.push(component);
            reject_link(&current)?;
        }
        if !path
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(root)
        {
            return Err("Update path escapes staging directory".into());
        }
        Ok(())
    }

    /// Keep handles alive through spawn. On Windows they deny replacement/writes/deletion.
    pub fn execute<F>(&self, handler: F) -> Result<InstallerExecutionPlan, String>
    where
        F: FnOnce(&Path, &[String]) -> Result<(), String>,
    {
        super::validate_update_version(&self.version)?;
        let _directory_lock = pin_directory(self.directory.path())?;
        self.checked_path(&self.artifact_path)?;
        self.checked_path(&self.installer_path)?;
        let mut artifact = locked_read(&self.artifact_path)?;
        let mut installer = locked_read(&self.installer_path)?;
        let mut bytes = Vec::new();
        artifact
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        verify_artifact_signature(&bytes, &self.signature, &self.trusted_key)?;
        bytes.clear();
        installer
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let saved_hash: [u8; 32] = Sha256::digest(&bytes).into();
        if saved_hash != self.installer_hash {
            return Err("Prepared installer changed after signature verification".into());
        }
        let plan = build_installer_plan(&self.installer_path, &self.version)?;
        handler(&plan.executable, &plan.args)?;
        Ok(plan)
    }
}

fn save_signed_zip_installer(
    bytes: &[u8],
    directory: &Path,
) -> Result<(PathBuf, [u8; 32]), String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() > 128 {
        return Err("Update archive has too many entries".into());
    }
    let mut selected = None;
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.enclosed_name().ok_or("Invalid update archive path")?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Update archive contains a link".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Update archive too large")?;
        if total > 512 * 1024 * 1024 {
            return Err("Update archive too large".into());
        }
        let extension = name
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !entry.is_file() || !matches!(extension.as_str(), "exe" | "msi") {
            continue;
        }
        if selected.is_some() {
            return Err("Ambiguous update archive: multiple installers".into());
        }
        let mut installer = Vec::new();
        (&mut entry)
            .take(512 * 1024 * 1024 + 1)
            .read_to_end(&mut installer)
            .map_err(|e| e.to_string())?;
        if installer.len() > 512 * 1024 * 1024 {
            return Err("Update installer too large".into());
        }
        // Hash authenticated bytes in memory, never an attacker-modifiable extracted file.
        let hash: [u8; 32] = Sha256::digest(&installer).into();
        let path = directory.join(format!("installer.{extension}"));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut f| f.write_all(&installer))
            .map_err(|e| e.to_string())?;
        selected = Some((path, hash));
    }
    selected.ok_or_else(|| "No installer in update archive".into())
}

fn reject_link(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    reject_link_metadata(&metadata)
}

fn reject_link_metadata(metadata: &std::fs::Metadata) -> Result<(), String> {
    if metadata.file_type().is_symlink() {
        return Err("Links are forbidden in update staging".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Reparse points are forbidden in update staging".into());
        }
    }
    Ok(())
}

fn locked_read(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ only; keep the exact verified artifact pinned.
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT: never follow a raced link.
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    reject_link_metadata(&file.metadata().map_err(|e| e.to_string())?)?;
    Ok(file)
}

fn pin_directory(path: &Path) -> Result<Option<File>, String> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .custom_flags(0x02000000 | 0x00200000) // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
            .open(path)
            .map_err(|e| e.to_string())?;
        reject_link_metadata(&file.metadata().map_err(|e| e.to_string())?)?;
        Ok(Some(file))
    }
    #[cfg(not(windows))]
    {
        reject_link(path)?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIG: &str = "untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    fn stage() -> StagedUpdate {
        StagedUpdate::new(b"test", SIG, KEY, "9.0.0", "exe").unwrap()
    }

    #[test]
    fn owned_random_directory_survives_clones_and_cleans_itself() {
        let first = stage();
        let second = stage();
        assert_ne!(first.temp_dir, second.temp_dir);
        let path = first.temp_dir.clone();
        let clone = first.clone();
        drop(first);
        assert!(path.exists());
        drop(clone);
        assert!(!path.exists());
        assert!(second.temp_dir.exists());
    }

    #[test]
    fn valid_saved_signature_allows_mock_execution_only() {
        let stage = stage();
        let mut invoked = false;
        stage
            .execute(|executable, args| {
                assert_eq!(executable, stage.installer_path);
                // Silent NSIS update with relaunch (0.9.2 installer plan).
                assert_eq!(args, ["/S", "/R", "/UPDATE"]);
                invoked = true;
                Ok(())
            })
            .unwrap();
        assert!(invoked);
    }

    #[test]
    fn replaced_saved_artifact_never_reaches_executor() {
        let stage = stage();
        std::fs::write(&stage.artifact_path, b"tampered").unwrap();
        assert!(stage
            .execute(|_, _| panic!("unverified artifact executed"))
            .is_err());
    }

    #[test]
    fn replaced_extracted_installer_never_reaches_executor() {
        let mut stage = stage();
        stage.installer_path = stage.temp_dir.join("extracted.exe");
        std::fs::write(&stage.installer_path, b"tampered").unwrap();
        assert!(stage
            .execute(|_, _| panic!("modified installer executed"))
            .is_err());
    }

    #[test]
    fn external_installer_is_rejected_and_external_sentinel_is_preserved() {
        let external = tempfile::tempdir().unwrap();
        let sentinel = external.path().join("sentinel.exe");
        std::fs::write(&sentinel, b"test").unwrap();
        let mut stage = stage();
        stage.installer_path = sentinel.clone();
        assert!(stage
            .execute(|_, _| panic!("external executable launched"))
            .is_err());
        drop(stage);
        assert_eq!(std::fs::read(sentinel).unwrap(), b"test");
    }

    #[test]
    fn invalid_signature_and_version_do_not_create_a_stage() {
        assert!(StagedUpdate::new(b"tampered", SIG, KEY, "9.0.0", "exe").is_err());
        assert!(StagedUpdate::new(b"test", SIG, KEY, "x/../../../outside", "exe").is_err());
        assert!(StagedUpdate::new(b"test", SIG, KEY, "9.0.0", "cmd").is_err());
    }

    fn archive(names: &[&str]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"authenticated installer").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn zipped_installer_uses_fixed_name_and_hashes_authenticated_memory() {
        let directory = tempfile::tempdir().unwrap();
        let (path, hash) =
            save_signed_zip_installer(&archive(&["release/setup.exe"]), directory.path()).unwrap();
        assert_eq!(path, directory.path().join("installer.exe"));
        assert_eq!(std::fs::read(path).unwrap(), b"authenticated installer");
        let expected: [u8; 32] = Sha256::digest(b"authenticated installer").into();
        assert_eq!(hash, expected);
    }

    #[test]
    fn zipped_traversal_and_multiple_executables_are_rejected() {
        for names in [&["../../outside.exe"][..], &["one.exe", "two.exe"][..]] {
            let directory = tempfile::tempdir().unwrap();
            assert!(save_signed_zip_installer(&archive(names), directory.path()).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn saved_executable_cannot_be_overwritten_during_mock_spawn() {
        let stage = stage();
        stage
            .execute(|path, _| {
                assert!(std::fs::write(path, b"tampered during spawn").is_err());
                Ok(())
            })
            .unwrap();
    }
}
