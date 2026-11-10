# Releases

The kit is used by several applications through a git dependency. Every version it publishes
says where it comes from, is a tag that never moves, and comes with a release and a changelog.

| Branch | Channel | Version (example) | Published as |
|---|---|---|---|
| `main` | beta | `0.2.0-beta.3` | a pre-release per commit, tagged `v0.2.0-beta.3` |
| a stable release of a `main` commit | stable | `0.2.0` | a GitHub release, tagged `v0.2.0` |
| `feat/*`, `fix/*` | alpha | `0.2.0-alpha.feat-12-tabs.4` | a pre-release per push, tagged `v0.2.0-alpha.feat-12-tabs.4`, all deleted with the branch |

There is no `dev` branch. Nothing is published anywhere but GitHub's releases of the repository,
and a release holds no binary: an application pins its tag (`tag = "v0.2.0"`, or an alpha or beta
tag while it tries something). **Releases are immutable**: a published version is never replaced,
its tag never moved, so each build is its own release, an alpha's too.

## Branches

Work happens on `feat/<n>-<topic>` and `fix/<n>-<topic>` branches, started from `main`. A branch
reaches `main` by **squash only**: one commit per pull request, the pull request's title as its
subject, `main` staying a straight line. Nobody commits on `main` (the `pre-commit` hook refuses
it, see `AGENTS.md`).

## The version

Computed, never typed (`tools/version.sh stable|beta|alpha [--branch <name>] [--at <commit>]`,
for HEAD by default):

- **It counts from the last release**: the highest stable tag `vX.Y.Z`, compared as versions,
  wherever it is, and from its commit on. The commits counted are those of the build's own
  history that the release does not hold, so a branch started before a release counts only its
  own commits. Pre-release tags (`v0.2.0-beta.3`) do not count. Without a stable tag, from
  `0.0.0` and the first commit. `v0.1.0` is the first stable.
- **The next version** follows the Angular commits counted (`tools/commit-message.sh` is the rule
  they follow), merges left out: `feat` bumps the minor, `fix` and `perf` the patch, a `!` after
  the type or a `BREAKING CHANGE:` footer the major, and nothing that bumps is the next patch.
  **While the major is 0, a breaking change bumps the minor**, so `1.0.0` is reached by saying so
  (below), never by a commit's type.
- **`Release-As: X.Y.Z` moves the version by hand.** A footer of that form in the message of a
  commit counted (a squash's body) sets the next version when it is above the computed one (equal
  changes nothing; with several, the highest wins), and is forgotten once released. A value
  below the computed version, or not `X.Y.Z`, is refused: `tools/version.sh` exits 1 naming the
  commit, and so fails the build until the commit is fixed.
- **The beta and alpha numbers** count the commits since the release (first parents only, so a
  merge of `main` into a branch counts once): the first beta after a release is `-beta.1`.
- **An alpha names its branch**: each byte of the name but ASCII letters, digits and dashes is a
  dash, `feat/12-tabs` giving `alpha.feat-12-tabs.<n>`. CI and its cleanup follow that one rule;
  two names that give the same one (`feat/a_b` and `feat/a-b`) share their alphas.
- **Nothing since the last release** has no version: `tools/version.sh` exits 3, printing
  nothing, and nothing is published (a run of an already released commit, a branch that has no
  commit of its own).

## What CI publishes

On a push to `main`, `feat/**` or `fix/**`, `.github/workflows/ci.yml` runs, beside its checks
(`verify`: format, clippy and the tests, with and without the `omarchy` feature):

- **`plan`** runs the branch's scripts with a token that can only read: it computes the version
  and writes the notes: the commit, how to depend on the tag, then the changelog
  (`tools/changelog.sh`): the Angular commits since the last stable tag, oldest first, grouped
  under breaking changes, features, fixes, performance and other changes, each
  `- **<scope>**: <subject>` with the pull request number its squash kept.
- **`publish`**, once `verify` passed on the pushed commit, runs none of the branch's code: it
  takes the notes `plan` made and alone holds the right to write releases and tags. It runs no
  git and no script; only `gh`, naming the file it reads. Right before writing, it reads the
  branch's tip again through GitHub's API: a branch moved on to another commit, or deleted,
  publishes nothing (the later push publishes itself). Then, a pre-release
  `vX.Y.Z-beta.N` on `main`, `vX.Y.Z-alpha.<branch>.N` on a feature branch, never marked
  latest.

  **Nothing is replaced**: a version already published is left as it is (run again, or a branch
  pushed over to the same number); a release left as a draft by a failed upload is deleted and
  made again. No alpha is pruned while its branch lives: they pile up, one per push, until the
  branch is deleted.

`.github/workflows/alpha-cleanup.yml` deletes **every** alpha of a branch, with their tags, when
the branch is deleted (an immutable release may be deleted). It shares the branch's `publish`
concurrency group, so it waits for one still running, which then finds the branch gone, or
publishes what the cleanup then deletes.

`.github/workflows/pull-request.yml` judges the shape of a pull request, reading only: its title
is an Angular subject of 72 characters at most, and its description ends with `Closes #<n>` of an
open issue. `ci.yml` judges each commit of the pull request the same way as the hooks.

## Releasing

A stable release is started by hand: **the `Release` workflow** (`.github/workflows/release.yml`),
from `main`, with the commit of `main` to release as its input, a full SHA or `main` for its head
(the default): `gh workflow run release.yml --ref main`, or "Run workflow" on its page.

- **`plan`** (a token that only reads) checks that it runs from `main`, that the input is a full
  SHA or `main`, nothing else, that the commit is on `main` and that a run of `ci.yml` on that
  very push succeeded (so its `verify` passed), then computes the version
  (`tools/version.sh stable --at <commit>`: **the version its beta had, without the
  pre-release**: `0.2.0-beta.3` releases `0.2.0`) and the changelog (the commits since the
  previous stable tag, up to the commit). A commit with nothing since the last stable release
  fails the run, saying so.
- **`publish`** holds the token that writes the tag: no checkout, no git, no code of any branch;
  only `gh`. It creates the release `vX.Y.Z` on the commit released, marked latest, with the
  changelog. A version already published fails the run and changes nothing; a draft left by a
  failed upload is made again.
- **The tag `vX.Y.Z` is put on the released commit**, never moved. No tag is pushed by hand. The
  betas that follow count from it.

## Settings of the repository

What the files cannot say, set once by the maintainer in the repository's settings: pull
requests merge by **squash only**, with the pull request's title as the commit subject and no
merge button for the other two; branches are deleted on merge (which runs `alpha-cleanup`);
**releases are immutable**; `main` accepts no direct push.

## What only a real push proves

The tests (`tests/releases.rs`) run the version and changelog scripts, the commit convention and
the hooks on throwaway repositories. What they cannot run is the workflows themselves on GitHub:
a push to a feature branch proves the alpha's (`plan`, then `publish` with the job's token, a
second push publishing the next alpha beside the first), deleting the branch its cleanup, the
first push to `main` the beta's, and the first `Release` run the stable tag on the commit
released, with the release marked latest.
