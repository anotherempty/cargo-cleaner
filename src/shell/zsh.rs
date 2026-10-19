use std::io;
use std::path::PathBuf;

use super::Shell;
use crate::error::{Error, FileOperation};

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
        let original = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(Error::file(FileOperation::Read, &path, error)),
        };
        let contents = match action {
            Action::Remove if original.contains(Self::ZSH_INIT) => {
                original.replace(Self::ZSH_INIT, "")
            }
            Action::Install if !original.contains(Self::ZSH_INIT) => {
                let mut contents = original;
                if !contents.is_empty() && !contents.ends_with('\n') {
                    contents.push('\n');
                }
                contents.push_str(Self::ZSH_INIT);
                contents
            }
            _ => return Ok(path),
        };
        std::fs::write(&path, contents)
            .map_err(|error| Error::file(FileOperation::Write, &path, error))?;
        Ok(path)
    }
}

#[derive(Clone, Copy)]
enum Action {
    Install,
    Remove,
}
