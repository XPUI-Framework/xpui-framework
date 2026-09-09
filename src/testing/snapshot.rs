//! Golden-file comparison, with no dependencies.
//!
//! A snapshot is the ordered list of everything the frame drew, as text,
//! committed beside the test, so a change to layout shows up as a diff a
//! reviewer can read. It asserts call order, clip lifecycle and the state a
//! widget was drawn in — none of which is visible in a picture; pixels belong
//! to whichever backend put them there.
//!
//! `no_run`: this writes the golden when one does not exist yet.
//!
//! ```rust,no_run
//! # use xpui::{App, NavigationScreen, Screen, Text, View, testing, vstack};
//! # struct Settings;
//! # impl Screen for Settings {
//! #     type Message = ();
//! #     fn body(&self) -> impl View<()> {
//! #         NavigationScreen::new(vstack![0; Text::new("Settings")])
//! #     }
//! #     fn update(&mut self, _message: ()) {}
//! # }
//! # testing::install();
//! # let mut app = App::new(Settings);
//! testing::reset();
//! app.render();
//! testing::assert_snapshot("settings_screen");
//! ```
//!
//! `UPDATE_SNAPSHOTS=1 cargo test --features testing` rewrites every golden
//! the run touched. Read the diff before committing it: an accepted snapshot
//! is an assertion you have made.

use std::path::PathBuf;
use std::{env, fs};

use super::ops;

/// Where goldens live, relative to the crate being tested.
const DIR: &str = "tests/snapshots";

fn path_for(name: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is the crate under test, so a backend crate keeps its
    // own goldens rather than writing into the framework's.
    let root = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set under cargo test");
    PathBuf::from(root).join(DIR).join(format!("{name}.txt"))
}

fn updating() -> bool {
    env::var("UPDATE_SNAPSHOTS").is_ok_and(|value| value != "0")
}

/// Asserts that everything drawn since the last `reset` matches the golden.
///
/// Writes the golden instead when `UPDATE_SNAPSHOTS` is set, and always writes
/// it when it does not exist yet — a new test should not need two runs.
pub fn assert_snapshot(name: &str) {
    assert_text_snapshot(name, &ops::render(&super::ops_log()));
}

/// The comparison itself, against any text. Private: the only text worth
/// committing under `tests/snapshots` is a draw log.
fn assert_text_snapshot(name: &str, actual: &str) {
    let path = path_for(name);

    let existing = fs::read_to_string(&path).ok();

    if updating() || existing.is_none() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("cannot create {}: {e}", parent.display()));
        }
        fs::write(&path, actual).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));

        // A golden that did not exist is now whatever the code happens to do,
        // which proves nothing. Say so rather than passing quietly.
        if existing.is_none() && !updating() {
            panic!(
                "snapshot `{name}` did not exist and has been written to {}.\n\
                 Read it, confirm it is what the screen should look like, then re-run.",
                path.display()
            );
        }
        return;
    }

    let expected = existing.expect("checked above");
    if expected == actual {
        return;
    }

    panic!(
        "snapshot `{name}` does not match {}\n\n{}\n\n\
         If this change is intended, re-run with UPDATE_SNAPSHOTS=1 and review the diff.",
        path.display(),
        diff(&expected, actual)
    );
}

/// A line-by-line diff, marked the way `diff -u` marks one.
///
/// Hand-rolled rather than pulled in: the whole crate has no dependencies, and
/// a snapshot of draw calls is short enough that a common-prefix/suffix trim
/// is as readable as a real edit script would be.
fn diff(expected: &str, actual: &str) -> String {
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();

    let prefix = expected
        .iter()
        .zip(&actual)
        .take_while(|(a, b)| a == b)
        .count();

    // Do not let the suffix scan run back past what the prefix already claimed,
    // or a file whose every line matches would count the same lines twice.
    let remaining = expected.len().min(actual.len()) - prefix;
    let suffix = expected
        .iter()
        .rev()
        .zip(actual.iter().rev())
        .take(remaining)
        .take_while(|(a, b)| a == b)
        .count();

    let mut out = String::new();
    if prefix > 0 {
        out.push_str(&format!("  ... {prefix} identical line(s)\n"));
    }
    for line in &expected[prefix..expected.len() - suffix] {
        out.push_str(&format!("- {line}\n"));
    }
    for line in &actual[prefix..actual.len() - suffix] {
        out.push_str(&format!("+ {line}\n"));
    }
    if suffix > 0 {
        out.push_str(&format!("  ... {suffix} identical line(s)\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::diff;

    #[test]
    fn diff_marks_only_what_changed() {
        let out = diff("a\nb\nc\n", "a\nB\nc\n");
        assert!(out.contains("- b"), "{out}");
        assert!(out.contains("+ B"), "{out}");
        assert!(!out.contains("- a"), "{out}");
        assert!(!out.contains("- c"), "{out}");
    }

    #[test]
    fn diff_of_identical_text_is_empty_of_changes() {
        let out = diff("a\nb\n", "a\nb\n");
        assert!(!out.contains("- "), "{out}");
        assert!(!out.contains("+ "), "{out}");
    }

    #[test]
    fn diff_handles_pure_insertion() {
        let out = diff("a\nc\n", "a\nb\nc\n");
        assert!(out.contains("+ b"), "{out}");
        assert!(!out.contains("- "), "{out}");
    }

    #[test]
    fn diff_handles_pure_deletion() {
        let out = diff("a\nb\nc\n", "a\nc\n");
        assert!(out.contains("- b"), "{out}");
        assert!(!out.contains("+ "), "{out}");
    }
}
