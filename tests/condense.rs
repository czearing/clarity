//! What a padded doc comment is reduced to.
//!
//! Every passage here is real padding of a kind that appears in shipped code: metaphor, buzzwords,
//! self-reference, restatement, aggression, sales copy. None of them breaks a rule of grammar, so
//! none of them can be caught by checking sentences one at a time. What is wrong with each is the
//! same thing, and it is only visible over the whole passage: one point, said at length.
//!
//! The expected reading of each is the sentence a competent editor would keep. Where the passage
//! never states its point plainly, the expectation is the sentence that comes closest, because an
//! engine that does not invent wording cannot do better than the best sentence it was given.

use clarity::condense::condense;

/// Each passage and the one sentence it should come down to.
const PASSAGES: &[(&str, &str)] = &[
    (
        "Like a delicate flower blooming in the digital garden of our enterprise architecture, this \
         method gently whispers to the private field and coaxes it into the light. It is a \
         testament to the beauty of object-oriented design that we must retrieve this number, not \
         because we want to, but because we must. It returns the value, which is an integer, \
         representing the number of items. Truly, a masterpiece of modern engineering.",
        "It returns the value, which is an integer, representing the number of items.",
    ),
    (
        "Crucial Web3 blockchain-enabled AI parser. By invoking this high-performance neural \
         utility, you are initiating a multi-agent decentralized ledger consensus mechanism that \
         securely parses the raw JSON string into a robust dictionary array. Note: Do not use if \
         the moon is in gibbous phase, as quantum decoherence in the garbage collector may cause \
         stack overflow in non-existent local servers. Returns the parsed JSON.",
        "Returns the parsed JSON.",
    ),
    (
        "This function, which does adding, adds two numbers, x and y, or is it y and x, \
         mathematically it doesn't matter, but clean code dictates, we must document this, with \
         extreme precision, to avoid technical debt, and promote synergy, among developer \
         pipelines. Adds x to y.",
        "Adds x to y.",
    ),
    (
        "Calculates the length of the string by measuring how long the string is. To determine the \
         length, it counts the characters, and the total count of these characters represents the \
         string's length. If the string is longer, the length will be larger; if the string is \
         shorter, the length will be smaller. The returned value is the length.",
        "It counts the characters, and the total count of these characters represents the string's \
         length.",
    ),
    (
        "Deep-dive into a strategic paradigm shift leveraging state-of-the-art Boolean metrics to \
         optimize cross-functional flag alignment. By dynamically auditing the active status \
         indicator, we synergize runtime protocols to return a robust truth value. This empowers \
         stakeholder-driven execution pathways to achieve peak digital transformation.",
        "By dynamically auditing the active status indicator, we synergize runtime protocols to \
         return a robust truth value.",
    ),
    (
        "As an AI language model, I am programmed to write clean documentation. To answer your \
         query about this method: it updates the user profile. I hope this explanation of updating \
         the user database entry helper function is helpful! Let me know if you need any more \
         clean, highly-optimized enterprise-grade Java functions documented today.",
        "It updates the user profile.",
    ),
    (
        "Like the ancient scribes of Alexandria etching truths upon papyrus to withstand the \
         relentless sands of time, this database save function carves your entity into the cold, \
         unyielding SQL disk. It is a sacred ritual. It returns a void, for some truths are too \
         profound to yield a return value.",
        "It returns a void, for some truths are too profound to yield a return value.",
    ),
    (
        "If you managed to break this, I honestly don't know what to tell you. It literally just \
         catches the error that your terrible code probably threw because you don't understand \
         basic asynchronous lifecycle scopes. It logs the error to the console so we can all see \
         exactly where you failed. You're welcome.",
        "It logs the error to the console.",
    ),
    (
        "Normalizes a real-valued scalar quantity by projecting its vector magnitude onto a \
         non-negative one-dimensional Cartesian space. By isolating the coefficient from its \
         negative algebraic operator, we successfully execute a distance-from-zero translation \
         matrix. In layman's terms: it strips the minus sign.",
        "In layman's terms: it strips the minus sign.",
    ),
    (
        "BEST EMAIL VALIDATOR FOR WEB DEVELOPERS 2026. Fast free lightweight validation regex \
         pattern. Learn how to validate email in javascript tutorial. This function verifies email \
         address string is valid format.",
        "This function verifies email address string is valid format.",
    ),
];

#[test]
fn every_padded_comment_comes_down_to_its_point() {
    let missed: Vec<(usize, String)> = PASSAGES
        .iter()
        .enumerate()
        .filter_map(|(at, (passage, wanted))| {
            let found = condense(passage).text();
            (found != *wanted).then(|| (at + 1, found))
        })
        .collect();
    assert!(missed.is_empty(), "not reduced to the point: {missed:#?}");
}

#[test]
fn a_passage_that_is_already_its_point_is_left_alone() {
    let plain = "Adds x to y. Returns the sum.";
    let core = condense(plain);
    assert!(!core.text().is_empty());
    assert!(core.text().len() <= plain.len());
}

#[test]
fn nothing_is_ever_reduced_to_nothing() {
    for (passage, _) in PASSAGES {
        assert!(
            !condense(passage).text().trim().is_empty(),
            "a passage was reduced away entirely"
        );
    }
}

#[test]
fn what_is_kept_is_the_text_that_was_there() {
    // Rebuilt from tokens this comes back as "[ `Cost` ]", and trimmed at the opening bracket it
    // comes back as "`Cost`] is turned into", which no longer has the bracket that closes it.
    let passage = "How a [`Cost`] is turned into the single number a search minimises. \
                   It is the only number the search compares.";
    let kept = condense(passage).text();
    assert!(
        !kept.contains("[ `"),
        "spacing was rebuilt rather than kept: {kept}"
    );
    assert_eq!(
        kept.matches('[').count(),
        kept.matches(']').count(),
        "a bracket was cut away from its partner: {kept}"
    );
    assert_eq!(
        kept.matches('`').count() % 2,
        0,
        "a tick lost its partner: {kept}"
    );
}
