//! What the engine writes, and where every word of it came from.
//!
//! There is nothing here that checks a sentence against a sentence somebody expected. What is
//! checked instead is that the writing is a function of the input: that every word was read out of
//! the input, that changing the input changes what is written, and that an input holding nothing
//! to say is refused rather than filled in. A test that pinned an expected sentence would be the
//! very thing this engine is built not to have, moved into the test suite.

use std::fs;
use std::path::{Path, PathBuf};

use clarity::read::{read_prose, read_tree};
use clarity_say::{compose, Said, MOST_CLAIMS};

/// Prose with enough shape for a sentence to be learned from it.
///
/// This is built a line at a time rather than held as one literal, so that what the engine is
/// given is plainly a body of text and not a sentence written here for it to find.
fn prose(lines: &[&str]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
        text.push('\n');
    }
    text
}

/// A tree of source under a directory of its own, so cases cannot collide.
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let home = std::env::temp_dir().join(format!("clarity-describe-{name}"));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).expect("a directory to work in");
    for (file, source) in files {
        fs::write(home.join(file), source).expect("a file to read");
    }
    home
}

/// What a note above a declaration is written with, in one language and in another.
///
/// The marks are held apart from the sentences so that neither fixture is a run of doc comments
/// sitting in this file. The crate audits its own prose by reading every doc comment in every file
/// under it, and a fixture written out in full would put these sentences into that audit.
const RUST: [&str; 2] = ["///", ""];

/// The two marks Rust writes a note about a whole file with, and a note about one thing with.
///
/// Held here for the same reason as the pair above: a fixture with these marks written out at the
/// start of a line is a doc comment as far as this crate's own prose audit is concerned, and the
/// audit would then read the fixture's line continuations as words nobody wrote.
const MODULE: &str = concat!("//", "!");
const ITEM: &str = concat!("//", "/");
const TYPESCRIPT: [&str; 2] = ["*", "/**"];

/// What each language declares the two things below as.
const RUST_DECLARATIONS: [&str; 2] = ["pub struct Ledger {}", "pub fn record() {}"];
const TYPESCRIPT_DECLARATIONS: [&str; 2] =
    ["export interface Ledger {}", "export function record() {}"];

/// The notes above two declarations, in the order the declarations come.
const LEDGER: [&[&str]; 2] = [
    (
        &[
            "A ledger records the money a household holds.",
            "",
            "Every entry names a date and an amount. The balance is the sum of the entries.",
        ]),
    (&[
            "Add an entry to the ledger.",
            "",
            "An entry is never removed once it is added. A mistaken entry is corrected by a further entry that reverses it.",
        ]),
];

/// Write the declarations out as a file in a language, marking each note the way it marks them.
fn noted(marks: [&str; 2], declarations: [&str; 2], notes: &[&[&str]]) -> String {
    let mut source = String::new();
    for (note, declared) in notes.iter().zip(declarations) {
        if !marks[1].is_empty() {
            source.push_str(marks[1]);
            source.push('\n');
        }
        for line in *note {
            source.push_str(marks[0]);
            source.push(' ');
            source.push_str(line);
            source.push('\n');
        }
        if !marks[1].is_empty() {
            source.push_str(" */");
            source.push('\n');
        }
        source.push_str(declared);
        source.push('\n');
        source.push('\n');
    }
    source
}

