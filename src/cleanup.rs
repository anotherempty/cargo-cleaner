use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::error::{CargoOperation, Error, FileOperation};

pub struct Cleanup;

impl Cleanup {
    pub fn before_command(args: &[OsString]) -> Result<(), Error> {
        let Some(invocation) = Invocation::parse(args) else {
            return Ok(());
        };
        let cwd = std::env::current_dir().map_err(Error::CurrentDirectory)?;
        let manifest = invocation.manifest.as_ref().map(|path| cwd.join(path));
        let start = manifest.as_deref().and_then(Path::parent).unwrap_or(&cwd);
        let Some(config) = Config::find(start)? else {
            return Ok(());
        };
        if matches!(config.policy, Policy::Disabled) {
            return Ok(());
        }

        let workspace = invocation.workspace()?;
        let target_override = invocation.target.as_ref().map(|path| cwd.join(path));
        let target = target_override
            .as_deref()
            .unwrap_or(&workspace.target_directory);
        let reason = match config.policy {
            Policy::Disabled => None,
            Policy::MaxTarget(limit) => {
                Self::directory_exceeds(target, limit)?.then_some("target size threshold exceeded")
            }
            Policy::MinFree(limit) => {
                let available =
                    fs2::available_space(&workspace.workspace_root).map_err(|source| {
                        Error::file(FileOperation::FreeSpace, &workspace.workspace_root, source)
                    })?;
                (available < limit.get()).then_some("free disk space below minimum")
            }
        };
        if let Some(reason) = reason {
            tracing::info!("cargo-cleaner: running cargo clean ({reason})");
            let status = invocation
                .command(CargoOperation::Clean)
                .arg("--target-dir")
                .arg(target)
                .status()
                .map_err(|source| Error::Execute {
                    operation: CargoOperation::Clean,
                    source,
                })?;
            if !status.success() {
                return Err(Error::CleanFailed(status));
            }
        }
        Ok(())
    }

    fn directory_exceeds(root: &Path, limit: ByteLimit) -> Result<bool, Error> {
        let root = match root.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(Error::file(FileOperation::Resolve, root, error)),
        };
        let mut pending = vec![root];
        let mut size = 0_u64;
        while let Some(path) = pending.pop() {
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(Error::file(FileOperation::Inspect, &path, error)),
            };
            if metadata.is_symlink() {
                // Do not traverse symlinked trees outside the artifact directory.
                continue;
            }
            if metadata.is_dir() {
                for entry in fs::read_dir(&path)
                    .map_err(|error| Error::file(FileOperation::Read, &path, error))?
                {
                    pending.push(
                        entry
                            .map_err(|error| Error::file(FileOperation::Read, &path, error))?
                            .path(),
                    );
                }
            } else if metadata.is_file() {
                size = size.saturating_add(metadata.len());
                if size > limit.get() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Config {
    policy: Policy,
}

impl TryFrom<ConfigFields> for Config {
    type Error = PolicyError;

    fn try_from(fields: ConfigFields) -> Result<Self, Self::Error> {
        let policy = match (fields.max_target_size_gib, fields.min_free_space_gib) {
            (Some(_), Some(_)) => return Err(PolicyError::ConflictingThresholds),
            (Some(value), None) => {
                Policy::MaxTarget(ByteLimit::from_gib(value, Threshold::MaxTarget)?)
            }
            (None, Some(value)) => Policy::MinFree(ByteLimit::from_gib(value, Threshold::MinFree)?),
            (None, None) => Policy::Disabled,
        };
        Ok(Self { policy })
    }
}

impl Config {
    const FILE_NAME: &str = ".cargo-cleaner.toml";
    // Keep 100 GiB available by default for Cargo builds and other applications.
    const DEFAULT_MIN_FREE_GIB: u64 = 100;

    fn home_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(|home| PathBuf::from(home).join(Self::FILE_NAME))
    }

    pub(crate) fn setup() -> Result<(), Error> {
        let path = Self::home_path().ok_or(Error::MissingHome)?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => writeln!(file, "min_free_space_gib = {}", Self::DEFAULT_MIN_FREE_GIB)
                .map_err(|error| Error::file(FileOperation::Write, &path, error)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                // Report unreadable paths instead of treating directories as config files.
                fs::read_to_string(&path)
                    .map(|_| ())
                    .map_err(|error| Error::file(FileOperation::Read, &path, error))
            }
            Err(error) => Err(Error::file(FileOperation::Create, &path, error)),
        }
    }

