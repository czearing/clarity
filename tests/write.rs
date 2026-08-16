//! What `fix --write` leaves on disk.
//!
//! Every other test reads a sentence and asks what the engine says about it. This one runs the
//! binary over a file and reads the file back, because a proposal that never reaches the bytes is
//! not a repair, and a repair that reaches the wrong bytes is worse than none. Each case is run
//! twice: a pass that has done its work has nothing left to say the second time, and a pass that
//! keeps finding the same thing is rewriting its own output.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A Rust file whose header comment holds `lines`, under a directory of its own so cases cannot
/// collide.
///
/// This builds the comment instead of holding it in one literal. The crate reads a file's prose a
/// line at a time, and any line that starts with a doc marker counts as a doc comment, so a
/// literal of several lines would put the fixture's own sentences in front of the crate's
/// self-audit. The audit would then measure the test rather than the crate.
fn written(name: &str, lines: &[&str]) -> PathBuf {
    let home = std::env::temp_dir().join(format!("clarity-write-{name}"));
    std::fs::create_dir_all(&home).expect("a directory to work in");
    let path = home.join(format!("{name}.rs"));
    let mut source = String::new();
    for line in lines {
        source.push_str("//!");
        source.push(' ');
        source.push_str(line);
        source.push('\n');
    }
    source.push_str("fn main() {}\n");
    std::fs::write(&path, &source).expect("the case is written down");
    path
}

/// What `written` would have put on disk, for comparing against what is there now.
fn unchanged(lines: &[&str]) -> String {
    let mut source = String::new();
    for line in lines {
        source.push_str("//!");
        source.push(' ');
        source.push_str(line);
        source.push('\n');
    }
    source.push_str("fn main() {}\n");
    source
}

/// The binary run over `path`, and what the file holds afterwards.
fn fix(path: &Path, flags: &[&str]) -> (String, String) {
    let run = Command::new(env!("CARGO_BIN_EXE_fix"))
        .args(flags)
        .arg(path)
        .output()
        .expect("the binary under test is built alongside it");
    let said = String::from_utf8_lossy(&run.stdout).into_owned();
    let left = std::fs::read_to_string(path).expect("the file is still there");
    (said, left)
}

#[test]
fn a_repair_reaches_the_file() {
    let path = written(
        "repair",
        &["The dog run fast. The cat sleeps here. The birds sings loudly."],
    );
    let (_, left) = fix(&path, &["--write"]);
    assert!(
        left.contains("The dog runs fast."),
        "the repair never reached the bytes: {left}"
    );
    assert!(
        left.contains("The birds sing loudly."),
        "only the first repair reached the bytes: {left}"
    );
    assert!(
        left.contains("The cat sleeps here."),
        "a sentence with nothing wrong was rewritten anyway: {left}"
    );
    assert!(left.ends_with("fn main() {}\n"), "the code was touched");

    let (said, again) = fix(&path, &["--write"]);
    assert_eq!(left, again, "a second pass rewrote its own output");
    assert!(
        said.contains("0 words corrected"),
        "the pass still has something to say about what it just wrote: {said}"
    );
}

#[test]
fn a_shorter_wording_reaches_the_file() {
    let path = written(
        "plain",
        &[
            "It is important to note that the parser is slow due to the fact that it waits.",
            "In order to run it, you must wait at this point in time.",
        ],
    );
    let (_, left) = fix(&path, &["--plain", "--write"]);
    assert!(
        left.contains("The parser is slow because it waits."),
        "the shorter wording never reached the bytes: {left}"
    );
    assert!(
        left.contains("To run it, you must wait now."),
        "the second line was left alone: {left}"
    );

    let (said, again) = fix(&path, &["--plain", "--write"]);
    assert_eq!(left, again, "a second pass rewrote its own output");
    assert!(
        said.contains("0 words corrected"),
        "the pass still has something to say about what it just wrote: {said}"
    );
}

#[test]
fn nothing_is_written_without_being_asked() {
    let lines = ["The dog run fast."];
    let path = written("unasked", &lines);
    let (said, left) = fix(&path, &[]);
    assert_eq!(
        left,
        unchanged(&lines),
        "the file was rewritten without --write"
    );
    assert!(
        said.contains("run again with --write"),
        "the pass did not say how to apply what it found: {said}"
    );
}

#[test]
fn a_word_the_lexicon_cannot_place_stops_the_repair() {
    // A correction resting on a guess is worth less than no correction, so a sentence holding a
    // word the engine cannot place is left exactly as it was.
    let lines = ["The zzqx run fast."];
    let path = written("unknown", &lines);
    let (_, left) = fix(&path, &["--write"]);
    assert_eq!(
        left,
        unchanged(&lines),
        "a repair was written over a word nobody placed"
    );
}
