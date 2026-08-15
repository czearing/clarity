//! Correct the prose in source and documentation files, and say what more could be cut.
//!
//! Two things are done here and they are not the same kind of thing. A repair swaps one inflected
//! form of a word for another, which cannot change what a sentence means, so it is applied. A
//! condensation removes whole clauses, which can, so it is only ever proposed and left for a
//! reader to accept.
//!
//! Nothing is written unless asked for, and nothing is written for a unit that rests on a word the
//! lexicon does not know, because a correction resting on a guess is worth less than no correction.

use std::path::Path;

use clarity::condense::condense;
use clarity::prose::placed;
use clarity::register::read;
use clarity::repair::{apply, repair_in};
use clarity::text::Text;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|arg| arg == "--write");
    let proposing = args.iter().any(|arg| arg == "--condense");
    let paths: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let mut mended = 0;
    let mut touched = 0;
    for path in &paths {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let rust = Path::new(path.as_str())
            .extension()
            .is_some_and(|kind| kind == "rs");
        let blocks = placed(&source, rust);
        if blocks.is_empty() {
            continue;
        }
        let swaps = swaps(&blocks);
        for (at, was, now) in &swaps {
            println!("{path}:{at} {was} -> {now}");
        }
        if !swaps.is_empty() {
            mended += swaps.len();
            touched += 1;
            if write {
                std::fs::write(path, rewritten(&source, &swaps)).ok();
            }
        }
        if proposing {
            propose(path, &blocks);
        }
    }
    let how = if write { "corrected" } else { "correctable" };
    println!(
        "{mended} words {how} across {touched} of {} files",
        paths.len()
    );
    if !write && mended > 0 {
        println!("run again with --write to apply them");
    }
}

/// Every word a repair would swap, as a byte in the file, the word there, and what replaces it.
///
/// The prose is judged as one passage rather than a line at a time, because a sentence wrapped
/// across three comment lines is one sentence and reading it in thirds finds faults that are not
/// there. The byte each line came from is what lets the answer be put back.
fn swaps(blocks: &[Vec<(usize, String)>]) -> Vec<(usize, String, String)> {
    let lines: Vec<(usize, String)> = blocks.concat();
    let prose = lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut found = Vec::new();
    for (unit, (register, _)) in Text::read(&prose).units.iter().zip(read(&prose)) {
        let Some(edits) = repair_in(unit, register) else {
            continue;
        };
        let mended = apply(unit, &edits);
        for edit in &edits {
            let Some(token) = unit.tokens.get(edit.at) else {
                continue;
            };
            let Some(now) = mended.tokens.get(edit.at) else {
                continue;
            };
            if token.word == now.word {
                continue;
            }
            if let Some(at) = source_byte(&lines, token.at.start) {
                found.push((at, token.word.clone(), now.word.clone()));
            }
        }
    }
    found.sort_by_key(|(at, _, _)| *at);
    found
}

/// Where a byte of the run-together prose sits in the file it was read out of.
///
/// The lines were joined with one space between them, so walking them in order and carrying the
/// length gives the answer without any searching, and a byte that falls on a joining space belongs
/// to no line and is refused rather than guessed at.
fn source_byte(lines: &[(usize, String)], at: usize) -> Option<usize> {
    let mut seen = 0;
    for (start, line) in lines {
        if at < seen + line.len() {
            return Some(start + (at - seen));
        }
        seen += line.len() + 1;
    }
    None
}

/// The file with every swap made, in one pass from the end so the earlier bytes keep their places.
fn rewritten(source: &str, swaps: &[(usize, String, String)]) -> String {
    let mut out = source.to_owned();
    for (at, was, now) in swaps.iter().rev() {
        let end = at + was.len();
        if out.get(*at..end) == Some(was.as_str()) {
            out.replace_range(*at..end, now);
        }
    }
    out
}

/// What the file's prose says once the padding is taken out of it.
fn propose(path: &str, blocks: &[Vec<(usize, String)>]) {
    for block in blocks {
        let Some((at, _)) = block.first() else {
            continue;
        };
        let passage = block
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let core = condense(&passage);
        let kept = core.text();
        if kept.split_whitespace().count() < passage.split_whitespace().count() {
            println!("{path}:{at} condense\n  was {passage}\n  now {kept}");
        }
    }
}
