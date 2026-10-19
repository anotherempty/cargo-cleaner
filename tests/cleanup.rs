#[cfg(all(unix, test))]
mod tests {

    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::{Command, Output};
    use std::sync::atomic::{AtomicU64, Ordering};

    const METADATA_CALL_INDEX: usize = 0;
    const CLEAN_CALL_INDEX: usize = 1;
    const FORWARD_AFTER_CLEAN_INDEX: usize = 2;
    const CLEAN_AND_FORWARD_CALLS: usize = 3;
    const METADATA_AND_FORWARD_CALLS: usize = 2;
    const METADATA_AND_FAILED_CLEAN_CALLS: usize = 2;
    const FORWARD_ONLY_CALLS: usize = 1;
    const CONFLICTING_THRESHOLDS_CONFIG: &str = "max_target_size_gib = 20\nmin_free_space_gib = 10";
    const ZERO_LIMIT_CONFIG: &str = "max_target_size_gib = 0";
    const NEGATIVE_LIMIT_CONFIG: &str = "min_free_space_gib = -1";
    const OVERFLOW_LIMIT_CONFIG: &str = "max_target_size_gib = 999999999999";
    const UNKNOWN_FIELD_CONFIG: &str = "max_target_szie_gib = 1";
    const BYTES_PER_GIB: u64 = 1 << 30;
    const OVERSIZED_ARTIFACT_BYTES: u64 = BYTES_PER_GIB + 1;
    const EXECUTABLE_MODE: u32 = 0o755;
    const FORWARDED_EXIT_CODE: i32 = 37;
    const CLEAN_FAILURE_EXIT_CODE: i32 = 9;
    const WRAPPER_FAILURE_EXIT_CODE: i32 = 1;
    const SUCCESS_EXIT_CODE: i32 = 0;
    const TARGET_LIMIT_CONFIG: &str = "max_target_size_gib = 1";
    // An intentionally unattainable free-space requirement forces cleanup.
    const LOW_DISK_CONFIG: &str = "min_free_space_gib = 1000000000";

    #[derive(Debug, thiserror::Error)]
    enum FixtureError {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        Path(#[from] std::env::JoinPathsError),
    }

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Err(error) = fs::remove_dir_all(&self.root) {
                assert!(std::thread::panicking(), "remove fixture: {error}");
            }
        }
    }

    impl Fixture {
        fn new(config: &str) -> std::io::Result<Self> {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "cargo-cleaner-policy-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("bin"))?;
            fs::create_dir_all(root.join("home"))?;
            fs::create_dir_all(root.join("target"))?;
            fs::create_dir_all(root.join("member/src"))?;
            fs::write(root.join(".cargo-cleaner.toml"), config)?;
            let script = root.join("bin/cargo");
            fs::write(
                &script,
                r#"#!/bin/sh
printf '%s\n' "$*" >> "$CLEANER_LOG"
case "$1" in +*) shift;; esac
case "$1" in
  metadata) printf '%s' "$CLEANER_METADATA";;
  clean) exit "${CLEANER_CLEAN_EXIT:-0}";;
  *) printf 'forwarded'; exit "$CLEANER_FORWARD_EXIT";;
