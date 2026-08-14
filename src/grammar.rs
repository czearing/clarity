//! Grammar as a cost. What English forbids costs infinity, what it merely disfavours costs a
//! little, and the search returns the cheapest reading.
//!
//! Every infinite transition names the rule that forbids it, so a rejection can always be
//! explained rather than asserted. See [`why`].

use fitkit::ask;
use fitkit::core::{Confidence, Evidence, Reported, Span};
use fitkit::fit::{Fit, Model, Segmented};

use crate::lexicon::Lexicon;
use crate::tag::{Form, Number, Person, Tag};
use crate::token::Token;

/// A rule that makes a pair of neighbouring tags impossible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A determiner and its noun must share number, as in "a dog" but not "a dogs".
    DeterminerNumber,
    /// A determiner must be followed by something that can head or modify a noun phrase.
    DeterminerTarget,
    /// A tensed verb must agree with the subject beside it.
    SubjectVerb,
    /// A modal takes the plain form, as in "can walk" but not "can walks".
    ModalTakesBase,
    /// Infinitival "to" takes the plain form.
    ToTakesBase,
    /// A preposition takes a noun phrase, not a tensed verb.
    PrepositionTarget,
    /// Two tensed verbs cannot sit side by side.
    DoubledTense,
    /// A participle needs an auxiliary before it.
    StrandedParticiple,
    /// A pronoun is a whole noun phrase, so no noun may be attached to it. A determiner after one
    /// is fine, since it opens a phrase of its own, as in "she gives him the book".
    PronounIsWhole,
    /// A noun used to modify another noun takes the singular, as in "dog books".
    AttributiveSingular,
    /// A sentence needs a tensed verb.
    NoPredicate,
    /// A sentence opens with a capital and closes with a mark.
    Unmarked,
    /// A contraction keeps its apostrophe.
    Unapostrophed,
}

impl Rule {
    /// What the rule requires, in one line.
    #[must_use]
    pub fn says(self) -> &'static str {
        match self {
            Self::DeterminerNumber => "a determiner and its noun must agree in number",
            Self::DeterminerTarget => "a determiner must introduce a noun phrase",
            Self::SubjectVerb => "a tensed verb must agree with its subject",
            Self::ModalTakesBase => "a modal is followed by the plain form of a verb",
            Self::ToTakesBase => "infinitival to is followed by the plain form of a verb",
            Self::PrepositionTarget => "a preposition is followed by a noun phrase",
            Self::DoubledTense => "two tensed verbs cannot be adjacent",
            Self::StrandedParticiple => "a participle needs an auxiliary before it",
            Self::PronounIsWhole => "a pronoun cannot have a noun attached to it",
            Self::AttributiveSingular => "a noun modifying another noun takes the singular",
            Self::NoPredicate => "a sentence needs a tensed verb",
            Self::Unmarked => "a sentence opens with a capital and closes with a mark",
            Self::Unapostrophed => "a contraction is spelled with an apostrophe",
        }
    }
}

/// The rule that forbids `from` followed by `to`, if any forbids it.
#[must_use]
pub fn why(from: Tag, to: Tag) -> Option<Rule> {
    match (from, to) {
        (Tag::Determiner(had), Tag::Noun(wants) | Tag::Proper(wants)) if had != wants => {
            Some(Rule::DeterminerNumber)
        }
        (Tag::Determiner(_), Tag::Verb(form)) if form != Form::Gerund => {
            Some(Rule::DeterminerTarget)
        }
        (Tag::Determiner(_), Tag::Modal | Tag::Preposition | Tag::Mark | Tag::To) => {
            Some(Rule::DeterminerTarget)
        }
        (Tag::Modal, next) if !leads_to_verb(next) => Some(Rule::ModalTakesBase),
        (Tag::To, next) if !leads_to_verb(next) => Some(Rule::ToTakesBase),
        (Tag::Preposition, Tag::Modal) => Some(Rule::PrepositionTarget),
        (Tag::Preposition, Tag::Verb(form)) if form != Form::Gerund => {
            Some(Rule::PrepositionTarget)
        }
        (subject, Tag::Verb(form))
            if subject.is_nominal() && is_tensed(form) && !agrees(subject, form) =>
        {
            Some(Rule::SubjectVerb)
        }
        (Tag::Pronoun(..), Tag::Noun(_) | Tag::Proper(_)) => Some(Rule::PronounIsWhole),
        (Tag::Noun(Number::Plural), Tag::Noun(_) | Tag::Proper(_)) => {
            Some(Rule::AttributiveSingular)
        }
        (Tag::Verb(before), Tag::Verb(after)) if is_tensed(before) && is_tensed(after) => {
            Some(Rule::DoubledTense)
        }
        (before, Tag::Verb(Form::Participle))
            if !matches!(before, Tag::Verb(_) | Tag::Modal | Tag::Adverb) =>
        {
            Some(Rule::StrandedParticiple)
        }
        _ => None,
    }
}

