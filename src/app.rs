use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, ExitCode};

pub struct App;

impl App {
    const ZSH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    pub fn run(args: Vec<OsString>) -> Result<ExitCode, String> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("init") if args.len() == 2 && args[1] == "zsh" => Self::print(Self::ZSH_INIT),
            Some("exec") if args.get(1).is_some_and(|arg| arg == "--") => Self::forward(&args[2..]),
            Some("--version" | "-V") if args.len() == 1 => {
                Self::print(concat!("cargo-cleaner ", env!("CARGO_PKG_VERSION"), "\n"))
            }
            None | Some("--help" | "-h") if args.len() <= 1 => Self::print(
                "Usage: cargo-cleaner init zsh\n       cargo-cleaner exec -- [CARGO_ARGS...]\n\n\
                 Enable in ~/.zshrc: eval \"$(cargo-cleaner init zsh)\"\n\n\
                 Configure automatic cleanup in per-project .cargo-cleaner.toml.\n",
            ),
            _ => Err("expected `init zsh` or `exec -- [CARGO_ARGS...]`; see --help".into()),
        }
    }

    fn print(text: &str) -> Result<ExitCode, String> {
        io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|error| format!("failed to write output: {error}"))?;
        Ok(ExitCode::SUCCESS)
    }

    fn forward(args: &[OsString]) -> Result<ExitCode, String> {
        crate::cleanup::Cleanup::before_command(args)?;
        let mut command = Command::new("cargo");
        command.args(args);

        // Execute the PATH executable, never the shell function. Replacing this
        // process preserves Cargo's signals, exit status, and standard streams.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            Err(format!("failed to execute cargo: {}", command.exec()))
        }

        #[cfg(not(unix))]
        {
            let status = command
                .status()
                .map_err(|error| format!("failed to execute cargo: {error}"))?;
            let code = status.code().ok_or("cargo exited without an exit code")?;
            std::process::exit(code);
        }
    }
}
