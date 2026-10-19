use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to {operation} {}: {source}", path.display())]
    File {
        operation: FileOperation,
        path: PathBuf,
        source: io::Error,
    },
    #[error("read current directory: {0}")]
    CurrentDirectory(#[source] io::Error),
    #[error("failed to write output: {0}")]
    Output(#[source] io::Error),
    #[error("failed to write help: {0}")]
    Help(#[source] io::Error),
    #[error("failed to execute cargo: {0}")]
    Forward(#[source] io::Error),
    #[error("failed to execute cargo {operation}: {source}")]
    Execute {
        operation: CargoOperation,
        source: io::Error,
    },
    #[error("cargo clean failed ({0}); requested command was not run")]
    CleanFailed(ExitStatus),
    #[error("cargo metadata failed ({status}): {}", String::from_utf8_lossy(stderr).trim())]
    MetadataFailed { status: ExitStatus, stderr: Vec<u8> },
    #[error("parse cargo metadata: {0}")]
    Metadata(#[source] serde_json::Error),
    #[error("invalid {}: {source}", path.display())]
    Config {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{}: {source}", path.display())]
    Policy {
        path: PathBuf,
        source: crate::cleanup::PolicyError,
    },
    #[error("cannot locate global config: HOME is not set")]
    MissingHome,
    #[error("cannot locate .zshrc: neither ZDOTDIR nor HOME is set")]
    MissingShellDirectory,
    #[cfg(not(unix))]
    #[error("cargo exited without an exit code")]
    MissingExitCode,
}

impl Error {
    pub fn file(operation: FileOperation, path: &std::path::Path, source: io::Error) -> Self {
        Self::File {
            operation,
            path: path.to_owned(),
            source,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FileOperation {
    Read,
    Write,
    Create,
    Resolve,
    Inspect,
    FreeSpace,
}

impl fmt::Display for FileOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Create => "create",
            Self::Resolve => "resolve",
            Self::Inspect => "inspect",
            Self::FreeSpace => "read free disk space at",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CargoOperation {
    Clean,
    Metadata,
}

impl fmt::Display for CargoOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.argument())
    }
}

impl CargoOperation {
    pub fn argument(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Metadata => "metadata",
        }
    }
}
