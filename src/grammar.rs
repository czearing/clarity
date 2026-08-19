//! Grammar as a cost. What English forbids costs infinity, what it merely disfavours costs a
//! little, and the search returns the cheapest reading.
//!
//! Every infinite transition names the rule that forbids it, so a rejection can always be
//! explained rather than asserted. See [`why`].

use std::sync::OnceLock;

use fitkit::ask;
use fitkit::core::{Confidence, Cost, Evidence, Reported, Scale, Span};
use fitkit::fit::{Fit, Model, Segmented};

use crate::frame::{Frame, Subject, Wants};
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
    /// A tensed verb outside a command needs a subject.
    Subjectless,
    /// A sentence needs a tensed verb.
    NoPredicate,
    /// A sentence opens with a capital and closes with a mark.
    Unmarked,
    /// A contraction keeps its apostrophe.
    Unapostrophed,
}

impl Rule {
    /// What a step costs where this rule is broken.
    ///
    /// The rule names itself in the cost, so a rule charged by the frame and again by a pairwise
    /// check within one step is paid for once. That is what makes it safe to move a rule into the
    /// state without hunting down the charge it used to have.
    #[must_use]
    pub fn breach(self) -> Cost {
        Cost::breach(self as u32)
    }

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
            Self::Subjectless => "a tensed verb needs a subject",
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
        (Tag::Determiner(_), Tag::Verb(form)) if !modifies_a_noun(form) => {
            Some(Rule::DeterminerTarget)
        }
        (Tag::Determiner(_), Tag::Modal | Tag::Preposition | Tag::Mark(_) | Tag::To) => {
            Some(Rule::DeterminerTarget)
        }
        (Tag::Preposition, Tag::Modal) => Some(Rule::PrepositionTarget),
        (Tag::Preposition, Tag::Verb(form)) if form != Form::Gerund => {
            Some(Rule::PrepositionTarget)
        }
        (subject, Tag::Verb(form))
            if subject.is_nominal() && is_tensed(form) && !agrees(subject, form) =>
        {
            Some(Rule::SubjectVerb)
        }
        (Tag::Verb(before), Tag::Verb(after))
            if is_tensed(before)
                && matches!(
                    after,
                    Form::ThirdSingular | Form::PastSingular | Form::PastPlural
                ) =>
        {
            Some(Rule::DoubledTense)
        }
        (Tag::Pronoun(..), Tag::Noun(_) | Tag::Proper(_)) => Some(Rule::PronounIsWhole),
        (Tag::Noun(Number::Plural), Tag::Noun(_) | Tag::Proper(_)) => {
            Some(Rule::AttributiveSingular)
        }
        _ => None,
    }
}

