//! What a clause has seen so far.
//!
//! A tag says what one word is. It cannot say whether the clause around that word already has a
//! subject, or whether the subject was singular, or whether a verb has been used up. Those are
//! properties of the reading rather than of the word, and every rule that needs one of them was
//! therefore unable to take part in the search.
//!
//! That is why agreement across distance, a missing predicate, and two tensed verbs in a row were
//! all found by scanning the finished reading instead. Scanning after the fact is too late: by then
//! the search has already chosen a reading that hid the problem, because nothing in the search
//! charged it for hiding it. Reading "these dog runs" as a pronoun followed by two verbs costs
//! nothing pairwise, so it wins, and the scan can only report on what it was handed.
//!
//! A frame carries that context inside the search. The state is a tag together with a frame, the
//! frame after a step is decided by the frame before it and the tag chosen, and the rules that
//! needed context become ordinary transition costs. The search can no longer buy its way out of a
//! fault by moving the fault somewhere the checker was not looking.
//!
//! Only what a rule actually consults is kept. English agreement asks whether the subject is third
//! person singular and nothing finer, so that is one bit rather than a person and a number; the
//! rest would multiply the search for no gain.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::tag::{Break, Form, Number, Person, Tag};

/// How far along the phrase being read is.
///
/// A determiner names a noun phrase before its head arrives, so a word read in that gap belongs to
/// the phrase and cannot be the verb of a clause: "a tokenised sentence" said its subject was still
/// being spelled out, not that the sentence had none. A preposition or a joining word instead ties
/// what follows to what came before, which is what tells "the key to the cabinets", one phrase,
/// apart from "the conventions a passage holds to", where a second phrase begun with nothing
/// linking it is the subject of a clause of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phrase {
    /// A determiner has been read and its head noun has not.
    Open,
    /// A preposition or a joining word has tied what comes next to what came before.
    Linked,
    /// No phrase is in progress: the last one read is complete, or none was begun.
    Whole,
}

/// What the clause knows about its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Subject {
    /// Nothing has been read yet, so a verb here is a command rather than a verb without a subject.
    Empty,
    /// Words have been read and none of them was a subject.
    None,
    /// A third person singular subject, the only one English inflects a present verb for.
    Third,
    /// A first person singular subject, which takes the plain form but "was" rather than "were".
    First,
    /// Any other subject, which every present tense verb meets in its plain form.
    Other,
    /// A command, which is allowed to have no subject and agrees with nothing.
    Command,
}

/// What a word has asked for and not yet been given.
///
/// The demander is named rather than the demand, because "can walks" and "to walks" are the same
/// failure but not the same complaint, and a report that cannot tell them apart is worth less.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wants {
    /// Nothing outstanding.
    Nothing,
    /// A modal is waiting for the plain form of a verb.
    BaseForModal,
    /// Infinitival "to" is waiting for the plain form of a verb.
    BaseForTo,
}

/// What the clause around a word has seen.
///
/// A subordinate clause suspends the sentence rather than replacing it, so the outer clause is set
/// aside rather than thrown away. That is what lets "the dog, which barks, run" still be answered
/// for: when the inner clause closes, the outer subject comes back and is still waiting for a verb
/// that agrees with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Frame {
    /// The subject the next tensed verb must answer to.
    pub subject: Subject,
    /// Whether the clause already has a tensed verb.
    pub tensed: bool,
    /// The clause this one interrupted, waiting to be resumed.
    pub outer: Option<(Subject, bool)>,
    /// Whether any clause of this sentence has taken a tensed verb.
    ///
    /// A full stop ends the clause it closes, so the frame it leaves has no subject and no tense.
    /// Asking that frame whether the sentence had a predicate would answer no for every sentence,
    /// which is why the answer is kept rather than read off the end.
    pub ever: bool,
    /// Whether the subject in the slot was read in this clause or inherited from the last one.
    ///
    /// A predicate joined to another answers to the same subject, as in "a sentence is credited
    /// with nothing and keeps only its price", so the slot may not be emptied at the join. But a
    /// joined clause may also bring its own subject, as in "the dogs run and the cat sleeps", and
    /// an inherited subject has to give way to it. Within a clause the opposite holds: the first
    /// noun phrase is the subject and every later one is a modifier. One bit tells the two apart.
    pub read: bool,
    /// How far along the phrase being read is.
    pub phrase: Phrase,
    /// What a word has demanded and not yet received.
    ///
    /// A grammar written as a list of forbidden pairs has a hole wherever a pair was not listed,
    /// and a search whose job is to be cheap will find exactly those holes. A demand closes the
    /// hole from the other side: the word that needs something says so, and not receiving it costs
    /// whatever is between them.
    pub wants: Wants,
}

