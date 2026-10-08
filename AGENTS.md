# Agent instructions

## Project skills

Use [git-workflow](.agents/skills/git-workflow/SKILL.md) for all Git and pull request work in this repository.

Use [rust-quality](.agents/skills/rust-quality/SKILL.md) for Rust formatting and Clippy checks with warnings denied.

## Rust structures

1. Keep each type and its implementations together: definition -> trait implementations -> inherent implementations.
2. Do not insert unrelated types between a type definition and its implementations.
3. Apply this ordering to structs and enums. Keep related documentation and attributes with their item.
4. Prefer explicit types and simple ownership: borrow through &T, &mut T, &str, or slices when ownership transfer is unnecessary; take ownership when storage, lifetimes, or API semantics require it. Avoid unnecessary cloning. Introduce abstractions only for existing needs.
5. Keep fields private unless callers need direct access. Use constructors when initialization must preserve invariants.
6. Handle recoverable failures with `Result` and propagate errors with useful context. Do not use `unwrap()`, `expect()`, `unwrap_err()`, `expect_err()`, or their unchecked variants outside test code. Likewise, use `panic!`, `todo!`, and `unimplemented!` only in test code. Documented invariants do not exempt non-test code from this rule; handle failures explicitly instead.
7. Use object-oriented design adapted to Rust: encapsulate related state and behavior in structs or enums, expose behavior through methods, prefer composition, and use traits for shared interfaces when needed. Do not define standalone helper functions. Keep behavior in the relevant type's `impl` block or inline trivial wrappers. Only required entry points and test functions remain free functions. Avoid artificial types or unnecessary abstractions solely to imitate class hierarchies.

## Module layout

1. Use a single file (`module_name.rs`) when a module fits in one file.
2. When a module contains multiple files, use a directory (`module_name/`) with `mod.rs` as its entry point and place its submodules inside that directory.
3. Place out-of-line module declarations (`mod name;` or `pub mod name;`) at the top of the file, after file-level attributes such as `#![cfg(...)]`. Keep declaration-specific attributes such as `#[cfg(...)]` immediately above their declaration. Place inline modules (`mod name { ... }`), including test modules, at the bottom of the file.

## Commits and pull requests

1. Keep each commit and pull request focused on one logical change. Avoid unrelated cleanup.
2. Use Conventional Commits for commit subjects and PR titles: `type: short description`, with an optional scope when it adds clarity (`fix(cli): preserve forwarded arguments`). Use `feat` for new behavior, `fix` for bug fixes, and `docs`, `refactor`, `test`, `build`, `ci`, or `chore` as appropriate.
3. Write concise, imperative subjects (for example, `add`, not `added`) without a trailing period. Use normal technical prose. Add a commit body only when rationale or context is not obvious from the subject and diff.
4. Mark breaking changes with `!` before the colon and explain the impact and migration in a `BREAKING CHANGE:` footer.
5. Start PR descriptions with the problem and resulting behavior. For small changes, use one or two sentences plus validation. Include relevant checks and their actual results; state when checks were not run and why. Add risks, migration steps, and issue links only when relevant.
6. Keep PR titles and descriptions aligned with the final diff. Omit conversation history, abandoned approaches, and boilerplate sections that add no useful information.

References: [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) and [GitHub review guidance](https://docs.github.com/en/pull-requests/concepts/helping-others-review-your-changes).

## Interaction

1. Use terse, precise communication. Prefer short sentences; omit filler and repetition.
2. Preserve technical accuracy, constraints, edge cases, and tradeoffs. Use normal prose when compression would obscure meaning or risk.
3. Use numbered lists for multiple points. Preserve exact identifiers, commands, and error messages.
4. Act on clear requests. Make routine, reversible choices without repeated confirmation.
5. Ask concise questions when missing information materially affects scope or correctness. Continue independent work while awaiting answers.
6. Give brief progress updates during sustained work. Report changes, validation, and unresolved limitations at completion.
7. Do not claim checks passed unless they were run successfully. Explain blocked actions and their causes.
8. The user has the final say on decisions. Explicit user instructions override repository guidance and agent preferences. Explain material tradeoffs, then follow the user's decision within applicable system and developer constraints.
