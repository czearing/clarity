//! A generator that has never been told a word.
//!
//! Give it text and it learns how that text is written. Give it claims about something and it
//! writes about them, using the vocabulary it learned, in the order the text's own authors put
//! words in. It has no sentences in it, no phrases, no word lists, and no notion of what any
//! subject is called. Point it at a crate and it writes like the crate; point it at a book and it
//! writes like the book. Nothing about it changes in between.
//!
//! Two searches from [fitkit](https://github.com/czearing/fitkit) do all of the deciding. A
//! subset search over the claims fixes what is worth stating, and therefore how many clauses
//! there are. A path search over the input's own text fixes what each clause says: its states are
//! the places of one sentence the input wrote about that part, and its path runs through them in
//! the order they were written. So a clause is a sentence somebody wrote, reported whole, and the
//! engine's decisions are decisions about which sentence rather than about language.
//!
//! ```
//! use clarity_say::{compose, Claim, Corpus, Feature};
//! use fitkit::{Evidence, Span};
//!
//! let mut corpus = Corpus::new();
//! let absent = Feature::of("absent");
//! corpus.attach(&[absent], "The answer may be missing. Nothing stands in for it.", Span::new(0, 52));
//! corpus.attach(&[absent], "The value may be missing. A reader is told so.", Span::new(52, 97));
//! corpus.observe("A reader checks the file. The file says what it holds.", Span::new(97, 150));
//! corpus.settle();
//!
//! let claim = Claim::new(absent, Evidence::certain(Span::new(0, 26), 1.0)).unwrap();
//! let said = compose(&corpus, &[claim], 1).unwrap();
//!
//! // Every word came from a region of the input, and can say which.
//! assert!(!said.sources().unwrap().is_empty());
//! ```

mod compose;
mod corpus;
mod said;

pub use crate::compose::{compose, Claim, MOST_CLAIMS};
pub use crate::corpus::{Corpus, Feature, Word};
pub use crate::said::{Clause, Said, Slot};
