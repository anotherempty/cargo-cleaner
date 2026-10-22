use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Xonsh;

impl Shell for Xonsh {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new xonsh session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new xonsh session.\n",
            path.display()
        ))
    }
}

impl Xonsh {
    const XONSH_INIT: &str = "aliases['cargo'] = ['cargo-target-cleaner', 'exec', '--']\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let home = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .ok_or(Error::MissingHome)?;
        let path = PathBuf::from(home).join(".xonshrc");
        ShellConfig::update(&path, Self::XONSH_INIT, action)?;
        Ok(path)
    }
}
