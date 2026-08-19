//! What is wrong with a sentence, and what could not be judged.
//!
//! A fault is only ever reported with the rule that produced it. Words the lexicon does not know
//! are listed separately, because a sentence resting on them was not checked, only guessed at.

use fitkit::core::Span;
use fitkit::fit::recover;
use fitkit::Reported;

use crate::grammar::{
    disagrees, doubles, excused, is_imperative, stranded, subjectless, unmet, why, Grammar, Rule,
    Sentence, State,
};
use crate::lexicon::Lexicon;
use crate::register::{Convention, Register};
use crate::style::Note;
use crate::tag::{Break, Tag};
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
    judge(&Reading::of_in(sentence, register), register)
}

/// A unit read under each shape it could have.
///
/// Whether a unit is a sentence or a phrase cannot be settled by reading it one way and then
/// asking what was wrong, because a reading charged for a missing verb supplies one and leaves no
/// evidence that it was ever missing. Both readings have to exist before either can be preferred,
/// and what prefers one is the passage: a file whose lines are nearly all summary phrases pays
/// once to say so, and a lone verbless line in a paragraph does not earn that.
#[derive(Clone, Debug)]
pub struct Shapes {
    /// The unit read as a sentence, which is asked for a predicate.
    sentence: Reading,
    /// The unit read as a phrase, which is not.
    phrase: Reading,
}

impl Shapes {
    /// Read `unit` under both shapes.
    #[must_use]
    pub fn of(unit: &Sentence) -> Self {
        Self {
            sentence: Reading::of_in(unit, Register::STRICT),
            phrase: Reading::of_in(unit, Register::STRICT.without(Convention::Predicate)),
        }
    }

    /// The reading `register` asks for.
    #[must_use]
    pub fn under(&self, register: Register) -> &Reading {
        if register.waives(Convention::Predicate) {
            &self.phrase
        } else {
            &self.sentence
        }
    }
}

/// A sentence as the engine read it.
///
/// The reading depends on the register in one respect only: whether a unit is allowed to be a
/// phrase. Everything else a register decides is decided about a finished reading, because which
/// faults are held against a passage is not a question about what its words are. Whether a verb
/// is expected at all is a different kind of question, and one the search has to be told the
/// answer to, since a search charged for a missing verb will find one somewhere.
#[derive(Clone, Debug)]
pub struct Reading {
    /// The sentence with its contractions mended, which is what was read.
    sentence: Sentence,
    /// The tag and clause context chosen for each token.
    states: Vec<State>,
    /// Tokens the lexicon could not place.
    unknown: Vec<usize>,
    /// Tokens spelled as a contraction without its apostrophe.
    fused: Vec<usize>,
}

impl Reading {
    /// Read `sentence` once, as a sentence.
    #[must_use]
    pub fn of(sentence: &Sentence) -> Self {
        Self::of_in(sentence, Register::STRICT)
    }

    /// Read `sentence` once under `register`.
    #[must_use]
    pub fn of_in(sentence: &Sentence, register: Register) -> Self {
        // A contraction is always read with its apostrophe put back, whatever the register.
        // Spelling must never blind the reading: leaving "dont" whole would hide the disagreement
        // in "the train dont move" behind a word the lexicon cannot place. Whether the spelling
        // itself is a fault is a separate question, settled by the register.
        let fused: Vec<usize> = crate::token::fused(&sentence.tokens).collect();
        let mended = Sentence {
            tokens: crate::token::mend(&sentence.tokens),
        };
        let unknown: Vec<usize> = mended
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                // A quoted term is a name the writer supplied, and it is placed as one. Holding
                // it against the lexicon asks whether English lists a thing somebody just named,
                // which it never will, and every named term would count as a word nobody can
                // place.
                !token.mention
                    && matches!(
                        fitkit::ask(&Lexicon, token),
                        Ok(Reported::Unreported) | Err(_)
                    )
            })
            .map(|(index, _)| index)
            .collect();
        let states = states(
            &mended,
            Grammar {
                phrasal: register.waives(Convention::Predicate),
                ..Grammar::default()
            },
        );
        Self {
            sentence: mended,
            states,
            unknown,
            fused,
        }
    }

    /// The tag chosen for each token.
    #[must_use]
    pub fn tags(&self) -> Vec<Tag> {
        self.states.iter().map(|state| state.tag).collect()
    }
}

/// Hold a reading to a register.
#[must_use]
pub fn judge(reading: &Reading, register: Register) -> Report {
    let tags = reading.tags();
    let sentence = &reading.sentence;
    let mut faults: Vec<Fault> = reading
        .states
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            why(pair[0].tag, pair[1].tag)
                .filter(|rule| !matches!(rule, Rule::SubjectVerb | Rule::DoubledTense))
                .filter(|rule| !excused(pair[0].frame, *rule))
                .map(|rule| Fault {
                    at: Span::new(index, index + 2),
                    rule,
                })
        })
        .collect();
    faults.extend(broken_agreement(&reading.states));
    if !register.waives(Convention::Apostrophes) {
        faults.extend(reading.fused.iter().map(|at| Fault {
            at: Span::new(*at, at + 1),
            rule: Rule::Unapostrophed,
        }));
    }
    if !register.waives(Convention::Predicate)
        && !tags.is_empty()
        && !is_imperative(&tags)
        && reading
            .states
            .last()
            .is_some_and(|state| !state.frame.tensed && !state.frame.ever && !state.frame.open())
    {
        faults.push(Fault {
            at: Span::new(0, tags.len()),
            rule: Rule::NoPredicate,
        });
    }
    if let Some(rule) = reading
        .states
        .last()
        .and_then(|state| unmet(state.frame, Tag::Mark(Break::Stop)))
    {
        faults.push(Fault {
            at: Span::new(tags.len().saturating_sub(1), tags.len()),
            rule,
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
        unknown: reading.unknown.clone(),
        notes,
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

/// Where the reading breaks agreement or gives a settled clause a second tensed verb.
///
/// Both are read straight off the frames the search already committed to. There is no second pass
/// over the sentence looking for a subject, because the subject was carried the whole way.
fn broken_agreement(reading: &[State]) -> Vec<Fault> {
    reading
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let at = Span::new(index, index + 2);
            if disagrees(pair[0].frame, pair[1].tag) {
                Some(Fault {
                    at,
                    rule: Rule::SubjectVerb,
                })
            } else if doubles(pair[0].frame, pair[1].tag) {
                Some(Fault {
                    at,
                    rule: Rule::DoubledTense,
                })
            } else if subjectless(pair[0].frame, pair[1].tag) {
                Some(Fault {
                    at,
                    rule: Rule::Subjectless,
                })
            } else if stranded(pair[0].frame, pair[1].tag) {
                Some(Fault {
                    at,
                    rule: Rule::StrandedParticiple,
                })
            } else {
                unmet(pair[0].frame, pair[1].tag).map(|rule| Fault { at, rule })
            }
        })
        .collect()
}

/// The state chosen for each token under `grammar`, tag and clause context together.
///
/// A reading the search refuses is no reading at all, so nothing is judged from it. That leaves
/// the sentence unchecked rather than checked and found clean, which is the honest outcome.
fn states(sentence: &Sentence, grammar: Grammar) -> Vec<State> {
    let Ok(chosen) = recover(&grammar, sentence) else {
        return Vec::new();
    };
    chosen
        .get()
        .controls
        .iter()
        .map(|control| control.params)
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
