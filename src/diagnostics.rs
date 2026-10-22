use tracing_subscriber::prelude::*;

pub(crate) struct Diagnostics;

impl Diagnostics {
    pub(crate) const PROGRESS_TARGET: &str = "cargo_target_cleaner::progress";

    pub(crate) fn subscriber(verbose: bool) -> impl tracing::Subscriber + Send + Sync {
        tracing_subscriber::registry()
            .with(tracing_subscriber::filter::filter_fn(move |metadata| {
                *metadata.level() <= tracing::Level::INFO
                    && (verbose || metadata.target() != Self::PROGRESS_TARGET)
            }))
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .without_time()
                    .with_ansi(false)
                    .with_target(false),
            )
    }
}
