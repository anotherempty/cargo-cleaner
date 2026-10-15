use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

pub struct Cleanup;

impl Cleanup {
    pub fn before_command(args: &[OsString]) -> Result<(), String> {
        let Some(invocation) = Invocation::parse(args) else {
            return Ok(());
        };
        let cwd =
            std::env::current_dir().map_err(|error| format!("read current directory: {error}"))?;
        let start = invocation
            .manifest
            .as_ref()
            .map(|path| cwd.join(path))
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| cwd.clone());
        let Some((path, config)) = Config::find(&start)? else {
            return Ok(());
        };
        let (max_target, min_free) = config.limits(&path)?;
        if max_target.is_none() && min_free.is_none() {
            return Ok(());
        }

        let workspace = invocation.workspace()?;
        let target = invocation
            .target
            .as_ref()
            .map(|path| cwd.join(path))
            .unwrap_or_else(|| workspace.target_directory.clone());
        let oversized = match max_target {
            Some(limit) => Self::directory_exceeds(&target, limit)?,
            None => false,
        };
        let low_disk = match min_free {
            Some(limit) => {
                fs2::available_space(&workspace.workspace_root).map_err(|error| {
                    format!(
                        "read free disk space at {}: {error}",
                        workspace.workspace_root.display()
                    )
                })? < limit
            }
            None => false,
        };
        if oversized || low_disk {
            eprintln!(
                "cargo-cleaner: running cargo clean ({})",
                if oversized {
                    "target size threshold exceeded"
                } else {
                    "free disk space below minimum"
                }
            );
            let status = invocation
                .command("clean")
                .arg("--target-dir")
                .arg(&target)
                .status()
                .map_err(|error| format!("execute cargo clean: {error}"))?;
            if !status.success() {
                return Err(format!(
                    "cargo clean failed ({status}); requested command was not run"
                ));
            }
        }
        Ok(())
    }

    fn directory_exceeds(root: &Path, limit: u64) -> Result<bool, String> {
        let root = match root.canonicalize() {
            Ok(root) => root,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("resolve {}: {error}", root.display())),
        };
        let mut pending = vec![root];
        let mut size = 0_u64;
        while let Some(path) = pending.pop() {
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("inspect {}: {error}", path.display())),
            };
            if metadata.is_symlink() {
                // Do not traverse symlinked trees outside the artifact directory.
                continue;
            }
            if metadata.is_dir() {
                for entry in fs::read_dir(&path)
                    .map_err(|error| format!("read {}: {error}", path.display()))?
                {
                    pending.push(
                        entry
                            .map_err(|error| format!("read {}: {error}", path.display()))?
                            .path(),
                    );
                }
            } else if metadata.is_file() {
                size = size.saturating_add(metadata.len());
                if size > limit {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    max_target_size_gib: Option<u64>,
    min_free_space_gib: Option<u64>,
}

impl Config {
    fn home_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(|home| PathBuf::from(home).join(".cargo-cleaner.toml"))
    }

    pub(crate) fn setup() -> Result<(), String> {
        let path = Self::home_path().ok_or("cannot locate global config: HOME is not set")?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => file
                .write_all(b"min_free_space_gib = 100\n")
                .map_err(|error| format!("write {}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                // Report unreadable paths instead of treating directories as config files.
                fs::read_to_string(&path)
                    .map(|_| ())
                    .map_err(|error| format!("read {}: {error}", path.display()))
            }
            Err(error) => Err(format!("create {}: {error}", path.display())),
        }
    }

    fn find(start: &Path) -> Result<Option<(PathBuf, Self)>, String> {
        let home = Self::home_path();
        for directory in start.ancestors() {
            let path = directory.join(".cargo-cleaner.toml");
            if home.as_ref() == Some(&path) {
                break;
            }
            if let Some(config) = Self::read(&path)? {
                return Ok(Some((path, config)));
            }
        }
        if let Some(path) = home
            && let Some(config) = Self::read(&path)?
        {
            return Ok(Some((path, config)));
        }
        Ok(None)
    }

    fn read(path: &Path) -> Result<Option<Self>, String> {
        match fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents)
                .map(Some)
                .map_err(|error| format!("invalid {}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("read {}: {error}", path.display())),
        }
    }

    fn limits(&self, path: &Path) -> Result<(Option<u64>, Option<u64>), String> {
        if self.max_target_size_gib.is_some() && self.min_free_space_gib.is_some() {
            return Err(format!(
                "{}: set only one of max_target_size_gib or min_free_space_gib",
                path.display()
            ));
        }
        Ok((
            Self::bytes(self.max_target_size_gib, "max_target_size_gib", path)?,
            Self::bytes(self.min_free_space_gib, "min_free_space_gib", path)?,
        ))
    }

    fn bytes(value: Option<u64>, name: &str, path: &Path) -> Result<Option<u64>, String> {
        value
            .map(|value| {
                if value == 0 {
                    return Err(format!(
                        "{}: {name} must be positive; omit it to disable",
                        path.display()
                    ));
                }
                value
                    .checked_mul(1024 * 1024 * 1024)
                    .ok_or_else(|| format!("{}: {name} is too large", path.display()))
            })
            .transpose()
    }
}

struct Invocation {
    toolchain: Option<OsString>,
    manifest: Option<PathBuf>,
    target: Option<PathBuf>,
    shared: Vec<OsString>,
}

impl Invocation {
    fn parse(args: &[OsString]) -> Option<Self> {
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
            result.toolchain = args.next().cloned();
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
                result.manifest = Some(PathBuf::from(value));
            } else if arg == "--target-dir" {
                result.target = Some(PathBuf::from(args.next()?));
            } else if arg == "--config" || arg == "-Z" || arg == "--color" {
                result.shared.push(arg.clone());
                result.shared.push(args.next()?.clone());
            } else if arg == "--locked" || arg == "--offline" || arg == "--frozen" {
                result.shared.push(arg.clone());
            } else if let Some(text) = arg.to_str() {
                if let Some(value) = text.strip_prefix("--manifest-path=") {
                    result.manifest = Some(PathBuf::from(value));
                } else if let Some(value) = text.strip_prefix("-m") {
                    result.manifest = Some(PathBuf::from(value.strip_prefix('=').unwrap_or(value)));
                } else if let Some(value) = text.strip_prefix("--target-dir=") {
                    result.target = Some(PathBuf::from(value));
                } else if text.starts_with("-C") {
                    // Cargo's unstable directory switch changes config resolution.
                    // Forward it untouched rather than clean the wrong workspace.
                    return None;
                } else if text.starts_with("--config=")
                    || text.starts_with("--color=")
                    || text.starts_with("-Z")
                {
                    result.shared.push(arg.clone());
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
                            && text[1..].chars().all(|c| c == 'v' || c == 'q'))
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

    fn command(&self, subcommand: &str) -> Command {
        let mut command = Command::new("cargo");
        if let Some(toolchain) = &self.toolchain {
            command.arg(toolchain);
        }
        command.arg(subcommand).args(&self.shared);
        if let Some(manifest) = &self.manifest {
            command.arg("--manifest-path").arg(manifest);
        }
        command
    }

    fn workspace(&self) -> Result<Workspace, String> {
        let output = self
            .command("metadata")
            .args([OsStr::new("--no-deps"), OsStr::new("--format-version=1")])
            .output()
            .map_err(|error| format!("execute cargo metadata: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("parse cargo metadata: {error}"))
    }
}

#[derive(Deserialize)]
struct Workspace {
    workspace_root: PathBuf,
    target_directory: PathBuf,
}
