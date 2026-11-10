//! The scripts of the releases (`docs/releases.md`) on throwaway git repositories: the version
//! each channel gets, the changelog, the commit convention and the guard of `main`. Nothing here
//! talks to GitHub; what only a push proves is said in the document.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository's own scripts and hooks.
fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// A throwaway git repository on `main`, none of the machine's git settings.
struct Repo {
    dir: tempfile::TempDir,
}

/// What a script printed and how it ended.
struct Ran {
    out: String,
    err: String,
    code: i32,
}

impl Repo {
    fn new() -> Repo {
        let repo = Repo {
            dir: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "--quiet", "--initial-branch=main"]);
        repo
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(self.dir.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid");
        command
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.command("git").args(args).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// A commit with that message, its own file changed.
    fn commit(&self, message: &str) {
        let n = self.git(&["rev-list", "--all", "--count"]);
        std::fs::write(self.dir.path().join(format!("file-{n}")), message).unwrap();
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "-m", message]);
    }

    fn tag(&self, name: &str) {
        self.git(&["tag", name]);
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }

    fn branch(&self, name: &str) {
        self.git(&["checkout", "--quiet", "-b", name]);
    }

    fn checkout(&self, name: &str) {
        self.git(&["checkout", "--quiet", name]);
    }

    fn run(&self, tool: &str, args: &[&str]) -> Ran {
        let output: Output = self.command(script(tool)).args(args).output().unwrap();
        Ran {
            out: String::from_utf8(output.stdout).unwrap(),
            err: String::from_utf8(output.stderr).unwrap(),
            code: output.status.code().unwrap(),
        }
    }

    /// What `tools/version.sh` printed for a channel, which must have succeeded.
    fn version(&self, args: &[&str]) -> String {
        let ran = self.run("tools/version.sh", args);
        assert_eq!(ran.code, 0, "version.sh {args:?}: {}", ran.err);
        ran.out.trim().to_owned()
    }

    fn changelog(&self, args: &[&str]) -> String {
        let ran = self.run("tools/changelog.sh", args);
        assert_eq!(ran.code, 0, "changelog.sh {args:?}: {}", ran.err);
        ran.out
    }

    /// The repository as the kit is: `v0.1.0` on its first commit.
    fn released() -> Repo {
        let repo = Repo::new();
        repo.commit("feat(theme): the first look");
        repo.tag("v0.1.0");
        repo
    }
}

// ---- tools/version.sh ----

#[test]
fn a_commit_to_main_after_a_release_is_a_beta_of_the_next_version() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)");
    assert_eq!(repo.version(&["beta"]), "0.2.0-beta.1");
    repo.commit("fix(menu): its edge (#4)");
    assert_eq!(repo.version(&["beta"]), "0.2.0-beta.2");
}

#[test]
fn the_stable_version_is_the_betas_without_its_pre_release() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)");
    repo.commit("fix(menu): its edge (#4)");
    assert_eq!(repo.version(&["beta"]), "0.2.0-beta.2");
    assert_eq!(repo.version(&["stable"]), "0.2.0");
}

#[test]
fn a_stable_release_of_an_older_commit_of_main_has_the_version_that_commit_had() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    let patch = repo.head();
    repo.commit("feat(menu): a menu (#4)");
    assert_eq!(repo.version(&["stable", "--at", &patch]), "0.1.1");
    assert_eq!(repo.version(&["beta", "--at", &patch]), "0.1.1-beta.1");
    assert_eq!(repo.version(&["stable"]), "0.2.0");
}

#[test]
fn a_fix_alone_is_the_next_patch_and_what_bumps_nothing_is_too() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    assert_eq!(repo.version(&["stable"]), "0.1.1");
    let other = Repo::released();
    other.commit("docs(repo): say it (#3)");
    assert_eq!(other.version(&["stable"]), "0.1.1");
    let perf = Repo::released();
    perf.commit("perf(menu): faster (#3)");
    assert_eq!(perf.version(&["stable"]), "0.1.1");
}

#[test]
fn before_1_0_a_feature_raises_the_minor_and_a_breaking_change_too() {
    let repo = Repo::released();
    repo.commit("feat(menu)!: a new shape (#3)");
    assert_eq!(repo.version(&["stable"]), "0.2.0");
    let footer = Repo::released();
    footer.commit("fix(menu): a rename (#3)\n\nBREAKING CHANGE: Menu::new takes a name");
    assert_eq!(footer.version(&["stable"]), "0.2.0");
}

