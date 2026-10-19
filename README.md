# cargo-cleaner

A Cargo wrapper that automatically frees disk space before build commands. It runs
`cargo clean` when configured limits are crossed, then forwards your command to Cargo.

## Setup

Requires Cargo and a supported shell. Install a prebuilt binary for Linux or macOS (x86_64 or
ARM64) using [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall --git https://github.com/anotherempty/cargo-cleaner cargo-cleaner
```

This uses binaries from GitHub Releases without requiring a crates.io package.

Or use the shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/anotherempty/cargo-cleaner/releases/latest/download/cargo-cleaner-installer.sh | sh
```

Ensure `$CARGO_HOME/bin` (normally `~/.cargo/bin`) is on `PATH`, then run:

```sh
cargo-cleaner init zsh
```

Use `cargo-cleaner init bash` for Bash or `cargo-cleaner init zsh` for Zsh.
Xonsh: `cargo-cleaner init xonsh` adds an alias to `~/.xonshrc`.

Tcsh: `cargo-cleaner init tcsh` adds an alias to `~/.tcshrc`.

PowerShell 7.3+: `cargo-cleaner init powershell` uses `pwsh` to locate and
update `$PROFILE.CurrentUserAllHosts`.

Nushell: `cargo-cleaner init nu` updates `nushell/config.nu` under
`XDG_CONFIG_HOME`, or the platform config directory (on macOS,
`~/Library/Application Support`; on Windows, `%APPDATA%`).

Ion: `cargo-cleaner init ion` updates `$XDG_CONFIG_HOME/ion/initrc`
(default `~/.config/ion/initrc`).

Fish: `cargo-cleaner init fish` writes
`$XDG_CONFIG_HOME/fish/conf.d/cargo-cleaner.fish` (default `~/.config/fish/conf.d/`).

Elvish 0.21+: `cargo-cleaner init elvish` updates
`$XDG_CONFIG_HOME/elvish/rc.elv` (default `~/.config/elvish/rc.elv`,
or `%APPDATA%/elvish/rc.elv` on Windows).

Cmd: use `cargo-cleaner init cmd` with Clink 1.6.11+. Setup writes
`cargo-cleaner.lua` under `CLINK_PROFILE` or `%LOCALAPPDATA%/clink`.
On Windows, set `HOME` to your user directory for cleanup configuration.
The Cmd alias applies to interactive commands, not batch scripts.

Bash setup writes to `~/.bashrc`; login shells must source it from their profile.
Open a new shell session and use Cargo normally. Zsh setup adds a function to
`~/.zshrc` (or `$ZDOTDIR/.zshrc`) and creates the default global config if missing.
Remove any existing `cargo` alias first; an existing `cargo` function is replaced.
IDEs and processes invoking Cargo directly are unaffected.

To remove shell integration, run `cargo-cleaner uninstall zsh` and open a new
shell. The binary and cleanup configs remain installed.

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

Thresholds are positive integer GiB. Omit `[free_space]` to disable free-space
cleanup and project registration. Existing config files are preserved during setup.

For a project-specific size limit, add `.cargo-cleaner.toml` at the project root:

```toml
max_target_size_gib = 10
verbose = true
```

The nearest project config replaces the global size limit; an empty file disables
size-based cleanup. `verbose` inherits the global setting when omitted.
`[free_space]` is global-only and cannot be set or disabled by a project config.

## How it works

1. Before `build`, `check`, `test`, `run`, `bench`, `doc`, `clippy`, `rustc`, or
   `rustdoc`, the wrapper asks Cargo for the workspace and target directory.
   Standard single-letter aliases are supported; other commands pass through.
2. With cross-project cleanup enabled, existing targets are recorded in
   `~/.cargo-cleaner/projects.toml`. Projects are discovered as you use the wrapper,
   without scanning your disk.
3. The free-space policy runs first, then the target-size policy. Each cleans at
   most one target. Cleanup removes **all Cargo artifacts in the selected target**,
   including other profiles or projects sharing it. Source files remain intact.
4. The original command runs with its arguments and exit status preserved.
   Configuration, measurement, registry, or cleanup errors stop the command.

Cleanup does not guarantee enough space for the next build. Cross-project cleanup
can select a target on another filesystem, which may not free space where needed.

## Development

Install Rust and zsh, then run:

```sh
cargo test --all-targets --all-features
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Tests use temporary projects; shell integration tests require zsh.

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
Releases. Pull requests check the release plan; crates.io publishing is separate.

## License

Licensed under either the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
