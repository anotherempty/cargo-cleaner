use std::process::Command;

#[test]
fn rejects_unsupported_shell_without_printing_shell_code() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
        .args(["init", "fish"])
        .output()
        .expect("run cleaner");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected `init zsh`"));
}

#[cfg(all(unix, test))]
mod zsh {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::Command;

    struct Fixture(PathBuf);

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl Fixture {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let path = std::env::temp_dir().join(format!(
                "cargo-cleaner-shell-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos()
            ));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }
    }

    #[test]
    fn init_preserves_config_migrates_legacy_and_is_repeatable() {
        let fixture = Fixture::new().expect("create fixture");
        let rc = fixture.0.join(".zshrc");
        fs::write(
            &rc,
            "# keep me\neval \"$(cargo-cleaner init zsh)\"\nexport KEEP=yes",
        )
        .expect("write config");
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
                .args(["init", "zsh"])
                .env("ZDOTDIR", &fixture.0)
                .env_remove("HOME")
                .output()
                .expect("run init");
            assert!(output.status.success(), "{:?}", output);
        }
        assert_eq!(
            fs::read_to_string(rc).expect("read config"),
            "# keep me\nexport KEEP=yes\ncargo() {\n    command cargo-cleaner exec -- \"$@\"\n}\n"
        );
    }

    #[test]
    fn init_reports_missing_home_and_unwritable_config() {
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .args(["init", "zsh"])
            .env_remove("HOME")
            .env_remove("ZDOTDIR")
            .output()
            .expect("run init");
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("neither ZDOTDIR nor HOME is set")
        );
        let fixture = Fixture::new().expect("create fixture");
        fs::create_dir(fixture.0.join(".zshrc")).expect("create directory");
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .args(["init", "zsh"])
            .env("HOME", &fixture.0)
            .env_remove("ZDOTDIR")
            .output()
            .expect("run init");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read"));
    }

    #[test]
    fn uninstall_removes_integration_preserves_config_and_is_repeatable() {
        let fixture = Fixture::new().expect("create fixture");
        let rc = fixture.0.join(".zshrc");
        let original = "# keep me\ncargo_custom() { echo custom; }\n";
        fs::write(&rc, original).expect("write config");
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .args(["init", "zsh"])
            .env("ZDOTDIR", &fixture.0)
            .env_remove("HOME")
            .output()
            .expect("run init");
        assert!(output.status.success());
        let mut contents = fs::read_to_string(&rc).expect("read config");
        contents.push_str("eval \"$(cargo-cleaner init zsh)\"\n# trailing config\n");
        fs::write(&rc, contents).expect("write legacy config");
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
                .args(["uninstall", "zsh"])
                .env("ZDOTDIR", &fixture.0)
                .env_remove("HOME")
                .output()
                .expect("run uninstall");
            assert!(output.status.success(), "{output:?}");
            assert_eq!(
                fs::read_to_string(&rc).expect("read config"),
                format!("{original}# trailing config\n")
            );
        }
    }

    #[test]
    fn uninstall_missing_config_does_not_create_file() {
        let fixture = Fixture::new().expect("create fixture");
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .args(["uninstall", "zsh"])
            .env("HOME", &fixture.0)
            .env_remove("ZDOTDIR")
            .output()
            .expect("run uninstall");
        assert!(output.status.success(), "{output:?}");
        assert!(!fixture.0.join(".zshrc").exists());
    }

    #[test]
    fn shell_forwards_arguments_streams_environment_directory_and_status() {
        let fixture = Fixture::new().expect("create fixture");
        let cargo = fixture.0.join("cargo");
        fs::write(
            &cargo,
            "#!/bin/sh\nprintf '%s\\0' \"$@\"\nprintf '%s\\0' \"$PWD\" \"$CLEANER_TEST_VALUE\"\nprintf 'cargo diagnostic' >&2\nexit 37\n",
        )
        .expect("write fake cargo");
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).expect("make executable");
        std::os::unix::fs::symlink(
            env!("CARGO_BIN_EXE_cargo-cleaner"),
            fixture.0.join("cargo-cleaner"),
        )
        .expect("link wrapper");

        let mut paths = vec![fixture.0.clone()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let output = Command::new("zsh")
            .args([
                "-f",
                "-c",
                "cargo-cleaner init zsh >/dev/null || exit; source \"$HOME/.zshrc\" || exit; cargo \"$@\"",
                "shell-test",
                "+stable",
                "build",
                "--manifest-path",
                "directory with spaces/Cargo.toml",
                "",
                "literal * $HOME ;",
            ])
            .current_dir(&fixture.0)
            .env("PATH", std::env::join_paths(paths).expect("join PATH"))
            .env("HOME", &fixture.0)
            .env_remove("ZDOTDIR")
            .env("CLEANER_TEST_VALUE", "inherited value")
            .output()
            .expect("run zsh; install zsh to run shell integration tests");

        assert_eq!(output.status.code(), Some(37));
        assert_eq!(output.stderr, b"cargo diagnostic");
        let expected = format!(
            "+stable\0build\0--manifest-path\0directory with spaces/Cargo.toml\0\0literal * $HOME ;\0{}\0inherited value\0",
            fixture
                .0
                .canonicalize()
                .expect("resolve directory")
                .display()
        );
        assert_eq!(output.stdout, expected.as_bytes());
    }
}
