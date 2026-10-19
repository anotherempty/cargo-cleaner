# cargo-cleaner

A Cargo command wrapper with automatic cleanup across known projects. Before build-related
commands, it runs `cargo clean` when configured size or free-space thresholds are
crossed, then forwards the original command.

## Install

Once a release is published, download a prebuilt archive from
[GitHub Releases](https://github.com/anotherempty/cargo-cleaner/releases), or install
the latest release with:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/anotherempty/cargo-cleaner/releases/latest/download/cargo-cleaner-installer.sh | sh
```

Distributions cover Linux and macOS on x86_64 and ARM64. The installer places the
binary in `$CARGO_HOME/bin`, or `~/.cargo/bin` when `CARGO_HOME` is unset.
Cargo and zsh are still required to use the wrapper and its shell integration.

To build and install from a local checkout:

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

Setup also creates `~/.cargo-cleaner.toml` with a `free_space` object (`min_free_space_gib = 100`, `across_projects = true`) if
missing. Existing files, including empty files, are preserved. `HOME` must be set;
`ZDOTDIR` affects only shell integration, not the global cleanup config.

## Global configuration and project overrides

Set top-level `verbose = true` to show cleanup progress, target measurements,
selection decisions, and timings on stderr. It defaults to `false`. The nearest
project configuration can override the global value with `verbose = true` or
`verbose = false`; omitting it inherits the global value. Place this setting
before any `[free_space]` table. Cleanup announcements and errors remain visible
regardless of verbosity.

Set the global free-space policy in `~/.cargo-cleaner.toml`:

```toml
[free_space]
min_free_space_gib = 100
across_projects = true
```

Both fields are required. Set `across_projects = true` to register projects and clean the
largest known target. Set `across_projects = false` to clean only the current project without
reading or updating the registry.

The `free_space` object is global-only. Project files cannot override or disable it;
putting this object in a project file is an error. The global configuration is always
read and validated, even when a project has its own configuration. Omitting this
object from the global file disables the free-space policy and registry operations.
The 100 GiB default is written only during setup.

Set a local target-size limit in your project's `.cargo-cleaner.toml`:

```toml
max_target_size_gib = 20
```

The global file can also provide `max_target_size_gib` as a default. The nearest
project file replaces only that size policy. An empty project file disables the
local size policy while leaving the global free-space policy active. The size limit and free-space object
can both be present in the global file and operate independently.

Thresholds are positive integers in GiB (1,073,741,824 bytes):

- `free_space.min_free_space_gib`: clean when available space on the current
  workspace's filesystem is **less than** this minimum. `across_projects` controls which
  project is selected.
- `max_target_size_gib`: clean the current target when its total file size is
  **greater than** this limit.

The global free-space policy runs first, including registration of the current
project when `across_projects = true`, followed by the size policy. If different projects exceed the two
conditions, each policy can clean its selected target. If the free-space cleanup
removes the current target, the size check sees it as empty.

The nearest project `.cargo-cleaner.toml` is found by searching upward from the
current directory, or the manifest directory when `--manifest-path` is supplied.
Search stops at the home directory when reached. A nearer project file replaces
the parent's size policy. Put the file at the workspace root to share the size
limit across its members. Unknown keys, invalid values, measurement errors, and
failed cleanup stop the requested command with a diagnostic.

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
rebuild artifacts. Each policy cleans at most once; this does not guarantee enough space for the
next build and does not coordinate a separate clean/build transaction with other
Cargo processes. Cargo's own locking still applies.

## Project registry

When the global free-space policy has `across_projects = true`, it identifies the current
workspace and target directory before each supported build command. If the target
directory exists, the policy registers the project in `~/.cargo-cleaner/projects.toml` before cleanup is evaluated.
`HOME` must be set. Projects are discovered through wrapper invocations; there is
no filesystem-wide scan. A first build without a target directory is registered
on a later invocation after the target exists.

Registration runs even when free space is sufficient or an empty project config
disables the local size policy. When `free_space` is absent or `across_projects = false`, the registry
is neither read nor updated. Canonical target paths are deduplicated,
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
size-based cleanup for a project, leave its `.cargo-cleaner.toml` empty. The global
free-space policy continues to apply.

## Development

```sh
cargo test --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Shell integration tests require zsh. Cleanup tests use isolated temporary projects;
one test invokes real Cargo to verify workspace target discovery, cleanup, and
rebuilding together.

## Distributions and releases

Release packaging uses [cargo-dist](https://axodotdev.github.io/cargo-dist/), pinned
to version `0.33.0` in `dist-workspace.toml`. Install that version locally:

```sh
cargo install cargo-dist --version 0.33.0 --locked
```

After changing distribution settings, regenerate the workflow and check the plan:

```sh
dist generate
dist generate --check
dist plan
```

Build archives for your current platform with `dist build --artifacts=local`.
Artifacts are written to `target/distrib/`.

To publish a release:

1. Update the package version in `Cargo.toml` and refresh `Cargo.lock` with Cargo.
2. Run the development checks above and `dist plan`.
3. Commit and push the release changes, including the generated workflow.
4. Create and push the matching version tag, for example `git tag v0.1.0` followed
   by `git push origin v0.1.0` for the current version.

The generated GitHub Actions workflow checks release plans on pull requests.
Version tags build the platform archives, shell installer, and SHA-256 checksums,
then publish them to GitHub Releases. Tags with a prerelease suffix produce a
GitHub prerelease. This workflow does not publish to crates.io.

## License

Licensed under either the [MIT license](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
