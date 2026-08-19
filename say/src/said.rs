//! What was said, and the proof that a search said it.
//!
//! Nothing in this module can be built from outside the crate. There is no constructor, no
//! `From<String>`, no `Default`, and no way to reach the words except by reading them back. The
//! only route to a [`Said`] is [`compose`](crate::compose), and the only route through that is a
//! search over a corpus. A caller who has decided in advance what the output should read like has
//! nowhere to put it.
//!
//! Every word carries the region of the input it came from, so a reader can ask any sentence
//! where it got a word and be answered with a span rather than an assurance.

use fitkit::{Answer, Chosen, Span};

/// One position in a clause, holding the word the search put there.
///
/// A slot may be silent. The search decides how long a clause is by choosing, at each position
/// after the first, either a word or the end of the clause, so length is an outcome of the search
/// and not a number a caller passed in.
#[derive(Clone, Debug)]
pub struct Slot {
    spelling: String,
    span: Option<Span>,
    attached: bool,
}

impl Slot {
    /// A slot holding a word.
    pub(crate) fn spoken(spelling: String, span: Span, attached: bool) -> Self {
        Self {
            spelling,
            span: Some(span),
            attached,
        }
    }

    /// A slot the search chose to leave silent, ending the clause.
    pub(crate) const fn silent() -> Self {
        Self {
            spelling: String::new(),
            span: None,
            attached: false,
        }
    }

    /// The word here, if the search put one here.
    #[must_use]
    pub fn word(&self) -> Option<&str> {
        self.span.map(|_| self.spelling.as_str())
    }

    /// The region of the input this word was read from.
    #[must_use]
    pub const fn source(&self) -> Option<Span> {
        self.span
    }

    /// Whether this word is written with no space before it.
    #[must_use]
    pub const fn is_attached(&self) -> bool {
        self.attached
    }
}

/// One clause, and the search that chose its words.
///
/// The witness is kept rather than unwrapped, so a clause can always be asked what it cost, how
/// many words were available at each position, and how close the next-best reading was.
#[derive(Debug)]
pub struct Clause {
    chosen: Chosen<Vec<Slot>>,
}

impl Clause {
    /// Only reachable from a completed search.
    pub(crate) const fn from_search(chosen: Chosen<Vec<Slot>>) -> Self {
        Self { chosen }
    }

    /// The words, in the order the search put them.
    pub fn words(&self) -> impl Iterator<Item = &Slot> {
        self.chosen.get().iter().filter(|slot| slot.span.is_some())
    }

    /// What the chosen reading cost, in the corpus's own currency.
    #[must_use]
    pub const fn cost(&self) -> f64 {
        self.chosen.cost()
    }

    /// How many words the search weighed, and how close the runner-up was.
    #[must_use]
    pub const fn trace(&self) -> fitkit::Trace {
        self.chosen.trace()
    }

    /// The clause written out.
    #[must_use]
    pub fn text(&self) -> String {
        let mut written = String::new();
        for slot in self.words() {
            if !written.is_empty() && !slot.attached {
                written.push(' ');
            }
            written.push_str(&slot.spelling);
        }
        written
    }
}

/// A passage, and the search that chose what it would mention.
///
/// Two searches stand behind one of these. A subset search over the claims decided which of them
/// were worth stating, which is what fixes how many clauses there are; a path search over the
/// corpus decided the words of each. Neither was shown the answer it was scoring.
///
/// There is no way to build one except by composing, and that is the point rather than an
/// oversight. A caller who could assemble a passage out of words of their own would have a
/// template, and the compiler is the only thing that can promise there is not one. Each of these
/// is rejected before it runs:
///
/// ```compile_fail
/// // A passage cannot be assembled out of parts a caller holds.
/// let said = clarity_say::Said { chosen: Vec::new() };
/// ```
///
/// ```compile_fail
/// // Nor can a clause, which is where words would have to enter.
/// let clause = clarity_say::Clause { chosen: Vec::new() };
/// ```
///
/// ```compile_fail
/// // Nor can a single word be spoken into one.
/// let slot = clarity_say::Slot::new("whatever a caller wants said");
/// ```
#[derive(Debug)]
pub struct Said {
    chosen: Chosen<Vec<Answer<Clause>>>,
    /// Where the run of clauses breaks, measured from the input rather than decided here.
    breaks: Vec<usize>,
}

impl Said {
    /// Only reachable from a completed search.
    pub(crate) const fn from_search(
        chosen: Chosen<Vec<Answer<Clause>>>,
        breaks: Vec<usize>,
    ) -> Self {
        Self { chosen, breaks }
    }

    /// The clauses, gathered into the passages the input's own vocabulary puts them in.
    ///
    /// A document is not a list. Two parts an author discusses in the same words belong in one
    /// passage and two they never discuss together do not, and that is a measurement of the input
    /// taken before anything is printed. Nothing here decides where a paragraph should fall; it
    /// reports where the shared vocabulary between one part and the next drops below what this
    /// input holds between neighbours generally.
    ///
    /// # Errors
    ///
    /// Refuses where any clause could not be composed from the corpus.
    pub fn passages(&self) -> Answer<Vec<Vec<&Clause>>> {
        let clauses = self.clauses()?;
        let mut passages = Vec::new();
        let mut passage: Vec<&Clause> = Vec::new();
        for (position, clause) in clauses.into_iter().enumerate() {
            if position > 0 && self.breaks.contains(&position) && !passage.is_empty() {
                passages.push(core::mem::take(&mut passage));
            }
            passage.push(clause);
        }
        if !passage.is_empty() {
            passages.push(passage);
        }
        Ok(passages)
    }

    /// The clauses, or the first refusal that stopped one being written.
    ///
    /// # Errors
    ///
    /// Refuses where any clause could not be composed from the corpus.
    pub fn clauses(&self) -> Answer<Vec<&Clause>> {
        self.chosen
            .get()
            .iter()
            .map(|clause| clause.as_ref().map_err(|refusal| *refusal))
            .collect()
    }

    /// How many claims the search decided were worth stating.
    #[must_use]
    pub fn stated(&self) -> usize {
        self.chosen.get().len()
    }

    /// What the selection cost.
    #[must_use]
    pub const fn cost(&self) -> f64 {
        self.chosen.cost()
    }

    /// How many claims were weighed, and how close the runner-up selection was.
    #[must_use]
    pub const fn trace(&self) -> fitkit::Trace {
        self.chosen.trace()
    }

    /// Every region of the input that a word here was read from.
    ///
    /// This is what makes the passage checkable. A word whose span is not in this list does not
    /// exist, because there is no way to put one there.
    ///
    /// # Errors
    ///
    /// As [`Said::clauses`].
    pub fn sources(&self) -> Answer<Vec<Span>> {
        let mut spans = Vec::new();
        for clause in self.clauses()? {
            spans.extend(clause.words().filter_map(Slot::source));
        }
        Ok(spans)
    }

    /// The passage written out, one clause after another.
    ///
    /// # Errors
    ///
    /// As [`Said::clauses`].
    pub fn text(&self) -> Answer<String> {
        let clauses = self.clauses()?;
        let mut written = String::new();
        for clause in clauses {
            let part = clause.text();
            if part.is_empty() {
                continue;
            }
            if !written.is_empty() {
                written.push(' ');
            }
            written.push_str(&part);
        }
        Ok(written)
    }
}
