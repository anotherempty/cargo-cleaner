// Clippy has no configuration flags for these test-only exceptions.
#![cfg_attr(test, allow(clippy::todo, clippy::unimplemented))]

mod app;
mod cleanup;
mod error;
mod shell;

fn main() -> std::process::ExitCode {
    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .without_time()
        .with_ansi(false)
        .with_target(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    match app::App::run() {
        Ok(code) => code,
        Err(error) => {
            tracing::error!("cargo-cleaner: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
