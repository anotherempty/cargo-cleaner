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
    const ZSH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    pub fn run() -> Result<ExitCode, String> {
        let app = Self::parse();
        match app.command {
            Some(CliCommand::Init { shell: Shell::Zsh }) => Self::configure_zsh(true),
            Some(CliCommand::Uninstall { shell: Shell::Zsh }) => Self::configure_zsh(false),
            Some(CliCommand::Exec { args }) => Self::forward(&args),
            None => {
                Self::command()
                    .print_help()
                    .map_err(|error| format!("failed to write help: {error}"))?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }

    fn configure_zsh(install: bool) -> Result<ExitCode, String> {
        let directory = std::env::var_os("ZDOTDIR")
            .filter(|value| !value.is_empty())
            .or_else(|| std::env::var_os("HOME").filter(|value| !value.is_empty()))
            .ok_or("cannot locate .zshrc: neither ZDOTDIR nor HOME is set")?;
        let path = std::path::PathBuf::from(directory).join(".zshrc");
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
        if install {
            Self::print(&format!(
                "Cargo integration configured in {}. Open a new zsh session to activate it.\n",
                path.display()
            ))
        } else {
            Self::print(&format!(
                "Cargo integration removed from {}. Open a new zsh session or run `unfunction cargo` in the current session.\n",
                path.display()
            ))
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
    /// Install shell integration in the shell configuration file.
    Init {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Remove shell integration from the shell configuration file.
    Uninstall {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Run Cargo, checking cleanup thresholds before build commands.
    Exec {
        #[arg(last = true, value_name = "CARGO_ARGS")]
        args: Vec<OsString>,
    },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Zsh,
}
