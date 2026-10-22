// Clippy has no configuration flags for these test-only exceptions.
#![cfg_attr(test, allow(clippy::todo, clippy::unimplemented))]

mod app;
mod cleanup;
mod diagnostics;
mod error;
mod shell;

fn main() -> std::process::ExitCode {
    let subscriber = diagnostics::Diagnostics::subscriber(false);
    let _guard = tracing::subscriber::set_default(subscriber);
    match app::App::run() {
        Ok(code) => code,
        Err(error) => {
            tracing::error!("cargo-target-cleaner: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
