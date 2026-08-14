//! Passages of different kinds, and what the engine works out about each without being told.
//!
//! No test names a form of writing to the engine. Each passage is handed over as text and the
//! register is recovered. What is asserted is which conventions the passage turned out to hold to,
//! and, in every case, that a real fault still surfaces.

use clarity::register::{of, read, Convention, Register};

const TECHNICAL: &str = "The parser reads the file. It returns a tree. \
                         The caller checks the result. A failure raises an error.";

const STORY: &str = "The old man walked to the door. He opened it slowly. \
                     A cold wind came in. He did not move.";

const MESSAGE: &str = "hey\nim running late\nthe train dont move\ncant help it\nsee you soon";

const HAIKU: &str = "an old pond\na frog jumps in\nthe sound of water";

const LYRIC: &str = "i want it i want it\ni need it i need it\ni want it i want it\n\
                     i need it i need it";

fn register(passage: &str) -> Register {
    of(passage)
}

#[test]
fn technical_writing_holds_to_every_convention() {
    assert_eq!(register(TECHNICAL), Register::STRICT);
}

#[test]
fn a_story_holds_to_every_convention() {
    assert_eq!(register(STORY), Register::STRICT);
}

#[test]
fn a_message_drops_marks_and_apostrophes() {
    let found = register(MESSAGE);
    assert!(found.waives(Convention::Marks), "{found:?}");
    assert!(found.waives(Convention::Apostrophes), "{found:?}");
}

#[test]
fn a_poem_drops_the_predicate() {
    assert!(register(HAIKU).waives(Convention::Predicate));
}

#[test]
fn a_lyric_drops_repetition() {
    assert!(register(LYRIC).waives(Convention::Fresh));
}

#[test]
fn nothing_the_engine_infers_excuses_a_real_fault() {
    let spoiled = [
        (TECHNICAL, "The parser read the file."),
        (STORY, "The old man walk to the door."),
        (MESSAGE, "the trains dont moves"),
        (HAIKU, "the frogs jumps in"),
        (LYRIC, "i wants it i wants it"),
    ];
    for (passage, fault) in spoiled {
        let with = format!("{passage}\n{fault}");
        let found = read(&with);
        let last = found.last().expect("a unit");
        assert!(!last.1.faults.is_empty(), "missed the fault in: {fault}");
    }
}

#[test]
fn every_passage_is_read_without_an_unknown_word() {
    for passage in [TECHNICAL, STORY, MESSAGE, HAIKU, LYRIC] {
        for (_, report) in read(passage) {
            assert!(report.unknown.is_empty(), "unknown in: {passage}");
        }
    }
}
