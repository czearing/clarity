//! Wording that costs a reader more than it pays.
//!
//! Every entry here is a phrase that can be replaced by a shorter one with the same meaning, or a
//! repetition that carries none. Nothing is flagged on suspicion: each finding names the shorter
//! wording, so a writer can see the trade rather than take an instruction.
//!
//! Style is judged after grammar, on the reading grammar produced, and a rewrite is only offered
//! when the result is still grammatical.

use fitkit::core::Span;

use crate::grammar::Sentence;
use crate::tag::Tag;

/// Why a phrase costs more than it pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flaw {
    /// The phrase says the same thing twice.
    Redundant,
    /// A long connective where a one word one exists.
    Roundabout,
    /// An opening that delays the subject, such as "there are".
    Delayed,
    /// A verb turned into a noun and propped up by a weaker verb.
    Buried,
    /// A qualifier that changes nothing.
    Filler,
    /// A phrase worn smooth by overuse.
    Worn,
    /// A word repeated close to itself.
    Echo,
}

impl Flaw {
    /// The convention this flaw breaks, so a register can excuse it.
    #[must_use]
    pub fn convention(self) -> crate::register::Convention {
        match self {
            Self::Echo => crate::register::Convention::Fresh,
            _ => crate::register::Convention::Plain,
        }
    }

    /// What the flaw costs a reader, in one line.
    #[must_use]
    pub fn says(self) -> &'static str {
        match self {
            Self::Redundant => "this says the same thing twice",
            Self::Roundabout => "a shorter connective says this",
            Self::Delayed => "this delays the subject",
            Self::Buried => "the verb is hidden inside a noun",
            Self::Filler => "this qualifier changes nothing",
            Self::Worn => "this phrase is worn smooth and carries no content",
            Self::Echo => "this word was just used",
        }
    }
}

/// One stretch of wording and what to put in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The tokens involved.
    pub at: Span,
    /// What is wrong.
    pub flaw: Flaw,
    /// The shorter wording, or nothing when the phrase should simply go.
    pub instead: &'static str,
}