/// Everything the engine wrote about an input, as one line per clause.
fn written(text: &str) -> Vec<String> {
    let reading = read_prose(text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    lines(&said)
}

/// Read the clauses off a composition.
fn lines(said: &Said) -> Vec<String> {
    said.clauses()
        .expect("every clause was composed")
        .iter()
        .map(|clause| {
            clause
                .words()
                .filter_map(clarity_say::Slot::word)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// A body of prose about one subject, long enough to hold sentences and repetition.
fn ledger() -> String {
    prose(&[
        "A ledger records the money a household holds. Every entry names a date and an amount.",
        "The balance is the sum of the entries. A balance that disagrees with the bank is an \
         error in the ledger or an error in the bank.",
        "An entry is never removed. A mistaken entry is corrected by a further entry that \
         reverses it, so the history of the account survives its corrections.",
        "Interest is added by the bank once a month. The ledger records interest as an entry \
         like any other, because money that arrives is money the household holds.",
    ])
}

#[test]
fn every_word_written_was_read_out_of_the_input() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let mut words = 0;
    for clause in said.clauses().expect("every clause was composed") {
        for slot in clause.words() {
            let word = slot.word().expect("a word that was said");
            let at = slot.source().expect("the place it was read from");
            assert!(
                text.to_lowercase().contains(&word.to_lowercase()),
                "{word:?} was written but never read, so something here can speak on its own"
            );
            assert!(
                at.end > at.start,
                "{word:?} cites an empty place in the input"
            );
            words += 1;
        }
    }
    assert!(words > 0, "nothing was written, so this proves nothing");
}

#[test]
fn a_different_input_is_written_about_differently() {
    let first = written(&ledger());
    let second = written(&prose(&[
        "A kiln fires clay until it turns to stone. The heat is raised slowly and lowered slowly.",
        "Clay that dries unevenly cracks in the kiln. A potter dries a pot under cloth so that \
         the rim loses water no faster than the base.",
        "A glaze melts at a temperature of its own. A glaze fired below that temperature stays \
         rough, and one fired above it runs off the pot onto the shelf.",
        "The shelf is painted with a wash that the glaze will not stick to, because a pot stuck \
         to a shelf is two things broken instead of one.",
    ]));
    assert_ne!(
        first, second,
        "two unrelated inputs were written about in the same words"
    );
}

#[test]
fn a_reading_is_refused_when_the_input_says_nothing() {
    assert!(
        read_prose("").is_err(),
        "an empty input was read as though it held something"
    );
    assert!(
        read_prose("word word word word").is_err(),
        "text that never finishes a sentence was read as though it held one"
    );
}

#[test]
fn a_repository_is_written_about_from_what_its_authors_wrote() {
    let home = tree(
        "rust",
        &[("ledger.rs", &noted(RUST, RUST_DECLARATIONS, &LEDGER))],
    );
    let reading = read_tree(&home).expect("a tree the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let written = lines(&said).join(" ").to_lowercase();
    assert!(
        !written.trim().is_empty(),
        "a documented repository was written about in silence"
    );
    assert!(
        written.contains("ledger") || written.contains("entry"),
        "what was written is not about what the repository documents: {written:?}"
    );
}

#[test]
fn a_language_it_was_never_taught_is_read_the_same_way() {
    let home = tree(
        "typescript",
        &[(
            "ledger.ts",
            &noted(TYPESCRIPT, TYPESCRIPT_DECLARATIONS, &LEDGER),
        )],
    );
    let reading = read_tree(&home).expect("a tree the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let written = lines(&said).join(" ").to_lowercase();
    assert!(
        written.contains("ledger") || written.contains("entry"),
        "a note written above a declaration was not read: {written:?}"
    );
}

#[test]
fn the_words_of_a_property_are_the_words_the_input_used_for_it() {
    // Two subjects in one input, discussed in vocabularies that do not overlap. What is written
    // about each has to come from that one, or the composition is splicing unrelated passages.
    let text = prose(&[
        "The kiln fires clay until it turns to stone. The kiln is heated slowly and cooled \
         slowly, because clay that cools quickly cracks in the kiln.",
        "The ledger records the money a household holds. The ledger names a date and an amount \
         for every entry, and the balance is the sum of the entries in the ledger.",
    ]);
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    for clause in said.clauses().expect("every clause was composed") {
        let written = clause
            .words()
            .filter_map(clarity_say::Slot::word)
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let kiln = written.contains("kiln") || written.contains("clay");
        let ledger = written.contains("ledger") || written.contains("entries");
        assert!(
            !(kiln && ledger),
            "one clause was spliced out of two subjects: {written:?}"
        );
    }
}

#[test]
fn a_large_input_is_written_about_in_the_time_a_reader_will_wait() {
    let home = Path::new(env!("CARGO_MANIFEST_DIR"));
    let started = std::time::Instant::now();
    let reading = read_tree(home).expect("the crate can read itself");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let taken = started.elapsed();
    assert!(
        !lines(&said).is_empty(),
        "the crate wrote nothing about itself"
    );
    assert!(
        taken.as_secs() < 30,
        "writing took {taken:?}, which is longer than anyone waits"
    );
}

#[test]
fn the_words_of_a_clause_are_chosen_and_not_copied() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let mut skipped = false;
    for clause in said.clauses().expect("every clause was composed") {
        let places: Vec<_> = clause
            .words()
            .filter_map(clarity_say::Slot::source)
            .collect();
        for pair in places.windows(2) {
            assert!(
                pair[1].start >= pair[0].end,
                "a clause went back over a place it had used"
            );
            if pair[1].start > pair[0].end {
                skipped = true;
            }
        }
    }
    assert!(
        skipped,
        "every clause was a run of the input copied out, so nothing was chosen"
    );
}

#[test]
fn how_much_is_said_is_a_decision_and_not_a_setting() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let all = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let one = compose(reading.corpus(), reading.claims(), 1).expect("something to say");
    let all = all.clauses().expect("every clause was composed").len();
    let one = one.clauses().expect("every clause was composed").len();
    assert_eq!(
        one, 1,
        "a selection held to one part still stated {one} of them"
    );
    assert!(
        all > one,
        "the selection stated {all} parts whether or not it was held to fewer"
    );
}

#[test]
fn every_line_is_a_sentence_the_input_itself_wrote() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let source = squashed(&text);
    let mut checked = 0;
    for clause in said.clauses().expect("every clause was composed") {
        let line = squashed(&clause.text());
        assert!(
            source.contains(&line),
            "a line was written that the input never wrote: {line:?}"
        );
        checked += 1;
    }
    assert!(checked > 0, "nothing was written, so this proves nothing");
}

#[test]
fn no_two_words_were_put_together_by_the_engine() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    // Compared with the spacing taken out altogether, because the input writes a mark against the
    // word before it and a pair read off the clause has a space where the input had none.
    let source = tight(&text);
    let mut pairs = 0;
    for clause in said.clauses().expect("every clause was composed") {
        let words: Vec<String> = clause
            .words()
            .filter_map(clarity_say::Slot::word)
            .map(str::to_owned)
            .collect();
        for pair in words.windows(2) {
            let together = tight(&pair.join(" "));
            assert!(
                source.contains(&together),
                "two words were run together that the input never ran together: {together:?}"
            );
            pairs += 1;
        }
    }
    assert!(pairs > 0, "nothing was written, so this proves nothing");
}

/// Text with its spacing flattened, so that a line wrapped in the input still matches.
fn squashed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text with its spacing removed, for comparing two words the input may have written together.
fn tight(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().concat()
}

#[test]
fn what_comes_out_is_a_document_of_passages_and_not_a_list_of_lines() {
    let text = ledger();
    let reading = read_prose(&text).expect("prose the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    let passages = said.passages().expect("every clause was composed");
    let clauses: usize = passages.iter().map(Vec::len).sum();
    assert!(clauses > 2, "too little was said to prove anything");
    // The point of the test: a bare list of lines is one clause per passage throughout, so this
    // fails against one. What is asserted is not a shape chosen here but that the engine found
    // some part of the input whose claims belong together, which a list never does.
    assert!(
        passages.iter().any(|passage| passage.len() > 1),
        "every passage holds one clause, so this is a list and not a document"
    );
    // And it is a document rather than one wall of text: an input that says several unrelated
    // things is parted somewhere.
    assert!(
        passages.len() > 1,
        "everything landed in one passage, so nothing was parted"
    );
    for passage in &passages {
        for clause in passage {
            let text = clause.text();
            let opening = text.chars().next().expect("a clause that says something");
            assert!(
                opening.is_uppercase() || opening.is_ascii_punctuation() || opening.is_numeric(),
                "a passage begins mid-sentence: {text:?}"
            );
            let closing = text.chars().last().expect("a clause that says something");
            assert!(
                matches!(closing, '.' | '!' | '?' | '"' | ')' | '`'),
                "a clause stops without finishing: {text:?}"
            );
        }
    }
}

/// Build a repository of near-identical modules: the same summary line, one word apart.
fn same_thing_many_times(subjects: &[&str]) -> Vec<String> {
    let files: Vec<(String, String)> = subjects
        .iter()
        .map(|subject| {
            let doc = [
                format!("{MODULE} A bounded store of records held for the {subject} service."),
                MODULE.to_owned(),
                format!(
                    "{MODULE} Records are written once and are held until they expire. Nothing"
                ),
                format!("{MODULE} outside this service reaches them, and the expiry clock is the"),
                format!("{MODULE} only thing that removes one."),
                String::new(),
                format!("{ITEM} Put a record in, returning the position it was written at."),
                "pub fn put() -> usize { 0 }".to_owned(),
                String::new(),
                format!("{ITEM} Read the record at a position, if one is still held there."),
                "pub fn get() -> Option<usize> { None }".to_owned(),
            ];
            (format!("{subject}.rs"), doc.join("\n"))
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let home = tree(&format!("repetition-{}", subjects.len()), &borrowed);
    let reading = read_tree(&home).expect("a tree the engine can read");
    let said = compose(reading.corpus(), reading.claims(), MOST_CLAIMS).expect("something to say");
    lines(&said)
}

#[test]
fn what_is_reported_does_not_grow_with_how_often_the_input_repeats_itself() {
    // Nothing marks these modules as duplicates. Each summary line is a true, finished, distinct
    // sentence about a real part, and every one of them is worth stating on its own. What must
    // not happen is that stating one is never weighed against having stated another: if the
    // objective rewards two claims for sharing vocabulary it is monotone, the best subset is
    // every claim, and the report is an index that grows a line for every module added.
    let twelve = same_thing_many_times(&[
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu",
    ]);
    let twenty_four = same_thing_many_times(&[
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "phi",
        "chi", "psi", "omega",
    ]);
    assert!(
        !twelve.is_empty() && !twenty_four.is_empty(),
        "a repository that repeats itself was written about in silence"
    );
    assert!(
        twelve.len() * 3 <= 12,
        "twelve modules saying one thing were reported in {} lines: {twelve:#?}",
        twelve.len()
    );
    assert!(
        twenty_four.len() <= twelve.len(),
        "doubling the modules that say one thing grew the report from {} lines to {}: {twenty_four:#?}",
        twelve.len(),
        twenty_four.len()
    );
}
