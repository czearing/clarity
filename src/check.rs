//! What is wrong with a sentence, and what could not be judged.
//!
//! A fault is only ever reported with the rule that produced it. Words the lexicon does not know
//! are listed separately, because a sentence resting on them was not checked, only guessed at.

use fitkit::core::Span;
use fitkit::fit::recover;
use fitkit::Reported;

use crate::grammar::{clauses, is_imperative, why, Grammar, Rule, Sentence};
use crate::lexicon::Lexicon;
use crate::register::{Convention, Register};
use crate::style::Note;
use crate::tag::{Form, Number, Tag};
use crate::token::Token;

/// One broken rule and where it broke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fault {
    /// The words involved, from the first to the last.
    pub at: Span,
    /// The rule that forbids it.
    pub rule: Rule,
}

/// The verdict on a sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The tag chosen for each token.
    pub tags: Vec<Tag>,
    /// Rules the sentence breaks.
    pub faults: Vec<Fault>,
    /// Positions of words the lexicon could not place.
    pub unknown: Vec<usize>,
    /// Wording the register does not excuse.
    pub notes: Vec<Note>,
}

impl Report {
    /// Whether the sentence breaks no rule and rests on no unknown word.
    ///
    /// False when anything at all was guessed, so a pass is always a claim the engine can defend.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.faults.is_empty() && self.unknown.is_empty() && self.notes.is_empty()
    }
}

/// Check `sentence`.
///
/// Reads it once, then insists on a predicate: a string of words with no tensed verb is not a
/// sentence, so if the cheapest reading has none, each word that could carry tense is tried in
/// turn and the cheapest sentence wins. That step is what turns "the child walk" from a noun
/// phrase into a subject and a verb that disagree.
///
/// A predicate means a subject and a tensed verb, not a tensed verb alone. Without the subject,
/// "an old pond" reads as a sentence whose verb is "pond", and nothing is ever reported.
#[must_use]
pub fn check(sentence: &Sentence) -> Report {
    check_in(sentence, Register::STRICT.without(Convention::Marks))
}

/// Check `sentence` under a register.
///
/// A relaxed register drops the requirement it names and no other. Agreement is never relaxed.
#[must_use]
pub fn check_in(sentence: &Sentence, register: Register) -> Report {
    // A contraction is always read with its apostrophe put back, whatever the register. Spelling
    // must never blind the reading: leaving "dont" whole would hide the disagreement in "the train
    // dont move" behind a word the lexicon cannot place. Whether the spelling itself is a fault is
    // a separate question, settled below by the register.
    let fused: Vec<usize> = crate::token::fused(&sentence.tokens).collect();
    let mended = Sentence {
        tokens: crate::token::mend(&sentence.tokens),
    };
    let sentence = &mended;
    let unknown: Vec<usize> = sentence
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            matches!(
                fitkit::ask(&Lexicon, token),
                Ok(Reported::Unreported) | Err(_)
            )
        })
        .map(|(index, _)| index)
        .collect();

    let mut tags = read(sentence, Grammar::default());
    if clauses(&tags).is_empty() && !is_imperative(&tags) {
        if let Some(better) = read_as_a_sentence(sentence) {
            tags = better;
        }
    }

    let mut faults: Vec<Fault> = tags
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            why(pair[0], pair[1])
                .filter(|rule| !matches!(rule, Rule::SubjectVerb | Rule::DoubledTense))
                .map(|rule| Fault {
                    at: Span::new(index, index + 2),
                    rule,
                })
        })
        .collect();
    faults.extend(distant_disagreement(sentence, &tags));
    faults.extend(doubled_tense(sentence, &tags));
    if !register.waives(Convention::Apostrophes) {
        faults.extend(fused.iter().map(|at| Fault {
            at: Span::new(*at, at + 1),
            rule: Rule::Unapostrophed,
        }));
    }
    if !register.waives(Convention::Predicate)
        && !tags.is_empty()
        && clauses(&tags).is_empty()
        && !is_imperative(&tags)
    {
        faults.push(Fault {
            at: Span::new(0, tags.len()),
            rule: Rule::NoPredicate,
        });
    }
    if !register.waives(Convention::Marks) && !marked(sentence) {
        faults.push(Fault {
            at: Span::new(0, sentence.tokens.len()),
            rule: Rule::Unmarked,
        });
    }
    let notes = crate::style::read(sentence, &tags)
        .into_iter()
        .filter(|note| !register.waives(note.flaw.convention()))
        .collect();
    Report {
        tags,
        faults,
        unknown,
        notes,
    }
}

