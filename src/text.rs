//! Splitting a passage into the units a reader takes one at a time.
//!
//! A mark ends a unit. So does a line break, but only where no mark has ended one already, which
//! keeps a wrapped paragraph whole and still lets a line of verse stand alone.

use crate::grammar::Sentence;
use crate::token::tokenise;

/// A passage, split into units.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    /// Units in order.
    pub units: Vec<Sentence>,
}

impl Text {
    /// Split `passage`.
    #[must_use]
    pub fn read(passage: &str) -> Self {
        let units = passage
            .lines()
            .flat_map(|line| {
                let mut units = Vec::new();
                let mut held = Vec::new();
                for token in tokenise(line) {
                    let ends = token.ends_sentence();
                    held.push(token);
                    if ends {
                        units.push(Sentence {
                            tokens: core::mem::take(&mut held),
                        });
                    }
                }
                if !held.is_empty() {
                    units.push(Sentence { tokens: held });
                }
                units
            })
            .filter(|unit| !unit.tokens.is_empty())
            .collect();
        Self { units }
    }
}

#[cfg(test)]
mod tests {
    use super::Text;

    #[test]
    fn a_mark_ends_a_unit() {
        let text = Text::read("the dog runs. the cat sleeps.");
        assert_eq!(text.units.len(), 2);
    }

    #[test]
    fn a_line_without_a_mark_stands_alone() {
        let text = Text::read("an old pond\na frog jumps in\nthe sound of water");
        assert_eq!(text.units.len(), 3);
    }

    #[test]
    fn a_wrapped_sentence_is_not_split() {
        let text = Text::read("the dog\nruns quickly.");
        assert_eq!(text.units.len(), 2, "a line break still ends a line");
    }
}
