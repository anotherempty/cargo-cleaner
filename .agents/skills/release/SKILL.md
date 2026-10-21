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
   branch push, version-tag push, and cleanup of a failed unpublished attempt as
   described below. A request only to prepare or validate a release
   stops before publishing. Creating or editing this skill does not authorize a release.
2. Inspect the working tree, branch, remotes, and local and remote tags. Preserve
   unrelated work. Unless the user explicitly specifies another release source,
   fetch `origin` and prepare the release from the latest fetched `origin/main`.
   Use that commit as the base for release changes, not local `main` or the current
   branch. Use a separate checkout or worktree when needed to preserve local work.
3. Use the requested version. If none is supplied and the package's current version
   has no release tag, use that version. If its tag belongs to a failed unpublished
   attempt, clean up that attempt and reuse the version. If the version was already
   released, ask which next version to release rather than choosing a bump.
4. Use `v<package-version>` as the tag. Check both remote tags and GitHub Releases
   for that version before creating anything. Preserve tags for published releases;
   a pushed tag alone does not mean a release was published.

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

1. Inspect the failed jobs and record the cause, run URL, tag, tag object ID, and
   commit. Do not treat an in-progress or inaccessible run as failed. A local
   validation failure before tagging needs no remote cleanup.
2. Before cleanup, wait for all runs and reruns associated with that tag to finish,
   or cancel them and verify they have stopped. This prevents a remaining job from
   publishing after cleanup. Retain workflow logs and artifacts for diagnosis.
3. Check GitHub Releases again, including drafts. Clean up only an attempt known
   to have failed before publication. Preserve any published release or prerelease
   and its tag, even if a later workflow job failed. If publication state cannot be
   verified, report the blocker rather than deleting anything.
4. Remove any draft release created by this failed attempt, including its draft
   assets. Delete the remote version tag only if its object ID still matches the
   recorded attempt; use an explicit expected-value lease for the deletion. Remove
   the local tag only if it also matches. Leave unrelated tags, releases, commits,
   and working-tree changes intact. Verify that the draft and tags are gone. If a
   tool or permission is unavailable, report exactly which cleanup remains.
5. Fix the cause, rerun the affected validation, commit and push the fix, then
   recreate the same version tag on the verified commit and monitor the new run.
   A failed unpublished attempt does not require a version bump or additional
   confirmation for this cleanup and retry. Never move a tag in place or force-push
   a branch; remove the failed tag before creating its replacement.
6. If the same failure recurs after a fix, clean up the failed unpublished attempt
   and report the blocker instead of repeatedly publishing retries. Do not change
   credentials, repository protections, or publishing scope to bypass a failure.
   Report what was removed, what was retained, and whether a retry succeeded.
