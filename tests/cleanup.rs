#[cfg(all(unix, test))]
mod tests {

    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::{Command, Output};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    impl Fixture {
        fn new(config: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
  *) printf 'forwarded'; exit 37;;
esac
"#,
            )?;
            fs::set_permissions(script, fs::Permissions::from_mode(0o755))?;
            Ok(Self {
                root: root.canonicalize()?,
            })
        }

        fn command(&self, args: &[&str]) -> Result<Command, Box<dyn std::error::Error>> {
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

        fn run(&self, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
            Ok(self.command(args)?.output()?)
        }

        fn calls(&self) -> String {
            fs::read_to_string(self.root.join("calls")).unwrap_or_default()
        }

        fn artifact(&self, relative: &str, size: u64) -> Result<(), Box<dyn std::error::Error>> {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().ok_or("artifact has no parent")?)?;
            fs::File::create(path)?.set_len(size)?;
            Ok(())
        }
    }

    #[test]
    fn oversize_target_cleans_before_forwarding_with_toolchain_and_config() {
        let fixture = Fixture::new("max_target_size_gib = 1\n").unwrap();
        fixture
            .artifact("target/debug/artifact", (1 << 30) + 1)
            .unwrap();
        let output = fixture
            .run(&["+stable", "build", "--config", "net.offline=true"])
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert_eq!(output.stdout, b"forwarded");
        let calls = fixture.calls();
        let lines: Vec<_> = calls.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("+stable metadata --config net.offline=true"));
        assert!(lines[1].starts_with("+stable clean --config net.offline=true --target-dir "));
        assert_eq!(lines[2], "+stable build --config net.offline=true");
    }

    #[test]
    fn threshold_equality_does_not_clean() {
        let fixture = Fixture::new("max_target_size_gib = 1").unwrap();
        fixture.artifact("target/artifact", 1 << 30).unwrap();
        assert_eq!(fixture.run(&["test"]).unwrap().status.code(), Some(37));
        assert_eq!(fixture.calls().lines().count(), 2);
        assert!(!fixture.calls().contains("clean"));
    }

    #[test]
    fn low_disk_alone_triggers_cleanup() {
        let fixture = Fixture::new("min_free_space_gib = 1000000000").unwrap();
        assert_eq!(fixture.run(&["check"]).unwrap().status.code(), Some(37));
        assert!(
            fixture
                .calls()
                .lines()
                .nth(1)
                .unwrap()
                .starts_with("clean ")
        );
    }

    #[test]
    fn target_override_is_measured_and_cleaned() {
        let fixture = Fixture::new("max_target_size_gib = 1").unwrap();
        fixture
            .artifact("custom target/artifact", (1 << 30) + 1)
            .unwrap();
        let output = fixture
            .run(&["build", "--target-dir=../../custom target"])
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert!(
            fixture
                .calls()
                .lines()
                .nth(1)
                .unwrap()
                .ends_with("/member/src/../../custom target")
        );
    }

    #[test]
    fn manifest_selects_project_config_from_outside_project() {
        let fixture = Fixture::new("max_target_size_gib = 1").unwrap();
        fixture.artifact("target/artifact", (1 << 30) + 1).unwrap();
        let manifest = fixture.root.join("Cargo.toml");
        let output = fixture
            .command(&["build", "--manifest-path", manifest.to_str().unwrap()])
            .unwrap()
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(37));
        assert!(
            fixture
                .calls()
                .lines()
                .nth(1)
                .unwrap()
                .starts_with("clean --manifest-path ")
        );
    }

    #[test]
    fn invalid_configuration_blocks_cleanup_and_build() {
        for config in [
            "max_target_size_gib = 0",
            "min_free_space_gib = -1",
            "max_target_size_gib = 999999999999",
            "max_target_szie_gib = 1",
            "not toml",
        ] {
            let fixture = Fixture::new(config).unwrap();
            assert!(!fixture.run(&["build"]).unwrap().status.success());
            assert!(fixture.calls().is_empty());
        }
    }

    #[test]
    fn both_thresholds_are_rejected_before_running_cargo() {
        let fixture = Fixture::new("max_target_size_gib = 20\nmin_free_space_gib = 10").unwrap();
        let output = fixture.run(&["build"]).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("set only one of max_target_size_gib or min_free_space_gib")
        );
        assert!(fixture.calls().is_empty());
    }

    #[test]
    fn failed_clean_blocks_build() {
        let fixture = Fixture::new("min_free_space_gib = 1000000000").unwrap();
        let output = fixture
            .command(&["build"])
            .unwrap()
            .env("CLEANER_CLEAN_EXIT", "9")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("requested command was not run"));
        assert_eq!(fixture.calls().lines().count(), 2);
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
                0
            } else {
                37
            };
            assert_eq!(fixture.run(&args).unwrap().status.code(), Some(expected));
            assert_eq!(fixture.calls().lines().count(), 1);
        }
    }

    #[test]
    fn absent_or_empty_config_preserves_forwarding() {
        for absent in [false, true] {
            let fixture = Fixture::new("").unwrap();
            if absent {
                fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
            }
            assert_eq!(fixture.run(&["build"]).unwrap().status.code(), Some(37));
            assert_eq!(fixture.calls(), "build\n");
        }
    }

    #[test]
    fn global_config_applies_outside_home_and_project_config_replaces_it() {
        for project in [None, Some(""), Some("max_target_size_gib = 1")] {
            let fixture = Fixture::new(project.unwrap_or("")).unwrap();
            fs::write(
                fixture.root.join("home/.cargo-cleaner.toml"),
                "min_free_space_gib = 1000000000",
            )
            .unwrap();
            if project.is_none() {
                fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
            }
            assert_eq!(fixture.run(&["build"]).unwrap().status.code(), Some(37));
            assert_eq!(
                fixture.calls().contains("clean --target-dir"),
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
        assert_eq!(fixture.run(&["build"]).unwrap().status.code(), Some(37));
        fs::remove_file(fixture.root.join("calls")).unwrap();
        fs::remove_file(fixture.root.join(".cargo-cleaner.toml")).unwrap();
        let output = fixture.run(&["build"]).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid"));
        assert!(fixture.calls().is_empty());
    }

    #[test]
    fn nearest_config_overrides_parent_and_symlinks_are_not_traversed() {
        let fixture = Fixture::new("min_free_space_gib = 1000000000").unwrap();
        fs::write(
            fixture.root.join("member/.cargo-cleaner.toml"),
            "max_target_size_gib = 1",
        )
        .unwrap();
        fixture.artifact("outside/artifact", (1 << 30) + 1).unwrap();
        std::os::unix::fs::symlink(
            fixture.root.join("outside"),
            fixture.root.join("target/link"),
        )
        .unwrap();
        assert_eq!(fixture.run(&["build"]).unwrap().status.code(), Some(37));
        assert_eq!(fixture.calls().lines().count(), 2);
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
        fs::write(
            fixture.root.join(".cargo-cleaner.toml"),
            "min_free_space_gib = 1000000000\n",
        )
        .unwrap();
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
        let fixture = Fixture::new("max_target_size_gib = 1").unwrap();
        fixture.artifact("target/artifact", (1 << 30) + 1).unwrap();
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
        assert_eq!(output.status.code(), Some(37));
        assert!(fixture.calls().lines().nth(1).unwrap().ends_with("/target"));
        assert!(
            fixture
                .calls()
                .lines()
                .nth(2)
                .unwrap()
                .ends_with("-- --help --target-dir unrelated")
        );
    }
}
