//! What is wrong with a sentence, and what could not be judged.
//!
//! A fault is only ever reported with the rule that produced it. Words the lexicon does not know
//! are listed separately, because a sentence resting on them was not checked, only guessed at.

use fitkit::core::Span;
use fitkit::fit::{recover, Fit};
use fitkit::Reported;

use crate::grammar::{clauses, why, Grammar, Rule, Sentence};
use crate::lexicon::Lexicon;
use crate::register::{Convention, Register};
use crate::style::Note;
use crate::tag::{Form, Tag};
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
    let mended;
    let sentence = if register.waives(Convention::Apostrophes) {
        mended = Sentence {
            tokens: crate::token::mend(&sentence.tokens),
        };
        &mended
    } else {
        sentence
    };
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
    if !register.waives(Convention::Predicate) && clauses(&tags).is_empty() {
        if let Some(better) = insist_on_a_predicate(sentence) {
            tags = better;
        }
    }

    let mut faults: Vec<Fault> = tags
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            why(pair[0], pair[1])
                .filter(|rule| *rule != Rule::SubjectVerb)
                .map(|rule| Fault {
                    at: Span::new(index, index + 2),
                    rule,
                })
        })
        .collect();
    faults.extend(distant_disagreement(&tags));
    faults.extend(doubled_tense(sentence, &tags));
    if !register.waives(Convention::Apostrophes) {
        faults.extend(crate::token::fused(&sentence.tokens).map(|at| Fault {
            at: Span::new(at, at + 1),
            rule: Rule::Unapostrophed,
        }));
    }
    if !register.waives(Convention::Predicate) && !tags.is_empty() && clauses(&tags).is_empty() {
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
    (0..tags.len().saturating_sub(2))
        .filter(|&at| tags[at].is_finite_verb() && negator(at + 1) && settled(tags[at + 2]))
        .map(|at| Fault {
            at: Span::new(at, at + 3),
            rule: Rule::DoubledTense,
        })
        .collect()
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

/// The cheapest reading in which some word carries tense, if any word can.
fn insist_on_a_predicate(sentence: &Sentence) -> Option<Vec<Tag>> {
    (0..sentence.tokens.len())
        .map(|at| {
            read(
                sentence,
                Grammar {
                    predicate_at: Some(at),
                },
            )
        })
        .filter(|tags| !clauses(tags).is_empty())
        .min_by(|left, right| price(sentence, left).total_cmp(&price(sentence, right)))
}

/// What a reading costs, so two of them can be compared.
fn price(sentence: &Sentence, tags: &[Tag]) -> f64 {
    let plain = Grammar::default();
    let words: f64 = plain
        .evidence(sentence)
        .iter()
        .zip(tags)
        .map(|(evidence, tag)| plain.emission(&evidence.value, tag))
        .sum();
    let pairs: f64 = tags
        .windows(2)
        .map(|pair| plain.transition(&pair[0], &pair[1]))
        .sum();
    words + pairs
}

/// Agreement judged over the clause rather than the pair, so a noun inside a modifier cannot be
/// mistaken for the subject.
fn distant_disagreement(tags: &[Tag]) -> Vec<Fault> {
    clauses(tags)
        .into_iter()
        .filter(|&(at, verb)| why(tags[at], tags[verb]).is_some())
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