/// Phrases with a shorter equivalent. Longest first, so the longest match wins.
const PHRASES: &[(&str, &str, Flaw)] = &[
    ("in the not too distant future", "soon", Flaw::Roundabout),
    ("in today s fast paced world", "", Flaw::Worn),
    ("it is important to note that", "", Flaw::Worn),
    ("it should be noted that", "", Flaw::Worn),
    ("for all intents and purposes", "", Flaw::Worn),
    ("at the end of the day", "", Flaw::Worn),
    ("in the event that", "if", Flaw::Roundabout),
    ("due to the fact that", "because", Flaw::Roundabout),
    ("owing to the fact that", "because", Flaw::Roundabout),
    ("in spite of the fact that", "although", Flaw::Roundabout),
    ("despite the fact that", "although", Flaw::Roundabout),
    ("for the purpose of", "to", Flaw::Roundabout),
    ("with regard to", "about", Flaw::Roundabout),
    ("in relation to", "about", Flaw::Roundabout),
    ("in the process of", "", Flaw::Roundabout),
    ("a large number of", "many", Flaw::Roundabout),
    ("a majority of", "most", Flaw::Roundabout),
    ("in order to", "to", Flaw::Roundabout),
    ("at this point in time", "now", Flaw::Roundabout),
    ("in the near future", "soon", Flaw::Roundabout),
    ("prior to", "before", Flaw::Roundabout),
    ("subsequent to", "after", Flaw::Roundabout),
    ("in the absence of", "without", Flaw::Roundabout),
    ("has the ability to", "can", Flaw::Roundabout),
    ("is able to", "can", Flaw::Roundabout),
    ("make a decision", "decide", Flaw::Buried),
    ("reach a conclusion", "conclude", Flaw::Buried),
    ("give consideration to", "consider", Flaw::Buried),
    ("provide an explanation", "explain", Flaw::Buried),
    ("carry out an analysis", "analyse", Flaw::Buried),
    ("take into consideration", "consider", Flaw::Buried),
    ("place an emphasis on", "emphasise", Flaw::Buried),
    ("conduct an investigation", "investigate", Flaw::Buried),
    ("each and every", "every", Flaw::Redundant),
    ("first and foremost", "first", Flaw::Redundant),
    ("absolutely essential", "essential", Flaw::Redundant),
    ("completely eliminate", "eliminate", Flaw::Redundant),
    ("advance planning", "planning", Flaw::Redundant),
    ("past history", "history", Flaw::Redundant),
    ("end result", "result", Flaw::Redundant),
    ("final outcome", "outcome", Flaw::Redundant),
    ("basic fundamentals", "fundamentals", Flaw::Redundant),
    ("close proximity", "proximity", Flaw::Redundant),
    ("free gift", "gift", Flaw::Redundant),
    ("added bonus", "bonus", Flaw::Redundant),
    ("unexpected surprise", "surprise", Flaw::Redundant),
    ("new innovation", "innovation", Flaw::Redundant),
    ("various different", "various", Flaw::Redundant),
    ("rich tapestry", "", Flaw::Worn),
    ("game changer", "", Flaw::Worn),
    ("deep dive", "", Flaw::Worn),
    ("seamlessly integrate", "", Flaw::Worn),
    ("robust framework", "", Flaw::Worn),
    ("leverage", "use", Flaw::Worn),
    ("utilise", "use", Flaw::Worn),
    ("utilize", "use", Flaw::Worn),
    ("delve", "", Flaw::Worn),
    ("myriad", "many", Flaw::Worn),
    ("plethora", "many", Flaw::Worn),
    ("paradigm", "", Flaw::Worn),
    ("synergy", "", Flaw::Worn),
    ("holistic", "", Flaw::Worn),
];

/// Qualifiers that change nothing they attach to.
const FILLERS: &[&str] = &[
    "very",
    "really",
    "quite",
    "rather",
    "actually",
    "basically",
    "literally",
    "simply",
];

/// Whether a word is a qualifier that changes nothing wherever it appears.
#[must_use]
pub fn is_empty(word: &str) -> bool {
    FILLERS.contains(&word)
}

/// What is wrong with the wording of `sentence`.
///
/// A listed phrase claims its words, so nothing inside one is reported twice.
#[must_use]
pub fn read(sentence: &Sentence, tags: &[Tag]) -> Vec<Note> {
    let keys: Vec<&str> = sentence
        .tokens
        .iter()
        .map(|token| token.key.as_str())
        .collect();
    let mut notes = Vec::new();
    let mut at = 0;
    while at < keys.len() {
        if let Some(note) = phrase_at(&keys, at) {
            at = note.at.end;
            notes.push(note);
            continue;
        }
        if FILLERS.contains(&keys[at]) {
            notes.push(Note {
                at: Span::new(at, at + 1),
                flaw: Flaw::Filler,
                instead: "",
            });
        }
        at += 1;
    }
    let phrases: Vec<Span> = notes.iter().map(|note| note.at).collect();
    let fresh = |note: &Note| !phrases.iter().any(|span| span.overlaps(note.at));
    notes.extend(delayed(&keys).into_iter().filter(fresh));
    notes.extend(echoes(&keys, tags).into_iter().filter(fresh));
    notes.sort_by_key(|note| note.at.start);
    notes
}

/// The longest listed phrase starting at `at`.
fn phrase_at(keys: &[&str], at: usize) -> Option<Note> {
    PHRASES.iter().find_map(|(phrase, instead, flaw)| {
        let mut end = at;
        for word in phrase.split(' ') {
            if keys.get(end) != Some(&word) {
                return None;
            }
            end += 1;
        }
        Some(Note {
            at: Span::new(at, end),
            flaw: *flaw,
            instead,
        })
    })
}

