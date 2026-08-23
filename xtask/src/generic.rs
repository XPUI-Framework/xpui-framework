//! The one rule this repository exists to keep: the framework may not name a
//! product, a device or a backend.
//!
//! Carried by `xpui` alone. Every other repository in the organisation is
//! allowed to say `Badger`, and a check forbidding it there would be noise.

use std::fs;

use crate::paths::tracked;

/// Words the framework may not contain, lowercased.
///
/// If you want to reach for a backend from inside the framework, add a trait
/// method instead — that is what the seam is for. A review comment does not
/// run, which is why this does.
///
/// The list has drifted before: a copy in CI knew four of these to the gate's
/// eleven, so a forbidden word could land with a green tick. There is one
/// list now because there is one gate.
const FORBIDDEN: [&str; 11] = [
    "crosspoint",
    "xteink",
    "freeink",
    "embedded_graphics",
    "badger",
    "tufty",
    "pimoroni",
    "seeed",
    "sticky",
    // The space is optional, and it is the *closed* spelling that a Rust
    // identifier would use: `x4pro`, `X4Pro`, `--board x4pro`. A pattern that
    // required the space caught only the form least likely to appear in
    // source.
    "x4pro",
    "gfxrenderer",
];

/// Whether a line names a forbidden word, and which.
///
/// Compared against the line with its spaces removed as well as the line
/// itself, so `x4 pro` and `x4pro` are one rule rather than two entries.
fn forbidden_in(line: &str) -> Option<&'static str> {
    let lowered = line.to_lowercase();
    let closed: String = lowered.chars().filter(|c| !c.is_whitespace()).collect();
    FORBIDDEN
        .into_iter()
        .find(|word| lowered.contains(word) || closed.contains(word))
}

/// No source file under `src/` names a product, a device or a backend.
///
/// Comments included, deliberately: a doc comment that explains the framework
/// by naming one backend teaches the next reader that the seam is negotiable.
pub fn framework_is_generic() -> Result<String, String> {
    let mut found = Vec::new();
    let mut scanned = 0;
    for file in tracked("*.rs") {
        let path = file.to_string_lossy();
        if !path.starts_with("src/") {
            continue;
        }
        scanned += 1;
        let text = fs::read_to_string(&file).unwrap_or_default();
        for (index, line) in text.lines().enumerate() {
            if let Some(word) = forbidden_in(line) {
                found.push(format!("  {path}:{}  {word}", index + 1));
            }
        }
    }
    if scanned == 0 {
        return Err("no sources under src/ — the filter is matching nothing".into());
    }
    if found.is_empty() {
        Ok(format!(
            "{scanned} file(s), none naming a product or a backend"
        ))
    } else {
        Err(format!(
            "{}\n\nThe framework depends on traits; backends implement them. Add a\n\
             trait method rather than a special case.",
            found.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_forbidden_word_is_caught_in_the_spelling_code_would_use() {
        // Not a count and not a case check: those pass a mutation that
        // replaces a real word with a fake one, and that is how `x4 ?pro`
        // became `x4 pro` and stopped catching `x4pro` — the only spelling a
        // Rust identifier can have.
        for word in [
            "CrossPoint",
            "Xteink",
            "FreeInk",
            "embedded_graphics",
            "Badger 2040",
            "Tufty2040",
            "Pimoroni",
            "Seeed",
            "Sticky",
            "X4PRO",
            "x4pro",
            "GfxRenderer",
        ] {
            assert!(
                forbidden_in(&format!("let board = {word};")).is_some(),
                "{word} would land in the framework unnoticed"
            );
        }
    }

    #[test]
    fn an_ordinary_line_is_not_forbidden() {
        assert!(forbidden_in("pub fn draw_text(&mut self, at: Point) {").is_none());
        assert!(forbidden_in("/// The panel this screen is painted onto.").is_none());
    }
}
