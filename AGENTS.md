# AGENTS.md — gpui-design-system

Instructions for any coding agent working in this repository. `CLAUDE.md` is a symbolic link
to this file: there is one contract, not two that drift apart.

## What the kit is

The components, tokens and motion shared by applications built on GPUI through
[gpui-kit](https://crates.io/crates/gpui-kit) and GPUI Fast: buttons, rows, cards, menus, dialogs,
drawers, toasts, fields, tabs, skeletons, the palette every screen reads, and the motion they
share. Applications use it through a git dependency pinned to a tag. Every screen of an
application is built from these pieces; a second, slightly different version of one of them is a
defect.

**Read [`README.md`](README.md) before any work**, and [`docs/releases.md`](docs/releases.md)
before touching a version, a workflow or a script of `tools/`. The kit's tests are its contract.

The kit is generic: it knows nothing of what an application is about. The application tells it
its name, its themes and the desktops whose theme it offers to follow.

## Commands

```
cargo build
cargo test
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --example gallery
tools/commit-message.sh --range origin/main..HEAD
git config core.hooksPath .githooks     # once per clone: the commit-msg and pre-commit hooks
```

## Principles

### 1. Think before coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them; don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If the code and the issue disagree, the issue wins; if the issue is wrong, say so instead of
  quietly deviating.

### 2. Simplicity first

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what the task asks. No abstractions for single-use code.
- No configurability that was not requested.
- No error handling for impossible scenarios. If 200 lines could be 50, rewrite.

### 3. Surgical changes

**Touch only what you must. Clean up only your own mess.**

- Don't "improve" adjacent code, comments or formatting. Don't refactor what isn't broken.
- Match existing style. If you notice unrelated dead code, mention it, don't delete it.
- Every changed line must trace directly to the task.

### 4. The failing test first

**Define success criteria. Loop until verified.**

- Write the test that fails first, see it fail, then make it pass.
- "Fix the bug" → a test that reproduces it, then the fix.
- Every scenario of the issue gets a test named after it. A scenario without a test is a
  defect, not a TODO.

## Code style

- Rust 2024, `cargo fmt`, `cargo clippy -- -D warnings` clean, with and without the default
  features.
- Code, identifiers, comments, docs, issues and commits are in English.
- Building blocks: `gpui-kit` (pinned `=0.8.0`) on GPUI Fast. Adding another dependency of
  weight is said in the pull request.
- A comment says why, not what.
- Scripts are bash (`set -euo pipefail`), tested on throwaway git repositories (`tests/releases.rs`).

## Components and motion (non-negotiable)

**A component is generic.** It names no application, no product, no domain: its sources,
examples and tests, comments included, hold none of the words an application is made of. A scan
(`tests/no_app_word.rs`) holds it; a word that belongs to one application belongs to that
application. What an application must tell the kit (its name, its themes, its desktops) it tells
through `Identity`, never through a constant of the kit.

**Everything that moves is animated.** The same durations and easing as everywhere
(`src/motion.rs`), through the kit's helpers (`src/interact.rs`, `src/reveal.rs`), never a second
system:

- hover and press of every interactive element (a button, a row, a link, a chip, a toggle): its
  colours fade over `HOVER`. **GPUI's own `.hover(..)` switches at once and is never used** (a
  test refuses it);
- a state that changes (selected, chosen, focused, open): the fill, the ink, the ring, the
  chevron move over `PANEL` or `HOVER`;
- an element that appears or leaves, a message, an error, a block that is shown or hidden: it
  unfolds and fades in, folds away and fades out, with the gap around it;
- a row inserted into, removed from or moved in a list: it unfolds, folds away or slides;
- a fold or an unfold, a panel or a menu that opens and closes, a page and each of its sections
  that show up (staggered);
- with reduced motion, every one of them jumps instead.

A new component comes with its scenarios in `tests/` (hover, state change, arrival and leaving),
and a pull request lists each element it animated. The other rules the tests hold: four kinds of
button, one confirmation dialog, results as toasts, a visible label on every control, a state
said once, one list pattern, the spacing scale (`tests/rules.rs`).

## Testing

- **Tests use GPUI's test platform only and never open a window on a screen.** An example is
  built, never run, by a test.
- **Tests never use the real configuration, the real data folder, the real home or the user's
  repositories.** They use temporary folders, throwaway git repositories and a temporary `HOME`.
- A test that fails only under load is rerun alone before the failure is called real.
- `cargo test` and `cargo test --no-default-features` both pass: without the `omarchy` feature,
  the kit names no desktop.

## Releases

Versions are computed from the commits, never typed (`docs/releases.md`): a push to `feat/*` or
`fix/*` publishes an alpha, a commit of `main` a beta, and a stable release is started by hand
with the `Release` workflow. A release holds no binary: applications pin a tag. A published
version is never replaced and a tag never moved. The workflows give a read-only token to
anything that runs the repository's code, and keep the token that writes in a job that runs none
(no checkout of branch code there): keep that split.

## Git and pull requests (non-negotiable)

- **There is no `dev` branch.** A branch starts from `origin/main` (`feat/<n>-<topic>`,
  `fix/<n>-<topic>`) and its pull request targets `main`.
- **Never commit, merge, rebase, push or force-push on `main` directly.** If you are on it,
  create a branch first. The `pre-commit` hook refuses it.
- Never rewrite published history. No `--no-verify`.
- One commit = one intent. No `wip` commits. Don't mix formatting and logic in one commit.
- Commits use the Git user configured on the machine.
- The subject is `<type>(<scope>): <subject>`: the **scope is required**, the subject is 72
  characters at most, starts lowercase, has no final period, and the type is one of feat, fix,
  refactor, test, docs, chore, build, ci, perf. `tools/commit-message.sh` is the judge (the
  `commit-msg` hook, CI and the pull request's title check run it).
- Scopes: the component's or module's name (`button`, `menu`, `theme`, `motion`, `toast`…) or
  `repo` (layout, docs, tools, hooks) and `ci` (workflows).
- **Pull requests into `main` are merged by squash only.** The title is a plain Angular subject
  (it becomes the commit) and `feat`, `fix` and `perf` decide the next version. The description
  is written in the first person as the maintainer and ends with `Closes #<n>`. A breaking
  change says `!` after the scope, or a `BREAKING CHANGE:` footer; `Release-As: X.Y.Z` in the
  body moves the next version by hand.
- A stable release is started by hand, never by a push (`docs/releases.md`, "Releasing").

## Security and privacy

- **This repository is public.** Nothing private goes into it, an issue, a pull request or a
  commit: no name of a real employer or client project, no private path, host, account or token.
  Examples and fixtures use invented data.
- Secrets are never printed, logged or committed.
- Nothing is published outside the pull request and the repository's releases: no gist, no
  upload, no paste service.
- **A permission denial is never worked around**: do what remains, say what was refused.

## Working on a machine

- The machine is the maintainer's own, in use while you work. Never stop, restart or
  reconfigure their running programs or sessions.
- No artificial load: no stress runs, no parallel benchmark loops. Heavy builds run at low
  priority (`nice -n 19`), one at a time.
- Worktrees, logs and every other scratch file live outside the clone.
- Before removing a worktree, check it holds no uncommitted change and no unpushed commit
  (`git log HEAD --not --remotes`).

## GitHub gotchas

- Prefer REST (`gh api repos/…`) to GraphQL, which gets rate-limited.
- A body holding backticks or apostrophes is written to a file and passed with `--body-file`.

## When done

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and
`cargo test --no-default-features`, plus the manual checks the issue asks for, and report the
real output. If something fails, say so; don't claim green.