#[test]
fn from_1_0_a_feature_raises_the_minor_and_a_breaking_change_the_major() {
    let repo = Repo::new();
    repo.commit("feat(theme): the first look");
    repo.tag("v1.2.3");
    repo.commit("feat(menu): a menu (#3)");
    assert_eq!(repo.version(&["stable"]), "1.3.0");
    repo.commit("refactor(menu)!: a new shape (#4)");
    assert_eq!(repo.version(&["stable"]), "2.0.0");
}

#[test]
fn without_a_stable_tag_the_versions_count_from_the_first_commit() {
    let repo = Repo::new();
    repo.commit("feat(theme): the first look");
    repo.commit("fix(theme): a colour");
    assert_eq!(repo.version(&["beta"]), "0.1.0-beta.2");
    assert_eq!(repo.version(&["stable"]), "0.1.0");
}

#[test]
fn a_pre_release_tag_is_not_a_release() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)");
    repo.tag("v0.2.0-beta.1");
    repo.commit("fix(menu): its edge (#4)");
    repo.tag("v0.2.0-alpha.feat-1-menu.2");
    assert_eq!(repo.version(&["beta"]), "0.2.0-beta.2");
}

#[test]
fn the_highest_stable_tag_is_the_base_whatever_the_order_they_were_made_in() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)");
    repo.tag("v0.2.0");
    repo.commit("fix(menu): its edge (#4)");
    let base = repo.run("tools/version.sh", &["base"]);
    assert_eq!(base.code, 0);
    let mut words = base.out.split_whitespace();
    assert_eq!(words.next(), Some("v0.2.0"));
    assert_eq!(
        words.next(),
        Some(repo.git(&["rev-parse", "v0.2.0"]).as_str())
    );
    assert_eq!(repo.version(&["beta"]), "0.2.1-beta.1");
}

#[test]
fn an_alpha_names_its_branch_and_counts_its_commits() {
    let repo = Repo::released();
    repo.branch("feat/12-tabs");
    repo.commit("feat(tabs): tabs");
    repo.commit("fix(tabs): their edge");
    assert_eq!(repo.version(&["alpha"]), "0.2.0-alpha.feat-12-tabs.2");
}

#[test]
fn an_alpha_names_the_branch_it_is_given_with_every_odd_byte_as_a_dash() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    assert_eq!(
        repo.version(&["alpha", "--branch", "feat/a_b.c"]),
        "0.1.1-alpha.feat-a-b-c.1"
    );
    // `feat-a` is not `feat-a-b`, and the slug of an accented name is dashes, never a refusal.
    assert_eq!(
        repo.version(&["alpha", "--branch", "fix/é"]),
        "0.1.1-alpha.fix---.1"
    );
}

#[test]
fn a_branch_started_before_a_release_counts_only_its_own_commits() {
    let repo = Repo::released();
    repo.branch("fix/13-edge");
    repo.commit("fix(menu): its edge");
    repo.checkout("main");
    repo.commit("feat(tabs): tabs (#12)");
    repo.tag("v0.2.0");
    repo.commit("docs(repo): say it (#14)");
    // The release holds the first commit of the branch's history and the ones that came after it
    // on main: the branch's version counts from the release, on top of it, with its own commit.
    repo.checkout("fix/13-edge");
    assert_eq!(repo.version(&["alpha"]), "0.2.1-alpha.fix-13-edge.1");
}

#[test]
fn a_merge_into_a_branch_counts_once_and_bumps_nothing() {
    let repo = Repo::released();
    repo.branch("feat/12-tabs");
    repo.commit("fix(tabs): their edge");
    repo.checkout("main");
    repo.commit("docs(repo): say it (#3)");
    repo.checkout("feat/12-tabs");
    repo.git(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        "Merge branch 'main'",
        "main",
    ]);
    assert_eq!(repo.version(&["alpha"]), "0.1.1-alpha.feat-12-tabs.2");
}

#[test]
fn nothing_committed_since_the_release_is_nothing_to_publish_on_any_channel() {
    let repo = Repo::released();
    for args in [&["stable"][..], &["beta"], &["alpha", "--branch", "feat/x"]] {
        let ran = repo.run("tools/version.sh", args);
        assert_eq!(ran.code, 3, "{args:?}: {}", ran.err);
        assert_eq!(ran.out, "", "{args:?}");
    }
}

