# cargo-cleaner

A Cargo wrapper that automatically frees disk space before build commands. It runs
`cargo clean` when a target exceeds its size limit or free disk space falls below
its minimum, then forwards your command to Cargo.

## Setup

Requires Cargo and a supported shell. Install a prebuilt binary for Linux or macOS (x86_64 or
ARM64) using [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall --git https://github.com/anotherempty/cargo-cleaner cargo-cleaner
```

Or use the shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/anotherempty/cargo-cleaner/releases/latest/download/cargo-cleaner-installer.sh | sh
```

Ensure `$CARGO_HOME/bin` (normally `~/.cargo/bin`) is on `PATH`, then run:

```sh
cargo-cleaner init zsh
```

Replace `zsh` with the argument for your shell from the table below, then open a
new session:

| Shell | Argument | Integration file |
| --- | --- | --- |
| Bash | `bash` | `~/.bashrc` |
| Cmd + Clink 1.6.11+ | `cmd` | `%CLINK_PROFILE%/cargo-cleaner.lua`, otherwise `%LOCALAPPDATA%/clink/cargo-cleaner.lua` |
| Elvish 0.21+ | `elvish` | `<config>/elvish/rc.elv` |
| Fish | `fish` | `<config>/fish/conf.d/cargo-cleaner.fish` |
| Ion | `ion` | `<config>/ion/initrc` |
| Nushell | `nu` | `<config>/nushell/config.nu` |
| PowerShell 7.3+ | `powershell` | `$PROFILE.CurrentUserAllHosts` and adjacent `cargo-cleaner.ps1`, located using `pwsh` |
| Tcsh | `tcsh` | `~/.tcshrc`, or existing `~/.cshrc` if `.tcshrc` is absent |
| Xonsh | `xonsh` | `~/.xonshrc` |
| Zsh | `zsh` | `$ZDOTDIR/.zshrc`, otherwise `~/.zshrc` |

`<config>` uses `XDG_CONFIG_HOME` when set, otherwise `~/.config` on Unix or
`%APPDATA%` on Windows. Nushell defaults to `~/Library/Application Support` on macOS.
Bash login profiles must source `.bashrc`. Cmd integration applies to interactive
commands, not batch scripts. On Windows, build from source and set `HOME` to your
user directory; prebuilt Windows distributions are not yet provided.

Setup preserves the rest of your shell configuration and creates the default global
cleanup config if missing. Existing `cargo` functions or aliases may be replaced; remove conflicting
aliases in Bash, Zsh, or PowerShell first. IDEs invoking Cargo directly are unaffected.
To remove integration, run `cargo-cleaner uninstall <shell>` and open a new session.
The binary and cleanup configs remain installed.

## Configuration

Global settings live in `~/.cargo-cleaner.toml`. Setup creates only the
`[free_space]` settings shown below; the other settings are optional:

```toml
verbose = false
max_target_size_gib = 20

[free_space]
min_free_space_gib = 100
across_projects = true
```

1. `max_target_size_gib`: clean the current target when its size exceeds this limit.
2. `free_space.min_free_space_gib`: clean when free space on the workspace's
   filesystem falls below this minimum.
3. `free_space.across_projects`: clean the largest known target when `true`, or
   only the current project's target when `false`. Both free-space fields are required.
4. `verbose`: show cleanup progress and timings on stderr; defaults to `false`.

Specify thresholds as positive whole numbers in GiB. Omit `[free_space]` to disable
free-space cleanup and project registration. Existing config files are preserved during setup.

For a project-specific size limit, add `.cargo-cleaner.toml` at the project root:

```toml
max_target_size_gib = 10
verbose = true
```

The nearest project configuration determines the size limit instead of the global
configuration; an empty file disables size-based cleanup. `verbose` inherits the
global setting when omitted.
`[free_space]` is global-only and cannot be set or disabled by a project config.

## How it works

1. Before `build`, `check`, `test`, `run`, `bench`, `doc`, `clippy`, `rustc`, or
   `rustdoc`, the wrapper asks Cargo for the workspace and target directory.
   Standard single-letter aliases are supported; other commands pass through.
2. With cross-project cleanup enabled, existing targets are recorded in
   `~/.cargo-cleaner/projects.toml`. Projects are discovered as you use the wrapper,
   without scanning your disk.
3. The free-space policy runs first, then the target-size policy. Each policy cleans
   at most one target directory. Cleanup removes **all Cargo artifacts in the selected target**,
   including other profiles or projects sharing it. Source files remain intact.
4. The original command runs with its arguments and exit status preserved.
   Configuration, measurement, registry, or cleanup errors stop the command.

Cleanup does not guarantee enough space for the next build. Cross-project cleanup
can select a target on another filesystem, which may not free space where needed.

## Development

Install Rust and the shells exercised by the tests (Bash, Elvish, Fish, Ion, Nushell,
PowerShell, Tcsh, Xonsh, and Zsh), then run:

```sh
cargo test --all-targets --all-features
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Tests use temporary projects and isolated shell configuration files. Cmd/Clink
setup/removal is tested. Its runtime behavior still needs to be tested on Windows.

Releases use [cargo-dist](https://axodotdev.github.io/cargo-dist/), configured in
`dist-workspace.toml`:

```sh
cargo install cargo-dist --version 0.33.0 --locked
dist generate --check
dist plan
dist build --artifacts=local
```

Run `dist generate` after changing distribution settings. Local artifacts go to
`target/distrib/`. To release, update `Cargo.toml` and `Cargo.lock`, run the checks,
commit and push, then push a matching version tag such as `v0.1.0`.
GitHub Actions publishes archives, a shell installer, and checksums to GitHub
Releases. Pull requests check the release plan.

## License

Licensed under either the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
