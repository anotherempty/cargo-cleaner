use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, ExitCode};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "cargo-cleaner",
    version,
    about = "Automatic per-project Cargo cleanup"
)]
pub struct App {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

impl App {
    pub fn run() -> Result<ExitCode, String> {
        let app = Self::parse();
        match app.command {
            Some(CliCommand::Init { shell }) => {
                crate::cleanup::Config::setup()?;
                Self::print(&shell.integration().setup()?)
            }
            Some(CliCommand::Uninstall { shell }) => Self::print(&shell.integration().remove()?),
            Some(CliCommand::Exec { args }) => Self::forward(&args),
            None => {
                Self::command()
                    .print_help()
                    .map_err(|error| format!("failed to write help: {error}"))?;
                Ok(ExitCode::SUCCESS)
            }
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

#[derive(Subcommand)]
enum CliCommand {
    /// Install shell integration and create the global cleanup config if missing.
    Init {
        #[arg(value_enum)]
        shell: ShellKind,
    },
    /// Remove shell integration from the shell configuration file.
    Uninstall {
        #[arg(value_enum)]
        shell: ShellKind,
    },
    /// Run Cargo, checking cleanup thresholds before build commands.
    Exec {
        #[arg(last = true, value_name = "CARGO_ARGS")]
        args: Vec<OsString>,
    },
}

#[derive(Clone, ValueEnum)]
enum ShellKind {
    Zsh,
}

impl ShellKind {
    fn integration(&self) -> &dyn crate::shell::Shell {
        match self {
            Self::Zsh => &crate::shell::Zsh,
        }
    }
}
