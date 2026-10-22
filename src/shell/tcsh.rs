use std::path::PathBuf;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::{Error, FileOperation};

pub struct Tcsh;

impl Shell for Tcsh {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new tcsh session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new tcsh session.\n",
            path.display()
        ))
    }
}

impl Tcsh {
    const TCSH_INIT: &str = "alias cargo 'cargo-target-cleaner exec --'\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let home = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .ok_or(Error::MissingHome)?;
        let home = PathBuf::from(home);
        let tcshrc = home.join(".tcshrc");
        let cshrc = home.join(".cshrc");
        let path = if tcshrc
            .try_exists()
            .map_err(|error| Error::file(FileOperation::Read, &tcshrc, error))?
            || !cshrc
                .try_exists()
                .map_err(|error| Error::file(FileOperation::Read, &cshrc, error))?
        {
            tcshrc
        } else {
            cshrc
        };
        ShellConfig::update(&path, Self::TCSH_INIT, action)?;
        Ok(path)
    }
}