esac
"#,
            )?;
            fs::set_permissions(script, fs::Permissions::from_mode(EXECUTABLE_MODE))?;
            Ok(Self {
                root: root.canonicalize()?,
            })
        }

        fn command(&self, args: &[&str]) -> Result<Command, FixtureError> {
            let mut paths = vec![self.root.join("bin")];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"));
            command
                .args(["exec", "--"])
                .args(args)
                .current_dir(self.root.join("member/src"))
                .env("HOME", self.root.join("home"))
                .env("PATH", std::env::join_paths(paths)?)
                .env("CLEANER_FORWARD_EXIT", FORWARDED_EXIT_CODE.to_string())
                .env("CLEANER_LOG", self.root.join("calls"))
                .env(
                    "CLEANER_METADATA",
                    serde_json::json!({
                        "workspace_root": self.root,
                        "target_directory": self.root.join("target"),
                    })
                    .to_string(),
                );
            Ok(command)
        }

        fn run(&self, args: &[&str]) -> Result<Output, FixtureError> {
            Ok(self.command(args)?.output()?)
        }

        fn calls(&self) -> std::io::Result<String> {
            match fs::read_to_string(self.root.join("calls")) {
                Ok(calls) => Ok(calls),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
                Err(error) => Err(error),
            }
        }

        fn artifact(&self, relative: &str, size: u64) -> std::io::Result<()> {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().ok_or(std::io::ErrorKind::InvalidInput)?)?;
            fs::File::create(path)?.set_len(size)?;
            Ok(())
        }
    }

    #[test]
    fn oversize_target_cleans_before_forwarding_with_toolchain_and_config() {
        let fixture = Fixture::new(TARGET_LIMIT_CONFIG).unwrap();
        fixture
            .artifact("target/debug/artifact", OVERSIZED_ARTIFACT_BYTES)
            .unwrap();
        let output = fixture
            .run(&["+stable", "build", "--config", "net.offline=true"])
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert_eq!(output.stdout, b"forwarded");
        let calls = fixture.calls().unwrap();
        let lines: Vec<_> = calls.lines().collect();
        assert_eq!(lines.len(), CLEAN_AND_FORWARD_CALLS);
        assert!(
            lines[METADATA_CALL_INDEX].starts_with("+stable metadata --config net.offline=true")
        );
        assert!(
            lines[CLEAN_CALL_INDEX]
                .starts_with("+stable clean --config net.offline=true --target-dir ")
        );
        assert_eq!(
            lines[FORWARD_AFTER_CLEAN_INDEX],
            "+stable build --config net.offline=true"
        );
    }

    #[test]
    fn threshold_equality_does_not_clean() {
        let fixture = Fixture::new(TARGET_LIMIT_CONFIG).unwrap();
        fixture.artifact("target/artifact", BYTES_PER_GIB).unwrap();
        assert_eq!(
            fixture.run(&["test"]).unwrap().status.code(),
            Some(FORWARDED_EXIT_CODE)
        );
        assert_eq!(
            fixture.calls().unwrap().lines().count(),
            METADATA_AND_FORWARD_CALLS
        );
        assert!(!fixture.calls().unwrap().contains("clean"));
    }

    #[test]
    fn low_disk_alone_triggers_cleanup() {
        let fixture = Fixture::new(LOW_DISK_CONFIG).unwrap();
        assert_eq!(
            fixture.run(&["check"]).unwrap().status.code(),
            Some(FORWARDED_EXIT_CODE)
        );
        assert!(
            fixture
                .calls()
                .unwrap()
                .lines()
                .nth(CLEAN_CALL_INDEX)
                .unwrap()
                .starts_with("clean ")
        );
    }

    #[test]
    fn target_override_is_measured_and_cleaned() {
        let fixture = Fixture::new(TARGET_LIMIT_CONFIG).unwrap();
        fixture
            .artifact("custom target/artifact", OVERSIZED_ARTIFACT_BYTES)
            .unwrap();
        let output = fixture
            .run(&["build", "--target-dir=../../custom target"])
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert!(
            fixture
                .calls()
                .unwrap()
                .lines()
                .nth(CLEAN_CALL_INDEX)
                .unwrap()
                .ends_with("/member/src/../../custom target")
        );
    }

    #[test]
    fn manifest_selects_project_config_from_outside_project() {
        let fixture = Fixture::new(TARGET_LIMIT_CONFIG).unwrap();
        fixture
            .artifact("target/artifact", OVERSIZED_ARTIFACT_BYTES)
            .unwrap();
        let manifest = fixture.root.join("Cargo.toml");
        let output = fixture
            .command(&["build", "--manifest-path", manifest.to_str().unwrap()])
            .unwrap()
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert!(
            fixture
                .calls()
                .unwrap()
                .lines()
                .nth(CLEAN_CALL_INDEX)
                .unwrap()
                .starts_with("clean --manifest-path ")
        );
    }

    #[test]
    fn invalid_configuration_blocks_cleanup_and_build() {
        for config in [
            ZERO_LIMIT_CONFIG,
            NEGATIVE_LIMIT_CONFIG,
            OVERFLOW_LIMIT_CONFIG,
            UNKNOWN_FIELD_CONFIG,
            "not toml",
        ] {
            let fixture = Fixture::new(config).unwrap();
            assert!(!fixture.run(&["build"]).unwrap().status.success());
            assert!(fixture.calls().unwrap().is_empty());
        }
    }

    #[test]
    fn both_thresholds_are_rejected_before_running_cargo() {
        let fixture = Fixture::new(CONFLICTING_THRESHOLDS_CONFIG).unwrap();
        let output = fixture.run(&["build"]).unwrap();
        assert_eq!(output.status.code(), Some(WRAPPER_FAILURE_EXIT_CODE));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("set only one of max_target_size_gib or min_free_space_gib")
        );
        assert!(fixture.calls().unwrap().is_empty());
    }

    #[test]
    fn failed_clean_blocks_build() {
        let fixture = Fixture::new(LOW_DISK_CONFIG).unwrap();
        let output = fixture
            .command(&["build"])
            .unwrap()
            .env("CLEANER_CLEAN_EXIT", CLEAN_FAILURE_EXIT_CODE.to_string())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(WRAPPER_FAILURE_EXIT_CODE));
        assert!(String::from_utf8_lossy(&output.stderr).contains("requested command was not run"));
        assert_eq!(
            fixture.calls().unwrap().lines().count(),
            METADATA_AND_FAILED_CLEAN_CALLS
        );
    }

    #[test]
    fn non_build_commands_and_help_bypass_cleanup() {
        for args in [
            vec!["clean"],
            vec!["fmt"],
            vec!["metadata"],
            vec!["build", "--help"],
            vec!["install", "some-package"],
        ] {
            let fixture = Fixture::new("invalid config").unwrap();
            let expected = if matches!(args[0], "clean" | "metadata") {
                SUCCESS_EXIT_CODE
            } else {
                FORWARDED_EXIT_CODE
            };
            assert_eq!(fixture.run(&args).unwrap().status.code(), Some(expected));
            assert_eq!(fixture.calls().unwrap().lines().count(), FORWARD_ONLY_CALLS);
        }
    }

    #[test]
    fn absent_or_empty_config_preserves_forwarding() {
        for absent in [false, true] {
            let fixture = Fixture::new("").unwrap();
            if absent {
                fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
            }
            assert_eq!(
                fixture.run(&["build"]).unwrap().status.code(),
                Some(FORWARDED_EXIT_CODE)
            );
            assert_eq!(fixture.calls().unwrap(), "build\n");
        }
    }

    #[test]
    fn global_config_applies_outside_home_and_project_config_replaces_it() {
        for project in [None, Some(""), Some(TARGET_LIMIT_CONFIG)] {
            let fixture = Fixture::new(project.unwrap_or("")).unwrap();
            fs::write(
                fixture.root.join("home/.cargo-cleaner.toml"),
                LOW_DISK_CONFIG,
            )
            .unwrap();
            if project.is_none() {
                fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
            }
            assert_eq!(
                fixture.run(&["build"]).unwrap().status.code(),
                Some(FORWARDED_EXIT_CODE)
            );
            assert_eq!(
                fixture.calls().unwrap().contains("clean --target-dir"),
                project.is_none()
            );
        }
    }

    #[test]
    fn invalid_global_config_blocks_build_unless_project_overrides_it() {
        let fixture = Fixture::new("").unwrap();
        fs::write(
            fixture.root.join("home/.cargo-cleaner.toml"),
            "invalid toml",
        )
        .unwrap();
        assert_eq!(
            fixture.run(&["build"]).unwrap().status.code(),
            Some(FORWARDED_EXIT_CODE)
        );
        fs::remove_file(fixture.root.join("calls")).unwrap();
        fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
        let output = fixture.run(&["build"]).unwrap();
        assert_eq!(output.status.code(), Some(WRAPPER_FAILURE_EXIT_CODE));
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid"));
        assert!(fixture.calls().unwrap().is_empty());
    }

    #[test]
    fn nearest_config_overrides_parent_and_symlinks_are_not_traversed() {
        let fixture = Fixture::new(LOW_DISK_CONFIG).unwrap();
        fs::write(
            fixture.root.join("member/.cargo-cleaner.toml"),
            TARGET_LIMIT_CONFIG,
        )
        .unwrap();
        fixture
            .artifact("outside/artifact", OVERSIZED_ARTIFACT_BYTES)
            .unwrap();
        std::os::unix::fs::symlink(
            fixture.root.join("outside"),
            fixture.root.join("target/link"),
        )
        .unwrap();
        assert_eq!(
            fixture.run(&["build"]).unwrap().status.code(),
            Some(FORWARDED_EXIT_CODE)
        );
        assert_eq!(
            fixture.calls().unwrap().lines().count(),
            METADATA_AND_FORWARD_CALLS
        );
    }

    #[test]
    fn real_cargo_cleans_workspace_configured_target_then_rebuilds_member() {
        let fixture = Fixture::new("").unwrap();
        fs::write(
            fixture.root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            fixture.root.join("member/Cargo.toml"),
            "[package]\nname = \"cleanup-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(fixture.root.join("member/src/main.rs"), "fn main() {}\n").unwrap();
        fs::create_dir(fixture.root.join(".cargo")).unwrap();
        fs::write(
            fixture.root.join(".cargo/config.toml"),
            "[build]\ntarget-dir = \"custom-target\"\n",
        )
        .unwrap();
        let build = Command::new(env!("CARGO"))
            .args(["build", "--offline"])
            .current_dir(&fixture.root)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
        let marker = fixture.root.join("custom-target/cleanup-marker");
        fs::write(&marker, "old artifact").unwrap();
        fs::write(fixture.root.join(".cargo-cleaner.toml"), LOW_DISK_CONFIG).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-cleaner"))
            .args(["exec", "--", "build", "--offline"])
            .current_dir(fixture.root.join("member/src"))
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!marker.exists());
        assert!(
            fixture
                .root
                .join("custom-target/debug/cleanup-fixture")
                .is_file()
        );
        assert!(fixture.root.join("member/src/main.rs").is_file());
        assert!(String::from_utf8_lossy(&output.stderr).contains("running cargo clean"));
    }
    #[test]
    fn attached_manifest_selects_configuration_and_program_arguments_are_ignored() {
        let fixture = Fixture::new(TARGET_LIMIT_CONFIG).unwrap();
        fixture
            .artifact("target/artifact", OVERSIZED_ARTIFACT_BYTES)
            .unwrap();
        let manifest = format!("-m{}", fixture.root.join("Cargo.toml").display());
        let output = fixture
            .command(&[
                "run",
                &manifest,
                "--",
                "--help",
                "--target-dir",
                "unrelated",
            ])
            .unwrap()
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(FORWARDED_EXIT_CODE));
        assert!(
            fixture
                .calls()
                .unwrap()
                .lines()
                .nth(CLEAN_CALL_INDEX)
                .unwrap()
                .ends_with("/target")
        );
        assert!(
            fixture
                .calls()
                .unwrap()
                .lines()
                .nth(FORWARD_AFTER_CLEAN_INDEX)
                .unwrap()
                .ends_with("-- --help --target-dir unrelated")
        );
    }
}
