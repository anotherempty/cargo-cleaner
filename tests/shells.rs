#[cfg(windows)]
#[path = "shells/windows.rs"]
mod windows;

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
    fn xonsh_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("xonsh", ".xonshrc").unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "xonsh"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = fixture
            .command("xonsh")
            .unwrap()
            .args([
                "--no-rc",
                "-c",
                "$XONSH_SUBPROC_RAISE_ERROR = False\nsource ~/.xonshrc\ncargo @(['+stable', 'custom', '', 'with spaces', '*;$HOME'])",
            ])
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
    fn tcsh_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("tcsh", ".tcshrc").unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "tcsh"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let script = fixture.directory.path().join("check.csh");
        fs::write(
            &script,
            "source \"$HOME/.tcshrc\"\ncargo '+stable' 'custom' '' 'with spaces' '*;$HOME'\n",
        )
        .unwrap();
        let output = fixture
            .command("tcsh")
            .unwrap()
            .arg("-f")
            .arg(script)
            .args(["+stable", "custom", "", "with spaces", "*;$HOME"])
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
    fn tcsh_preserves_existing_startup_configuration() {
        for has_tcshrc in [false, true] {
            let fixture = Fixture::new().unwrap();
            let home = fixture.directory.path();
            let cshrc = "setenv CLEANER_STARTUP_SOURCE cshrc\n";
            let tcshrc = "setenv CLEANER_STARTUP_SOURCE tcshrc\n";
            fs::write(home.join(".cshrc"), cshrc).unwrap();
            if has_tcshrc {
                fs::write(home.join(".tcshrc"), tcshrc).unwrap();
            }
            for action in ["init", "init", "uninstall", "uninstall"] {
                let output = fixture
                    .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
                    .unwrap()
                    .args([action, "tcsh"])
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{output:?}");
                let output = fixture
                    .command("tcsh")
                    .unwrap()
                    .args(["-c", "echo $CLEANER_STARTUP_SOURCE; alias cargo"])
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{output:?}");
                let source = if has_tcshrc { "tcshrc" } else { "cshrc" };
                let expected = if action == "init" {
                    format!("{source}\ncargo-cleaner exec --\n")
                } else {
                    format!("{source}\n")
                };
                assert_eq!(output.stdout, expected.as_bytes());
                assert!(output.stderr.is_empty(), "{output:?}");
                assert_eq!(home.join(".tcshrc").exists(), has_tcshrc);
            }
            assert_eq!(fs::read_to_string(home.join(".cshrc")).unwrap(), cshrc);
            if has_tcshrc {
                assert_eq!(fs::read_to_string(home.join(".tcshrc")).unwrap(), tcshrc);
            }
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
    fn powershell_selects_first_executable_when_path_has_multiple_matches() {
        const UNEXPECTED_EXECUTABLE_EXIT_CODE: i32 = 1;
        let fixture = Fixture::new().unwrap();
        let root = fixture.directory.path();
        let second = root.join("second");
        fs::create_dir(&second).unwrap();
        let executable = second.join("cargo-cleaner");
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\nprintf 'wrong executable' >&2\nexit {UNEXPECTED_EXECUTABLE_EXIT_CODE}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
        let mut paths = vec![root.to_owned(), second];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "powershell"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let output = fixture
            .command("pwsh")
            .unwrap()
            .env("PATH", std::env::join_paths(paths).unwrap())
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                ". $PROFILE.CurrentUserAllHosts; cargo custom; exit $LASTEXITCODE",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(FORWARDED_EXIT_CODE),
            "{output:?}"
        );
        assert_eq!(output.stdout, b"custom\0");
        assert_eq!(output.stderr, b"diagnostic");
    }

    #[test]
    fn powershell_preserves_failure_status_and_cargo_flags() {
        let fixture = Fixture::new().unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "powershell"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let output = fixture.command("pwsh").unwrap()
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
                ". $PROFILE.CurrentUserAllHosts; cargo custom -p example --release -v; if ($?) { throw 'Failure reported as success' }; cargo custom && $(throw 'Failure continued through &&'); exit $LASTEXITCODE"])
            .output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(FORWARDED_EXIT_CODE),
            "{output:?}"
        );
        assert_eq!(
            output.stdout,
            b"custom\0-p\0example\0--release\0-v\0custom\0"
        );
        assert_eq!(output.stderr, b"diagnosticdiagnostic");

        fs::write(
            fixture.directory.path().join("cargo"),
            "#!/bin/sh\nexit 0\n",
        )
        .unwrap();
        let output = fixture.command("pwsh").unwrap()
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
                ". $PROFILE.CurrentUserAllHosts; cargo custom; if (!$?) { throw 'Success reported as failure' }; cargo custom && Write-Output 'continued'; exit $LASTEXITCODE"])
            .output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"continued\n");
        assert!(output.stderr.is_empty(), "{output:?}");
    }

    #[test]
    fn powershell_forwards_pipeline_and_direct_stdin_once() {
        let fixture = Fixture::new().unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "powershell"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let input = "first line\nsecond line\n";
        let input_path = fixture.directory.path().join("input.txt");
        fs::write(&input_path, input).unwrap();
        for exit_code in [0, FORWARDED_EXIT_CODE] {
            fs::write(
                fixture.directory.path().join("cargo"),
                format!("#!/bin/sh\nprintf 'invoked' >&2\ncat\nexit {exit_code}\n"),
            )
            .unwrap();
            for script in [
                ". $PROFILE.CurrentUserAllHosts; 'first line', 'second line' | cargo custom; exit $LASTEXITCODE",
                ". $PROFILE.CurrentUserAllHosts; cargo custom; exit $LASTEXITCODE",
            ] {
                let output = fixture
                    .command("pwsh")
                    .unwrap()
                    .args([
                        "-NoLogo",
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        script,
                    ])
                    .stdin(fs::File::open(&input_path).unwrap())
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
                assert_eq!(output.stdout, input.as_bytes(), "{script}");
                assert_eq!(output.stderr, b"invoked");
            }
        }
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

    #[cfg(target_os = "linux")]
    #[test]
    fn ion_setup_removal_and_forwarding() {
        let fixture = Fixture::new().unwrap();
        fixture.lifecycle("ion", ".config/ion/initrc").unwrap();
        let output = fixture
            .command(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .unwrap()
            .args(["init", "ion"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = fixture.command("ion").unwrap()
            .args(["-c", "source $HOME/.config/ion/initrc\ncargo '+stable' 'custom' '' 'with spaces' '*;$HOME'"])
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
