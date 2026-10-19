use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Cmd;

impl Shell for Cmd {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new Cmd session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new Cmd session to deactivate it.\n",
            path.display()
        ))
    }
}

impl Cmd {
    const CMD_INIT: &str = "if not os.setalias('cargo', 'cargo-cleaner exec -- $*') then\n    error('cargo-cleaner: cannot set cargo alias')\nend\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let directory = std::env::var_os("CLINK_PROFILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .filter(|value| !value.is_empty())
                    .map(|directory| PathBuf::from(directory).join("clink"))
            })
            .ok_or(Error::MissingClinkDirectory)?;
        let path = directory.join("cargo-cleaner.lua");
        ShellConfig::update(&path, Self::CMD_INIT, action)?;
        Ok(path)
    }
}
