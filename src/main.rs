// Clippy has no configuration flags for these test-only exceptions.
#![cfg_attr(test, allow(clippy::todo, clippy::unimplemented))]

mod app;
mod cleanup;

fn main() -> std::process::ExitCode {
    match app::App::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("cargo-cleaner: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
