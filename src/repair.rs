//! The smallest change that makes a sentence grammatical.
//!
//! Only words at a fault are touched, and only by swapping one inflected form for another of the
//! same word. Nothing is added, removed, or reworded, so a repair never changes what was meant.

use crate::check::{check_in, Report};
use crate::grammar::Sentence;
use crate::register::{Convention, Register};

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
    repair_in(sentence, Register::STRICT)
}

/// The fewest edits that leave nothing `register` holds against the sentence.
///
/// Repairing under a register rather than under the strictest reading is what stops a summary line
/// being conjugated. "One pass of the trellis." is a heading, and a repair asked to find it a verb
/// answers "Ones", which is not a correction of anything. A unit is only worth repairing once it
/// has been read as the kind of thing it is.
#[must_use]
pub fn repair_in(sentence: &Sentence, register: Register) -> Option<Vec<Edit>> {
    // No swap of one form for another puts a capital at the front or a full stop at the end, so
    // holding a sentence to that convention here would only stop every repair the sentence did
    // have. Whether it is punctuated is reported where it can be acted on.
    let register = register.without(Convention::Marks);
    let report = check_in(sentence, register);
    if report.faults.is_empty() || !report.unknown.is_empty() {
        return None;
    }
    let sites = sites(&report);
    single(sentence, &sites, register).or_else(|| pair(sentence, &sites, register))
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
fn single(sentence: &Sentence, sites: &[usize], register: Register) -> Option<Vec<Edit>> {
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
                    register,
                )
            })
            .map(|word| vec![Edit { at, word }])
    })
}

/// Two swaps, for a fault no single word can settle.
fn pair(sentence: &Sentence, sites: &[usize], register: Register) -> Option<Vec<Edit>> {
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
                    if clears(sentence, &edits, register) {
                        return Some(edits);
                    }
                }
            }
        }
    }
    None
}

/// Whether applying `edits` leaves a sentence with no fault and nothing unknown.
fn clears(sentence: &Sentence, edits: &[Edit], register: Register) -> bool {
    let report = check_in(&apply(sentence, edits), register);
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
///
/// Inflection is a way of proposing a word, not a way of making one: applied to a full stop it
/// proposes ".es" and applied to a numeral it proposes "2es", and a repair that writes either has
/// done more damage than the fault it was called to mend. Three things bound it, and each answers
/// a different way of inventing a word.
fn forms(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    if !lower.chars().all(char::is_alphabetic) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (from, to) in PAIRS {
        if lower == *from {
            out.push((*to).to_string());
        }
        if lower == *to {
            out.push((*from).to_string());
        }
    }
    // A closed class word has the forms it has and no others. Inflecting one derives a word that
    // exists and means something else: "a" gives "as", "one" gives "ones", "it" gives "its", and
    // each is a real entry in the lexicon, so asking whether the result is a word does not catch
    // any of them. What catches them is that English does not make new determiners or pronouns.
    if !crate::lexicon::is_closed(&lower) {
        match plain(&lower) {
            Some(stem) => out.push(stem),
            None => out.extend(marked(&lower)),
        }
        if let Some(stem) = lower.strip_suffix("ed") {
            out.push(stem.to_string());
            out.extend(marked(stem));
        }
    }
    // What the lexicon lists, it has an opinion about, and a rule may not overrule it. "written"
    // is listed as a participle and "be" as a plain verb, so deriving "writtens" and "bes" is a
    // rule contradicting the entry it was applied to. Where the lexicon is silent the rule is all
    // there is and is trusted, which is how "file" still reaches "files".
    let listed = crate::lexicon::places(&lower);
    out.retain(|form| {
        *form != lower
            && !form.is_empty()
            && if listed {
                crate::lexicon::places(form)
            } else {
                crate::lexicon::knows(form)
            }
    });
    out.dedup();
    out
}

/// The word with the plural inflection on it, spelled the way English spells it.
///
/// One spelling, not every spelling. Offering both "{word}s" and "{word}es" and letting the
/// lexicon choose asks a question the lexicon cannot answer, because it places a word by its shape
/// and "categorys" has the shape of a plural. English decides this by the end of the stem and
/// leaves no choice. A sibilant takes \"es\". A consonant before a final \"y\" turns it into
/// \"ies\", and every other stem adds \"s\".
fn marked(word: &str) -> Option<String> {
    if word.is_empty() {
        return None;
    }
    if word.ends_with(['s', 'x', 'z']) || word.ends_with("ch") || word.ends_with("sh") {
        return Some(format!("{word}es"));
    }
    let mut letters = word.chars().rev();
    let last = letters.next()?;
    let before = letters.next();
    if last == 'y' && before.is_some_and(|before| !VOWELS.contains(before)) {
        return Some(format!("{}ies", &word[..word.len() - 1]));
    }
    if last == 'o' && before.is_some_and(|before| !VOWELS.contains(before)) {
        return Some(format!("{word}es"));
    }
    Some(format!("{word}s"))
}

/// The word with the plural inflection taken off, if that inflection is what put it there.
///
/// The same rule read backwards, and checked by running it forwards again. Stripping "es" from
/// "address" gives "addres", and only spelling the answer back out catches it: "addres" marked is
/// "addreses", which is not the word we started with, so the strip was never an inflection.
fn plain(word: &str) -> Option<String> {
    let stems = [
        word.strip_suffix("ies").map(|stem| format!("{stem}y")),
        word.strip_suffix("es").map(str::to_owned),
        word.strip_suffix('s').map(str::to_owned),
    ];
    stems
        .into_iter()
        .flatten()
        .find(|stem| marked(stem).as_deref() == Some(word))
}

/// The letters that make a preceding consonant matter to how a word is spelled.
const VOWELS: &str = "aeiou";

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
