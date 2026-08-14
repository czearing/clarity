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

    /// How many conventions this register lets go, which breaks ties toward reading strictly.
    fn breadth(self) -> u32 {
        self.0.count_ones()
    }
}

/// What one thing the reading cannot explain costs.
const FAULT: f64 = 100.0;

/// What adopting one convention costs a passage, paid once however many units are read under it.
///
/// More than a fault and less than two, so a convention broken once is a mistake and a convention
/// broken twice is how the passage is written. Nothing else decides where that line falls.
const ADOPT: f64 = FAULT * 1.5;

/// How many registers a passage may weigh at once, which bounds the search when units disagree.
const SPREAD: usize = 8;

/// Recovers what a passage holds itself to.
///
/// A register is not a run. A source file alternates one line summaries with whole paragraphs and a
/// poem may break off mid page, so pricing a change of register by adjacency charges writing for its
/// shape rather than its variety. What a passage pays here is each register it adopts, once, after
/// which any unit may be read under it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Voice;

impl Voice {
    /// What a reading costs under a register, counting all it does not excuse.
    fn cost(reading: &Reading, register: Register) -> f64 {
        let report = judge(reading, register);
        let unexplained = report.faults.len() + report.unknown.len() + report.notes.len();
        FAULT * f64::from(u32::try_from(unexplained).unwrap_or(u32::MAX))
    }

    /// The least register that excuses everything excusable in one reading.
    ///
    /// Waiving more than this costs without explaining, so no unit wants a larger register and the
    /// registers a passage might adopt are exactly the ones its own units ask for.
    fn asked_for(reading: &Reading) -> Register {
        let strict = Self::cost(reading, Register::STRICT);
        Convention::ALL
            .into_iter()
            .filter(|held| Self::cost(reading, Register::STRICT.without(*held)) < strict)
            .fold(Register::STRICT, Register::without)
    }

    /// Which adopted register a reading is read under, and what it still costs there.
    ///
    /// A register is not free to a unit that does not need it, so each unit reaches for the
    /// smallest one that answers for it and the rest of the passage is not read loosely.
    fn settle(reading: &Reading, adopted: &[Register]) -> (Register, f64) {
        adopted
            .iter()
            .map(|register| (*register, Self::cost(reading, *register)))
            .min_by(|left, right| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(left.0.breadth().cmp(&right.0.breadth()))
            })
            .unwrap_or((Register::STRICT, 0.0))
    }

    /// What a passage pays to adopt a set of registers and read every unit under the best of them.
    fn spend(readings: &[Reading], adopted: &[Register]) -> f64 {
        let paid: f64 = adopted
            .iter()
            .map(|register| ADOPT * f64::from(register.breadth()))
            .sum();
        let borne: f64 = readings
            .iter()
            .map(|reading| Self::settle(reading, adopted).1)
            .sum();
        paid + borne
    }

    /// The registers a passage adopts, which is the cheapest set drawn from what its units ask for.
    fn adopted(readings: &[Reading]) -> Vec<Register> {
        let mut asked: Vec<Register> = Vec::new();
        for reading in readings {
            let wanted = Self::asked_for(reading);
            if wanted != Register::STRICT && !asked.contains(&wanted) {
                asked.push(wanted);
            }
        }
        asked.truncate(SPREAD);
        let mut best = vec![Register::STRICT];
        let mut least = Self::spend(readings, &best);
        for choice in 1..(1u32 << asked.len()) {
            let mut set = vec![Register::STRICT];
            for (bit, register) in asked.iter().enumerate() {
                if choice & (1 << bit) != 0 {
                    set.push(*register);
                }
            }
            let spent = Self::spend(readings, &set);
            if spent < least {
                least = spent;
                best = set;
            }
        }
        best
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
    let readings: Vec<Reading> = text.units.iter().map(Reading::of).collect();
    let adopted = Voice::adopted(&readings);
    text.units
        .iter()
        .zip(&readings)
        .map(|(unit, reading)| {
            let register = Voice::settle(reading, &adopted).0;
            (register, check_in(unit, register))
        })
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
