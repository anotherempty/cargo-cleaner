# cargo-cleaner

A Cargo command wrapper with per-project automatic cleanup. Before build-related
commands, it runs `cargo clean` when configured size or free-space thresholds are
crossed, then forwards the original command.

## Install locally

```sh
cargo install --path .
```

Ensure Cargo's binary directory (normally `~/.cargo/bin`) is on your `PATH`.
Run setup:

```sh
cargo-cleaner init zsh
```

Setup writes the Cargo shell function to `~/.zshrc` (or `$ZDOTDIR/.zshrc` when
set), preserving existing configuration. Repeated setup does not duplicate the
function, and the previous `eval` initialization line is removed automatically.
Open a new zsh session to activate it. Continue using
normal commands such as `cargo build`, `cargo test`, and `cargo +stable check`.

## Configure each project

Create `.cargo-cleaner.toml` in your project or workspace root:

```toml
max_target_size_gib = 20
```

Or choose a free-space threshold instead:

```toml
min_free_space_gib = 10
```

Set at most one threshold, as a positive integer in GiB (1,073,741,824 bytes):

- `max_target_size_gib`: clean when the Cargo target directory's total file size
  is **greater than** this limit.
- `min_free_space_gib`: clean when available space on the filesystem containing
  the workspace root is **less than** this minimum.

The selected condition triggers one `cargo clean` before the requested command.
Setting both thresholds is an error and stops the command before Cargo runs.
No configuration file, or an empty file,
means no automatic cleanup. The example values are not built-in defaults.

The nearest `.cargo-cleaner.toml` is found by searching upward from the current
directory, or from the manifest directory when `--manifest-path` is supplied.
A nearer file replaces the parent configuration; settings are not merged.
Put the file at the workspace root to share thresholds across its members.
Unknown keys, invalid values, measurement errors, and failed cleanup stop the
requested command with a diagnostic.

Checks run for `build`, `check`, `test`, `run`, `bench`, `doc`, `clippy`, `rustc`,
and `rustdoc`, including the standard single-letter aliases. Explicit `clean`,
help, `fmt`, `install`, and custom Cargo aliases/subcommands pass through unchanged.
Invocations using the unstable `-C` directory switch also pass through without
cleanup. Arguments after `--` belong to the requested program and are not inspected.

Cargo metadata determines the workspace and configured target directory. An
explicit `--target-dir` overrides that target. Cargo environment settings,
`+toolchain`, `--config`, and manifest selection are respected. Free-space checks
use the workspace's filesystem, even if the target directory is on another disk.
Size checks sum regular file lengths, not allocated disk blocks, and do not follow
nested symlinks. Missing artifact directories count as empty.

Cleanup removes the **whole selected target directory's Cargo artifacts**, even
when building only one package or profile. Shared target directories can contain
other projects' artifacts. Source files remain intact, but the next build must
rebuild artifacts. Cleanup runs once; it does not guarantee enough space for the
next build and does not coordinate a separate clean/build transaction with other
Cargo processes. Cargo's own locking still applies.

## Forwarding and disabling

The wrapper forwards every original argument to the real Cargo executable on
`PATH`. It preserves the working directory, environment, standard streams, and
exit status of that command. On Unix it replaces the wrapper process with Cargo
after the cleanup check, preserving signals during the requested command.

This affects shells that load the function. IDEs and other processes invoking
Cargo directly do not use it. Existing `cargo` shell functions are replaced;
remove any `cargo` alias before enabling this integration.

To remove shell integration, run:

```sh
cargo-cleaner uninstall zsh
```

This removes the generated function and legacy initialization line from `.zshrc`,
using the same `ZDOTDIR`/`HOME` selection as setup. Other configuration stays intact;
repeated removal is safe. The binary and project cleanup configuration remain installed.
Open a new shell, or use `unfunction cargo` to disable it in the current zsh session. To disable only
cleanup for a project, leave its `.cargo-cleaner.toml` empty.

## Development

```sh
cargo test --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Shell integration tests require zsh. Cleanup tests use isolated temporary projects;
one test invokes real Cargo to verify workspace target discovery, cleanup, and
rebuilding together.
