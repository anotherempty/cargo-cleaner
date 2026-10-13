use std::io;
use std::path::PathBuf;

use super::Shell;

pub struct Zsh;

impl Shell for Zsh {
    fn setup(&self) -> Result<String, String> {
        let path = self.configure(true)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new zsh session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, String> {
        let path = self.configure(false)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new zsh session or run `unfunction cargo` in the current session.\n",
            path.display()
        ))
    }
}

impl Zsh {
    const ZSH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    fn configure(&self, install: bool) -> Result<PathBuf, String> {
        let directory = std::env::var_os("ZDOTDIR")
            .filter(|value| !value.is_empty())
            .or_else(|| std::env::var_os("HOME").filter(|value| !value.is_empty()))
            .ok_or("cannot locate .zshrc: neither ZDOTDIR nor HOME is set")?;
        let path = PathBuf::from(directory).join(".zshrc");
        let original = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
        };
        let mut contents = String::new();
        for line in original.split_inclusive('\n') {
            if line.trim() != "eval \"$(cargo-cleaner init zsh)\"" {
                contents.push_str(line);
            }
        }
        if !install {
            contents = contents.replace(Self::ZSH_INIT, "");
        } else if !contents.contains(Self::ZSH_INIT) {
            if !contents.is_empty() && !contents.ends_with('\n') {
                contents.push('\n');
            }
            contents.push_str(Self::ZSH_INIT);
        }
        if contents != original {
            std::fs::write(&path, contents)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
        Ok(path)
    }
}
