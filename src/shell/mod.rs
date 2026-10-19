mod zsh;

use crate::error::Error;
pub use zsh::Zsh;

/// Shell-specific integration lifecycle. Successful operations return a user-facing message.
pub trait Shell {
    fn setup(&self) -> Result<String, Error>;
    fn remove(&self) -> Result<String, Error>;
}
