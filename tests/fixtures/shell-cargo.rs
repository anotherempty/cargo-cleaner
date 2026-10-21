use std::io::{self, Read, Write};
use std::process::ExitCode;

const FAILURE_EXIT_CODE: u8 = 37;

fn main() -> io::Result<ExitCode> {
    // Isolate PowerShell profile discovery without changing the user's profile.
    if std::env::current_exe()?
        .file_stem()
        .is_some_and(|name| name == "pwsh")
    {
        io::stdout().write_all(
            std::env::var_os("TEST_PROFILE")
                .unwrap_or_default()
                .as_encoded_bytes(),
        )?;
        return Ok(ExitCode::SUCCESS);
    }
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    for argument in &arguments {
        io::stdout().write_all(argument.as_bytes())?;
        io::stdout().write_all(b"\0")?;
    }
    if arguments.iter().any(|argument| argument == "stdin") {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        io::stdout().write_all(&input)?;
    }
    io::stderr().write_all(b"diagnostic")?;
    Ok(if arguments.iter().any(|argument| argument == "success") {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(FAILURE_EXIT_CODE)
    })
}