impl Frame {
    /// The frame a sentence starts in.
    #[must_use]
    pub const fn opening() -> Self {
        Self {
            subject: Subject::Empty,
            tensed: false,
            outer: None,
            ever: false,
            read: false,
            phrase: Phrase::Whole,
            wants: Wants::Nothing,
        }
    }

    /// The subject a tensed verb arriving now would answer to.
    ///
    /// Normally that is the clause's own subject. But a clause held inside another that already
    /// has its verb cannot take a second one, so the verb belongs to the clause that was set
    /// aside and answers to its subject. Asking the frame this one question is what keeps the
    /// rules from each having to know about nesting.
    #[must_use]
    pub const fn answering(self) -> Subject {
        match self.outer {
            Some((subject, false)) if self.tensed => subject,
            _ => self.subject,
        }
    }

    /// Whether a subordinate clause is open, which suspends the sentence's own demands.
    #[must_use]
    pub const fn open(self) -> bool {
        self.outer.is_some()
    }

    /// Every frame a sentence can actually be in, in a fixed order.
    ///
    /// The fields describe more combinations than the language reaches. Rather than reason about
    /// which, the reachable ones are walked out from the opening frame, so the state space is
    /// exactly as large as the grammar makes it and shrinks on its own whenever a rule is
    /// tightened.
    #[must_use]
    pub fn every() -> &'static [Self] {
        static EVERY: OnceLock<Vec<Frame>> = OnceLock::new();
        EVERY.get_or_init(|| {
            let mut found = vec![Self::opening()];
            let mut seen: HashSet<Self> = found.iter().copied().collect();
            let mut at = 0;
            while at < found.len() {
                let frame = found[at];
                at += 1;
                for tag in Tag::every() {
                    let next = frame.after(tag);
                    if seen.insert(next) {
                        found.push(next);
                    }
                }
            }
            found
        })
    }

    /// Where this frame sits in [`every`](Frame::every).
    ///
    /// The reachable frames are found by search rather than enumerated, so a frame does not carry
    /// its own position and has to be looked one up.
    #[must_use]
    pub fn at(self) -> usize {
        static WHERE: OnceLock<HashMap<Frame, usize>> = OnceLock::new();
        WHERE
            .get_or_init(|| {
                Self::every()
                    .iter()
                    .enumerate()
                    .map(|(at, &frame)| (frame, at))
                    .collect()
            })
            .get(&self)
            .copied()
            .unwrap_or(0)
    }

    /// The frame after a word read as `tag`.
    #[must_use]
    pub fn after(self, tag: Tag) -> Self {
        let carried = Self {
            wants: demands(self, tag),
            phrase: phrasing(self.phrase, tag),
            ..self
        };
        match tag {
            // A subordinator sets the sentence aside and starts a clause with a subject of its own.
            // Only one is tracked: a clause inside a clause inside a clause is rare enough that
            // holding the whole stack would cost more than it explains.
            Tag::Subordinator if self.outer.is_none() => Self {
                subject: Subject::Empty,
                tensed: false,
                outer: Some((self.subject, self.tensed)),
                read: false,
                ..carried
            },
            // A mark closes whatever clause was open and hands the sentence back what it was
            // holding. A full stop at the end of a sentence with nothing open changes nothing,
            // which is what lets the last word of a sentence still be asked for its verb.
            // A full stop, a semicolon, or a question mark ends every clause it closes, so what
            // follows starts with nothing. A comma only pauses, and hands back a clause the
            // sentence had set aside.
            Tag::Mark(Break::Stop) => Self {
                subject: Subject::Empty,
                tensed: false,
                outer: None,
                ever: self.ever || self.tensed,
                read: false,
                ..carried
            },
            Tag::Mark(Break::Pause) => match self.outer {
                Some((subject, tensed)) => Self {
                    subject,
                    tensed,
                    outer: None,
                    ..carried
                },
                None => carried,
            },
            // Before the verb a joining word is joining subjects, and two subjects joined are
            // plural however singular each was: "agreement and predication are structural". After
            // the verb there is nothing left to join but clauses, so a fresh one starts.
            Tag::Coordinator
                if !self.tensed
                    && matches!(
                        self.subject,
                        Subject::Third | Subject::First | Subject::Other
                    ) =>
            {
                Self {
                    subject: Subject::Other,
                    ..carried
                }
            }
            // After the verb a joining word starts a fresh predicate, which answers to the same
            // subject unless a new noun phrase supplies one. Clearing the subject here forced the
            // joined verb to be subjectless and drove it to be read as a noun instead.
            Tag::Coordinator => Self {
                tensed: false,
                read: false,
                ..carried
            },
            _ => self.reading(tag, carried),
        }
    }

    /// The frame after a word that neither opened a clause nor closed one.
    ///
    /// Everything here is about the clause the word is inside: whether the word is its verb, and
    /// what the clause is about. Splitting it out keeps the boundaries in one place and the
    /// predicate in another, which is the same division the rules make.
    fn reading(self, tag: Tag, carried: Self) -> Self {
        match tag {
            // A verb spends the subject it agreed with, and the slot is then free to hold the
            // subject of whatever clause comes next. That is what lets "verifies the string is
            // valid" be read as a verb and its complement rather than as two verbs in one clause,
            // without any list of which verbs take a clause.
            // A clause held inside another closes when the outer clause's own verb arrives. The
            // inner clause already has its verb, and English does not give one clause two, so a
            // second tensed verb can only belong to the clause that was set aside. That is what
            // lets "the reports she writes are short" answer "are" to "reports" and not to "she".
            Tag::Verb(form)
                if is_tensed(form)
                    && self.tensed
                    && self.phrase != Phrase::Open
                    && matches!(self.outer, Some((_, false))) =>
            {
                Self {
                    subject: spent(self.answering()),
                    tensed: true,
                    ever: true,
                    outer: None,
                    read: true,
                    ..carried
                }
            }
            Tag::Verb(form) if is_tensed(form) && !self.tensed && self.phrase != Phrase::Open => {
                Self {
                    subject: spent(self.subject),
                    tensed: true,
                    ever: true,
                    ..carried
                }
            }
            // A modal is the finite verb of its clause and can be nothing else, so it takes the
            // tense and spends the subject whether or not a verb has already been read. Skipping it
            // in a clause that was already tensed left the modal's complement to be judged as a
            // finite verb against a subject two clauses away, which is what made "the category a
            // word can hold" read "hold" as a noun to escape an agreement it was never in.
            Tag::Modal if self.phrase != Phrase::Open => Self {
                subject: spent(self.subject),
                tensed: true,
                ever: true,
                ..carried
            },
            // A noun phrase begun where the clause already has its subject, has no verb yet, and
            // has nothing linking the two, is not a second subject: it is the subject of a clause
            // of its own, as in "the conventions a passage holds to". A preposition, a determiner
            // or a joining word all leave a phrase open, so the phrase bit is what says the two
            // are unlinked, and no relative pronoun has to be present for the clause to be there.
            _ if !self.tensed
                && self.read
                && self.phrase == Phrase::Whole
                && self.outer.is_none()
                && heads(tag) =>
            {
                Self {
                    subject: features(tag).unwrap_or(Subject::None),
                    read: features(tag).is_some(),
                    outer: Some((self.subject, self.tensed)),
                    ..carried
                }
            }
            // Before the verb, the first noun phrase is the subject and everything after it is a
            // modifier, which is why "the key to the cabinets" is answered for by "key". After the
            // verb the opposite holds: the nearest noun phrase is the one a following verb would
            // belong to, so it replaces whatever was there.
            _ if self.tensed => match features(tag) {
                Some(subject) => Self {
                    subject,
                    read: true,
                    ..carried
                },
                None => carried,
            },
            // Before the verb a noun phrase is the subject if the clause has not read one, and a
            // subject carried over from a joined clause has not been read here.
            _ if !self.read || matches!(self.subject, Subject::Empty | Subject::None) => Self {
                subject: features(tag).unwrap_or(Subject::None),
                read: features(tag).is_some(),
                ..carried
            },
            _ => carried,
        }
    }
}

