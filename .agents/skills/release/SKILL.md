---
name: release
description: Prepare and publish cargo-cleaner releases through cargo-dist and GitHub Actions. Use when asked to make, prepare, or retry a release of this repository.
---

# Release cargo-cleaner

Read the project [Git workflow](../git-workflow/SKILL.md),
[Cargo.toml](../../../Cargo.toml), [dist-workspace.toml](../../../dist-workspace.toml),
and [release workflow](../../../.github/workflows/release.yml).
Use their current values rather than hardcoding a tool version, repository, or platform list.

## Scope and version

1. A request to make or publish a release authorizes the necessary release commit,
   branch push, and version-tag push. A request only to prepare or validate a release
   stops before publishing. Creating or editing this skill does not authorize a release.
2. Inspect the working tree, branch, remotes, and local and remote tags. Preserve
   unrelated work. Default to releasing `main`; do not silently release a feature branch.
   Fetch the intended remote and reconcile its state before preparing the release.
3. Use the requested version. If none is supplied and the package's current version
   has no release tag, use that version. If it is already tagged, ask which next
   version to release rather than choosing a version bump for the user.
4. Use `v<package-version>` as the tag. Check both remote tags and GitHub Releases
   for that version before creating anything. Never replace or move a published tag.

## Prepare and validate

1. If changing the version, update the root package in `Cargo.toml` and refresh its
   entry in `Cargo.lock` with Cargo, without upgrading unrelated dependencies.
2. Use the cargo-dist version pinned in `dist-workspace.toml`. If Cargo is a mise
   shim without a selected Rust version, use an installed toolchain through
   `mise exec rust@<version> -- ...`; do not change global toolchain settings.
3. Apply the [Rust quality](../rust-quality/SKILL.md) workflow for formatting and
   Clippy. Separately run `cargo test --all-targets --all-features` as release
   validation; shell integration tests require zsh.
4. Run `dist generate --check` and `dist plan --tag <tag>`. If generated files are
   stale, run `dist generate`, review the changes, and repeat the check. Do not
   hand-edit the generated release workflow.
5. Run `dist build --artifacts=local` to validate packaging for the host platform.
   Check that the archives include the executable, README, and both license files.
   Local builds do not validate the other platforms; GitHub Actions builds those.
6. Review and commit only the release changes with a Conventional Commit, such as
   `chore: release <version>`. If the intended release commit already exists and
   needs no changes, use it without creating an empty commit.

## Publish and verify

1. Ensure the release workflow and configuration are committed and pushed to the
   intended remote branch before pushing the tag. Push only that branch and the
   specific release tag, never all local tags. Do not force-push.
2. Tag the verified release commit, then push `refs/tags/<tag>`. This is the action
   that triggers publication; branch pushes and PR merges alone do not release.
   A prerelease version suffix produces a GitHub prerelease.
3. Use available GitHub tools or the API to monitor the Release workflow for that
   tag and commit. Do not assume the `gh` executable is installed. Confirm workflow
   success and inspect the resulting release assets: configured binary archives,
   shell installer, checksums, and distribution metadata. This workflow does not
   publish to crates.io.
4. Report the version, commit, tag, release URL, and validation results. If the
   workflow is still running or cannot be inspected, report that publication is
   unverified rather than claiming success.

## Failed or repeated releases

Inspect existing tags, workflow runs, and release assets before retrying. If a tag
already points to the intended commit, reuse its run rather than recreating the tag.
A request to retry permits rerunning the failed workflow when appropriate, but not
deleting a release or rewriting a tag. If a fix needs a different commit, explain
the failure and obtain the user's version decision. Stop repeated failing attempts
and report the concrete blocker; do not change credentials, repository protections,
or publishing scope to bypass it.