/// Openings that hold the subject back.
///
/// "there is" always does. "it is" only does when something is pushed to the end, as in "it is
/// clear that dogs run", since "it is here" is about something.
fn delayed(keys: &[&str]) -> Vec<Note> {
    keys.windows(2)
        .enumerate()
        .filter(|(at, pair)| {
            matches!(pair[1], "is" | "are" | "was" | "were")
                && match pair[0] {
                    "there" => true,
                    "it" => keys[*at + 2..]
                        .iter()
                        .any(|key| matches!(*key, "that" | "to")),
                    _ => false,
                }
        })
        .map(|(at, _)| Note {
            at: Span::new(at, at + 2),
            flaw: Flaw::Delayed,
            instead: "",
        })
        .collect()
}

/// A content word repeated within a short window, which reads as an accident.
///
/// Only nouns, verbs, and adjectives count. Repeating "the" is how English works.
fn echoes(keys: &[&str], tags: &[Tag]) -> Vec<Note> {
    const WINDOW: usize = 8;
    let carries = |index: usize| {
        tags.get(index).is_some_and(|tag| {
            matches!(
                tag,
                Tag::Noun(_) | Tag::Verb(_) | Tag::Adjective | Tag::Adverb
            )
        })
    };
    let mut notes = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        if !carries(index) {
            continue;
        }
        let start = index.saturating_sub(WINDOW);
        if keys[start..index]
            .iter()
            .enumerate()
            .any(|(back, earlier)| {
                same_stem(earlier, key) && carries(start + back) && start + back != index
            })
        {
            notes.push(Note {
                at: Span::new(index, index + 1),
                flaw: Flaw::Echo,
                instead: "",
            });
        }
    }
    notes
}

/// Whether two words share a stem, ignoring a regular ending.
fn same_stem(left: &str, right: &str) -> bool {
    left.len() >= 3 && stem(left) == stem(right)
}

/// A word with a regular ending removed.
fn stem(word: &str) -> &str {
    for ending in ["ing", "ed", "es", "s"] {
        if let Some(rest) = word.strip_suffix(ending) {
            if rest.len() >= 3 {
                return rest;
            }
        }
    }
    word
}

#[cfg(test)]
mod tests {
    use super::{read, Flaw};
    use crate::check::check;
    use crate::grammar::Sentence;

    fn flaws(text: &str) -> Vec<Flaw> {
        let sentence = Sentence::read(text);
        let tags = check(&sentence).tags;
        read(&sentence, &tags)
            .into_iter()
            .map(|note| note.flaw)
            .collect()
    }

    #[test]
    fn a_roundabout_connective_is_named_with_its_replacement() {
        let sentence = Sentence::read("due to the fact that it rained");
        let notes = read(&sentence, &check(&sentence).tags);
        assert_eq!(notes[0].flaw, Flaw::Roundabout);
        assert_eq!(notes[0].instead, "because");
    }

    #[test]
    fn a_phrase_that_says_itself_twice_is_caught() {
        assert_eq!(flaws("each and every dog"), [Flaw::Redundant]);
    }

    #[test]
    fn a_buried_verb_is_caught() {
        assert_eq!(flaws("we make a decision"), [Flaw::Buried]);
    }

    #[test]
    fn a_worn_phrase_is_caught() {
        assert_eq!(flaws("it is important to note that dogs run"), [Flaw::Worn]);
    }

    #[test]
    fn a_repeated_word_is_caught_and_a_repeated_determiner_is_not() {
        assert_eq!(flaws("the dog walked the dog"), [Flaw::Echo]);
        assert!(flaws("the dog sees the cat").is_empty());
    }

    #[test]
    fn plain_writing_draws_no_note() {
        assert!(flaws("the dog runs").is_empty());
        assert!(flaws("she walks to the store").is_empty());
    }
}
