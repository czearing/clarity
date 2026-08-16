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
    /// What to write instead: `Some` states it, and `Some("")` means the words go.
    ///
    /// `None` is a finding with no rewrite behind it. Some wording is wrong in a way that only
    /// the writer can settle: an opening that holds the subject back has to be rebuilt around
    /// whatever the subject turns out to be, and a worn noun has to be replaced by the thing it
    /// was standing in for. Saying so in the type is what keeps a rewriting pass from guessing,
    /// because there is nothing there for it to write.
    pub instead: Option<&'static str>,
}

/// Phrases with a shorter equivalent. Longest first, so the longest match wins.
const PHRASES: &[(&str, Option<&str>, Flaw)] = &[
    (
        "in the not too distant future",
        Some("soon"),
        Flaw::Roundabout,
    ),
    ("in today s fast paced world", Some(""), Flaw::Worn),
    ("it is important to note that", Some(""), Flaw::Worn),
    ("it should be noted that", Some(""), Flaw::Worn),
    ("for all intents and purposes", Some(""), Flaw::Worn),
    ("at the end of the day", Some(""), Flaw::Worn),
    ("in the event that", Some("if"), Flaw::Roundabout),
    ("due to the fact that", Some("because"), Flaw::Roundabout),
    ("owing to the fact that", Some("because"), Flaw::Roundabout),
    (
        "in spite of the fact that",
        Some("although"),
        Flaw::Roundabout,
    ),
    ("despite the fact that", Some("although"), Flaw::Roundabout),
    // A verb can select the preposition these open with, as "stands in relation to" does, and
    // then the phrase is the verb's complement rather than a connective standing on its own. What
    // to put in its place depends on the verb, so the finding is reported and the wording is left
    // to the writer.
    ("for the purpose of", None, Flaw::Roundabout),
    ("with regard to", None, Flaw::Roundabout),
    ("in relation to", None, Flaw::Roundabout),
    ("in the process of", Some(""), Flaw::Roundabout),
    ("a large number of", Some("many"), Flaw::Roundabout),
    ("a majority of", Some("most"), Flaw::Roundabout),
    ("in order to", Some("to"), Flaw::Roundabout),
    ("at this point in time", Some("now"), Flaw::Roundabout),
    ("in the near future", Some("soon"), Flaw::Roundabout),
    ("prior to", Some("before"), Flaw::Roundabout),
    ("subsequent to", Some("after"), Flaw::Roundabout),
    ("in the absence of", Some("without"), Flaw::Roundabout),
    ("has the ability to", Some("can"), Flaw::Roundabout),
    ("is able to", Some("can"), Flaw::Roundabout),
    ("make a decision", Some("decide"), Flaw::Buried),
    ("reach a conclusion", Some("conclude"), Flaw::Buried),
    ("give consideration to", Some("consider"), Flaw::Buried),
    ("provide an explanation", Some("explain"), Flaw::Buried),
    ("carry out an analysis", Some("analyse"), Flaw::Buried),
    ("take into consideration", Some("consider"), Flaw::Buried),
    ("place an emphasis on", Some("emphasise"), Flaw::Buried),
    (
        "conduct an investigation",
        Some("investigate"),
        Flaw::Buried,
    ),
    ("each and every", Some("every"), Flaw::Redundant),
    ("first and foremost", Some("first"), Flaw::Redundant),
    ("absolutely essential", Some("essential"), Flaw::Redundant),
    ("completely eliminate", Some("eliminate"), Flaw::Redundant),
    ("advance planning", Some("planning"), Flaw::Redundant),
    ("past history", Some("history"), Flaw::Redundant),
    ("end result", Some("result"), Flaw::Redundant),
    ("final outcome", Some("outcome"), Flaw::Redundant),
    ("basic fundamentals", Some("fundamentals"), Flaw::Redundant),
    ("close proximity", Some("proximity"), Flaw::Redundant),
    ("free gift", Some("gift"), Flaw::Redundant),
    ("added bonus", Some("bonus"), Flaw::Redundant),
    ("unexpected surprise", Some("surprise"), Flaw::Redundant),
    ("new innovation", Some("innovation"), Flaw::Redundant),
    ("various different", Some("various"), Flaw::Redundant),
    ("rich tapestry", None, Flaw::Worn),
    ("game changer", None, Flaw::Worn),
    ("deep dive", None, Flaw::Worn),
    ("seamlessly integrate", None, Flaw::Worn),
    ("robust framework", None, Flaw::Worn),
    ("leverage", Some("use"), Flaw::Worn),
    ("utilise", Some("use"), Flaw::Worn),
    ("utilize", Some("use"), Flaw::Worn),
    ("delve", None, Flaw::Worn),
    ("myriad", Some("many"), Flaw::Worn),
    ("plethora", Some("many"), Flaw::Worn),
    ("paradigm", None, Flaw::Worn),
    ("synergy", None, Flaw::Worn),
    ("holistic", None, Flaw::Worn),
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

/// Whether the qualifier at `at` is being used as one.
///
/// A qualifier qualifies something, and only a verb, an adjective or another adverb can be
/// qualified. Where the next word cannot take a qualifier, the word is doing some other job and
/// removing it would break the sentence: "rather" in "rather than" opens a comparison, and what
/// is left after cutting it is not a shorter sentence but a broken one.
///
/// Nothing is listed. The test is what the following word can be, so any fixed pairing built on a
/// qualifier is protected by the same reading.
fn qualifies(tags: &[Tag], at: usize) -> bool {
    matches!(
        tags.get(at + 1),
        Some(Tag::Verb(_) | Tag::Adjective | Tag::Adverb | Tag::Modal)
    )
}

/// What is wrong with the wording of `sentence`.
///
/// A listed phrase claims its words, so nothing inside one is reported twice.
#[must_use]
pub fn read(sentence: &Sentence, tags: &[Tag]) -> Vec<Note> {
    // A named term is not the word it names. This crate writes "rather" is a qualifier, and
    // reading that as a use of the word had the plain pass proposing to delete it from the
    // sentence explaining it. A mention is given a key no phrase and no qualifier can match, so
    // wording is judged on the words the sentence uses and not on the ones it talks about.
    let keys: Vec<&str> = sentence
        .tokens
        .iter()
        .map(|token| {
            if token.mention {
                ""
            } else {
                token.key.as_str()
            }
        })
        .collect();
    let mut notes = Vec::new();
    let mut at = 0;
    while at < keys.len() {
        if let Some(note) = phrase_at(&keys, at) {
            at = note.at.end;
            notes.push(note);
            continue;
        }
        if FILLERS.contains(&keys[at]) && qualifies(tags, at) {
            notes.push(Note {
                at: Span::new(at, at + 1),
                flaw: Flaw::Filler,
                instead: Some(""),
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
            instead: *instead,
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
            instead: None,
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
                instead: None,
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
        assert_eq!(notes[0].instead, Some("because"));
    }

    #[test]
    fn a_qualifier_with_nothing_to_qualify_is_left_alone() {
        // "rather" empties an adjective it sits on, but in "rather than" it opens a comparison,
        // and cutting it leaves a sentence that has lost a word it needed.
        assert_eq!(flaws("spans are counted rather than summed"), []);
        assert_eq!(flaws("the parser is rather slow"), [Flaw::Filler]);
    }

    #[test]
    fn an_opening_that_holds_the_subject_back_offers_no_rewrite() {
        // What is wrong is reportable. What to do about it is not, because the sentence has to be
        // rebuilt around a subject the opening never named.
        let sentence = Sentence::read("there is no allocation here");
        let notes = read(&sentence, &check(&sentence).tags);
        assert_eq!(notes[0].flaw, Flaw::Delayed);
        assert_eq!(notes[0].instead, None);
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
