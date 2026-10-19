# cargo-cleaner

A Cargo command wrapper with automatic cleanup across known projects. Before build-related
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
function.
Open a new zsh session to activate it. Continue using
normal commands such as `cargo build`, `cargo test`, and `cargo +stable check`.

Setup also creates `~/.cargo-cleaner.toml` with `min_free_space_gib = 100` if
missing. Existing files, including empty files, are preserved. `HOME` must be set;
`ZDOTDIR` affects only shell integration, not the global cleanup config.

## Global configuration and project overrides

The global `~/.cargo-cleaner.toml` supplies the default policy for all projects,
including projects outside your home directory. Edit it to change the default.
Project configuration replaces the global policy entirely; thresholds are not merged.

Create `.cargo-cleaner.toml` in your project or workspace root:

```toml
max_target_size_gib = 20
```

Or choose a free-space threshold instead:

```toml
min_free_space_gib = 100
```

Set at most one threshold, as a positive integer in GiB (1,073,741,824 bytes):

- `max_target_size_gib`: clean when the Cargo target directory's total file size
  is **greater than** this limit.
- `min_free_space_gib`: clean when available space on the filesystem containing
  the workspace root is **less than** this minimum. It cleans the largest recorded
  target directory, which may belong to another project.

The selected condition triggers one `cargo clean` before the requested command.
Setting both thresholds is an error and stops the command before Cargo runs.
An empty project file disables automatic cleanup even when the global config has
a threshold. If neither a project nor global config exists, cleanup is skipped;
the 100 GiB default is written only during setup.

The nearest `.cargo-cleaner.toml` is found by searching upward from the current
directory, or from the manifest directory when `--manifest-path` is supplied.
A nearer file replaces the parent configuration; settings are not merged.
Search stops at the home directory when reached. If no project config is found,
the global config is used. An overridden global config is not read or validated.
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

## Project registry

Before each supported build command, Cargo metadata identifies the current
workspace and target directory. If the target directory exists, the project is
registered in `~/.cargo-cleaner/projects.toml` before cleanup is evaluated.
`HOME` must be set. Projects are discovered through wrapper invocations; there is
no filesystem-wide scan. A first build without a target directory is registered
on a later invocation after the target exists.

Registration also runs when no cleanup policy is configured or an empty config
disables cleanup for the current invocation. Such a project can still be selected
by a later invocation's free-space policy. Canonical target paths are deduplicated,
including targets shared by multiple workspaces. Missing target directories and
workspaces without a `Cargo.toml` are removed from the registry. Updates use a file
lock and atomic replacement to preserve registrations from concurrent commands.

When the free-space threshold is crossed, existing registered targets are measured
and the largest is cleaned once. Equal sizes use registry order. Selection spans
all recorded targets, including other filesystems; cleaning a different disk may
not improve free space on the current workspace's filesystem. The size-limit policy
continues to clean only the current project's target directory.

Cleaning another project runs `cargo clean` in its workspace with its manifest and
recorded target directory. The current invocation's `+toolchain`, `--config`, and
manifest flags are not applied to another project. Registry read/write errors,
invalid registry data, and failed cleanup stop the requested command.

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

This removes the generated function from `.zshrc`,
using the same `ZDOTDIR`/`HOME` selection as setup. Other configuration stays intact;
repeated removal is safe. The binary and global and project cleanup configurations remain installed.
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