/// Whether a verb form can stand between a determiner and the noun, as in "a tensed verb".
fn modifies_a_noun(form: Form) -> bool {
    matches!(form, Form::Gerund | Form::Participle | Form::Past)
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

/// The longest sentence this engine will read, in words.
const LONGEST: usize = 4096;

/// The most one word can stray from its plainest reading.
const CEILING: f64 = 8.0;

/// How a step's cost becomes the number the search minimises.
///
/// The gap between one broken rule and the next is wider than every friction the longest sentence
/// can accumulate, so a reading breaks a rule only when every reading does. The framework derives
/// that gap from the two numbers above rather than leaving it to be chosen by eye.
fn scale() -> Scale {
    Scale::over(LONGEST, CEILING)
}

/// What each step down a word's list of readings costs. Small enough to never outweigh a rule,
/// large enough to settle a tie in favour of the commoner reading.
const PREFERENCE: f64 = 0.05;

/// Pairs that are legal but unusual, priced so a plainer reading wins a tie.
fn friction(from: Tag, to: Tag) -> Cost {
    match (from, to) {
        (Tag::Determiner(_), Tag::Noun(_) | Tag::Adjective | Tag::Numeral)
        | (Tag::Adjective, Tag::Noun(_))
        | (Tag::Adverb, Tag::Adjective | Tag::Verb(_))
        | (Tag::Preposition, Tag::Determiner(_) | Tag::Noun(_) | Tag::Proper(_)) => Cost::FREE,
        (subject, Tag::Verb(_)) if subject.is_nominal() => Cost::FREE,
        (Tag::Noun(_), Tag::Noun(_)) => Cost::friction(0.5),
        (Tag::Proper(_), Tag::Noun(_)) => Cost::friction(1.0),
        _ => Cost::friction(0.25),
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
    // Set once the clause has its verb. A noun after that is an object, and a plain verb after
    // that is a complement, as "go" is in "this lets the matter go". Neither starts a clause, and
    // treating them as though they did invents disagreements the writer never wrote.
    let mut settled = false;
    for (index, &tag) in tags.iter().enumerate() {
        match tag {
            Tag::Preposition | Tag::Subordinator => {
                modifying = head.is_some();
                settled = false;
            }
            Tag::Verb(form) if is_tensed(form) && !settled => {
                if let Some(at) = head.take() {
                    found.push((at, index));
                    settled = true;
                }
                modifying = false;
            }
            Tag::Modal if !settled => {
                if let Some(at) = head.take() {
                    found.push((at, index));
                    settled = true;
                }
                modifying = false;
            }
            Tag::Mark(_) | Tag::Coordinator(_) => {
                head = None;
                modifying = false;
                settled = false;
            }
            _ if tag.is_nominal() && !modifying && !settled => head = Some(index),
            _ => {}
        }
    }
    found
}

/// Whether a reading opens with a plain verb, which makes it a command.
///
/// A command has no subject on the page: "check the file" is a sentence, and asking it for one
/// would be asking for a word English leaves out on purpose.
#[must_use]
pub fn is_imperative(tags: &[Tag]) -> bool {
    let mut rest = tags.iter();
    let mut first = rest.next();
    while first == Some(&Tag::Adverb) {
        first = rest.next();
    }
    first == Some(&Tag::Verb(Form::Base)) && tags.len() > 1
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

    /// Write the sentence back out.
    ///
    /// Words are joined by a single space, except before a mark or an ending that is a word of its
    /// own, so a repaired sentence reads as one.
    #[must_use]
    pub fn text(&self) -> String {
        let mut text = String::new();
        for token in &self.tokens {
            // A clitic joins the word before it, a mark joins the word before it, and a named
            // term joins nothing: "n't" written as a term is two inches of quoted text and needs
            // its space, however the same letters behave when they are an ending.
            let joined = !token.mention
                && (token.key.starts_with('\'')
                    || token.key == "n't"
                    || matches!(token.key.as_str(), "." | "," | "!" | "?" | ";" | ":"));
            if !text.is_empty() && !joined {
                text.push(' ');
            }
            text.push_str(&token.word);
        }
        text
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
    /// Whether this is the last word, which is where a sentence is asked for its predicate.
    pub last: bool,
}

/// The grammar of English as a fit over tag sequences.
///
/// A broken rule is priced, not forbidden, so every sentence gets a reading and the rules it
/// breaks are the ones the cheapest reading still pays for. A word is never given a category its
/// entry forbids, which is the one thing held absolute.
#[derive(Clone, Copy, Debug, Default)]
pub struct Grammar {
    /// Whether a unit may be a phrase rather than a sentence.
    ///
    /// A register that waives the predicate convention has to waive it inside the search, not
    /// after it. Charging a phrase for the verb it does not have makes the reading that invents
    /// one cheaper, and the invented verb then disagrees with something, so suppressing the
    /// missing-predicate fault afterwards leaves its consequences behind and reports those
    /// instead. A heading, a summary line, a list item and a table cell are noun phrases by
    /// convention, and under that convention nothing here should go looking for a verb.
    pub phrasal: bool,
    /// A position that must be read as a tensed verb, used to insist on a predicate.
    pub predicate_at: Option<usize>,
    /// Whether the first word must be read as a plain verb, used to test for a command.
    pub command: bool,
}

/// A tag together with what the clause around it has seen.
///
/// This is what the search chooses. Reading it as a tag alone was the mistake that made
/// agreement, predicates, and doubled tense into things that had to be looked for afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct State {
    /// What the word is.
    pub tag: Tag,
    /// What the clause has seen, once this word is part of it.
    pub frame: Frame,
}

impl Model for Grammar {
    type Signal = Sentence;
    type Params = State;

    fn name(&self) -> &'static str {
        "english grammar"
    }

    fn candidates(&self) -> Vec<State> {
        // A state is a frame and a tag, and which pairs exist is settled by the grammar rather
        // than by the sentence. Walking them out again for every sentence would rebuild the same
        // hundred thousand states each time, so they are walked once and handed out.
        static EVERY: OnceLock<Vec<State>> = OnceLock::new();
        EVERY
            .get_or_init(|| {
                let mut found = Vec::new();
                for &frame in Frame::every() {
                    for &tag in Tag::every() {
                        found.push(State { tag, frame });
                    }
                }
                found
            })
            .clone()
    }

    fn render(&self, input: &Sentence, _params: &State) -> Sentence {
        input.clone()
    }
}

/// Whether a subordinator has already opened a clause among these tokens.
///
/// What is asked is the lexicon's own answer, not a list of words, so a subordinator it learns
/// later licenses the mood without anything here changing.
fn subordinated(before: &[Token]) -> bool {
    before.iter().any(|token| {
        matches!(ask(&Lexicon, token), Ok(Reported::Known(tags)) if tags.contains(&Tag::Subordinator))
    })
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
                if token.mention {
                    allowed = Reported::Known(vec![Tag::Proper(Number::Singular)]);
                }
                // A command reading is offered only where the lexicon already allows a plain
                // verb. Forcing one on a word that cannot be one would read "one broken rule" as
                // an instruction to one something.
                if self.command && index == 0 {
                    if let Reported::Known(tags) = &allowed {
                        if tags.contains(&Tag::Verb(Form::Base)) {
                            allowed = Reported::Known(vec![Tag::Verb(Form::Base)]);
                        }
                    }
                }
                // The past subjunctive is spelled like the past plural and demands no number of
                // its subject: "as though it were" is not a disagreement but the one mood English
                // still inflects for. Only "be" distinguishes number in the preterite, so the form
                // that can carry the subjunctive is exactly the one the lexicon reports as plural
                // and no word has to be named here. It is offered only inside a clause a
                // subordinator opened, because that is what licenses the mood, and it is offered
                // last, so the plain plural reading is still what "the dogs were fast" gets.
                if let Reported::Known(tags) = &allowed {
                    if tags.contains(&Tag::Verb(Form::PastPlural))
                        && subordinated(&reference.tokens[..index])
                    {
                        let mut widened = tags.clone();
                        widened.push(Tag::Verb(Form::Past));
                        allowed = Reported::Known(widened);
                    }
                }
                if token.key == "to" && index > 0 && takes_infinitive(&reference.tokens[index - 1])
                {
                    allowed = Reported::Known(vec![Tag::To]);
                }
                let confidence = if matches!(allowed, Reported::Known(_)) {
                    Confidence::FULL
                } else {
                    Confidence::ZERO
                };
                let reading = Reading {
                    allowed,
                    at: index,
                    last: index + 1 == reference.tokens.len(),
                };
                Evidence::new(Span::new(index, index + 1), confidence, reading)
            })
            .collect()
    }

    fn emission(&self, evidence: &Reading, params: &State) -> f64 {
        // A frame is what the clause has seen, so at the first word it can only be what one word
        // makes of an empty clause. Every other frame there describes a history that did not happen.
        if evidence.at == 0 && params.frame != Frame::opening().after(params.tag) {
            return f64::INFINITY;
        }
        // A sentence is asked for its predicate here rather than by a second reading afterwards.
        // Charging for a missing verb inside the search is what makes the search look for one: the
        // reading that turns a noun into a verb to supply it is now the cheaper reading, and the
        // reading that quietly leaves the sentence verbless has to pay.
        // Worse than any single broken rule, so that a sentence with a fault in it is still read as
        // the sentence it was trying to be. Read "the dog run" as a subject that disagrees with its
        // verb, not as a heap of three nouns with nothing said about them.
        let bare = if evidence.last {
            // A tensed verb with nothing to answer to is as broken as no verb at all, and charging
            // for it is what stops an unknown word being pressed into service as the verb a
            // fragment does not have.
            let stands =
                (params.frame.tensed || params.frame.ever) && params.frame.subject != Subject::None;
            // A unit allowed to be a phrase has to be one phrase. Waiving the predicate without
            // asking for anything in its place lets "the dog run fast" read as a determiner and
            // three words with no fault anywhere, which is worse than the fault it was waiving:
            // a register meant to excuse headings would go on to excuse every disagreement in the
            // file. What a heading is instead asked for is that it hold together, so a second
            // noun phrase begun with nothing linking it to the first is the same breach it would
            // have been in a sentence.
            let clause = if self.phrasal {
                if params.frame.outer.is_none() && params.frame.subject != Subject::None {
                    Cost::FREE
                } else {
                    Rule::NoPredicate.breach()
                }
            } else if stands {
                Cost::FREE
            } else {
                Rule::NoPredicate.breach()
            };
            // A demand the sentence never answered is charged here for the same reason: a modal
            // left waiting is not free merely because the sentence stopped before the verb came.
            let owed = match params.frame.wants {
                Wants::Nothing => Cost::FREE,
                Wants::BaseForModal => Rule::ModalTakesBase.breach(),
                Wants::BaseForTo => Rule::ToTakesBase.breach(),
            };
            // A sentence that ends inside a subordinate clause is answerable for both of them, so
            // a clause opened and never given a verb cannot be a free place to hide one. It is the
            // same rule the outer clause would break, and naming it says so: a sentence with no
            // predicate anywhere is one fault, not two.
            let held = match params.frame.outer {
                Some((_, false)) => Rule::NoPredicate.breach(),
                _ => Cost::FREE,
            };
            clause + held + owed
        } else {
            Cost::FREE
        };
        if self.predicate_at == Some(evidence.at) && !params.tag.is_finite_verb() {
            return f64::INFINITY;
        }
        let ranked = match &evidence.allowed {
            Reported::Unreported => Some(Cost::FREE),
            Reported::Known(tags) => tags.iter().position(|tag| *tag == params.tag).map(|rank| {
                Cost::friction(
                    (PREFERENCE * f64::from(u32::try_from(rank).unwrap_or(u32::MAX))).min(CEILING),
                )
            }),
        };
        ranked.map_or(f64::INFINITY, |ranked| scale().price(ranked + bare))
    }

    fn transition(&self, from: &State, to: &State) -> f64 {
        // The frame after a word is decided by the frame before it and what the word was read as.
        // Any other pairing is not a costly reading of the sentence, it is not a reading at all.
        if to.frame != from.frame.after(to.tag) {
            return f64::INFINITY;
        }
        // Agreement is now local, because the subject is carried rather than searched for. This is
        // what makes "the key to the cabinets is missing" agree with "key" without any rule about
        // stepping over modifiers, and what makes a second tensed verb in a settled clause cost
        // something at the moment it is chosen.
        let structural = match why(from.tag, to.tag) {
            // Agreement and doubled tense are priced by the frame now, which sees the actual
            // subject and the actual clause rather than whatever happened to be the word before.
            // Two adjacent verbs are not two verbs of one clause, so charging them here would fine
            // "a word the lexicon cannot place is refused", where they belong to two clauses. It
            // is a wrong charge rather than a second one: a cost names the rule it charges, so the
            // frame and this arm charging the same rule at the same step would cost once.
            Some(Rule::SubjectVerb | Rule::DoubledTense) => Cost::FREE,
            Some(rule) if excused(from.frame, rule) => Cost::FREE,
            Some(rule) => rule.breach(),
            None => friction(from.tag, to.tag),
        };
        let owed = unmet(from.frame, to.tag).map_or(Cost::FREE, Rule::breach);
        let charged = structural
            + owed
            + if disagrees(from.frame, to.tag) {
                Rule::SubjectVerb.breach()
            } else {
                Cost::FREE
            }
            + if doubles(from.frame, to.tag) {
                Rule::DoubledTense.breach()
            } else {
                Cost::FREE
            }
            + if subjectless(from.frame, to.tag) {
                Rule::Subjectless.breach()
            } else {
                Cost::FREE
            }
            + if stranded(from.frame, to.tag) {
                Rule::StrandedParticiple.breach()
            } else {
                Cost::FREE
            };
        scale().price(charged)
    }

    fn apart(&self, at: usize, _state: &State) -> u64 {
        // A state is a tag together with the clause the tag leaves behind. Only the tag is
        // reported, so two states that agree about the tag are one answer however differently
        // they read the clause around it.
        (at % Tag::every().len()) as u64
    }

    fn onward(&self, from: &State) -> Option<Vec<u32>> {
        // A frame carries what the clause is still owed, so reading one more word settles where
        // the next state's frame must be. There are as many ways onward as there are tags, and no
        // others: every state whose frame is not the one this word leaves behind is unreachable
        // from here, not merely expensive.
        let tags = Tag::every().len();
        Some(
            Tag::every()
                .iter()
                .enumerate()
                .map(|(offset, &tag)| {
                    let at = from.frame.after(tag).at() * tags + offset;
                    u32::try_from(at).unwrap_or(0)
                })
                .collect(),
        )
    }
}