/// How far along the phrase being read is once `tag` has been read.
///
/// A modifier stands inside a phrase and leaves the question where it was. Everything that is not
/// a modifier settles it: a determiner opens a phrase, a joining word links one to what came
/// before, a head closes one, and a verb or a mark leaves no phrase in progress at all.
const fn phrasing(so_far: Phrase, tag: Tag) -> Phrase {
    match tag {
        Tag::Determiner(_) | Tag::Numeral => Phrase::Open,
        Tag::Preposition | Tag::Coordinator | Tag::Subordinator | Tag::To => Phrase::Linked,
        Tag::Adjective | Tag::Adverb | Tag::Verb(Form::Participle) => so_far,
        _ => Phrase::Whole,
    }
}

/// Whether `tag` can begin a noun phrase.
const fn heads(tag: Tag) -> bool {
    matches!(
        tag,
        Tag::Determiner(_) | Tag::Numeral | Tag::Noun(_) | Tag::Proper(_) | Tag::Pronoun(..)
    )
}

/// Whether the clause is still waiting for a plain verb once `tag` has been read.
///
/// A modal and infinitival "to" both demand one. An adverb may stand between the demand and what
/// answers it, as in "can not go", and everything else either answers the demand or fails it.
fn demands(frame: Frame, tag: Tag) -> Wants {
    match tag {
        Tag::Modal => Wants::BaseForModal,
        Tag::To => Wants::BaseForTo,
        Tag::Adverb => frame.wants,
        _ => Wants::Nothing,
    }
}

