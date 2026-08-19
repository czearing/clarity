//! Turning an input into the two things the engine needs, whatever the input is.
//!
//! The engine takes a corpus and a set of claims and knows nothing else. So everything that is
//! particular to a kind of input lives here, and nothing that is particular to a kind of input
//! lives anywhere else. A source tree, an article and a book differ only in how they are cut into
//! parts and where the words are found; once read they are the same two values, and the writing
//! that follows cannot tell which it was given.
//!
//! Nothing here decides what anything means. Reading a repository means finding its parts, taking
//! the words its authors already wrote, and recording which words were written about which part.
//! What those words will be used to say is settled by a search that runs later and elsewhere.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clarity_say::{Claim, Corpus, Feature, MOST_CLAIMS};
use fitkit::{Answer, Confidence, Evidence, Refusal, Span};

use crate::code::{findings, Fact};

/// An input, read: the language it is written in and the parts worth writing about.
///
/// The corpus and the claims come from the same pass over the same text, so every claim is about
/// something the corpus has words for. That is not a convention observed here; it is what makes a
/// composition possible at all, since a claim whose words were never read cannot be written.
#[derive(Debug)]
pub struct Reading {
    corpus: Corpus,
    claims: Vec<Claim>,
}

impl Reading {
    /// The language learned from the input.
    #[must_use]
    pub const fn corpus(&self) -> &Corpus {
        &self.corpus
    }

    /// The parts of the input worth writing about.
    #[must_use]
    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }
}

/// A part of the input, gathered before it is priced.
struct Part {
    feature: Feature,
    span: Span,
    /// How many things the part holds, which is what makes it worth mentioning.
    weight: usize,
    /// How many of those things the input says something about, which is how far it is trusted.
    spoken: usize,
}

/// Read a tree of source files: its parts are its files, and its words are its authors'.
///
/// A file's own doc comments and the identifiers its author chose are the only vocabulary this
/// gets. A repository that documents nothing offers nothing to say and is refused, which is the
/// correct outcome and not a failure: there is no sentence about a file nobody described.
///
/// # Errors
///
/// Refuses a tree that cannot be read, holds no source, or says nothing about itself.
pub fn read_tree(root: &Path) -> Answer<Reading> {
    let mut corpus = Corpus::new();
    let mut parts: Vec<Part> = Vec::new();
    let mut at = 0usize;
    let mut files = Vec::new();
    gather(root, &mut files);
    if files.is_empty() {
        return Err(Refusal::unreported("no source was found to read"));
    }
    files.sort();
    for path in files {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let start = at;
        let feature = part_of(&path);
        let items = if path.extension().and_then(|end| end.to_str()) == Some("rs") {
            findings(&source)
                .into_iter()
                .map(|piece| Item {
                    kind: Feature::of(piece.kind),
                    shapes: piece
                        .facts
                        .iter()
                        .map(|finding| shape_of(&finding.fact))
                        .collect(),
                    name: split(&piece.name),
                    doc: parted(&piece.doc),
                    documented: piece.documented,
                })
                .collect()
        } else {
            commented(&source)
        };
        let mut spoken = 0usize;
        for item in &items {
            let mut features = vec![feature, item.kind];
            features.extend(item.shapes.iter().copied());
            // The name its author chose is evidence about the item, in words a reader already
            // associates with it.
            corpus.attach(&features, &item.name, Span::new(at, at + item.name.len()));
            at += item.name.len();
            if item.documented {
                // A paragraph at a time, because a break between paragraphs is a break the author
                // put there. Run them together and a heading joins the sentence beneath it, and
                // the engine reports a line no author wrote as though one had.
                for paragraph in &item.doc {
                    corpus.attach(&features, paragraph, Span::new(at, at + paragraph.len()));
                    at += paragraph.len();
                }
                spoken += 1;
            }
        }
        let pieces = items;
        if pieces.is_empty() {
            continue;
        }
        parts.push(Part {
            feature,
            span: Span::new(start, at.max(start + 1)),
            weight: pieces.len(),
            spoken,
        });
    }
    settle(corpus, &parts)
}

