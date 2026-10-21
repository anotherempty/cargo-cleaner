use std::path::PathBuf;
use std::process::Command;

use super::Shell;
use super::config::{Action, ShellConfig};
use crate::error::Error;

pub struct PowerShell;

impl Shell for PowerShell {
    fn setup(&self) -> Result<String, Error> {
        let path = self.configure(Action::Install)?;
        Ok(format!(
            "Cargo integration configured in {}. Open a new powershell session to activate it.\n",
            path.display()
        ))
    }

    fn remove(&self) -> Result<String, Error> {
        let path = self.configure(Action::Remove)?;
        Ok(format!(
            "Cargo integration removed from {}. Open a new powershell session.\n",
            path.display()
        ))
    }
}

impl PowerShell {
    const POWERSHELL_INIT: &str = "Set-Alias -Name cargo -Value (Join-Path $PSScriptRoot 'cargo-cleaner.ps1') -Scope Global\n";
    const POWERSHELL_WRAPPER: &str = "if ($MyInvocation.ExpectingInput) {\n    $input | & (Get-Command cargo-cleaner -CommandType Application -TotalCount 1 -ErrorAction Stop) exec -- @args\n} else {\n    & (Get-Command cargo-cleaner -CommandType Application -TotalCount 1 -ErrorAction Stop) exec -- @args\n}\nexit $LASTEXITCODE\n";

    fn configure(&self, action: Action) -> Result<PathBuf, Error> {
        let output = Command::new("pwsh")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Write($PROFILE.CurrentUserAllHosts)",
            ])
            .output()
            .map_err(Error::PowerShellProfile)?;
        if !output.status.success() {
            return Err(Error::PowerShellProfileFailed {
                status: output.status,
                stderr: output.stderr,
            });
        }
        let profile = String::from_utf8(output.stdout).map_err(Error::ProfileEncoding)?;
        if profile.is_empty() {
            return Err(Error::EmptyPowerShellProfile);
        }
        let path = PathBuf::from(profile);
        let wrapper = path.with_file_name("cargo-cleaner.ps1");
        ShellConfig::update(&wrapper, Self::POWERSHELL_WRAPPER, action)?;
        ShellConfig::update(&path, Self::POWERSHELL_INIT, action)?;
        Ok(path)
    }
}
