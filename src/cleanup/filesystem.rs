use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, FileOperation};

#[cfg(unix)]
type FilesystemNumber = u64;
#[cfg(windows)]
type FilesystemNumber = u32;

/// An OS filesystem identity, refreshed from target paths on every registry load.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub(super) struct FilesystemId(FilesystemNumber);

impl TryFrom<String> for FilesystemId {
    type Error = std::num::ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse().map(Self)
    }
}

impl From<FilesystemId> for String {
    fn from(value: FilesystemId) -> Self {
        value.0.to_string()
    }
}

impl FilesystemId {
    #[cfg(unix)]
    pub(super) fn resolve(path: &Path) -> Result<Self, Error> {
        use std::os::unix::fs::MetadataExt;

        std::fs::metadata(path)
            .map(|metadata| Self(metadata.dev()))
            .map_err(|source| Error::file(FileOperation::Inspect, path, source))
    }

    #[cfg(windows)]
    pub(super) fn resolve(path: &Path) -> Result<Self, Error> {
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, GetFileInformationByHandle,
        };

        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .map_err(|source| Error::file(FileOperation::Inspect, path, source))?;
        let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        // SAFETY: the handle remains open and the output points to writable storage.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) }
            == 0
        {
            return Err(Error::file(
                FileOperation::Inspect,
                path,
                std::io::Error::last_os_error(),
            ));
        }
        // SAFETY: a successful call initialized the information structure.
        let information = unsafe { information.assume_init() };
        Ok(Self(information.dwVolumeSerialNumber))
    }

    /// Missing build directories inherit the filesystem of their nearest existing ancestor.
    pub(super) fn existing_path(target: &Path) -> Result<PathBuf, Error> {
        let mut path = target.to_owned();
        loop {
            match path.canonicalize() {
                Ok(existing) => return Ok(existing),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    match std::fs::symlink_metadata(&path) {
                        Ok(metadata) if metadata.is_symlink() => {
                            let destination = std::fs::read_link(&path).map_err(|source| {
                                Error::file(FileOperation::Resolve, &path, source)
                            })?;
                            path.pop();
                            return Self::existing_path(&path.join(destination));
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            return Err(Error::file(FileOperation::Inspect, &path, error));
                        }
                    }
                    if !path.pop() {
                        return Err(Error::file(FileOperation::Resolve, target, source));
                    }
                }
                Err(source) => return Err(Error::file(FileOperation::Resolve, &path, source)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemId;

    #[test]
    fn missing_target_uses_existing_ancestor() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("missing/target");
        let existing = FilesystemId::existing_path(&target).unwrap();
        assert_eq!(existing, directory.path().canonicalize().unwrap());
        assert_eq!(
            FilesystemId::resolve(&existing).unwrap(),
            FilesystemId::resolve(directory.path()).unwrap()
        );
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn missing_symlink_destination_uses_destination_ancestor() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let link = source.path().join("target");
        std::os::unix::fs::symlink(destination.path().join("missing"), &link).unwrap();
        assert_eq!(
            FilesystemId::existing_path(&link.join("nested")).unwrap(),
            destination.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn filesystem_keys_reject_non_numeric_and_overflow_values() {
        for value in ["disk", "-1", "18446744073709551616"] {
            assert!(FilesystemId::try_from(value.to_owned()).is_err());
        }
    }
}