/// Read prose: its parts are its paragraphs, and its words are its own.
///
/// This is the whole of what an article or a book needs. The parts of a document are the places
/// it starts a new one, which is a property of the text rather than of the subject, so the same
/// reading works on an encyclopedia entry and on a chapter.
///
/// # Errors
///
/// Refuses text with no paragraphs, or that yields nothing worth writing about.
pub fn read_prose(text: &str) -> Answer<Reading> {
    let mut corpus = Corpus::new();
    let mut parts = Vec::new();
    let mut at = 0usize;
    for (number, paragraph) in paragraphs(text).into_iter().enumerate() {
        let feature = Feature::keyed(number as u64);
        let span = Span::new(at, at + paragraph.len());
        corpus.attach(&[feature], paragraph, span);
        at += paragraph.len();
        let weight = paragraph.split_whitespace().count();
        parts.push(Part {
            feature,
            span,
            weight,
            spoken: weight,
        });
    }
    settle(corpus, &parts)
}

/// Price the parts by what was found in them and hand back a reading.
///
/// Two measurements, both taken from the input. How much a part holds is what makes it worth
/// mentioning; how much of it the input actually describes is how far a claim about it is
/// trusted. A part nobody wrote a word about is carried at the trust that deserves.
// A count of things in an input. A count large enough to lose a bit here is an input nobody has,
// and a share taken from one reads the same either way.
#[allow(clippy::cast_precision_loss)]
fn settle(mut corpus: Corpus, parts: &[Part]) -> Answer<Reading> {
    if parts.is_empty() {
        return Err(Refusal::unreported("the input has no parts to write about"));
    }
    corpus.settle();
    if corpus.terminator().is_none() {
        return Err(Refusal::unreported("the input never finishes a sentence"));
    }
    let largest = parts
        .iter()
        .map(|part| part.weight)
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let mut claims = Vec::new();
    let mut ranked: Vec<&Part> = parts.iter().collect();
    ranked.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then(a.span.start.cmp(&b.span.start))
    });
    for part in ranked.into_iter().take(MOST_CLAIMS) {
        let share = part.weight as f64 / largest;
        let told = if part.weight == 0 {
            0.0
        } else {
            part.spoken as f64 / part.weight as f64
        };
        if told <= 0.0 {
            continue;
        }
        let evidence = Evidence::new(part.span, Confidence::new(told), share);
        if let Ok(claim) = Claim::new(part.feature, evidence) {
            claims.push(claim);
        }
    }
    if claims.is_empty() {
        return Err(Refusal::unreported(
            "the input says nothing about any of its parts",
        ));
    }
    // Back into the order the input presents them, because a document that reports itself out of
    // order is reporting something the input did not say.
    claims.sort_by_key(|claim| claim.source().start);
    Ok(Reading { corpus, claims })
}

/// One thing an author wrote about, however the file it sits in is written.
struct Item {
    /// What the author called the kind of thing it is, hashed so the word cannot reach a reader.
    kind: Feature,
    /// What the code was found to do, when the language is one whose meaning was read.
    shapes: Vec<Feature>,
    /// The words the author built the name out of.
    name: String,
    /// What the author wrote about it, in the paragraphs they wrote it in.
    doc: Vec<String>,
    documented: bool,
}

/// The paragraphs of a note, which are the runs of lines its author left unbroken.
fn parted(lines: &[String]) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            if !run.is_empty() {
                paragraphs.push(run.join(" "));
                run.clear();
            }
            continue;
        }
        run.push(line);
    }
    if !run.is_empty() {
        paragraphs.push(run.join(" "));
    }
    paragraphs
}

/// Read a file in any language by the one convention every language shares: a note written above
/// the thing it is about.
///
/// Nothing here knows a language. A run of comment lines is a note; the first line of code under
/// it is the thing the note is about; the first word of that line is what its author calls that
/// kind of thing, and the rest are the words of its name. That is enough for a TypeScript
/// interface, a note above a Python function, or a type declared in Go, and none of it is a rule
/// about any of them.
fn commented(source: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut note: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(said) = uncomment(trimmed) {
            note.push(said);
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let mut names = declared(trimmed);
        if names.is_empty() {
            note.clear();
            continue;
        }
        let kind = Feature::of(&names.remove(0));
        if names.is_empty() {
            note.clear();
            continue;
        }
        let doc = parted(&std::mem::take(&mut note));
        let documented = !doc.is_empty();
        items.push(Item {
            kind,
            shapes: Vec::new(),
            name: names
                .iter()
                .map(|name| split(name))
                .collect::<Vec<_>>()
                .join(" "),
            doc,
            documented,
        });
    }
    items
}

