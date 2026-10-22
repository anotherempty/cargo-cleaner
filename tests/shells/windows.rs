use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const FORWARDED_EXIT_CODE: i32 = 37;

#[derive(Debug, thiserror::Error)]
enum FixtureError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Paths(#[from] std::env::JoinPathsError),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}

struct Fixture {
    directory: tempfile::TempDir,
    powershell: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, FixtureError> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let output = Command::new("pwsh")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-Command",
                "[Console]::Write((Get-Process -Id $PID).Path)",
            ])
            .output()?;
        assert!(output.status.success(), "{output:?}");
        let powershell = PathBuf::from(String::from_utf8(output.stdout)?);
        fs::write(root.join(".cargo-target-cleaner.toml"), "")?;
        fs::write(
            root.join("helper.rs"),
            include_str!("../fixtures/shell-cargo.rs"),
        )?;
        let output = Command::new("rustc")
            .arg("--edition=2024")
            .arg(root.join("helper.rs"))
            .arg("-o")
            .arg(root.join("cargo.exe"))
            .output()?;
        assert!(output.status.success(), "{output:?}");
        fs::copy(root.join("cargo.exe"), root.join("pwsh.exe"))?;
        fs::copy(
            env!("CARGO_BIN_EXE_cargo-target-cleaner"),
            root.join("cargo-target-cleaner.exe"),
        )?;
        Ok(Self {
            directory,
            powershell,
        })
    }

    fn command(&self, executable: impl AsRef<std::ffi::OsStr>) -> Result<Command, FixtureError> {
        let root = self.directory.path();
        let mut paths = vec![root.to_owned()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let mut command = Command::new(executable);
        command
            .current_dir(root)
            .env("HOME", root)
            .env("APPDATA", root.join("appdata"))
            .env_remove("XDG_CONFIG_HOME")
            .env("CLINK_PROFILE", root.join("clink"))
            .env("TEST_PROFILE", root.join("profile.ps1"))
            .env("PATH", std::env::join_paths(paths)?);
        Ok(command)
    }

    fn configure(&self, action: &str, shell: &str) -> Result<(), FixtureError> {
        let output = self
            .command(env!("CARGO_BIN_EXE_cargo-target-cleaner"))?
            .args([action, shell])
            .output()?;
        assert!(output.status.success(), "{output:?}");
        Ok(())
    }

    fn powershell(&self, script: &str) -> Result<Output, FixtureError> {
        Ok(self
            .command(&self.powershell)?
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .output()?)
    }
}

#[test]
fn powershell_lifecycle_arguments_status_and_stdin() {
    let fixture = Fixture::new().unwrap();
    let profile = fixture.directory.path().join("profile.ps1");
    let original = "# preserved profile\n";
    fs::write(&profile, original).unwrap();
    fixture.configure("init", "powershell").unwrap();
    let installed = fs::read(&profile).unwrap();
    fixture.configure("init", "powershell").unwrap();
    assert_eq!(fs::read(&profile).unwrap(), installed);
    let output = fixture.powershell(". ./profile.ps1; cargo '+stable' 'custom' '' 'with spaces' '*;$HOME'; exit $LASTEXITCODE").unwrap();
    assert_eq!(
        output.status.code(),
        Some(FORWARDED_EXIT_CODE),
        "{output:?}"
    );
    assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
    assert_eq!(output.stderr, b"diagnostic");
    let output = fixture
        .powershell(
            ". ./profile.ps1; cargo success; if (!$?) { throw 'success lost' }; exit $LASTEXITCODE",
        )
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let output = fixture.powershell(". ./profile.ps1; 'first', 'second' | cargo stdin; if ($?) { throw 'failure lost' }; exit $LASTEXITCODE").unwrap();
    assert_eq!(
        output.status.code(),
        Some(FORWARDED_EXIT_CODE),
        "{output:?}"
    );
    assert_eq!(output.stdout, b"stdin\0first\r\nsecond\r\n");
    assert_eq!(output.stderr, b"diagnostic");
    for action in ["uninstall", "uninstall"] {
        fixture.configure(action, "powershell").unwrap();
        assert_eq!(fs::read_to_string(&profile).unwrap(), original);
        assert!(
            fs::read(fixture.directory.path().join("cargo-target-cleaner.ps1"))
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn nushell_lifecycle_and_forwarding() {
    let fixture = Fixture::new().unwrap();
    let config = fixture.directory.path().join("appdata/nushell/config.nu");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let original = "# preserved config\n";
    fs::write(&config, original).unwrap();
    fixture.configure("init", "nu").unwrap();
    let installed = fs::read(&config).unwrap();
    fixture.configure("init", "nu").unwrap();
    assert_eq!(fs::read(&config).unwrap(), installed);
    let output = fixture.command("nu").unwrap()
        .args(["--no-config-file", "-c", "source appdata/nushell/config.nu; cargo '+stable' 'custom' '' 'with spaces' '*;$HOME'; exit $env.LAST_EXIT_CODE"])
        .output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(FORWARDED_EXIT_CODE),
        "{output:?}"
    );
    assert_eq!(output.stdout, b"+stable\0custom\0\0with spaces\0*;$HOME\0");
    assert_eq!(output.stderr, b"diagnostic");
    for action in ["uninstall", "uninstall"] {
        fixture.configure(action, "nu").unwrap();
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
    }
}

#[test]
fn cmd_clink_configuration_is_repeatable_and_preserves_user_content() {
    let fixture = Fixture::new().unwrap();
    let config = fixture
        .directory
        .path()
        .join("clink/cargo-target-cleaner.lua");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let original = "-- preserved config\n";
    fs::write(&config, original).unwrap();
    fixture.configure("init", "cmd").unwrap();
    let installed = fs::read_to_string(&config).unwrap();
    assert!(installed.starts_with(original));
    assert!(installed.contains("os.setalias('cargo', 'cargo-target-cleaner exec -- $*')"));
    fixture.configure("init", "cmd").unwrap();
    assert_eq!(fs::read_to_string(&config).unwrap(), installed);
    for action in ["uninstall", "uninstall"] {
        fixture.configure(action, "cmd").unwrap();
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
    }
}
