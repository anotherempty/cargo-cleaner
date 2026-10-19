mod bash;
mod cmd;
mod config;
mod elvish;
mod fish;
mod ion;
mod nu;
mod powershell;
mod tcsh;
mod xonsh;
mod zsh;

use crate::error::Error;
pub use bash::Bash;
pub use cmd::Cmd;
pub use elvish::Elvish;
pub use fish::Fish;
pub use ion::Ion;
pub use nu::Nu;
pub use powershell::PowerShell;
pub use tcsh::Tcsh;
pub use xonsh::Xonsh;
pub use zsh::Zsh;

/// Shell-specific integration lifecycle. Successful operations return a user-facing message.
pub trait Shell {
    fn setup(&self) -> Result<String, Error>;
    fn remove(&self) -> Result<String, Error>;
}
