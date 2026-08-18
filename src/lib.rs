//! A grammar engine that can explain every judgement it makes.
//!
//! Words are tagged by a dynamic program over the categories the lexicon allows, because English
//! marks a plural noun and a present tense verb with the same letter and only context tells them
//! apart. Grammar rules price the transitions, so the cheapest reading of a sentence is the one
//! that breaks the fewest rules, and the rules it still pays for are exactly what is wrong.
//!
//! Nothing is judged from a word the lexicon cannot place. Those are reported as unknown and the
//! sentence is not called clean, so a pass is always a claim the engine can defend.
//!
//! ```
//! use clarity::check::check;
//! use clarity::grammar::Sentence;
//! use clarity::repair::{apply, repair};
//!
//! let sentence = Sentence::read("the key to the cabinets are missing");
//! let report = check(&sentence);
//! assert_eq!(report.faults[0].rule.says(), "a tensed verb must agree with its subject");
//!
//! let edits = repair(&sentence).unwrap();
//! assert!(check(&apply(&sentence, &edits)).is_clean());
//! ```
//!
//! Built on [fitkit](https://github.com/czearing/fitkit), which supplies the search, the cited
//! laws, and the refusals.

/// The readme, compiled and run as a doctest so no example in it can go stale.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct Readme;

pub mod assess;
pub mod check;
pub mod clarity;
pub mod code;
pub mod condense;
pub mod document;
pub mod frame;
pub mod grammar;
pub mod lexicon;
pub mod out;
pub mod prose;
pub mod register;
pub mod repair;
pub mod style;
pub mod tag;
pub mod text;
pub mod token;