#[test]
fn a_commit_older_than_the_last_release_is_nothing_to_release() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    let old = repo.head();
    repo.commit("feat(menu): a menu (#4)");
    repo.tag("v0.2.0");
    let ran = repo.run("tools/version.sh", &["stable", "--at", &old]);
    assert_eq!(ran.code, 3, "{}", ran.err);
    assert_eq!(ran.out, "");
}

#[test]
fn release_as_sets_the_next_version_when_it_is_above_the_computed_one() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)\n\nRelease-As: 1.0.0");
    assert_eq!(repo.version(&["stable"]), "1.0.0");
    assert_eq!(repo.version(&["beta"]), "1.0.0-beta.1");
    repo.commit("feat(menu): a menu (#4)\n\nRelease-As: 0.5.0");
    // The highest of the requested versions, each above the computed one.
    assert_eq!(repo.version(&["stable"]), "1.0.0");
}

#[test]
fn release_as_is_forgotten_once_released() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)\n\nRelease-As: 1.0.0");
    repo.tag("v1.0.0");
    repo.commit("fix(menu): its other edge (#4)");
    assert_eq!(repo.version(&["stable"]), "1.0.1");
}

#[test]
fn release_as_below_the_computed_version_is_refused_naming_the_commit() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)\n\nRelease-As: 0.1.5");
    let ran = repo.run("tools/version.sh", &["stable"]);
    assert_eq!(ran.code, 1);
    assert_eq!(ran.out, "");
    assert!(ran.err.contains(&repo.head()), "{}", ran.err);
    assert!(ran.err.contains("below"), "{}", ran.err);
}

#[test]
fn release_as_that_is_not_a_version_is_refused() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)\n\nRelease-As: next");
    let ran = repo.run("tools/version.sh", &["beta"]);
    assert_eq!(ran.code, 1);
    assert!(ran.err.contains("not X.Y.Z"), "{}", ran.err);
}

#[test]
fn an_unknown_channel_is_refused() {
    let repo = Repo::released();
    for args in [&[][..], &["rc"]] {
        let ran = repo.run("tools/version.sh", args);
        assert_eq!(ran.code, 2, "{args:?}");
        assert_eq!(ran.out, "");
    }
}

// ---- tools/changelog.sh ----

