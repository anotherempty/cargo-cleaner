mod zsh;

pub use zsh::Zsh;

/// Shell-specific integration lifecycle. Successful operations return a user-facing message.
pub trait Shell {
    fn setup(&self) -> Result<String, String>;
    fn remove(&self) -> Result<String, String>;
}