/// Whether a rule a pair of tags breaks is answered for by the clause they sit in.
///
/// A preposition normally takes a noun phrase, but a clause held inside another may have had that
/// noun phrase fronted, which leaves the preposition with nothing after it: "whatever it still
/// pays for is wrong", "the conventions a passage holds to". Nothing is missing there, the object
/// is at the front of the clause, and the only thing that says so is that a clause is open.
#[must_use]
pub fn excused(frame: Frame, rule: Rule) -> bool {
    rule == Rule::PrepositionTarget && frame.open()
}

/// Whether a tensed verb fails to agree with the subject its clause is carrying.
///
/// The frame has already done the hard part. Because the subject travels with the clause rather
/// than being searched for backwards, this holds equally for `the dog runs` and for `the key to the
/// cabinets is missing`, and there is no rule about stepping over the words in between.
#[must_use]
pub fn disagrees(frame: Frame, tag: Tag) -> bool {
    // A verb answering a demand is not the verb the subject agrees with. "She can walk" has its
    // agreement settled by "can", and asking "walk" to agree as well would fault every modal that
    // kept the subject it agreed with.
    if frame.wants != Wants::Nothing {
        return false;
    }
    matches!(
        (frame.answering(), tag),
        (
            Subject::Third | Subject::Activity,
            Tag::Verb(Form::Base | Form::PastPlural)
        ) | (
            Subject::First,
            Tag::Verb(Form::ThirdSingular | Form::PastPlural)
        ) | (
            Subject::Other,
            Tag::Verb(Form::ThirdSingular | Form::PastSingular)
        )
    )
}

