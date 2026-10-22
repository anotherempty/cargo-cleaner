use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Ion;

impl Shell for Ion {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new ion session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new ion session.\n",
            path.display()
        ))
    }
}

impl Ion {
    const ION_INIT: &str = "alias cargo = \"cargo-target-cleaner exec --\"\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let path = ShellConfig::config_home()?.join("ion/initrc");
        ShellConfig::update(&path, Self::ION_INIT, action)?;
        Ok(path)
    }
}
