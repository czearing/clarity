//! A labelled corpus, and the measurement it supports.
//!
//! Every sentence is marked grammatical or not. A sentence resting on a word the lexicon cannot
//! place is counted as refused rather than judged, and refusals are reported separately, because
//! an engine that quietly guesses is worth less than one that says it does not know.

use clarity::check::check;
use clarity::grammar::Sentence;
use clarity::repair::{apply, repair};

/// Sentences that break no rule.
const GOOD: &[&str] = &[
    "the dog runs",
    "the dogs run",
    "she walks to the store",
    "he can walk",
    "they have walked",
    "i am here",
    "you are here",
    "we were here",
    "she was here",
    "the child sees the mice",
    "the children see the mouse",
    "a person writes",
    "several people write",
    "the sheep runs",
    "the sheep run",
    "the key to the cabinets is missing",
    "the keys to the cabinet are missing",
    "the analysis of the data is complete",
    "the criteria are clear",
    "he should have written",
    "she is walking",
    "the data are complete",
    "everyone knows",
    "nobody was here",
    "the men were running",
    "it does not matter",
    "they do not matter",
    "the woman gave the book to him",
    "we think that she is right",
    "because the dog ran, we walked",
    "the teacher of the students was late",
    "the students of the teacher were late",
    "one of the dogs is barking",
    "she has been walking",
    "they had been running",
    "the book on the shelves is old",
    "the books on the shelf are old",
    "he did not know",
    "we have seen the mice",
    "the geese were loud",
    "the goose was loud",
    "she wants to walk",
    "they need to run",
    "he tried to write",
    "the analyses are complete",
    "a criterion is clear",
    "this data is complete",
    "these criteria are clear",
    "the person was here",
    "the people were here",
    "she gives him the book",
    "the aircraft was late",
    "the aircraft were late",
    "the species is rare",
    "the species are rare",
    "he ought to know",
    "we should have known",
    "the woman who was here left",
    "nothing was found",
    "it is here",
    "the fish swims",
    "the fish swim",
    "she walked quickly to the store",
    "the old man was here",
    "we can see the light",
    // A relative clause with no relative pronoun. The second noun phrase is the subject of a
    // clause of its own, and nothing but the absence of a link between the two says so.
    "the conventions a passage holds to are listed",
    "a word the lexicon cannot place is refused",
    "the reports she writes are short",
    // A past participle standing between a determiner and its noun modifies the noun. It is not
    // the verb of the sentence, and the sentence is not missing a subject for it.
    "a tokenised sentence is read once",
    "the chosen reading is cheapest",
    // A preposition left with nothing after it, because its object is at the front of the clause.
    "whatever it still pays for is wrong",
    "the cabinet she took it from is empty",
];

/// Sentences that break a rule, with the rule named so a pass cannot be luck.
const BAD: &[(&str, &str)] = &[
    (
        "a dogs run",
        "a determiner and its noun must agree in number",
    ),
    (
        "these dog runs",
        "a determiner and its noun must agree in number",
    ),
    ("the dog run", "a tensed verb must agree with its subject"),
    ("the dogs runs", "a tensed verb must agree with its subject"),
    ("he walk", "a tensed verb must agree with its subject"),
    ("they walks", "a tensed verb must agree with its subject"),
    ("i walks", "a tensed verb must agree with its subject"),
    (
        "she can walks",
        "a modal is followed by the plain form of a verb",
    ),
    (
        "he must walked",
        "a modal is followed by the plain form of a verb",
    ),
    (
        "the children walks",
        "a tensed verb must agree with its subject",
    ),
    // The same shape as "the dog run" above, and named the same way. A singular subject with a
    // plain verb is a disagreement, not a missing verb: the verb is right there.
    (
        "the child walk",
        "a tensed verb must agree with its subject",
    ),
    (
        "the men was here",
        "a tensed verb must agree with its subject",
    ),
    (
        "the man were here",
        "a tensed verb must agree with its subject",
    ),
    ("i were here", "a tensed verb must agree with its subject"),
    (
        "the key to the cabinets are missing",
        "a tensed verb must agree with its subject",
    ),
    (
        "the keys to the cabinet is missing",
        "a tensed verb must agree with its subject",
    ),
    (
        "the analysis of the data are complete",
        "a tensed verb must agree with its subject",
    ),
    (
        "she wants to walks",
        "infinitival to is followed by the plain form of a verb",
    ),
    (
        "the criteria is clear",
        "a tensed verb must agree with its subject",
    ),
    (
        "every dogs run",
        "a determiner and its noun must agree in number",
    ),
];