/// Whether a demand for a plain verb is being walked past rather than answered.
///
/// This is the whole of `ModalTakesBase` and `ToTakesBase`, stated once from the side of the word
/// that needs something. Reading "moves" as a noun in "the trains do not moves" no longer escapes:
/// "do not" is still owed a verb, and a noun does not pay it.
#[must_use]
pub fn unmet(frame: Frame, tag: Tag) -> Option<Rule> {
    let rule = match frame.wants {
        Wants::Nothing => return None,
        Wants::BaseForModal => Rule::ModalTakesBase,
        Wants::BaseForTo => Rule::ToTakesBase,
    };
    if leads_to_verb(tag) || ends_a_clause(tag) {
        return None;
    }
    Some(rule)
}

/// Whether a participle has nothing to belong to.
///
/// A participle is not a verb by itself. It either follows an auxiliary, as in "was guessed", or
/// it modifies a noun phrase, as in "the tag chosen for each token", and both of those leave the
/// clause with something already in it. Only a participle in a clause that has read no subject and
/// taken no tense is stranded, so the rule is one question about the clause rather than a list of
/// the words that may precede it.
#[must_use]
pub fn stranded(frame: Frame, tag: Tag) -> bool {
    tag == Tag::Verb(Form::Participle)
        && frame.subject == Subject::Empty
        && !frame.tensed
        && frame.wants == Wants::Nothing
}

