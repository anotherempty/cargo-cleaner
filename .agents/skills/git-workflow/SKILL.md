---
name: git-workflow
description: Handle Git and pull request work in this project according to its contribution conventions. Use for initialization, status and diff review, staging, commits, branches, merges, rebases, conflict resolution, remotes, tags, pushes, and pull requests.
---

# Git workflow

Read [CONVENTIONS.md](../../../CONVENTIONS.md) and the commit and PR rules in [AGENTS.md](../../../AGENTS.md) before Git work. Treat these files as the maintained source of conventions.

## Local work

Inspect the current branch and relevant staged and unstaged changes before mutations. Check remotes and tracking branches when needed for the requested operation. Initialize Git only when required by the user's task.

Keep commits focused on the requested change. Preserve unrelated edits and staged work; stage explicit files or hunks and review the staged diff before committing. Do not include unrelated staged changes in a commit. Resolve conflicts with both branches' intended behavior in mind, then run checks appropriate to the changed code.

Write commit messages using AGENTS.md's conventions. Include non-obvious rationale when useful and report only validation actually performed.

## Remote work and pull requests

Use tools appropriate to the configured remote host. Determine the intended base and head from repository state and task context. Review the complete branch diff against the base before preparing a PR.

Follow AGENTS.md and CONVENTIONS.md for PR titles and descriptions. Explain the problem, resulting behavior, and actual validation concisely. Include risks or migration steps when relevant, and update the description when scope changes. Pass multiline CLI descriptions through a body file to preserve formatting.

Stay within the user's requested scope and existing authorization. A local commit alone does not authorize a push, PR merge, release, or shared-history rewrite. Do not ask again for actions already authorized. If permissions, credentials, or network access block an operation, report the failed action and concrete cause without bypassing restrictions.

## Completion

Check the resulting Git state. Report relevant commits, branches, or PR links, validation performed, and remaining blockers. Distinguish completed local work from remote actions that did not succeed.
