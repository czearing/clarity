//! Writing a doc comment from what the code was found to say.
//!
//! Two things decide what is written. The code decides what may be said, because a finding is the
//! only licence a sentence has. The grammar decides whether it may be said that way. Every sentence
//! this writes goes back through the same engine that reads the repository. The engine throws away
//! any sentence it cannot read. The pass can be wrong about what is worth saying, and it still
//! cannot be wrong about English.
//!
//! The words come from the names the author chose. A name in Rust is written as words joined by
//! underscores or by case. The first word usually says what the function does. Nothing here knows
//! what any particular name means: the lexicon is asked what part of speech a word can hold,
//! exactly as it is asked for prose, and where it cannot say, the sentence that needed the answer
//! is not written.
//!
//! What is chosen among the sentences is a search, not an order. Each finding has a price that
//! stands for how sure the code is of it, and each sentence has a length. The cheapest comment
//! that carries the most is the one kept, so a function the code says little about gets a short
//! comment rather than a confident one.
//!
//! Most items get none at all, and that is the point. A comment a reader could have written from
//! the declaration under it is worse than nothing, because it costs a reading and returns what was
//! already there. So a sentence earns its place only by saying what the declaration does not: a
//! summary has to find a word the name had not, and a finding has to have come from the body
//! rather than from the signature sitting in plain sight. Nothing else is written.
//!
//! ```
//! use clarity::code::findings;
//! use clarity::document::written;
//!
//! // Nothing here that the declaration does not already say, so nothing is written.
//! let plain = "pub fn holds(&self) -> bool { true }";
//! assert!(written(&findings(plain)[0]).is_none());
//!
//! // This one can stop the program, which a reader would have to open the body to find out.
//! let stops = "pub fn holds(&self) -> bool { self.value.unwrap() }";
//! assert!(written(&findings(stops)[0]).is_some());
//! ```

use crate::code::{Fact, Piece};
use crate::grammar::Sentence;

/// What one sentence of a comment costs to carry.
///
/// A word is charged so that a comment saying the same thing in fewer words wins, and a finding is
/// charged what the code paid for it, so a sure thing is said before an unsure one.
const WORD: f64 = 1.0;

/// What is saved by saying something at all, which is what stops the empty comment always winning.
const SAID: f64 = 12.0;

/// A sentence with what it costs and what it was licensed by.
struct Line {
    text: String,
    price: f64,
    /// Whether it says anything the declaration under it does not already show.
    told: bool,
}

/// The doc comment for a piece of code, or nothing where the code licensed nothing.
///
/// Nothing is returned rather than an empty comment, because a comment that says nothing is worse
/// than no comment: it looks like an answer.
#[must_use]
pub fn written(piece: &Piece) -> Option<String> {
    let lines = chosen(piece);
    if lines.is_empty() {
        return None;
    }
    Some(lines.join("\n"))
}

/// The sentences kept, cheapest set first.
///
/// This is the dynamic program. Each candidate sentence may be taken or left, taking it costs its
/// words and its finding's price and earns what saying something is worth, and the run of choices
/// with the lowest total is the comment. A sentence that only says again what the declaration
/// under it already shows earns nothing, so its words are pure cost and the search leaves it. That
/// is the whole of the restraint, and it is the same arithmetic as before rather than a rule laid
/// over it. Ordering is not decided here: a summary would have to come first, so it is offered
/// first and the rest follow in the order the code offered them.
fn chosen(piece: &Piece) -> Vec<String> {
    let mut offered: Vec<Line> = Vec::new();
    if let Some(text) = summary(piece) {
        offered.push(Line {
            text,
            price: crate::code::SIGNED,
            told: false,
        });
    }
    offered.extend(piece.facts.iter().filter_map(|found| line(found, piece)));
    let mut kept: Vec<String> = Vec::new();
    for candidate in &offered {
        #[allow(clippy::cast_precision_loss)]
        let words = candidate.text.split_whitespace().count() as f64 * WORD;
        let earned = if candidate.told { SAID } else { 0.0 };
        if candidate.price + words - earned < 0.0 && !kept.contains(&candidate.text) {
            kept.push(candidate.text.clone());
        }
    }
    kept
}