/// Whether a tag can close a clause whose verb was left out, as in "if any word can".
fn ends_a_clause(tag: Tag) -> bool {
    matches!(tag, Tag::Mark(_) | Tag::Coordinator(_) | Tag::Subordinator)
}
fn leads_to_verb(tag: Tag) -> bool {
    matches!(tag, Tag::Verb(Form::Base) | Tag::Adverb)
}

/// Whether a tensed verb is being taken with nothing to be the subject of.
///
/// Charged where it happens rather than at the end of the sentence, because a verb the sentence
/// never had a subject for is often buried in the middle: reading "keys" as a verb in "the keys to
/// the cabinet is missing" is what let that sentence dodge agreement entirely.
#[must_use]
pub fn subjectless(frame: Frame, tag: Tag) -> bool {
    frame.answering() == Subject::None
        && !frame.tensed
        && matches!(
            tag,
            Tag::Modal
                | Tag::Verb(
                    Form::Base
                        | Form::ThirdSingular
                        | Form::Past
                        | Form::PastSingular
                        | Form::PastPlural
                )
        )
}

/// Whether a second tensed verb is being added to a clause that already has one.
#[must_use]
pub fn doubles(frame: Frame, tag: Tag) -> bool {
    frame.tensed
        && frame.subject == Subject::Empty
        && !frame.open()
        && matches!(
            tag,
            Tag::Verb(Form::ThirdSingular | Form::Past | Form::PastSingular | Form::PastPlural)
        )
}

