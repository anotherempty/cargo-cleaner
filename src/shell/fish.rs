use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Fish;

impl Shell for Fish {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new fish session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new fish session.\n",
            path.display()
        ))
    }
}

impl Fish {
    const FISH_INIT: &str =
        "function cargo --wraps cargo\n    command cargo-cleaner exec -- $argv\nend\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let path = ShellConfig::config_home()?.join("fish/conf.d/cargo-cleaner.fish");
        ShellConfig::update(&path, Self::FISH_INIT, action)?;
        Ok(path)
    }
}
