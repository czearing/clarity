//! One call for everything the engine can say about a sentence.

use crate::check::{check, Report};
use crate::clarity::{score, Clarity};
use crate::grammar::Sentence;
use crate::repair::{apply, repair, Edit};
use crate::style::Note;

/// Everything the engine can say about a sentence.
#[derive(Clone, Debug)]
pub struct Assessment {
    /// The sentence as read.
    pub sentence: Sentence,
    /// Rules broken, words not placed, and the tag chosen for each word.
    pub report: Report,
    /// What the sentence costs to hold in mind.
    pub clarity: Clarity,
    /// Wording that costs more than it pays.
    pub notes: Vec<Note>,
    /// The smallest set of swaps that clears every fault, when one exists.
    pub edits: Vec<Edit>,
}

impl Assessment {
    /// Whether the sentence breaks no rule, rests on no unknown word, and wastes no words.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.report.is_clean() && self.notes.is_empty()
    }

    /// The sentence with its repairs applied.
    #[must_use]
    pub fn repaired(&self) -> Sentence {
        apply(&self.sentence, &self.edits)
    }
}

/// Assess `text`.
///
/// ```
/// let found = clarity::assess::assess("the key to the cabinets are missing");
/// assert!(!found.is_clean());
/// assert_eq!(found.report.faults[0].rule.says(), "a tensed verb must agree with its subject");
/// ```
#[must_use]
pub fn assess(text: &str) -> Assessment {
    let sentence = Sentence::read(text);
    let report = check(&sentence);
    let clarity = score(&report.tags);
    let notes = report.notes.clone();
    let edits = repair(&sentence).unwrap_or_default();
    Assessment {
        sentence,
        report,
        clarity,
        notes,
        edits,
    }
}

#[cfg(test)]
mod tests {
    use super::assess;

    #[test]
    fn plain_writing_draws_nothing() {
        assert!(assess("the dog runs").is_clean());
    }

    #[test]
    fn a_fault_and_a_wasted_phrase_are_both_reported() {
        let found = assess("it is important to note that the dogs runs");
        assert!(!found.report.faults.is_empty());
        assert!(!found.notes.is_empty());
    }
}