/// Whether a tag can stand between a modal or infinitival "to" and the plain verb it governs.
fn leads_to_verb(tag: Tag) -> bool {
    matches!(tag, Tag::Verb(Form::Base) | Tag::Adverb)
}

/// Whether a form carries tense, and so must agree with a subject.
#[must_use]
pub fn is_tensed(form: Form) -> bool {
    matches!(
        form,
        Form::Base | Form::ThirdSingular | Form::Past | Form::PastSingular | Form::PastPlural
    )
}

/// Whether `subject` can take a verb in `form`.
///
/// Plain [`Form::Past`] agrees with everything, which is true of every English verb but "be".
fn agrees(subject: Tag, form: Form) -> bool {
    if form == Form::Past {
        return true;
    }
    let singular = subject.number() == Some(Number::Singular);
    let third_singular = match subject {
        Tag::Pronoun(person, Number::Singular, _) => person == Person::Third,
        _ => singular,
    };
    match form {
        Form::ThirdSingular => third_singular,
        Form::Base => !third_singular,
        Form::PastSingular => singular && !matches!(subject, Tag::Pronoun(Person::Second, ..)),
        Form::PastPlural => !singular || matches!(subject, Tag::Pronoun(Person::Second, ..)),
        _ => true,
    }
}

/// What a broken rule costs. Far above any sum of frictions a sentence can accumulate, so a
/// reading breaks a rule only when every reading does.
const BREACH: f64 = 1000.0;

/// What a local disagreement costs while reading. Only a pull, not a breach, because the noun
/// beside a verb is often not its subject, as in "the key to the cabinets is missing". Agreement
/// is judged over the whole clause instead, once the subject is known.
const PULL: f64 = 3.0;

/// What each step down a word's list of readings costs. Small enough to never outweigh a rule,
/// large enough to settle a tie in favour of the commoner reading.
const PREFERENCE: f64 = 0.05;

/// Pairs that are legal but unusual, priced so a plainer reading wins a tie.
fn friction(from: Tag, to: Tag) -> f64 {
    match (from, to) {
        (Tag::Determiner(_), Tag::Noun(_) | Tag::Adjective | Tag::Numeral)
        | (Tag::Adjective, Tag::Noun(_))
        | (Tag::Adverb, Tag::Adjective | Tag::Verb(_))
        | (Tag::Preposition, Tag::Determiner(_) | Tag::Noun(_) | Tag::Proper(_)) => 0.0,
        (subject, Tag::Verb(_)) if subject.is_nominal() => 0.0,
        (Tag::Noun(_), Tag::Noun(_)) => 0.5,
        (Tag::Proper(_), Tag::Noun(_)) => 1.0,
        _ => 0.25,
    }
}

/// Whether a verb licenses a to-infinitive after it.
fn takes_infinitive(token: &Token) -> bool {
    crate::lexicon::TAKES_INFINITIVE.contains(&token.key.as_str())
}

/// Each tensed verb paired with the head of its subject.
///
/// The head is the first nominal of the phrase, not the nearest one, so a modifier between the
/// two is stepped over. This is what makes "the key to the cabinets is missing" agree with "key".
/// A pair is emitted only when a subject was found before the tensed word. A modal is that word
/// where one is present, since it carries the tense and the verb after it agrees with nothing.
#[must_use]
pub fn clauses(tags: &[Tag]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut head: Option<usize> = None;
    let mut modifying = false;
    for (index, &tag) in tags.iter().enumerate() {
        match tag {
            Tag::Preposition | Tag::Subordinator => modifying = head.is_some(),
            Tag::Verb(form) if is_tensed(form) => {
                if let Some(at) = head.take() {
                    found.push((at, index));
                }
                modifying = false;
            }
            Tag::Modal => {
                if let Some(at) = head.take() {
                    found.push((at, index));
                }
                modifying = false;
            }
            Tag::Mark | Tag::Coordinator => {
                head = None;
                modifying = false;
            }
            _ if tag.is_nominal() && !modifying => head = Some(index),
            _ => {}
        }
    }
    found
}

/// A tokenised sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sentence {
    /// Tokens in order.
    pub tokens: Vec<Token>,
}

impl Sentence {
    /// Tokenise `text`.
    #[must_use]
    pub fn read(text: &str) -> Self {
        Self {
            tokens: crate::token::tokenise(text),
        }
    }
}