/// A tensed verb, a negator, and a second verb that can only be tensed.
///
/// Only a negator may sit between them. A wider gap could be a relative clause, which is legal and
/// cannot be told apart without a parse, so it is left alone rather than guessed at.
fn doubled_tense(sentence: &Sentence, tags: &[Tag]) -> Vec<Fault> {
    let negator = |at: usize| {
        sentence
            .tokens
            .get(at)
            .is_some_and(|token| matches!(token.key.as_str(), "n't" | "not"))
    };
    let settled = |tag: Tag| {
        matches!(
            tag,
            Tag::Verb(Form::ThirdSingular | Form::PastSingular | Form::PastPlural)
        )
    };
    let mut found: Vec<Fault> = (0..tags.len().saturating_sub(2))
        .filter(|&at| tags[at].is_finite_verb() && negator(at + 1) && settled(tags[at + 2]))
        .map(|at| Fault {
            at: Span::new(at, at + 3),
            rule: Rule::DoubledTense,
        })
        .collect();
    found.extend(adjacent_tenses(tags));
    found
}

/// Two tensed verbs side by side, where nothing licenses the second.
///
/// A free relative puts them there legitimately, as in "what English forbids costs infinity", so
/// the scan stays quiet while a subordinator is open. A complement verb is legitimate too, as
/// "work" is in "help make this work", so the first of the pair must itself be a verb taking a
/// subject rather than one taken as a complement.
fn adjacent_tenses(tags: &[Tag]) -> Vec<Fault> {
    let settled = |tag: Tag| {
        matches!(
            tag,
            Tag::Verb(Form::ThirdSingular | Form::PastSingular | Form::PastPlural)
        )
    };
    let mut found = Vec::new();
    let mut open = false;
    let mut subject = false;
    for (at, &tag) in tags.iter().enumerate() {
        match tag {
            Tag::Subordinator => open = true,
            Tag::Mark | Tag::Coordinator => {
                open = false;
                subject = false;
            }
            _ => {}
        }
        let takes_a_subject = subject || settled(tag);
        if !open
            && tag.is_finite_verb()
            && takes_a_subject
            && tags.get(at + 1).copied().is_some_and(settled)
        {
            found.push(Fault {
                at: Span::new(at, at + 2),
                rule: Rule::DoubledTense,
            });
        }
        if tag.is_nominal() {
            subject = true;
        } else if tag.is_finite_verb() || matches!(tag, Tag::Modal) {
            subject = false;
        }
    }
    found
}

/// Whether a subordinate clause stands between a noun and a verb.
///
/// A relative pronoun keeps the two together: in "the key that opens the door", "opens" really does
/// answer to "key". Every other subordinator starts a clause with a subject of its own, so the noun
/// outside it never governs the verb inside it. "The sentence as read" has no disagreement in it,
/// and reporting one would be reading across a boundary the writer put there.
fn adverbial(sentence: &Sentence, tags: &[Tag], at: usize, verb: usize) -> bool {
    const RELATIVE: &[&str] = &[
        "that",
        "which",
        "who",
        "whom",
        "whose",
        "what",
        "whoever",
        "whichever",
        "whatever",
    ];
    (at + 1..verb).any(|index| {
        matches!(tags[index], Tag::Subordinator)
            && sentence
                .tokens
                .get(index)
                .is_some_and(|token| !RELATIVE.contains(&token.key.as_str()))
    })
}

