use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Elvish;

impl Shell for Elvish {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new elvish session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new elvish session.\n",
            path.display()
        ))
    }
}

impl Elvish {
    const ELVISH_INIT: &str = "fn cargo {|@args| e:cargo-target-cleaner exec -- $@args }\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let path = ShellConfig::config_home()?.join("elvish/rc.elv");
        ShellConfig::update(&path, Self::ELVISH_INIT, action)?;
        Ok(path)
    }
}