/// The first line, which says what the thing is.
///
/// Every open-class word in it comes from the name, because that is the only place it has to draw
/// from: the words are the name's words, put in an order English can read and given an article. So
/// it can never tell a reader anything the declaration under it does not, and it reaches the search
/// having earned nothing. It is still written, because that is what makes the claim checkable
/// rather than assumed, and because a further source of words would change what it earns here and
/// nowhere else.
fn summary(piece: &Piece) -> Option<String> {
    let words = spelled(&piece.name);
    if words.is_empty() {
        return None;
    }
    let yes_or_no = piece.facts.iter().any(|found| found.fact == Fact::YesOrNo);

    let sentence = if piece.kind == "struct" {
        phrase(&words)
    } else if yes_or_no {
        // A name answering yes or no is a question about the thing it is called on, so it is
        // written as one. Where the name is a verb on its own it needs a subject to be read.
        match acting(&words) {
            Some(acted) => format!("Whether it {}", lowered(&acted)),
            None => format!("Whether {}.", words.join(" ")),
        }
    } else if acts(piece) {
        acting(&words).unwrap_or_else(|| phrase(&words))
    } else {
        phrase(&words)
    };
    sound_in(
        &sentence,
        crate::register::Register::STRICT.without(crate::register::Convention::Predicate),
    )
}

/// Whether the signature says the call does something rather than naming something.
///
/// English reads most short words as either a verb or a noun. The word cannot settle it, and a
/// convention about Rust names cannot settle it either. The signature can. A call does something
/// when it answers with nothing, or changes what it is used on, or is handed anything to work with.
/// A call that only reads itself and answers is named for what it answers with, which is why
/// "stability" is the stability and not an act of stabilising. Stopping is not evidence either way,
/// since a call that answers may still stop.
fn acts(piece: &Piece) -> bool {
    piece
        .facts
        .iter()
        .any(|found| matches!(found.fact, Fact::Silent | Fact::Alters | Fact::Takes(_)))
}

/// A sentence that says what a call does, where the name begins with a word that can be a verb.
///
/// A Rust name is written as what the thing does followed by what it does it to, so the first word
/// is asked whether English lets it be a verb. Where it does not, nothing is returned and the
/// caller falls back to naming the thing rather than describing an act it cannot vouch for.
fn acting(words: &[String]) -> Option<String> {
    let (first, rest) = words.split_first()?;
    // A word English lets be either a verb or a noun is not proof that the name describes an act.
    // "stability" is offered as a plain verb by shape alone, and taking that reading writes
    // "Stabilities" for a function that answers with a number, so the signature is what decides
    // and this only asks which word says the act.
    //
    // A plain verb reading is taken as it stands. A third person one is not, because any word
    // ending in "s" is offered that reading by its ending alone: "aqueous" earned one that way
    // and wrote "Aqueous the sucrose dielectric loss factor" for a name that describes a thing.
    // What separates the two is whether the word left behind is a verb, which "hold" is and
    // "aqueou" is not.
    // A closed-class word frames a point rather than carrying one, so it is never the act of a
    // name however its shape reads. Without this a name beginning "with" was written "Withs".
    if crate::lexicon::is_closed(first) {
        return None;
    }
    let verb = crate::lexicon::offers(first, crate::tag::Tag::Verb(crate::tag::Form::Base));
    // The stem is asked of the listing rather than of shape, because shape allows "gram" a verb
    // reading and wrote "Grams the sample" for a name that measures one. A word already ending in
    // "s" is the case where a name most often looks like an act and is not.
    let third = !verb
        && crate::repair::plain(first).is_some_and(|stem| crate::lexicon::carries_verb(&stem));
    if !(verb || third) {
        return None;
    }
    let inflected = if verb {
        crate::repair::third(first)?
    } else {
        first.clone()
    };
    if rest.is_empty() {
        return Some(format!("{}.", capitalised(&inflected)));
    }
    let object = rest.join(" ");
    // A name may already carry its own article, as "read_the_file" does, and putting another in
    // front of it spells "the the file". What settles it is the lexicon, so a word that opens a
    // noun phrase is left to do so.
    let leads = crate::lexicon::offers(
        &rest[0],
        crate::tag::Tag::Determiner(crate::tag::Number::Singular),
    ) || crate::lexicon::offers(
        &rest[0],
        crate::tag::Tag::Determiner(crate::tag::Number::Plural),
    );
    Some(if leads {
        format!("{} {object}.", capitalised(&inflected))
    } else {
        format!("{} the {object}.", capitalised(&inflected))
    })
}