/// What a line says once the marks that make it a comment are taken off it, if it is one.
///
/// Every language marks its notes with punctuation and then writes words. Taking the punctuation
/// off the front and the back leaves the words, and leaves a line that is not a note alone.
fn uncomment(line: &str) -> Option<String> {
    let opens = ["///", "//!", "//", "/**", "/*", "*/", "#", "<!--", "--"];
    let mut rest = None;
    for open in opens {
        if let Some(tail) = line.strip_prefix(open) {
            rest = Some(tail);
            break;
        }
    }
    // A line inside a block note is continued with a single mark, but a bare mark elsewhere is
    // multiplication, so this only counts when what follows it is a space or nothing.
    let rest = rest.or_else(|| {
        line.strip_prefix('*')
            .filter(|tail| tail.is_empty() || tail.starts_with(' '))
    })?;
    let rest = rest.trim_end_matches("-->").trim_end_matches("*/").trim();
    // A word written against the mark that introduces it is naming a part of the note rather than
    // saying anything, so it is dropped and what it introduces is kept.
    let rest = rest.strip_prefix('@').map_or(rest, |tagged| {
        tagged
            .split_once(char::is_whitespace)
            .map_or("", |(_, said)| said)
    });
    Some(rest.trim().to_owned())
}

/// The words a line of code declares: what kind of thing it is, then what it is called.
///
/// A declaration opens with words and then reaches punctuation that is the language's rather than
/// the author's. Everything up to that punctuation was chosen by a person, which is what makes it
/// worth reading; everything after it is syntax.
fn declared(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for letter in line.chars() {
        if letter.is_alphanumeric() || letter == '_' || letter == '$' {
            current.push(letter);
        } else if letter.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            break;
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Every source file under a root, ignoring what a build leaves behind.
fn gather(root: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            gather(&path, found);
        } else if path
            .extension()
            .and_then(|end| end.to_str())
            .is_some_and(|end| READABLE.contains(&end))
        {
            found.push(path);
        }
    }
}

/// The kinds of file this can read words out of.
///
/// Not a list of languages it understands. A file is worth opening when a person wrote sentences
/// in it, and these are the endings under which people do.
const READABLE: [&str; 14] = [
    "rs", "ts", "tsx", "js", "jsx", "mjs", "go", "py", "java", "c", "h", "cpp", "cs", "swift",
];

/// The part a file belongs to, keyed by where it sits rather than by what it is called.
fn part_of(path: &Path) -> Feature {
    Feature::of(&path.to_string_lossy())
}

/// A key for one shape the code was found to have.
///
/// The label is hashed on the way in and never stored, so none of these words can reach a reader.
/// They are here to tell one shape from another and for nothing else.
fn shape_of(fact: &Fact) -> Feature {
    match fact {
        Fact::MayBeAbsent => Feature::keyed(1),
        Fact::MayFail => Feature::keyed(2),
        Fact::Many => Feature::keyed(3),
        Fact::YesOrNo => Feature::keyed(4),
        Fact::Number => Feature::keyed(5),
        Fact::Alters => Feature::keyed(6),
        Fact::Reads => Feature::keyed(7),
        Fact::Silent => Feature::keyed(8),
        Fact::Halts(_) => Feature::keyed(9),
        Fact::Misnumbered(..) => Feature::keyed(10),
        Fact::Takes(_) => Feature::keyed(11),
        Fact::Answers(_) => Feature::keyed(12),
    }
}

/// Split an identifier into the words its author built it from.
///
/// Programmers write several words as one token and mark the joins, by a separator or by a change
/// of case. Undoing that recovers words somebody chose, which is why an undocumented item is not
/// silent: its name was written by the same person, in the same vocabulary.
fn split(name: &str) -> String {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for letter in name.chars() {
        if letter == '_' || letter == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous_lower = false;
        } else {
            // Only a lowercase letter followed by a capital marks a join. A run of capitals is one
            // word somebody shouted, not a word for every letter in it.
            if letter.is_uppercase() && previous_lower && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            current.push(letter.to_ascii_lowercase());
            previous_lower = letter.is_lowercase() || letter.is_numeric();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words.join(" ")
}

/// Cut text where it starts again, which is where a blank line is.
fn paragraphs(text: &str) -> Vec<&str> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

/// How many parts of each kind were read, for a caller that wants to report on the reading.
#[must_use]
pub fn tally(reading: &Reading) -> BTreeMap<&'static str, usize> {
    let mut counted = BTreeMap::new();
    counted.insert("claims", reading.claims.len());
    counted.insert("words", reading.corpus.vocabulary());
    counted.insert("tokens", reading.corpus.tokens());
    counted
}
