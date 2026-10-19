#[cfg(all(unix, test))]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::process::Command;

    const EXECUTABLE_MODE: u32 = 0o755;
    const FORWARDED_EXIT_CODE: i32 = 37;
    const REPEAT_COUNT: usize = 2;

    #[derive(Debug, thiserror::Error)]
    enum FixtureError {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        Path(#[from] std::env::JoinPathsError),
    }

    struct Fixture {
        directory: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Result<Self, FixtureError> {
            let directory = tempfile::tempdir()?;
            let root = directory.path();
            fs::write(root.join(".cargo-cleaner.toml"), "")?;
            fs::write(
                root.join("cargo"),
                format!(
                    "#!/bin/sh\nprintf '%s\\0' \"$@\"\nprintf 'diagnostic' >&2\nexit {FORWARDED_EXIT_CODE}\n"
                ),
            )?;
            fs::set_permissions(
                root.join("cargo"),
                fs::Permissions::from_mode(EXECUTABLE_MODE),
            )?;
            symlink(
                env!("CARGO_BIN_EXE_cargo-cleaner"),
                root.join("cargo-cleaner"),
            )?;
            Ok(Self { directory })
        }

        fn command(&self, executable: &str) -> Result<Command, FixtureError> {
            let mut paths = vec![self.directory.path().to_owned()];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            let mut command = Command::new(executable);
            command
                .current_dir(self.directory.path())
                .env("HOME", self.directory.path())
                .env_remove("ZDOTDIR")
                .env("XDG_CONFIG_HOME", self.directory.path().join(".config"))
                .env("CLINK_PROFILE", self.directory.path().join("clink"))
                .env("PATH", std::env::join_paths(paths)?);
            Ok(command)
        }

        fn lifecycle(&self, shell: &str, config: &str) -> Result<(), FixtureError> {
            let path = self.directory.path().join(config);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let original = "# user configuration\n";
            fs::write(&path, original)?;
            let mut installed = None;
            for _ in 0..REPEAT_COUNT {
                let output = self
                    .command(env!("CARGO_BIN_EXE_cargo-cleaner"))?
                    .args(["init", shell])
                    .output()?;
                assert!(output.status.success(), "{output:?}");
                let contents = fs::read_to_string(&path)?;
                assert!(contents.starts_with(original));
                if let Some(previous) = &installed {
                    assert_eq!(&contents, previous);
                }
                installed = Some(contents);
            }
            for _ in 0..REPEAT_COUNT {
                let output = self
                    .command(env!("CARGO_BIN_EXE_cargo-cleaner"))?
                    .args(["uninstall", shell])
                    .output()?;
                assert!(output.status.success(), "{output:?}");
                assert_eq!(fs::read_to_string(&path)?, original);
            }
            Ok(())
        }
    }

    #[test]
    fn powershell_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture
            .lifecycle("powershell", ".config/powershell/profile.ps1")
            .unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "powershell"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let output = fixture.command("pwsh").unwrap()
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
                ". $PROFILE.CurrentUserAllHosts; cargo '+stable' 'custom' '' 'with spaces' '*;$HOME'; exit $LASTEXITCODE"])
            .output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(FORWARDED_EXIT_CODE),
            "{output:?}"
        );
        assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
        assert_eq!(output.stderr, b"diagnostic");
    }

    #[test]
    fn nu_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture
            .lifecycle("nu", ".config/nushell/config.nu")
            .unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "nu"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let script = fixture.directory.path().join("check.nu");
        fs::write(&script, "source .config/nushell/config.nu\ncargo '+stable' 'custom' '' 'with spaces' '*;$HOME'\nexit $env.LAST_EXIT_CODE\n").unwrap();
        let output = fixture
            .command("nu")
            .unwrap()
            .args(["--no-config-file"])
            .arg(script)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(FORWARDED_EXIT_CODE),
            "{output:?}"
        );
        assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
        assert_eq!(output.stderr, b"diagnostic");
    }

    #[test]
    fn ion_setup_and_removal() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("ion", ".config/ion/initrc").unwrap();
    }

    #[test]
    fn fish_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture
            .lifecycle("fish", ".config/fish/conf.d/cargo-cleaner.fish")
            .unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "fish"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = fixture
            .command("fish")
            .unwrap()
            .args([
                "--no-config",
                "-c",
                "source $HOME/.config/fish/conf.d/cargo-cleaner.fish; cargo $argv",
                "+stable",
                "custom",
                "",
                "with spaces",
                "*;$HOME",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
        assert_eq!(output.stderr, b"diagnostic");
    }

    #[test]
    fn elvish_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture
            .lifecycle("elvish", ".config/elvish/rc.elv")
            .unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "elvish"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = fixture
            .command("elvish")
            .unwrap()
            .args([
                "-c",
                "eval (slurp < $E:HOME/.config/elvish/rc.elv); cargo $@args",
                "+stable",
                "custom",
                "",
                "with spaces",
                "*;$HOME",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
        assert!(String::from_utf8_lossy(&output.stderr).contains("diagnostic"));
    }

    #[test]
    fn cmd_setup_and_removal() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("cmd", "clink/cargo-cleaner.lua").unwrap();
    }

    #[test]
    fn bash_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("bash", ".bashrc").unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "bash"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = fixture
            .command("bash")
            .unwrap()
            .args([
                "--noprofile",
                "--norc",
                "-c",
                "source \"$HOME/.bashrc\"; cargo \"$@\"",
                "test",
                "+stable",
                "custom",
                "",
                "with spaces",
                "*;$HOME",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
        assert_eq!(output.stderr, b"diagnostic");
    }
}
