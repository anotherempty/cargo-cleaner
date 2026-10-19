---
name: git-workflow
description: Handle Git and pull request work in this project according to its contribution conventions. Use for initialization, status and diff review, staging, commits, branches, merges, rebases, conflict resolution, remotes, tags, pushes, and pull requests.
---

# Git workflow

Read [CONVENTIONS.md](../../../CONVENTIONS.md) before Git work. Follow it alongside the commit and PR rules below.

## Commits and pull requests

1. Keep each commit and pull request focused on one logical change. Avoid unrelated cleanup.
2. Use Conventional Commits for commit subjects and PR titles: `type: short description`, with an optional scope when it adds clarity (`fix(cli): preserve forwarded arguments`). Use `feat` for new behavior, `fix` for bug fixes, and `docs`, `refactor`, `test`, `build`, `ci`, or `chore` as appropriate.
3. Write concise, imperative subjects (for example, `add`, not `added`) without a trailing period. Use normal technical prose. Add a commit body only when rationale or context is not obvious from the subject and diff.
4. Mark breaking changes with `!` before the colon and explain the impact and migration in a `BREAKING CHANGE:` footer.
5. Start PR descriptions with the problem and resulting behavior. For small changes, use one or two sentences plus validation. Include relevant checks and their actual results; state when checks were not run and why. Add risks, migration steps, and issue links only when relevant.
6. Keep PR titles and descriptions aligned with the final diff. Omit conversation history, abandoned approaches, and boilerplate sections that add no useful information.

References: [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) and [GitHub review guidance](https://docs.github.com/en/pull-requests/concepts/helping-others-review-your-changes).

## Local work

Inspect the current branch and relevant staged and unstaged changes before mutations. Check remotes and tracking branches when needed for the requested operation. Initialize Git only when required by the user's task.

Keep commits focused on the requested change. Preserve unrelated edits and staged work; stage explicit files or hunks and review the staged diff before committing. Do not include unrelated staged changes in a commit. Resolve conflicts with both branches' intended behavior in mind, then run checks appropriate to the changed code.

## Remote work and pull requests

Use tools appropriate to the configured remote host. Determine the intended base and head from repository state and task context. Review the complete branch diff against the base before preparing a PR.

Follow the commit and PR rules above and CONVENTIONS.md for PR titles and descriptions. Pass multiline CLI descriptions through a body file to preserve formatting.

Stay within the user's requested scope and existing authorization. A local commit alone does not authorize a push, PR merge, release, or shared-history rewrite. Do not ask again for actions already authorized. If permissions, credentials, or network access block an operation, report the failed action and concrete cause without bypassing restrictions.

## Completion

Check the resulting Git state. Report relevant commits, branches, or PR links, validation performed, and remaining blockers. Distinguish completed local work from remote actions that did not succeed.
