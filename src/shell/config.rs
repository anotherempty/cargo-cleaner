use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, FileOperation};

pub(super) struct ShellConfig;

impl ShellConfig {
    pub(super) fn config_home() -> Result<PathBuf, Error> {
        if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }
        #[cfg(windows)]
        if let Some(path) = std::env::var_os("APPDATA").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }
        let home = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .ok_or(Error::MissingHome)?;
        Ok(PathBuf::from(home).join(".config"))
    }

    pub(super) fn update(path: &Path, script: &str, action: Action) -> Result<(), Error> {
        let original = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(Error::file(FileOperation::Read, path, error)),
        };
        let contents = match action {
            Action::Remove if original.contains(script) => original.replace(script, ""),
            Action::Install if !original.contains(script) => {
                let mut contents = original;
                if !contents.is_empty() && !contents.ends_with('\n') {
                    contents.push('\n');
                }
                contents.push_str(script);
                contents
            }
            _ => return Ok(()),
        };
        if matches!(action, Action::Install)
            && let Some(parent) = path.parent()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| Error::file(FileOperation::Create, parent, error))?;
        }
        std::fs::write(path, contents)
            .map_err(|error| Error::file(FileOperation::Write, path, error))?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) enum Action {
    Install,
    Remove,
}