/// The subject of a clause as the verb sees it.
///
/// Two nouns joined by a coordinator are one subject and that subject is plural, however singular
/// each half of it may be. "Transformation, alignment, and synergy are" is right, and reading only
/// the noun nearest the verb reports it as wrong. The clause is scanned from its head to its verb
/// for a coordinator standing between two nominals, which is what joining looks like; a coordinator
/// joining anything else, such as two clauses, leaves the subject alone.
fn subject(tags: &[Tag], at: usize, verb: usize) -> Tag {
    let joined = (at + 1..verb).any(|index| {
        matches!(tags[index], Tag::Coordinator)
            && tags[..index].iter().rev().any(|tag| tag.is_nominal())
            && tags[index + 1..verb].iter().any(|tag| tag.is_nominal())
    });
    if joined {
        Tag::Noun(Number::Plural)
    } else {
        tags[at]
    }
}

/// Whether a unit opens with a capital and closes with a mark.
fn marked(sentence: &Sentence) -> bool {
    let opens = sentence
        .tokens
        .first()
        .is_some_and(|token| token.capitalised);
    let closes = sentence.tokens.last().is_some_and(Token::ends_sentence);
    opens && closes
}

/// The tag chosen for each token under `grammar`.
fn read(sentence: &Sentence, grammar: Grammar) -> Vec<Tag> {
    recover(&grammar, sentence)
        .controls
        .iter()
        .map(|control| control.params)
        .collect()
}

/// The reading in which the sentence has a predicate, when one exists that breaks no rule.
///
/// English spells the plural noun and the third person singular verb alike, so the cheapest
/// reading of "a predicate means a subject" can make "means" a noun and leave no verb behind.
/// Reading it again with each word in turn made to carry tense, or with the first word made a
/// command, recovers the sentence. Only a reading that breaks nothing at all is taken: one that
/// breaks a rule would be a phrase pressed into a sentence, and reporting the damage would blame
/// the writer for a mistake the engine made.
fn read_as_a_sentence(sentence: &Sentence) -> Option<Vec<Tag>> {
    let clean = |tags: &[Tag]| {
        tags.windows(2).all(|pair| why(pair[0], pair[1]).is_none())
            && distant_disagreement(sentence, tags).is_empty()
    };
    let command = Grammar {
        command: true,
        ..Grammar::default()
    };
    let tries =
        std::iter::once(read(sentence, command)).chain((0..sentence.tokens.len()).map(|at| {
            read(
                sentence,
                Grammar {
                    predicate_at: Some(at),
                    ..Grammar::default()
                },
            )
        }));
    tries
        .filter(|tags| clean(tags))
        .find(|tags| !clauses(tags).is_empty() || is_imperative(tags))
}

/// Agreement judged over the clause rather than the pair, so a noun inside a modifier cannot be
/// mistaken for the subject.
fn distant_disagreement(sentence: &Sentence, tags: &[Tag]) -> Vec<Fault> {
    clauses(tags)
        .into_iter()
        .filter(|&(at, verb)| !adverbial(sentence, tags, at, verb))
        .filter(|&(at, verb)| why(subject(tags, at, verb), tags[verb]).is_some())
        .map(|(at, verb)| Fault {
            at: Span::new(at, verb + 1),
            rule: Rule::SubjectVerb,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{check, Rule};
    use crate::grammar::Sentence;

    fn faults(text: &str) -> Vec<Rule> {
        check(&Sentence::read(text))
            .faults
            .iter()
            .map(|fault| fault.rule)
            .collect()
    }

    #[test]
    fn a_correct_sentence_is_clean() {
        assert!(check(&Sentence::read("the dog runs")).is_clean());
        assert!(check(&Sentence::read("she can walk")).is_clean());
    }

    #[test]
    fn a_determiner_that_disagrees_is_caught_and_named() {
        assert_eq!(faults("a dogs run"), [Rule::DeterminerNumber]);
    }

    #[test]
    fn a_modal_followed_by_an_inflected_verb_is_caught() {
        assert_eq!(faults("she can walks"), [Rule::ModalTakesBase]);
    }

    #[test]
    fn a_word_the_lexicon_cannot_place_is_reported_rather_than_passed() {
        let report = check(&Sentence::read("the qq runs"));
        assert_eq!(report.unknown, [1]);
        assert!(
            !report.is_clean(),
            "a sentence resting on a guess is not clean"
        );
    }
}