/// A sentence the lexicon cannot place is refused, not judged.
#[test]
fn an_unknown_word_is_refused_rather_than_judged() {
    let report = check(&Sentence::read("the zzz runs"));
    assert_eq!(report.unknown, [1]);
    assert!(!report.is_clean());
}

#[test]
fn every_good_sentence_passes() {
    let failed: Vec<_> = GOOD
        .iter()
        .filter(|text| {
            let report = check(&Sentence::read(text));
            !report.faults.is_empty()
        })
        .collect();
    assert!(failed.is_empty(), "false alarms: {failed:?}");
}

#[test]
fn no_good_sentence_rests_on_an_unknown_word() {
    let refused: Vec<_> = GOOD
        .iter()
        .filter(|text| !check(&Sentence::read(text)).unknown.is_empty())
        .collect();
    assert!(refused.is_empty(), "refused: {refused:?}");
}

#[test]
fn every_bad_sentence_is_caught_and_correctly_named() {
    let missed: Vec<_> = BAD
        .iter()
        .filter(|(text, rule)| {
            let report = check(&Sentence::read(text));
            !report.faults.iter().any(|fault| fault.rule.says() == *rule)
        })
        .collect();
    assert!(missed.is_empty(), "missed or misnamed: {missed:?}");
}

#[test]
fn every_bad_sentence_has_a_repair_that_leaves_nothing_to_report() {
    let unfixed: Vec<_> = BAD
        .iter()
        .filter(|(text, _)| {
            let sentence = Sentence::read(text);
            repair(&sentence).is_none_or(|edits| !check(&apply(&sentence, &edits)).is_clean())
        })
        .collect();
    assert!(unfixed.is_empty(), "no repair: {unfixed:?}");
}

#[test]
fn a_repair_changes_as_little_as_it_can() {
    for (text, _) in BAD {
        let edits = repair(&Sentence::read(text)).expect("a repair exists");
        assert!(edits.len() <= 2, "{text} needed {} edits", edits.len());
    }
}

/// Wording that is grammatical but wastes the reader's time, with the flaw each one shows.
const WASTEFUL: &[(&str, clarity::style::Flaw)] = &[
    (
        "due to the fact that it rained we stayed",
        clarity::style::Flaw::Roundabout,
    ),
    (
        "in order to walk she left",
        clarity::style::Flaw::Roundabout,
    ),
    ("each and every dog runs", clarity::style::Flaw::Redundant),
    ("the end result was good", clarity::style::Flaw::Redundant),
    ("we make a decision", clarity::style::Flaw::Buried),
    (
        "it is important to note that dogs run",
        clarity::style::Flaw::Worn,
    ),
    ("we leverage the framework", clarity::style::Flaw::Worn),
    ("the very good dog runs", clarity::style::Flaw::Filler),
    ("there are three dogs", clarity::style::Flaw::Delayed),
    ("the dog walked the dog", clarity::style::Flaw::Echo),
    ("at the end of the day dogs run", clarity::style::Flaw::Worn),
    (
        "she has the ability to run",
        clarity::style::Flaw::Roundabout,
    ),
];

#[test]
fn every_wasteful_sentence_is_named_correctly() {
    let missed: Vec<_> = WASTEFUL
        .iter()
        .filter(|(text, flaw)| {
            !clarity::assess::assess(text)
                .notes
                .iter()
                .any(|note| note.flaw == *flaw)
        })
        .collect();
    assert!(missed.is_empty(), "missed: {missed:?}");
}

#[test]
fn no_plain_sentence_is_called_wasteful() {
    let scolded: Vec<_> = GOOD
        .iter()
        .filter(|text| !clarity::assess::assess(text).notes.is_empty())
        .collect();
    assert!(scolded.is_empty(), "false alarms: {scolded:?}");
}
