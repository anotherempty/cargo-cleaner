use std::ffi::OsString;
use std::io::{self, Write};
use std::process::{Command, ExitCode};

pub struct App;

impl App {
    const ZSH_INIT: &str = "cargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n";

    pub fn run(args: Vec<OsString>) -> Result<ExitCode, String> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("init") if args.len() == 2 && args[1] == "zsh" => Self::configure_zsh(true),
            Some("uninstall") if args.len() == 2 && args[1] == "zsh" => Self::configure_zsh(false),
            Some("exec") if args.get(1).is_some_and(|arg| arg == "--") => Self::forward(&args[2..]),
            Some("--version" | "-V") if args.len() == 1 => {
                Self::print(concat!("cargo-cleaner ", env!("CARGO_PKG_VERSION"), "\n"))
            }
            None | Some("--help" | "-h") if args.len() <= 1 => Self::print(
                "Usage: cargo-cleaner init zsh\n       cargo-cleaner uninstall zsh\n       cargo-cleaner exec -- [CARGO_ARGS...]\n\n\
                 Install shell integration: cargo-cleaner init zsh\n\n\
                 Configure automatic cleanup in per-project .cargo-cleaner.toml.\n",
            ),
            _ => Err(
                "expected `init zsh`, `uninstall zsh`, or `exec -- [CARGO_ARGS...]`; see --help"
                    .into(),
            ),
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