#[test]
fn the_changelog_groups_the_commits_since_the_last_release_by_type() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    repo.commit("feat(tabs): tabs (#4)");
    repo.commit("docs(repo): say it (#5)");
    repo.commit("perf(menu): faster (#6)");
    repo.commit("feat(toast)!: a new shape (#7)");
    repo.commit("feat(field): a field (#8)");
    assert_eq!(
        repo.changelog(&[]),
        "\
### Breaking changes

- **toast**: a new shape (#7)

### Features

- **tabs**: tabs (#4)
- **toast**: a new shape (#7)
- **field**: a field (#8)

### Fixes

- **menu**: its edge (#3)

### Performance

- **menu**: faster (#6)

### Other changes

- **repo**: say it (#5)
"
    );
}

#[test]
fn the_changelog_starts_after_the_last_stable_tag_and_not_a_pre_release() {
    let repo = Repo::released();
    repo.commit("feat(menu): a menu (#3)");
    repo.tag("v0.2.0");
    repo.commit("fix(menu): its edge (#4)");
    repo.tag("v0.2.1-beta.1");
    repo.commit("fix(tabs): their edge (#5)");
    assert_eq!(
        repo.changelog(&[]),
        "### Fixes\n\n- **menu**: its edge (#4)\n- **tabs**: their edge (#5)\n"
    );
}

#[test]
fn the_changelog_of_a_commit_stops_at_it() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    let at = repo.head();
    repo.commit("feat(tabs): tabs (#4)");
    assert_eq!(
        repo.changelog(&["--at", &at]),
        "### Fixes\n\n- **menu**: its edge (#3)\n"
    );
}

#[test]
fn the_changelog_of_a_range_is_that_range() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge (#3)");
    repo.commit("feat(tabs): tabs (#4)");
    repo.commit("fix(tabs): their edge (#5)");
    assert_eq!(
        repo.changelog(&["HEAD~2..HEAD"]),
        "### Features\n\n- **tabs**: tabs (#4)\n\n### Fixes\n\n- **tabs**: their edge (#5)\n"
    );
}

#[test]
fn the_changelog_without_a_release_holds_every_commit_and_skips_what_is_no_convention() {
    let repo = Repo::new();
    repo.commit("feat(theme): the first look");
    repo.commit("a note that follows no convention");
    repo.commit("fix(theme): a colour");
    assert_eq!(
        repo.changelog(&[]),
        "### Features\n\n- **theme**: the first look\n\n### Fixes\n\n- **theme**: a colour\n"
    );
}

#[test]
fn the_changelog_of_nothing_is_empty() {
    let repo = Repo::released();
    assert_eq!(repo.changelog(&[]), "");
}

#[test]
fn a_breaking_footer_is_listed_under_the_breaking_changes() {
    let repo = Repo::released();
    repo.commit("fix(menu): a rename (#3)\n\nBREAKING CHANGE: Menu::new takes a name");
    assert_eq!(
        repo.changelog(&[]),
        "### Breaking changes\n\n- **menu**: a rename (#3)\n\n### Fixes\n\n- **menu**: a rename (#3)\n"
    );
}

// ---- tools/commit-message.sh ----

fn judged(subject: &str) -> Ran {
    let repo = Repo::new();
    let file = repo.dir.path().join("message");
    std::fs::write(&file, format!("{subject}\n\nA body.\n")).unwrap();
    repo.run("tools/commit-message.sh", &[file.to_str().unwrap()])
}

#[test]
fn a_subject_of_the_convention_is_accepted() {
    for subject in [
        "feat(menu): a menu",
        "fix(theme): a colour (#12)",
        "refactor(repo)!: a new layout",
        "ci(repo): publish the betas",
        "Merge branch 'main' into feat/x",
    ] {
        let ran = judged(subject);
        assert_eq!(ran.code, 0, "{subject}: {}", ran.err);
    }
}

#[test]
fn a_subject_over_72_characters_is_refused() {
    let long = format!("feat(menu): {}", "a".repeat(61));
    assert_eq!(long.len(), 73);
    let ran = judged(&long);
    assert_eq!(ran.code, 1);
    assert!(ran.err.contains("73 characters"), "{}", ran.err);
    // 72 is allowed.
    let fits = format!("feat(menu): {}", "a".repeat(60));
    assert_eq!(judged(&fits).code, 0);
}

#[test]
fn a_subject_without_a_scope_is_refused() {
    let ran = judged("feat: a menu");
    assert_eq!(ran.code, 1);
    assert!(ran.err.contains("scope required"), "{}", ran.err);
}

#[test]
fn a_subject_with_a_final_period_is_refused() {
    assert_eq!(judged("feat(menu): a menu.").code, 1);
}

#[test]
fn a_subject_with_a_capital_or_an_unknown_type_is_refused() {
    assert_eq!(judged("feat(menu): A menu").code, 1);
    assert_eq!(judged("feature(menu): a menu").code, 1);
    assert_eq!(judged("wip").code, 1);
}

#[test]
fn comment_lines_of_a_message_file_are_not_the_subject() {
    let repo = Repo::new();
    let file = repo.dir.path().join("message");
    std::fs::write(&file, "# Please enter the message\nfeat(menu): a menu\n").unwrap();
    let ran = repo.run("tools/commit-message.sh", &[file.to_str().unwrap()]);
    assert_eq!(ran.code, 0, "{}", ran.err);
}

#[test]
fn a_range_is_refused_when_one_of_its_commits_is() {
    let repo = Repo::released();
    repo.commit("fix(menu): its edge");
    assert_eq!(
        repo.run("tools/commit-message.sh", &["--range", "v0.1.0..HEAD"])
            .code,
        0
    );
    repo.commit("Fix the menu.");
    let ran = repo.run("tools/commit-message.sh", &["--range", "v0.1.0..HEAD"]);
    assert_eq!(ran.code, 1);
    assert!(ran.err.contains("Fix the menu."), "{}", ran.err);
}

// ---- the hooks ----

#[test]
fn the_commit_msg_hook_runs_the_judge() {
    let repo = Repo::new();
    let file = repo.dir.path().join("message");
    std::fs::write(&file, "feat: a menu\n").unwrap();
    let ran = repo.run(".githooks/commit-msg", &[file.to_str().unwrap()]);
    assert_eq!(ran.code, 1);
}

#[test]
fn a_commit_on_main_is_refused_by_the_pre_commit_hook_and_on_a_branch_is_not() {
    let repo = Repo::released();
    assert_eq!(repo.run(".githooks/pre-commit", &[]).code, 1);
    repo.branch("feat/2-releases");
    assert_eq!(repo.run(".githooks/pre-commit", &[]).code, 0);
    repo.checkout("main");
    let ran = repo.run("tools/branch-guard.sh", &["dev"]);
    assert_eq!(ran.code, 0, "no dev branch is special here: {}", ran.err);
}
