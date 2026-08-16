//! How hard a sentence is to hold in mind.
//!
//! The score is integration cost from dependency locality theory: attaching a word to the head it
//! depends on costs one unit for every new discourse referent introduced in between. Referents,
//! not words, because the effect is memory for entities rather than length.
//!
//! Readability formulas are deliberately absent. Flesch-Kincaid and its relatives count syllables
//! and sentence length, which correlate with difficulty in the corpora they were fitted to but do
//! not cause it, and rewriting to satisfy them can leave a text harder to read. Nothing here is
//! scored by a measure that cannot say why.
//!
//! Gibson, "Linguistic complexity: locality of syntactic dependencies", Cognition 68, 1998,
//! pages 1 to 76. Gibson, "The dependency locality theory", in Image, Language, Brain, MIT Press,
//! 2000, pages 95 to 126.

use fitkit::core::Span;

use crate::tag::Tag;

/// The cost of one dependency and where it was paid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strain {
    /// From the dependent to its head.
    pub at: Span,
    /// Discourse referents introduced in between.
    pub referents: usize,
}

/// What a reading costs to hold in mind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clarity {
    /// Every dependency that cost more than an adjacent one would.
    pub strains: Vec<Strain>,
    /// Total integration cost.
    pub cost: usize,
    /// The longest single dependency, which is what a reader notices.
    pub worst: usize,
}

impl Clarity {
    /// Integration cost per word, which is comparable between sentences of different lengths.
    #[must_use]
    pub fn per_word(&self, words: usize) -> f64 {
        if words == 0 {
            0.0
        } else {
            f64::from(u32::try_from(self.cost).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(words).unwrap_or(u32::MAX))
        }
    }
}

/// Score a reading.
///
/// Measures the one dependency a tag sequence settles without a parse: a tensed verb back to the
/// head of its subject. That link is long enough to strain a reader and can be located exactly,
/// which no other dependency here can claim.
#[must_use]
pub fn score(tags: &[Tag]) -> Clarity {
    let mut strains: Vec<Strain> = crate::grammar::clauses(tags)
        .into_iter()
        .map(|(subject, verb)| strain(subject, verb, tags))
        .collect();
    strains.retain(|strain| strain.referents > 0);
    let cost = strains.iter().map(|strain| strain.referents).sum();
    let worst = strains
        .iter()
        .map(|strain| strain.referents)
        .max()
        .unwrap_or(0);
    Clarity {
        strains,
        cost,
        worst,
    }
}

/// One dependency, priced by the referents crossed.
fn strain(from: usize, to: usize, tags: &[Tag]) -> Strain {
    let crossed = tags[from + 1..to]
        .iter()
        .filter(|tag| introduces_referent(**tag))
        .count();
    Strain {
        at: Span::new(from, to + 1),
        referents: crossed,
    }
}

/// Whether a tag introduces something a reader must remember.
///
/// Nouns and tensed verbs, following Gibson. Determiners and marks introduce nothing of their own.
fn introduces_referent(tag: Tag) -> bool {
    matches!(tag, Tag::Noun(_) | Tag::Proper(_)) || tag.is_finite_verb()
}

#[cfg(test)]
mod tests {
    use super::score;
    use crate::check::check;
    use crate::grammar::Sentence;

    fn cost(text: &str) -> usize {
        score(&check(&Sentence::read(text)).tags).cost
    }

    #[test]
    fn an_adjacent_dependency_costs_nothing() {
        assert_eq!(cost("the dog runs"), 0);
    }

    #[test]
    fn distance_between_a_subject_and_its_verb_costs_more() {
        let near = cost("the key opens the door");
        let far = cost("the key to the cabinets in the office opens the door");
        assert!(far > near, "near {near} should be cheaper than far {far}");
    }
}