/// What the subject slot holds once a verb has used it.
///
/// A verb with a subject leaves the slot empty and ready for the next clause. A verb with no
/// subject leaves that fact behind, because a sentence whose only verb answered to nothing is a
/// fragment and has to stay one.
const fn spent(subject: Subject) -> Subject {
    match subject {
        Subject::Empty => Subject::Command,
        // A clause keeps who it is about after its verb has agreed with it, because a second verb
        // joined to the first answers to the same subject: "a sentence is credited with nothing
        // and keeps only its price". Emptying the slot made the joined verb subjectless, and the
        // cheapest escape was to read it as a plural noun. A noun phrase that follows replaces the
        // subject on its own, so nothing is needed here to let the next clause have its own.
        other => other,
    }
}

/// Whether a form carries tense.
fn is_tensed(form: Form) -> bool {
    matches!(
        form,
        Form::Base | Form::ThirdSingular | Form::Past | Form::PastSingular | Form::PastPlural
    )
}

/// What a tag contributes as a subject, if it can be one.
fn features(tag: Tag) -> Option<Subject> {
    match tag {
        // A gerund phrase is a subject in its own right and a singular one, so "running tests is
        // easy" agrees. Without this the noun inside the phrase is mistaken for the subject.
        Tag::Noun(Number::Singular)
        | Tag::Proper(Number::Singular)
        | Tag::Pronoun(Person::Third, Number::Singular, _)
        | Tag::Verb(Form::Gerund) => Some(Subject::Third),
        Tag::Pronoun(Person::First, Number::Singular, _) => Some(Subject::First),
        Tag::Noun(Number::Plural) | Tag::Proper(Number::Plural) | Tag::Pronoun(..) => {
            Some(Subject::Other)
        }
        _ => None,
    }
}
