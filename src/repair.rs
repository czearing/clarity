//! The smallest change that makes a sentence grammatical.
//!
//! Only words at a fault are touched, and only by swapping one inflected form for another of the
//! same word. Nothing is added, removed, or reworded, so a repair never changes what was meant.

use crate::check::{check, Report};
use crate::grammar::Sentence;

/// A word replaced by another form of itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Which token.
    pub at: usize,
    /// What it becomes.
    pub word: String,
}

/// The fewest edits that leave no fault, if any set of single-word swaps does.
///
/// Returns nothing when the sentence is already clean, rests on an unknown word, or cannot be
/// fixed by inflection alone. Refusing is the right answer for the last case: a sentence needing
/// a rewrite is not a sentence needing a repair.
#[must_use]
pub fn repair(sentence: &Sentence) -> Option<Vec<Edit>> {
    let report = check(sentence);
    if report.faults.is_empty() || !report.unknown.is_empty() {
        return None;
    }
    let sites = sites(&report);
    single(sentence, &sites).or_else(|| pair(sentence, &sites))
}

/// Positions a fault touches, latest first.
///
/// The later word is tried first, so a verb is brought to its subject rather than a subject
/// rewritten to suit its verb. The subject is usually what the writer meant.
fn sites(report: &Report) -> Vec<usize> {
    let mut sites: Vec<usize> = report
        .faults
        .iter()
        .flat_map(|fault| [fault.at.start, fault.at.end.saturating_sub(1)])
        .collect();
    sites.sort_unstable();
    sites.dedup();
    sites.reverse();
    sites
}

/// One swap that clears every fault.
fn single(sentence: &Sentence, sites: &[usize]) -> Option<Vec<Edit>> {
    sites.iter().find_map(|&at| {
        forms(&sentence.tokens[at].word)
            .into_iter()
            .find(|word| {
                clears(
                    sentence,
                    &[Edit {
                        at,
                        word: word.clone(),
                    }],
                )
            })
            .map(|word| vec![Edit { at, word }])
    })
}

/// Two swaps, for a fault no single word can settle.
fn pair(sentence: &Sentence, sites: &[usize]) -> Option<Vec<Edit>> {
    for (index, &first) in sites.iter().enumerate() {
        for &second in &sites[index + 1..] {
            for left in forms(&sentence.tokens[first].word) {
                for right in forms(&sentence.tokens[second].word) {
                    let edits = vec![
                        Edit {
                            at: first,
                            word: left.clone(),
                        },
                        Edit {
                            at: second,
                            word: right,
                        },
                    ];
                    if clears(sentence, &edits) {
                        return Some(edits);
                    }
                }
            }
        }
    }
    None
}

/// Whether applying `edits` leaves a sentence with no fault and nothing unknown.
fn clears(sentence: &Sentence, edits: &[Edit]) -> bool {
    let report = check(&apply(sentence, edits));
    report.is_clean()
}

/// `sentence` with `edits` applied.
#[must_use]
pub fn apply(sentence: &Sentence, edits: &[Edit]) -> Sentence {
    let mut out = sentence.clone();
    for edit in edits {
        if let Some(token) = out.tokens.get_mut(edit.at) {
            *token = crate::token::retype(token, &edit.word);
        }
    }
    out
}

/// Other forms of the same word, and other determiners of the same kind.
///
/// Regular inflection only. An irregular form the lexicon knows is reachable because the lexicon
/// lists both members of the pair, and one that it does not know is out of reach, which the
/// caller reports as a refusal to repair rather than a guess.
fn forms(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let mut out = Vec::new();
    for (from, to) in PAIRS {
        if lower == *from {
            out.push((*to).to_string());
        }
        if lower == *to {
            out.push((*from).to_string());
        }
    }
    if let Some(stem) = lower.strip_suffix("es") {
        out.push(stem.to_string());
    }
    if let Some(stem) = lower.strip_suffix('s') {
        out.push(stem.to_string());
    } else {
        out.push(format!("{lower}s"));
        out.push(format!("{lower}es"));
    }
    if let Some(stem) = lower.strip_suffix("ed") {
        out.push(stem.to_string());
        out.push(format!("{stem}s"));
    }
    out.retain(|form| *form != lower && !form.is_empty());
    out.dedup();
    out
}

/// Words whose counterpart no rule derives.
const PAIRS: &[(&str, &str)] = &[
    ("a", "the"),
    ("an", "the"),
    ("this", "these"),
    ("that", "those"),
    ("every", "all"),
    ("is", "are"),
    ("was", "were"),
    ("has", "have"),
    ("does", "do"),
    ("man", "men"),
    ("woman", "women"),
    ("child", "children"),
    ("person", "people"),
    ("mouse", "mice"),
    ("foot", "feet"),
    ("tooth", "teeth"),
    ("criterion", "criteria"),
    ("analysis", "analyses"),
    ("datum", "data"),
];

#[cfg(test)]
mod tests {
    use super::{apply, repair};
    use crate::check::check;
    use crate::grammar::Sentence;

    fn fixed(text: &str) -> String {
        let sentence = Sentence::read(text);
        let edits = repair(&sentence).expect("a fault this small has a repair");
        apply(&sentence, &edits)
            .tokens
            .iter()
            .map(|token| token.word.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn a_verb_is_brought_into_agreement() {
        assert_eq!(fixed("the dogs runs"), "the dogs run");
        assert_eq!(fixed("the dog run"), "the dog runs");
    }

    #[test]
    fn a_determiner_is_brought_into_agreement() {
        assert_eq!(fixed("a dogs run"), "the dogs run");
        assert_eq!(fixed("every dogs run"), "all dogs run");
    }

    #[test]
    fn an_irregular_pair_is_reachable() {
        assert_eq!(fixed("the man were here"), "the man was here");
    }

    #[test]
    fn a_repair_leaves_nothing_to_report() {
        for text in [
            "she can walks",
            "the criteria is clear",
            "the key to the cabinets are here",
        ] {
            let sentence = Sentence::read(text);
            let edits = repair(&sentence).expect("a repair exists");
            assert!(
                check(&apply(&sentence, &edits)).is_clean(),
                "{text} left a fault"
            );
        }
    }

    #[test]
    fn a_clean_sentence_needs_no_repair() {
        assert_eq!(repair(&Sentence::read("the dog runs")), None);
    }
}
