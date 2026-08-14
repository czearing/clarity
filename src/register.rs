//! What a passage holds itself to, recovered rather than declared.
//!
//! Some requirements are part of the language. A verb agrees with its subject in a text message,
//! in a poem, and in a specification alike, and no amount of context excuses it. Others are
//! conventions of a medium: a closing full stop, an apostrophe in a contraction, a tensed verb in
//! every line, a word not repeated. Writing that drops one of those is not wrong, it is written
//! somewhere else.
//!
//! A register is which conventions a passage holds to. Nothing here names a kind of writing. The
//! passage is read under every register and the one that explains it for least wins, so technical
//! prose, a message, and a poem separate themselves without any of them being described.

use fitkit::core::{Evidence, Span};
use fitkit::fit::{recover, Fit, Model, Segmented};

use crate::check::{check_in, judge, Reading, Report};
use crate::text::Text;

/// A requirement a passage may or may not hold itself to.
///
/// Only requirements that vary by medium belong here. Agreement does not, so it is absent, and
/// no register can excuse it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// Every unit has a tensed verb.
    Predicate,
    /// Every unit opens with a capital and closes with a mark.
    Marks,
    /// Contractions are spelled with an apostrophe.
    Apostrophes,
    /// A content word is not repeated close to itself.
    Fresh,
    /// No filler and no worn phrase.
    Plain,
}

impl Convention {
    /// Every convention, in bit order.
    const ALL: [Self; 5] = [
        Self::Predicate,
        Self::Marks,
        Self::Apostrophes,
        Self::Fresh,
        Self::Plain,
    ];

    /// What holding to this convention requires, in one line.
    #[must_use]
    pub fn says(self) -> &'static str {
        match self {
            Self::Predicate => "every sentence has a tensed verb",
            Self::Marks => "every sentence opens with a capital and closes with a mark",
            Self::Apostrophes => "contractions are spelled with an apostrophe",
            Self::Fresh => "a word is not repeated close to itself",
            Self::Plain => "no filler and no worn phrase",
        }
    }
}

/// The conventions a passage holds to.
///
/// A set, not a name. Adding a convention adds a dimension without changing anything else here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Register(u8);

impl Register {
    /// Holds to everything.
    pub const STRICT: Self = Self(0);

    /// Whether this register lets `convention` go.
    #[must_use]
    pub fn waives(self, convention: Convention) -> bool {
        let bit = Convention::ALL
            .iter()
            .position(|held| *held == convention)
            .unwrap_or(0);
        self.0 & (1 << bit) != 0
    }

    /// The conventions this register still holds to.
    #[must_use]
    pub fn holds(self) -> Vec<Convention> {
        Convention::ALL
            .into_iter()
            .filter(|held| !self.waives(*held))
            .collect()
    }

    /// Every register, which is every subset of the conventions.
    #[must_use]
    pub fn every() -> Vec<Self> {
        (0..(1u8 << Convention::ALL.len())).map(Self).collect()
    }

    /// This register with `convention` let go.
    #[must_use]
    pub fn without(self, convention: Convention) -> Self {
        let bit = Convention::ALL
            .iter()
            .position(|held| *held == convention)
            .unwrap_or(0);
        Self(self.0 | (1 << bit))
    }

    /// What choosing this register costs before any fault is counted.
    fn price(self) -> f64 {
        PERMIT * f64::from(self.0.count_ones())
    }
}

/// What one thing the reading cannot explain costs.
const FAULT: f64 = 100.0;

/// What letting one convention go costs for one unit. Half a fault, so a convention is dropped
/// only when doing so explains more than half the units it covers.
const PERMIT: f64 = FAULT / 2.0;

/// What changing register costs, which keeps it from following every unit.
const SWITCH: f64 = FAULT;

impl Segmented for Text {
    fn extent(&self) -> usize {
        self.units.len()
    }

    fn slice(&self, span: Span) -> Self {
        Self {
            units: self.units[span.start..span.end.min(self.units.len())].to_vec(),
        }
    }

    fn splice(&mut self, span: Span, part: Self) {
        self.units
            .splice(span.start..span.end.min(self.units.len()), part.units);
    }
}

/// Recovers what a passage holds itself to.
#[derive(Clone, Copy, Debug, Default)]
pub struct Voice;

impl Model for Voice {
    type Signal = Text;
    type Params = Register;

    fn name(&self) -> &'static str {
        "register"
    }

    fn candidates(&self) -> Vec<Register> {
        Register::every()
    }

    fn render(&self, input: &Text, _params: &Register) -> Text {
        input.clone()
    }
}

