use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Nu;

impl Shell for Nu {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new nu session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new nu session.\n",
            path.display()
        ))
    }
}

impl Nu {
    const NU_INIT: &str =
        "def --wrapped cargo [...args] {\n    ^cargo-target-cleaner exec -- ...$args\n}\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let directory = ShellConfig::config_home()?;
        #[cfg(target_os = "macos")]
        let directory = if std::env::var_os("XDG_CONFIG_HOME").is_none_or(|value| value.is_empty())
        {
            let home = std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .ok_or(Error::MissingHome)?;
            PathBuf::from(home).join("Library/Application Support")
        } else {
            directory
        };
        let path = directory.join("nushell/config.nu");
        ShellConfig::update(&path, Self::NU_INIT, action)?;
        Ok(path)
    }
}
