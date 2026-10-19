use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Zsh;

impl Shell for Zsh {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new zsh session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new zsh session or run `unfunction cargo` in the current session.\n",
            path.display()
        ))
    }
}

impl Zsh {
    const ZSH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let directory = std::env::var_os("ZDOTDIR")
            .filter(|value| !value.is_empty())
            .or_else(|| std::env::var_os("HOME").filter(|value| !value.is_empty()))
            .ok_or(Error::MissingShellDirectory)?;
        let path = PathBuf::from(directory).join(".zshrc");
        ShellConfig::update(&path, Self::ZSH_INIT, action)?;
        Ok(path)
    }
}