impl Fit for Voice {
    // The reading, not the sentence. A register decides which faults are held against a passage,
    // never what its words are, so reading a unit once and judging that reading under each
    // register gives the same answer as reading it under each register, for a thirty second of
    // the work.
    type Evidence = Reading;

    fn evidence(&self, reference: &Text) -> Vec<Evidence<Reading>> {
        reference
            .units
            .iter()
            .enumerate()
            .map(|(index, unit)| Evidence::certain(Span::new(index, index + 1), Reading::of(unit)))
            .collect()
    }

    fn emission(&self, evidence: &Reading, params: &Register) -> f64 {
        let report = judge(evidence, *params);
        let unexplained = report.faults.len() + report.unknown.len() + report.notes.len();
        FAULT * f64::from(u32::try_from(unexplained).unwrap_or(u32::MAX)) + params.price()
    }

    fn transition(&self, from: &Register, to: &Register) -> f64 {
        if from == to {
            0.0
        } else {
            SWITCH
        }
    }
}

/// Read a passage, letting it say for itself what it holds to.
///
/// ```
/// use clarity::register::{read, Convention};
///
/// let haiku = read("an old pond\na frog jumps in\nthe sound of water");
/// assert!(haiku[0].0.waives(Convention::Predicate));
/// assert!(haiku.iter().all(|(_, report)| report.faults.is_empty()));
///
/// let prose = read("The dog run.");
/// assert!(!prose[0].1.faults.is_empty(), "agreement is never waived");
/// ```
#[must_use]
pub fn read(passage: &str) -> Vec<(Register, Report)> {
    let text = Text::read(passage);
    let plan = recover(&Voice, &text);
    text.units
        .iter()
        .zip(plan.controls)
        .map(|(unit, control)| (control.params, check_in(unit, control.params)))
        .collect()
}

/// The register a passage mostly holds to.
///
/// Units are read one after another and may differ, so what is returned is the one the greatest
/// number of them settled on. An opening line too short to say anything cannot outvote the body.
#[must_use]
pub fn of(passage: &str) -> Register {
    let found = read(passage);
    let mut tally: Vec<(Register, usize)> = Vec::new();
    for (register, _) in &found {
        if let Some(seen) = tally.iter_mut().find(|(known, _)| known == register) {
            seen.1 += 1;
        } else {
            tally.push((*register, 1));
        }
    }
    tally
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map_or(Register::STRICT, |(found, _)| found)
}

#[cfg(test)]
mod tests {
    use super::{read, Convention, Register};

    fn registers(passage: &str) -> Vec<Register> {
        read(passage)
            .into_iter()
            .map(|(register, _)| register)
            .collect()
    }

    fn waives(passage: &str, convention: Convention) -> bool {
        registers(passage)
            .iter()
            .all(|register| register.waives(convention))
    }

    #[test]
    fn a_passage_of_fragments_lets_the_predicate_go() {
        assert!(waives(
            "an old pond\na frog jumps in\nthe sound of water",
            Convention::Predicate
        ));
    }

    #[test]
    fn technical_prose_holds_to_everything() {
        let found = registers(
            "The parser reads the file. It returns a tree. The caller checks the result.",
        );
        assert!(
            found.iter().all(|register| *register == Register::STRICT),
            "{found:?}"
        );
    }

    #[test]
    fn a_message_lets_the_marks_go_but_not_agreement() {
        let found = read("i am on my way\nthe train were late\nsee you soon");
        assert!(found[0].0.waives(Convention::Marks), "{:?}", found[0].0);
        assert!(!found[1].1.faults.is_empty(), "agreement is never waived");
    }

    #[test]
    fn one_fragment_does_not_change_how_a_paragraph_is_read() {
        let found =
            registers("The dog runs. The cat sleeps. No matter. The birds sing. The sun rises.");
        assert!(
            found.iter().all(|register| *register == Register::STRICT),
            "{found:?}"
        );
    }

    #[test]
    fn a_passage_that_drops_apostrophes_is_read_with_them() {
        let found = read("i dont know\nshe cant come\nwe wont wait");
        assert!(
            found[0].0.waives(Convention::Apostrophes),
            "{:?}",
            found[0].0
        );
        assert!(found.iter().all(|(_, report)| report.unknown.is_empty()));
    }

    #[test]
    fn a_refrain_is_not_a_mistake_where_repetition_is_the_form() {
        let found = read("i want it i want it\ni need it i need it\ni want it i want it");
        assert!(found[0].0.waives(Convention::Fresh), "{:?}", found[0].0);
        assert!(found.iter().all(|(_, report)| report.notes.is_empty()));
    }
}
