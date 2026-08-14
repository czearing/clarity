//! Read prose out of source and documentation files and report what the engine cannot handle.
//!
//! Doc comments and markdown paragraphs are the hardest prose the engine will meet: dense,
//! technical, and full of words that no lexicon lists. Running the engine over its own writing is
//! the cheapest way to find where it is wrong.

use std::collections::BTreeMap;
use std::path::Path;

use clarity::register::read;
use clarity::text::Text;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    let mut faults: Vec<String> = Vec::new();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let mut units = 0;
    for path in &paths {
        let source = std::fs::read_to_string(path).unwrap_or_default();
        let prose = if Path::new(path).extension().is_some_and(|kind| kind == "rs") {
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
                *unknown.entry(unit.tokens[*at].word.clone()).or_default() += 1;
            }
            for fault in &report.faults {
                *tally.entry(fault.rule.says()).or_default() += 1;
                let shown: Vec<String> = unit
                    .tokens
                    .iter()
                    .zip(&report.tags)
                    .map(|(token, tag)| format!("{}/{tag:?}", token.word))
                    .collect();
                faults.push(format!(
                    "{path}: {} <- {}\n      {}",
                    unit.text(),
                    fault.rule.says(),
                    shown.join(" ")
                ));
            }
        }
    }
    println!(
        "units {units}, unknown words {}, faults {}",
        unknown.len(),
        faults.len()
    );
    for (rule, count) in &tally {
        println!("rule {count:4} {rule}");
    }
    for (word, count) in &unknown {
        println!("unknown {word} x{count}");
    }
    for fault in &faults {
        println!("fault {fault}");
    }
}

/// Prose from doc comments, skipping fenced code and hidden doctest lines.
fn from_source(source: &str) -> String {
    let mut prose = String::new();
    let mut fenced = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        else {
            continue;
        };
        let rest = rest.trim();
        if rest.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || rest.starts_with('#') || rest.starts_with('|') || rest.is_empty() {
            continue;
        }
        prose.push_str(rest);
        prose.push(' ');
    }
    prose
}

/// Prose from markdown, skipping fenced code, headings, and tables.
fn from_markdown(source: &str) -> String {
    let mut prose = String::new();
    let mut fenced = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.starts_with('#') || line.starts_with('|') || line.is_empty() {
            continue;
        }
        prose.push_str(line);
        prose.push(' ');
    }
    prose
}
