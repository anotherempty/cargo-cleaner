use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct Bash;

impl Shell for Bash {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new bash session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new bash session or run `unset -f cargo` in the current session.\n",
            path.display()
        ))
    }
}

impl Bash {
    const BASH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let directory = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .ok_or(Error::MissingHome)?;
        let path = PathBuf::from(directory).join(".bashrc");
        ShellConfig::update(&path, Self::BASH_INIT, action)?;
        Ok(path)
    }
}
