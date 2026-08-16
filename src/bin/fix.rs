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

use clarity::check::check_in;
use clarity::condense::condense;
use clarity::grammar::Sentence;
use clarity::prose::placed;
use clarity::register::{read, Register};
use clarity::repair::{apply, repair_in};
use clarity::style;
use clarity::text::Text;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|arg| arg == "--write");
    let proposing = args.iter().any(|arg| arg == "--condense");
    let wordy = args.iter().any(|arg| arg == "--plain");
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
        let swaps = if wordy {
            wordier(&blocks)
        } else {
            swaps(&blocks)
        };
        for (at, was, now, unit) in &swaps {
            println!("{path}:{at} {was} -> {now}\n  in {unit}");
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
    let swapped = plural(mended, "word");
    let over = plural(paths.len(), "file");
    println!("{swapped} {how} across {touched} of {over}");
    if !write && mended > 0 {
        println!("run again with --write to apply them");
    }
}

/// `count` of `thing`, counted the way English counts.
fn plural(count: usize, thing: &str) -> String {
    if count == 1 {
        format!("{count} {thing}")
    } else {
        format!("{count} {thing}s")
    }
}

/// Every word a repair would swap, as a byte in the file, the word there, and what replaces it.
///
/// The prose is judged as one passage rather than a line at a time, because a sentence wrapped
/// across three comment lines is one sentence and reading it in thirds finds faults that are not
/// there. The byte each line came from is what lets the answer be put back.
fn swaps(blocks: &[Vec<(usize, String)>]) -> Vec<Swap> {
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
                found.push((at, token.word.clone(), now.word.clone(), unit.text()));
            }
        }
    }
    found.sort_by_key(|(at, ..)| *at);
    found
}

/// A byte in the file, the word written there, what replaces it, and the sentence it sits in.
///
/// The sentence travels with the swap because a reader cannot judge "is" becoming "are" without
/// it, and a repair nobody can judge is a repair nobody should apply.
type Swap = (usize, String, String, String);

/// Every stretch of padding a shorter wording replaces, as a byte in the file.
///
/// This is a different kind of change from a repair and a safer one. A repair derives a word and
/// has to be stopped from deriving one that does not exist. A shorter wording is not derived at
/// all: each one is written down beside the phrase it replaces, so what is applied is a stated
/// equivalence rather than a guess, and the worst it can be is unwanted.
fn wordier(blocks: &[Vec<(usize, String)>]) -> Vec<Swap> {
    let lines: Vec<(usize, String)> = blocks.concat();
    let prose = lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut found = Vec::new();
    for (unit, (register, _)) in Text::read(&prose).units.iter().zip(read(&prose)) {
        let tags = check_in(unit, register).tags;
        for note in style::read(unit, &tags) {
            // A note that states no replacement is a finding, not an edit, and there is nothing
            // here to write in its place.
            //
            // Whether the passage waives plainness is deliberately not asked. A register decides
            // what is held against writing, and a passage of solid padding will happily say that
            // padding is how it is written. Asking for this is an instruction, not a report.
            let Some(instead) = note.instead else {
                continue;
            };
            let (Some(first), Some(last)) = (
                unit.tokens.get(note.at.start),
                unit.tokens.get(note.at.end.saturating_sub(1)),
            ) else {
                continue;
            };
            // A phrase that straddles two comment lines is one phrase in the prose and two
            // stretches in the file, and there is no single byte range to write it back to.
            let (Some(from), Some(to)) = (
                source_byte(&lines, first.at.start),
                source_byte(&lines, last.at.end - 1),
            ) else {
                continue;
            };
            if to + 1 - from != last.at.end - first.at.start {
                continue;
            }
            let was = prose[first.at.start..last.at.end].to_owned();
            let mut now = instead.to_owned();
            if now.is_empty() {
                // The phrase goes entirely, and the space after it goes with it. If it was what
                // opened the sentence, the word left standing first has to be given the capital
                // the phrase was carrying.
                let after = last.at.end + 1;
                let Some(next) = unit.tokens.iter().find(|token| token.at.start >= after) else {
                    continue;
                };
                if next.at.start != after {
                    continue;
                }
                let Some(end) = source_byte(&lines, next.at.end - 1) else {
                    continue;
                };
                if end + 1 - from != next.at.end - first.at.start {
                    continue;
                }
                let was = prose[first.at.start..next.at.end].to_owned();
                now = if first.capitalised {
                    capitalised(&next.word)
                } else {
                    next.word.clone()
                };
                if sound(&prose, unit, first.at.start, next.at.end, &now, register) {
                    found.push((from, was, now, unit.text()));
                }
                continue;
            }
            if first.capitalised {
                now = capitalised(&now);
            }
            if sound(&prose, unit, first.at.start, last.at.end, &now, register) {
                found.push((from, was, now, unit.text()));
            }
        }
    }
    found.sort_by_key(|(at, ..)| *at);
    found.dedup_by_key(|(at, ..)| *at);
    found
}

/// Whether the sentence still reads once the swap is made.
///
/// A stated equivalence is only safe if what it leaves behind is a better sentence. Cutting an opening
/// that was holding the subject leaves nothing to hold it, and no table of phrases can know that
/// in advance. So the rewrite is carried out on the spot and read back with the same reader that
/// judged the original, and a swap that leaves more wrong than it found is not offered.
fn sound(
    prose: &str,
    unit: &Sentence,
    from: usize,
    to: usize,
    now: &str,
    register: Register,
) -> bool {
    let Some(start) = unit.tokens.first().map(|token| token.at.start) else {
        return false;
    };
    let end = prose[start..]
        .find(['.', '!', '?'])
        .map_or(prose.len(), |at| start + at + 1);
    if to > end || from < start {
        return false;
    }
    let before = &prose[start..end];
    let after = format!("{}{now}{}", &prose[start..from], &prose[to..end]);
    // Both readers, not just the grammar one. A shorter wording that reads as well but repeats a
    // word the sentence already carries has moved the cost rather than removed it, and the only
    // way to know is to read the result the same way the original was read.
    let faults = |text: &str| {
        Text::read(text)
            .units
            .iter()
            .map(|unit| {
                let report = check_in(unit, register);
                report.faults.len() + style::read(unit, &report.tags).len()
            })
            .sum::<usize>()
    };
    faults(&after) <= faults(before)
}

/// `word` with its first letter in upper case.
fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + letters.as_str()
    })
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
fn rewritten(source: &str, swaps: &[Swap]) -> String {
    let mut out = source.to_owned();
    for (at, was, now, _) in swaps.iter().rev() {
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