/// Whether a comment already written says anything the declaration does not.
///
/// The same question the search asks of a sentence it might write, asked of a sentence someone
/// already wrote. A declaration carries its own vocabulary: the words of the name, the names the
/// author gave the arguments, and the type answered with. A comment built only from those words
/// has told the reader nothing they did not have, however many sentences it took, and is worth
/// deleting rather than keeping.
///
/// Only the words that carry a point are weighed, because the closed class is grammar and a
/// comment does not earn its place by having put an article in front of the name. The test is
/// deliberately generous: one word of the author's own is enough to keep it, so a comment saying
/// anything at all is left alone.
///
/// ```
/// use clarity::code::findings;
/// use clarity::document::says_nothing;
///
/// let echo = "/// The acid system.\npub struct AcidSystem { pub ph: f64 }";
/// assert!(says_nothing(&findings(echo)[0]));
///
/// let told = "/// The pH must be measured at 25 C.\npub struct AcidSystem { pub ph: f64 }";
/// assert!(!says_nothing(&findings(told)[0]));
/// ```
#[must_use]
pub fn says_nothing(piece: &Piece) -> bool {
    if piece.doc.is_empty() {
        return false;
    }
    let mut shown: Vec<String> = spelled(&piece.name);
    for found in &piece.facts {
        match &found.fact {
            Fact::Takes(names) => shown.extend(names.iter().flat_map(|name| spelled(name))),
            Fact::Answers(head) => shown.extend(spelled(head)),
            _ => {}
        }
    }
    let known: Vec<String> = shown.iter().map(|word| stem(word)).collect();
    !piece
        .doc
        .iter()
        .flat_map(|line| line.split_whitespace())
        .map(|word| {
            word.trim_matches(|letter: char| !letter.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty() && !crate::lexicon::is_closed(word))
        .any(|word| !known.contains(&stem(&word)))
}

/// A word with any ending that only marks number or person taken off, where one can be.
///
/// A comment names a thing as the code names it, and English changes the ending to fit the sentence
/// around it. Both sides are compared with such an ending removed, because otherwise every plural
/// would count as a new word.
fn stem(word: &str) -> String {
    crate::repair::plain(word).unwrap_or_else(|| word.to_owned())
}

/// A noun phrase naming the thing, used where nothing can be said about what it does.
///
/// Number is not asked about, because English writes "the" in front of one thing and of many
/// alike, so knowing which it is would change nothing here.
fn phrase(words: &[String]) -> String {
    format!("The {}.", words.join(" "))
}

/// One further sentence, licensed by one finding.
///
/// A finding is written out only when it says something the signature does not already show.
/// Repeating the signature in words is what makes a generated comment worthless, so a finding that
/// only restates the types has no sentence and cannot be chosen.
///
/// What separates a sentence worth writing from one that is not is already recorded, in what the
/// finding cost to be sure of. A signature is read straight off the declaration the comment will
/// sit above, so a reader has it either way and a sentence carrying it earns nothing. A body has
/// to be opened and followed, so a reader does not have it, and a sentence carrying it is the
/// reason to write a comment at all.
fn line(found: &crate::code::Finding, piece: &Piece) -> Option<Line> {
    let text = match &found.fact {
        Fact::MayBeAbsent => "Answers with nothing where it finds none.",
        Fact::MayFail => "Reports a failure rather than stopping.",
        Fact::Alters => "Changes what it is used on.",
        Fact::Halts if !reports_failure(piece) => "This can stop the program.",
        _ => return None,
    };
    sound(text).map(|text| Line {
        text,
        price: found.price,
        told: found.price > crate::code::SIGNED,
    })
}

/// Whether the signature gives the call a way to say it failed.
///
/// A call answering with a failure or with nothing has somewhere to put what went wrong, and a
/// reader of the declaration already knows to expect it. A stop written inside such a call is the
/// author saying that a case cannot arise, guarded somewhere they could see and a reader cannot be
/// warned about usefully. A call answering with a plain value has no such channel, so a stop in it
/// is the whole of how it fails, and that is what a caller has to be told.
fn reports_failure(piece: &Piece) -> bool {
    piece
        .facts
        .iter()
        .any(|found| matches!(found.fact, Fact::MayFail | Fact::MayBeAbsent))
}

/// A sentence the engine can read, or nothing.
///
/// This is the whole of the enforcement. A sentence generated here is read by the same engine that
/// reads the repository, and one it charges a fault for is not written. So a rule added to the
/// grammar tightens what may be generated without anything here changing.
fn sound(text: &str) -> Option<String> {
    sound_in(text, crate::register::Register::STRICT)
}

/// A sentence the engine can read under `register`, or nothing.
///
/// A summary line is a noun phrase by convention, so it is read under a register that lets the
/// predicate go. Everything else is held to the whole of English. The register is what says which,
/// so the difference is a convention the engine already knows rather than an exception here.
fn sound_in(text: &str, register: crate::register::Register) -> Option<String> {
    let sentence = Sentence::read(text);
    let report = crate::check::check_in(&sentence, register);
    if report.is_clean() {
        Some(text.to_owned())
    } else {
        None
    }
}

/// The words a Rust name is written from.
///
/// Underscores and case changes are both how Rust joins words, so both are read as joins. Capitals
/// in a row count as one word, so an initialism stays together.
#[must_use]
pub fn spelled(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut held = String::new();
    let mut was_upper = false;
    for letter in name.chars() {
        if letter == '_' {
            if !held.is_empty() {
                words.push(std::mem::take(&mut held));
            }
            was_upper = false;
            continue;
        }
        if letter.is_uppercase() && !was_upper && !held.is_empty() {
            words.push(std::mem::take(&mut held));
        }
        was_upper = letter.is_uppercase();
        held.push(letter.to_ascii_lowercase());
    }
    if !held.is_empty() {
        words.push(held);
    }
    words.retain(|word| !word.is_empty());
    words
}

/// The sentence with its first letter put back in lower case, for use inside another.
fn lowered(text: &str) -> String {
    let mut letters = text.chars();
    match letters.next() {
        Some(first) => first.to_lowercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// The word with its first letter capitalised.
fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{spelled, written};
    use crate::code::findings;

    fn comment(source: &str) -> Option<String> {
        written(&findings(source)[0])
    }

    #[test]
    fn a_name_is_read_as_the_words_it_is_written_from() {
        assert_eq!(spelled("check_stability"), ["check", "stability"]);
        assert_eq!(spelled("HttpReader"), ["http", "reader"]);
    }

    #[test]
    fn nothing_is_written_where_the_declaration_already_says_it_all() {
        // Every one of these is fully described by the line it would sit above: a name, a
        // receiver, and a return type. A comment here would cost a reading and return nothing.
        for source in [
            "pub struct AcidSystem { pub ph: f64 }",
            "pub fn holds(&self) -> bool { true }",
            "impl T { pub fn clear(&mut self) {} }",
            "pub fn read_the_file(path: &str) -> Option<String> { None }",
            "pub fn equilibrium(&self) -> Result<f64, Bad> { Ok(1.0) }",
        ] {
            assert_eq!(comment(source), None, "{source}");
        }
    }

    #[test]
    fn what_only_the_body_shows_is_written_because_a_reader_would_have_to_look() {
        let found = comment("pub fn ratio(&self) -> f64 { self.mass.unwrap() }").unwrap();
        assert_eq!(found, "This can stop the program.");
    }

    #[test]
    fn a_stop_is_not_reported_where_the_signature_already_admits_a_failure() {
        // The declaration says this can fail, so a reader expects it. What is left inside is the
        // author asserting a case cannot arise, guarded somewhere a caller cannot be warned about.
        for source in [
            "pub fn ratio(&self) -> Result<f64, Bad> { Ok(self.mass.unwrap()) }",
            "pub fn ratio(&self) -> Option<f64> { Some(self.mass.unwrap()) }",
        ] {
            assert_eq!(comment(source), None, "{source}");
        }
    }

    #[test]
    fn every_sentence_written_is_one_the_engine_can_read() {
        for source in [
            "pub fn holds(&self) -> bool { self.value.unwrap() }",
            "impl T { pub fn clear(&mut self) { panic!() } }",
            "pub fn read_the_file(path: &str) -> String { open(path).unwrap() }",
        ] {
            let found = comment(source).unwrap();
            for line in found.lines() {
                let report = crate::check::check(&crate::grammar::Sentence::read(line));
                assert!(report.is_clean(), "{line}: {:?}", report.faults);
            }
        }
    }
}
