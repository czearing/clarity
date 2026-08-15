//! Holding the crate's own writing to the engine it ships.
//!
//! Every other test names a sentence and says what the engine should make of it, which measures
//! the engine against writing chosen to measure it. This one measures it against writing that was
//! not: the crate's own doc comments and documentation, written to explain the crate and not to
//! exercise it. Prose nobody wrote for the test is the only prose that can tell you where the
//! engine actually stands.
//!
//! The bound is the count that was measured, so any change that reads this prose worse fails here
//! and any change that reads it better asks to be recorded. What the remaining faults are, and why
//! they are still here, is in `docs/LIMITS.md`.

use std::path::Path;
use std::sync::OnceLock;

use clarity::prose::{from_markdown, from_source};
use clarity::register::read;
use clarity::text::Text;

/// Every file whose prose is held to the engine.
fn written() -> Vec<String> {
    let mut found = Vec::new();
    for directory in ["src", "src/bin", "tests", "docs", "."] {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let kind = path.extension().and_then(|kind| kind.to_str());
            if matches!(kind, Some("rs" | "md")) {
                found.push(path.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

/// What the engine finds in the crate's own writing, read once however often it is asked for.
fn measured() -> &'static (usize, usize, Vec<String>) {
    static MEASURED: OnceLock<(usize, usize, Vec<String>)> = OnceLock::new();
    MEASURED.get_or_init(measure)
}

/// Read every file and count what the engine cannot answer for.
fn measure() -> (usize, usize, Vec<String>) {
    let mut units = 0;
    let mut unknown = Vec::new();
    let mut faults = Vec::new();
    for path in written() {
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let prose = if Path::new(&path)
            .extension()
            .is_some_and(|kind| kind == "rs")
        {
            from_source(&source)
        } else {
            from_markdown(&source)
        };
        if prose.trim().is_empty() {
            continue;
        }
        for (unit, (_, report)) in Text::read(&prose).units.iter().zip(read(&prose)) {
            units += 1;
            for at in &report.unknown {
                unknown.push(unit.tokens[*at].word.clone());
            }
            for fault in &report.faults {
                faults.push(format!("{path}: {} <- {}", unit.text(), fault.rule.says()));
            }
        }
    }
    (units, unknown.len(), faults)
}

/// How much of its own prose the engine cannot yet read, as measured.
const FAULTS: usize = 44;

/// How many words of its own prose the engine cannot place, as measured.
const UNKNOWN: usize = 1;

/// How little of its own prose there must be before the count above means anything.
const UNITS: usize = 500;

#[test]
fn the_engine_reads_its_own_writing_no_worse_than_it_did() {
    let (units, unknown, faults) = measured();
    let (units, unknown) = (*units, *unknown);
    assert!(
        units >= UNITS,
        "only {units} units of prose were found, so the counts below prove nothing"
    );
    assert!(
        faults.len() <= FAULTS,
        "{} faults, up from {FAULTS}:\n{}",
        faults.len(),
        faults.join("\n")
    );
    assert!(
        unknown <= UNKNOWN,
        "{unknown} unplaceable words, up from {UNKNOWN}"
    );
}

/// The readme states this count, so the readme is checked against it.
#[test]
fn the_readme_states_the_count_that_was_measured() {
    let readme = std::fs::read_to_string("README.md").expect("the readme is next to the tests");
    let claim = readme
        .lines()
        .find(|line| line.contains("the crate's own prose"))
        .expect("the readme no longer says what it cannot read");
    assert!(
        claim.contains(&FAULTS.to_string()),
        "the readme says {claim:?}, and the measurement is {FAULTS}"
    );
}

#[test]
fn the_bound_is_the_measurement_and_not_a_ceiling_left_slack() {
    let (_, unknown, faults) = measured();
    let unknown = *unknown;
    assert_eq!(
        faults.len(),
        FAULTS,
        "the bound is stale: record {} and say in docs/LIMITS.md what changed",
        faults.len()
    );
    assert_eq!(unknown, UNKNOWN, "the unplaceable word count is stale");
}