impl Segmented for Sentence {
    fn extent(&self) -> usize {
        self.tokens.len()
    }

    fn slice(&self, span: Span) -> Self {
        Self {
            tokens: self.tokens[span.start..span.end.min(self.tokens.len())].to_vec(),
        }
    }

    fn splice(&mut self, span: Span, part: Self) {
        self.tokens
            .splice(span.start..span.end.min(self.tokens.len()), part.tokens);
    }
}

/// What one word permits, and where it sits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The tags the lexicon allows, or nothing when the word is unknown.
    pub allowed: Reported<Vec<Tag>>,
    /// Position in the sentence.
    pub at: usize,
}

/// The grammar of English as a fit over tag sequences.
///
/// A broken rule is priced, not forbidden, so every sentence gets a reading and the rules it
/// breaks are the ones the cheapest reading still pays for. A word is never given a category its
/// entry forbids, which is the one thing held absolute.
#[derive(Clone, Copy, Debug, Default)]
pub struct Grammar {
    /// A position that must be read as a tensed verb, used to insist on a predicate.
    pub predicate_at: Option<usize>,
}

impl Model for Grammar {
    type Signal = Sentence;
    type Params = Tag;

    fn name(&self) -> &'static str {
        "english grammar"
    }

    fn candidates(&self) -> Vec<Tag> {
        Tag::every()
    }

    fn render(&self, input: &Sentence, _params: &Tag) -> Sentence {
        input.clone()
    }
}

impl Fit for Grammar {
    type Evidence = Reading;

    fn evidence(&self, reference: &Sentence) -> Vec<Evidence<Reading>> {
        reference
            .tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                // A capital at the start of a unit is required of every word, so it says nothing.
                let opening;
                let token = if index == 0 && token.capitalised {
                    opening = Token {
                        capitalised: false,
                        ..token.clone()
                    };
                    &opening
                } else {
                    token
                };
                let mut allowed = ask(&Lexicon, token).unwrap_or(Reported::Unreported);
                if token.key == "to" && index > 0 && takes_infinitive(&reference.tokens[index - 1])
                {
                    allowed = Reported::Known(vec![Tag::To]);
                }
                let confidence = if matches!(allowed, Reported::Known(_)) {
                    Confidence::FULL
                } else {
                    Confidence::ZERO
                };
                let reading = Reading { allowed, at: index };
                Evidence::new(Span::new(index, index + 1), confidence, reading)
            })
            .collect()
    }

    fn emission(&self, evidence: &Reading, params: &Tag) -> f64 {
        if self.predicate_at == Some(evidence.at) && !params.is_finite_verb() {
            return f64::INFINITY;
        }
        match &evidence.allowed {
            Reported::Unreported => 0.0,
            Reported::Known(tags) => tags
                .iter()
                .position(|tag| tag == params)
                .map_or(f64::INFINITY, |rank| {
                    PREFERENCE * f64::from(u32::try_from(rank).unwrap_or(u32::MAX))
                }),
        }
    }

    fn transition(&self, from: &Tag, to: &Tag) -> f64 {
        match why(*from, *to) {
            Some(Rule::SubjectVerb) => PULL,
            Some(_) => BREACH,
            None => friction(*from, *to),
        }
    }
}

#[cfg(test)]
mod tests {
    use fitkit::fit::recover;

    use super::{why, Grammar, Rule, Sentence};
    use crate::tag::{Form, Number, Tag};

    fn tags(text: &str) -> Vec<Tag> {
        recover(&Grammar::default(), &Sentence::read(text))
            .controls
            .iter()
            .map(|c| c.params)
            .collect()
    }

    #[test]
    fn context_decides_between_a_plural_noun_and_a_present_tense_verb() {
        let sentence = tags("the dog runs");
        assert_eq!(sentence[2], Tag::Verb(Form::ThirdSingular));
        let sentence = tags("the dogs run");
        assert_eq!(sentence[1], Tag::Noun(Number::Plural));
    }

    #[test]
    fn a_disagreeing_determiner_names_the_rule_that_forbids_it() {
        assert_eq!(
            why(Tag::Determiner(Number::Singular), Tag::Noun(Number::Plural)),
            Some(Rule::DeterminerNumber)
        );
    }

    #[test]
    fn a_modal_refuses_an_inflected_verb() {
        assert_eq!(
            why(Tag::Modal, Tag::Verb(Form::ThirdSingular)),
            Some(Rule::ModalTakesBase)
        );
        assert_eq!(why(Tag::Modal, Tag::Verb(Form::Base)), None);
    }
}