    fn find(start: &Path) -> Result<Option<Self>, Error> {
        let home = Self::home_path();
        for directory in start.ancestors() {
            let path = directory.join(Self::FILE_NAME);
            if home.as_ref() == Some(&path) {
                break;
            }
            if let Some(config) = Self::read(&path)? {
                return Ok(Some(config));
            }
        }
        match home {
            Some(path) => Self::read(&path),
            None => Ok(None),
        }
    }

    fn read(path: &Path) -> Result<Option<Self>, Error> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Error::file(FileOperation::Read, path, error)),
        };
        let fields: ConfigFields = toml::from_str(&contents).map_err(|source| Error::Config {
            path: path.to_owned(),
            source,
        })?;
        Self::try_from(fields)
            .map(Some)
            .map_err(|source| Error::Policy {
                path: path.to_owned(),
                source,
            })
    }
}

// Raw TOML boundary representation; only Config owns a validated policy.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFields {
    max_target_size_gib: Option<u64>,
    min_free_space_gib: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Policy {
    Disabled,
    MaxTarget(ByteLimit),
    MinFree(ByteLimit),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteLimit(NonZeroU64);

impl ByteLimit {
    const BYTES_PER_GIB: u64 = 1 << 30;

    fn from_gib(value: u64, threshold: Threshold) -> Result<Self, PolicyError> {
        let gib = NonZeroU64::new(value).ok_or(PolicyError::Zero(threshold))?;
        gib.get()
            .checked_mul(Self::BYTES_PER_GIB)
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or(PolicyError::TooLarge(threshold))
    }

    fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Threshold {
    MaxTarget,
    MinFree,
}

impl std::fmt::Display for Threshold {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MaxTarget => "max_target_size_gib",
            Self::MinFree => "min_free_space_gib",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum PolicyError {
    #[error("set only one of max_target_size_gib or min_free_space_gib")]
    ConflictingThresholds,
    #[error("{0} must be positive; omit it to disable")]
    Zero(Threshold),
    #[error("{0} is too large")]
    TooLarge(Threshold),
}

struct Invocation<'a> {
    toolchain: Option<&'a OsStr>,
    manifest: Option<&'a Path>,
    target: Option<&'a Path>,
    shared: Vec<&'a OsStr>,
}

impl<'a> Invocation<'a> {
    const METADATA_FORMAT: &str = "--format-version=1";

    fn parse(args: &'a [OsString]) -> Option<Self> {
        let mut result = Self {
            toolchain: None,
            manifest: None,
            target: None,
            shared: Vec::new(),
        };
        let mut args = args.iter().peekable();
        if args
            .peek()
            .is_some_and(|arg| arg.as_encoded_bytes().starts_with(b"+"))
        {
            result.toolchain = args.next().map(OsString::as_os_str);
        }
        let mut found_command = false;
        while let Some(arg) = args.next() {
            if arg == "--" {
                break;
            }
            if arg == "--help" || arg == "-h" || arg == "--version" || arg == "-V" {
                return None;
            }
            if arg == "--manifest-path" || arg == "-m" {
                let value = args.next()?;
                result.manifest = Some(Path::new(value));
            } else if arg == "--target-dir" {
                result.target = Some(Path::new(args.next()?));
            } else if arg == "--config" || arg == "-Z" || arg == "--color" {
                result.shared.push(arg.as_os_str());
                result.shared.push(args.next()?.as_os_str());
            } else if arg == "--locked" || arg == "--offline" || arg == "--frozen" {
                result.shared.push(arg.as_os_str());
            } else if let Some(text) = arg.to_str() {
                if let Some(value) = text.strip_prefix("--manifest-path=") {
                    result.manifest = Some(Path::new(value));
                } else if let Some(value) = text.strip_prefix("-m") {
                    result.manifest = Some(Path::new(value.strip_prefix('=').unwrap_or(value)));
                } else if let Some(value) = text.strip_prefix("--target-dir=") {
                    result.target = Some(Path::new(value));
                } else if text.starts_with("-C") {
                    // Cargo's unstable directory switch changes config resolution.
                    // Forward it untouched rather than clean the wrong workspace.
                    return None;
                } else if text.starts_with("--config=")
                    || text.starts_with("--color=")
                    || text.starts_with("-Z")
                {
                    result.shared.push(arg.as_os_str());
                } else if !found_command {
                    if matches!(
                        text,
                        "build"
                            | "b"
                            | "check"
                            | "c"
                            | "test"
                            | "t"
                            | "run"
                            | "r"
                            | "bench"
                            | "doc"
                            | "d"
                            | "clippy"
                            | "rustc"
                            | "rustdoc"
                    ) {
                        found_command = true;
                    } else if text != "--quiet"
                        && text != "--verbose"
                        && !(text.starts_with('-')
                            && text
                                .strip_prefix('-')
                                .is_some_and(|flags| flags.chars().all(|c| c == 'v' || c == 'q')))
                    {
                        return None;
                    }
                }
            } else if !found_command || arg.as_encoded_bytes().starts_with(b"-") {
                return None;
            }
        }
        found_command.then_some(result)
    }

    fn command(&self, operation: CargoOperation) -> Command {
        let mut command = Command::new("cargo");
        if let Some(toolchain) = &self.toolchain {
            command.arg(toolchain);
        }
        command.arg(operation.argument()).args(&self.shared);
        if let Some(manifest) = &self.manifest {
            command.arg("--manifest-path").arg(manifest);
        }
        command
    }

    fn workspace(&self) -> Result<Workspace, Error> {
        let output = self
            .command(CargoOperation::Metadata)
            .args([OsStr::new("--no-deps"), OsStr::new(Self::METADATA_FORMAT)])
            .output()
            .map_err(|source| Error::Execute {
                operation: CargoOperation::Metadata,
                source,
            })?;
        if !output.status.success() {
            return Err(Error::MetadataFailed {
                status: output.status,
                stderr: output.stderr,
            });
        }
        serde_json::from_slice(&output.stdout).map_err(Error::Metadata)
    }
}

#[derive(Deserialize)]
struct Workspace {
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::{ByteLimit, Config, ConfigFields, Policy, PolicyError, Threshold};

    #[test]
    fn byte_limits_reject_zero_and_overflow_for_each_threshold() {
        const MAX_GIB: u64 = u64::MAX / ByteLimit::BYTES_PER_GIB;
        const OVERFLOW_GIB: u64 = MAX_GIB + 1;
        for threshold in [Threshold::MaxTarget, Threshold::MinFree] {
            assert_eq!(
                ByteLimit::from_gib(0, threshold),
                Err(PolicyError::Zero(threshold))
            );
            assert_eq!(
                ByteLimit::from_gib(OVERFLOW_GIB, threshold),
                Err(PolicyError::TooLarge(threshold))
            );
            assert_eq!(
                ByteLimit::from_gib(MAX_GIB, threshold).unwrap().get(),
                MAX_GIB * ByteLimit::BYTES_PER_GIB
            );
        }
    }

    #[test]
    fn config_conversion_enforces_exclusive_thresholds() {
        const LIMIT_GIB: u64 = 1;
        assert_eq!(
            Config::try_from(ConfigFields {
                max_target_size_gib: Some(LIMIT_GIB),
                min_free_space_gib: Some(LIMIT_GIB),
            }),
            Err(PolicyError::ConflictingThresholds)
        );
        assert_eq!(
            Config::try_from(ConfigFields {
                max_target_size_gib: None,
                min_free_space_gib: None,
            })
            .unwrap()
            .policy,
            Policy::Disabled
        );
        let limit = ByteLimit::from_gib(LIMIT_GIB, Threshold::MaxTarget).unwrap();
        assert_eq!(
            Config::try_from(ConfigFields {
                max_target_size_gib: Some(LIMIT_GIB),
                min_free_space_gib: None,
            })
            .unwrap()
            .policy,
            Policy::MaxTarget(limit)
        );
        assert_eq!(
            Config::try_from(ConfigFields {
                max_target_size_gib: None,
                min_free_space_gib: Some(LIMIT_GIB),
            })
            .unwrap()
            .policy,
            Policy::MinFree(limit)
        );
    }
}