#[cfg(test)]
mod tests {
    use fitkit::fit::recover;

    use fitkit_guards::assert_friction_never_reaches_a_rule;

    use super::{
        disagrees, scale, unmet, why, Cost, Frame, Grammar, Rule, Sentence, CEILING, LONGEST,
    };
    use crate::tag::{Form, Join, Number, Tag};

    fn tags(text: &str) -> Vec<Tag> {
        recover(&Grammar::default(), &Sentence::read(text))
            .expect("a reading")
            .get()
            .controls
            .iter()
            .map(|c| c.params.tag)
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
        let after_modal = Frame::opening().after(Tag::Modal);
        assert_eq!(
            unmet(after_modal, Tag::Verb(Form::ThirdSingular)),
            Some(Rule::ModalTakesBase)
        );
        assert_eq!(unmet(after_modal, Tag::Verb(Form::Base)), None);
    }

    #[test]
    fn a_demand_survives_an_adverb_and_names_who_made_it() {
        let waiting = Frame::opening().after(Tag::To).after(Tag::Adverb);
        assert_eq!(
            unmet(waiting, Tag::Noun(Number::Singular)),
            Some(Rule::ToTakesBase)
        );
        assert_eq!(unmet(waiting, Tag::Verb(Form::Base)), None);
    }

    #[test]
    fn a_demand_that_is_answered_is_no_longer_owed() {
        let answered = Frame::opening()
            .after(Tag::Modal)
            .after(Tag::Verb(Form::Base));
        assert_eq!(unmet(answered, Tag::Noun(Number::Singular)), None);
    }
    #[test]
    fn no_run_of_friction_reaches_a_broken_rule() {
        assert_friction_never_reaches_a_rule(scale(), LONGEST, Cost::friction(CEILING));
    }

    #[test]
    fn two_joined_gerunds_agree_either_way_and_one_gerund_does_not() {
        // "Cutting and rejoining a log is all it needs" names one act; "cutting and rejoining are
        // two operations" counts two. Nothing in the sentence says which was meant, so neither
        // form is charged. A single gerund is settled, and is still held to the singular.
        let joined = Frame::opening()
            .after(Tag::Verb(Form::Gerund))
            .after(Tag::Coordinator(Join::Sum))
            .after(Tag::Verb(Form::Gerund));
        assert!(!disagrees(joined, Tag::Verb(Form::ThirdSingular)));
        assert!(!disagrees(joined, Tag::Verb(Form::Base)));

        let alone = Frame::opening().after(Tag::Verb(Form::Gerund));
        assert!(!disagrees(alone, Tag::Verb(Form::ThirdSingular)));
        assert!(disagrees(alone, Tag::Verb(Form::Base)));
    }

    #[test]
    fn two_joined_nouns_are_still_plural() {
        // The exception is about gerunds only. Joined nouns count, and a singular verb after them
        // is the disagreement it has always been.
        let joined = Frame::opening()
            .after(Tag::Noun(Number::Singular))
            .after(Tag::Coordinator(Join::Sum))
            .after(Tag::Noun(Number::Singular));
        assert!(disagrees(joined, Tag::Verb(Form::ThirdSingular)));
        assert!(!disagrees(joined, Tag::Verb(Form::Base)));
    }

    #[test]
    fn one_rule_charged_by_the_frame_and_by_a_pair_costs_once() {
        let once = Rule::SubjectVerb.breach();
        assert_eq!((once + once).breaches(), 1);
        assert_eq!((once + Rule::DoubledTense.breach()).breaches(), 2);
    }
}
