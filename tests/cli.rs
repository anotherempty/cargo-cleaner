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
                "eval \"$(cargo-cleaner init zsh)\" || exit; cargo \"$@\"",
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
