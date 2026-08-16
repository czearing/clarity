//! Cutting a passage down to the point it was written to make.
//!
//! Verbose writing is not wrong sentence by sentence. Every sentence in a padded doc comment can
//! parse, agree, and close correctly while the passage as a whole says one thing in six sentences.
//! No rule of grammar is broken, so [`check`](crate::check) has nothing to report, and no single
//! sentence is at fault, so [`style`] can only trim the edges. What is wrong is the
//! *selection*: most of the passage carries nothing the reader came for.
//!
//! So the choice of what to keep is made once, over the whole passage, as a search. Each sentence
//! is priced by what it costs a reader and credited with the content it carries, and the cheapest
//! selection that still carries the content wins. This is a weighted set cover, solved exactly by
//! a walk over subsets of the content rather than by taking the best sentence at each step, since
//! two sentences that each look weak can together be the only ones that cover the point.
//!
//! # What counts as content
//!
//! Nothing here holds a list of technical words, and nothing here holds a list of empty ones. Both
//! are inferred, because a list would only work for the domain it was written for.
//!
//! A term carries content when it is *concrete* and *returned to*. Concreteness is read off the
//! word's own shape: English builds its abstractions with a small closed set of endings, and a
//! word wearing one of them names an idea rather than a thing. "Transformation", "alignment", and
//! "synergy" are the passage talking about itself; "string", "error", and "length" are the passage
//! talking about the code. Being returned to is read off the passage: a writer padding a comment
//! reaches for a different grand word every clause, while the thing being documented is named
//! again and again because there is no other way to name it.
//!
//! Neither signal alone is enough, and that is the point of combining them. A repeated empty word
//! stays empty, and a concrete word used once in passing is not what the passage is about.

use crate::clarity;
use crate::grammar::Sentence;
use crate::lexicon;
use crate::register::{Convention, Register};
use crate::style::{self, Flaw};
use crate::tag::Tag;
use crate::text::Text;
use crate::token::Token;

/// How many content terms the cover is solved over.
///
/// The walk over subsets is exponential in this, so it is held low enough to stay instant and high
/// enough to hold everything a paragraph is actually about. Terms past it are ranked out by weight,
/// which means they were the least concrete or the least returned to.
const TERMS: usize = 12;

/// What one term of content is worth keeping a sentence for.
const TERM: f64 = 1.0;

/// What one sentence costs a reader per token, in the same units as the content it carries.
///
/// Set so that a sentence must carry about one term of content per eight words to be worth keeping.
const TOKEN: f64 = 0.125;

/// What one unit of integration cost adds to a sentence's price.
const STRAIN: f64 = 0.05;

/// What each letter past `PLAIN` in a content word adds to a sentence's price.
const LATINATE: f64 = 0.15;

/// How long a content word may be before it starts costing the reader.
///
/// English keeps its everyday vocabulary short and borrows its grand vocabulary from Latin, so
/// length is a usable stand-in for how far a word sits from plain speech. This is not the syllable
/// counting that [`clarity`](crate::clarity) argues against: nothing here claims a long word is
/// hard to read. The claim is narrower and about choice rather than difficulty. Where a passage
/// says the same thing twice, once in borrowed vocabulary and once in native, the native saying is
/// the one the writer would have kept, and length is what tells them apart.
const PLAIN: usize = 6;

/// What one flaw or broken rule adds to a sentence's price.
///
/// A sentence the checker faults is one the reader has to work out, so it is priced alongside the
/// wording flaws rather than reported separately and then ignored by the choice of what to keep.
const FLAW: f64 = 0.5;

/// Endings that build an abstract noun out of something else.
///
/// This is the closed set English uses. A word wearing one of these names an idea, and a
/// passage made of them is describing itself rather than its subject.
const ABSTRACT: &[&str] = &[
    "tion", "sion", "ment", "ance", "ence", "ity", "ism", "ness", "ology", "ency", "ancy", "ship",
    "hood", "dom",
];

/// The result of cutting a passage down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Core {
    /// The sentences worth keeping, in the order they were written.
    pub kept: Vec<String>,
    /// How many sentences carried nothing the reader came for.
    pub dropped: usize,
    /// The terms the passage turned out to be about, most central first.
    pub about: Vec<String>,
}

impl Core {
    /// The kept sentences as one passage.
    #[must_use]
    pub fn text(&self) -> String {
        self.kept.join(" ")
    }
}

/// One candidate sentence, priced and credited.
struct Candidate {
    trimmed: String,
    price: f64,
    carries: u16,
}

/// Cut a passage down to the sentences that carry its point.
///
/// # Panics
///
/// Never. The unwraps below are on comparisons of finite prices.
#[must_use]
pub fn condense(passage: &str) -> Core {
    let text = Text::read(passage);
    let about = terms(&text.units);
    let candidates: Vec<Candidate> = text.units.iter().map(|unit| price(unit, &about)).collect();

    let chosen = choose(&candidates, about.len());
    let kept: Vec<String> = chosen
        .iter()
        .map(|&at| candidates[at].trimmed.clone())
        .collect();
    Core {
        dropped: candidates.len() - kept.len(),
        kept,
        about: about.into_iter().map(|(word, _)| word).collect(),
    }
}

/// The selection of sentences worth what they cost.
///
/// A walk over subsets of the content, in sentence order. The state is which terms are already
/// held, so a sentence that repeats ground already covered is credited with nothing and keeps only
/// its price, which is how a passage that says one thing six times is reduced to saying it once.
///
/// Coverage is not forced. Forcing it would guarantee that the sentence with the most unusual words
/// survives, and in padded writing that is exactly the worst sentence. Instead each term is worth a
/// fixed amount and each sentence costs what it costs, and a sentence is kept only when it brings
/// back more than it takes. A passage that is all padding keeps nothing, which is the honest answer.
fn choose(candidates: &[Candidate], terms: usize) -> Vec<usize> {
    let width = 1usize << terms;
    let mut best = vec![f64::NEG_INFINITY; width];
    let mut from: Vec<Option<(usize, usize)>> = vec![None; width];
    best[0] = 0.0;

    for (at, one) in candidates.iter().enumerate() {
        let mut next = best.clone();
        let mut step = from.clone();
        for (held, &worth) in best.iter().enumerate() {
            if !worth.is_finite() {
                continue;
            }
            let after = held | usize::from(one.carries);
            let gained = f64::from(after.count_ones() - held.count_ones()) * TERM;
            let total = worth + gained - one.price;
            if total > next[after] {
                next[after] = total;
                step[after] = Some((held, at));
            }
        }
        best = next;
        from = step;
    }

    let mut held = 0;
    let mut top = f64::NEG_INFINITY;
    for (mask, &value) in best.iter().enumerate() {
        if value > top {
            top = value;
            held = mask;
        }
    }

    if from[held].is_none() {
        // Every sentence costs more than it brings back, which happens when a passage is padding
        // around a single point. Dropping all of it would lose the point along with the padding,
        // so the one sentence that comes closest to paying for itself is kept.
        // Carrying the point comes first and cheapness second. The other order would pick whichever
        // sentence was shortest, and the shortest sentence in a padded passage is usually a sign
        // off rather than the point.
        return (0..candidates.len())
            .min_by(|&left, &right| {
                let held = |at: usize| candidates[at].carries.count_ones();
                held(right).cmp(&held(left)).then_with(|| {
                    candidates[left]
                        .price
                        .partial_cmp(&candidates[right].price)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            })
            .into_iter()
            .collect();
    }

    let mut chosen = Vec::new();
    while let Some((before, at)) = from[held] {
        chosen.push(at);
        if before == held {
            break;
        }
        held = before;
    }
    chosen.reverse();
    chosen.sort_unstable();
    chosen
}

/// Price one sentence and credit it with the terms it carries.
fn price(unit: &Sentence, about: &[(String, f64)]) -> Candidate {
    let report = crate::check::check(unit);
    let tags = report.tags.clone();
    let notes = style::read(unit, &tags);
    let strain = clarity::score(&tags);
    let flaws = notes
        .iter()
        .filter(|note| !matches!(note.flaw, Flaw::Echo))
        .count();

    let mut carries = 0u16;
    for token in &unit.tokens {
        if let Some(at) = about.iter().position(|(word, _)| *word == stem(&token.key)) {
            carries |= 1 << at;
        }
    }

    let latinate: usize = unit
        .tokens
        .iter()
        .filter(|token| bears_content(token))
        .map(|token| token.key.len().saturating_sub(PLAIN))
        .sum();

    Candidate {
        trimmed: trim(unit, &tags, about, carries),
        #[allow(clippy::cast_precision_loss)]
        price: unit.tokens.len() as f64 * TOKEN
            + strain.cost as f64 * STRAIN
            + (flaws + report.faults.len()) as f64 * FLAW
            + latinate as f64 * LATINATE,
        carries,
    }
}

/// Cut the framing off a sentence that the sentence does not need.
///
/// Padded writing wraps its point in an approach and a sign off: a clause that says the writer is
/// about to make a point, and a clause that comments on having made it. Neither is removable by a
/// rule about wording, because both are grammatical, and neither is removable by dropping the
/// sentence, because the point is inside it.
///
/// So the same question is asked one level down. A stretch at the front or the back is cut when
/// what remains still carries every term the whole sentence carried and still reads as a sentence
/// on its own. Nothing is decided by recognising an opening phrase, which would only ever catch
/// the openings someone thought to list. The test is whether the passage loses anything, and an
/// approach loses nothing by definition, since it is the writer clearing their throat.
fn trim(unit: &Sentence, tags: &[Tag], about: &[(String, f64)], carries: u16) -> String {
    let mut best = unit.tokens.len();
    let mut start = 0;

    for at in 0..unit.tokens.len().saturating_sub(1) {
        if !matches!(tags.get(at), Some(Tag::Mark(_))) || unit.tokens[at].ends_sentence() {
            continue;
        }
        if holds(&unit.tokens[at + 1..], about) == carries
            && stands_alone(&unit.tokens[at + 1..], &tags[at + 1..], true)
        {
            start = at + 1;
        }
    }

    for at in (start + 1..unit.tokens.len()).rev() {
        let breaks = matches!(
            tags.get(at),
            Some(Tag::Mark(_) | Tag::Subordinator | Tag::Coordinator(_))
        );
        if !breaks {
            continue;
        }
        let at = opens(&tags[start..at]) + start;
        if at <= start {
            continue;
        }
        if holds(&unit.tokens[start..at], about) == carries
            && stands_alone(&unit.tokens[start..at], &tags[start..at], start > 0)
            && severable(&tags[at..])
        {
            best = at;
        }
    }

    let kept = &unit.tokens[start..best];
    // What is left is put back to the checker before it is offered. A cut that drops a subject and
    // leaves "does not put a stop in the middle of a word" is not a shorter sentence, it is a
    // broken one, and the rule that says so is already written down. Judging the offer by the same
    // reader that judges the prose is what keeps the two from disagreeing.
    let loose = Register::STRICT.without(Convention::Marks);
    let kept = if faults(
        &Sentence {
            tokens: kept.to_vec(),
        },
        loose,
    ) > faults(unit, loose)
    {
        &unit.tokens[..]
    } else {
        kept
    };
    let start = if kept.len() == unit.tokens.len() {
        0
    } else {
        start
    };
    let mut text = Sentence {
        tokens: kept.to_vec(),
    }
    .text();
    if let Some(first) = text.chars().next() {
        if start > 0 {
            text = first.to_uppercase().collect::<String>() + &text[first.len_utf8()..];
        }
    }
    // A cut made at a comma leaves the comma behind, and a comma before a full stop is a mark
    // waiting for a clause that is no longer there.
    while text.ends_with([',', ';', ':']) {
        text.pop();
    }
    if !text.ends_with('.') && !text.ends_with('?') && !text.ends_with('!') {
        text.push('.');
    }
    text
}

/// How much a register holds against a stretch of tokens.
fn faults(sentence: &Sentence, register: Register) -> usize {
    crate::check::check_in(sentence, register).faults.len()
}

/// Where the connective at the end of a stretch begins.
///
/// A connective is not always one word. "rather than", "as though" and "so that" each join a
/// clause the way a single subordinator does, and a cut placed between their two words leaves the
/// first stranded: what remains ends on "rather", which is not the end of anything. So the
/// boundary is walked back over whatever leads into the connective, and a cut is made where the
/// connective starts rather than where the tagger happened to name it.
///
/// No connective is listed. What is read off the tags is that an adverb or a second subordinator
/// immediately before one is leading into it, because neither can end a clause that the next word
/// then continues.
fn opens(head: &[Tag]) -> usize {
    let mut at = head.len();
    while at > 0
        && matches!(
            head[at - 1],
            Tag::Adverb | Tag::Subordinator | Tag::Coordinator(_) | Tag::Preposition | Tag::To
        )
    {
        at -= 1;
    }
    at
}

/// Whether what is being cut off the end is a clause and not a complement.
///
/// A sign off is a clause: it has a subject and a verb, and the sentence before it was already
/// finished. A complement is not, and the words before it are waiting for it. "rather than a
/// subject rewritten to suit its verb" holds no tensed verb, so it completes what came before
/// rather than commenting on it, and cutting it leaves the sentence ending on "rather".
///
/// Asking whether the tail stands is the same question already asked of the head, put to the other
/// side of the cut. Neither side may be a piece of a sentence. Nothing here lists a connective:
/// what disqualifies "rather than" is that no clause follows it, which is read off the tags.
fn severable(tail: &[Tag]) -> bool {
    let ends_sentence = tail
        .iter()
        .rposition(|tag| matches!(tag, Tag::Mark(crate::tag::Break::Stop)));
    let body = &tail[..ends_sentence.unwrap_or(tail.len())];
    body.iter().any(|tag| tag.is_nominal())
        && body
            .iter()
            .any(|tag| tag.is_finite_verb() || matches!(tag, Tag::Modal))
}

/// Which of the passage's terms a stretch of tokens holds.
fn holds(tokens: &[Token], about: &[(String, f64)]) -> u16 {
    let mut held = 0u16;
    for token in tokens {
        if let Some(at) = about.iter().position(|(word, _)| *word == stem(&token.key)) {
            held |= 1 << at;
        }
    }
    held
}

/// Whether a stretch of tokens is a sentence rather than a piece of one.
fn stands_alone(tokens: &[Token], tags: &[Tag], moved: bool) -> bool {
    if tokens.len() < 3 {
        return false;
    }
    // A relative or coordinated clause has a subject and a verb like any other, so those alone do
    // not make it able to stand. What disqualifies it is the word it opens with, which announces
    // that it is hanging off something earlier. Once that something has been cut, it is hanging
    // off nothing.
    if moved
        && matches!(
            tags.first(),
            Some(
                Tag::Subordinator
                    | Tag::Coordinator(_)
                    | Tag::Preposition
                    | Tag::To
                    | Tag::Mark(_)
                    | Tag::Verb(_)
                    | Tag::Modal
                    | Tag::Adverb
            )
        )
    {
        return false;
    }
    let has_subject = tags.iter().any(|tag| tag.is_nominal());
    let has_verb = tags
        .iter()
        .any(|tag| tag.is_finite_verb() || matches!(tag, Tag::Modal));
    has_subject && has_verb
}

/// What the passage is about, most central first.
fn terms(units: &[Sentence]) -> Vec<(String, f64)> {
    let mut counted: Vec<(String, f64)> = Vec::new();
    for unit in units {
        for token in &unit.tokens {
            if !bears_content(token) {
                continue;
            }
            let stem = stem(&token.key);
            if let Some(found) = counted.iter_mut().find(|(word, _)| *word == stem) {
                found.1 += 1.0;
            } else {
                counted.push((stem, 1.0));
            }
        }
    }
    for (word, weight) in &mut counted {
        if is_abstract(word) {
            *weight *= 0.25;
        }
    }
    // A term the passage keeps returning to is what the passage is about. Padding reaches for a
    // different grand word every clause, so a word used once is usually decoration. When nothing
    // recurs the passage is too short to have a habit, and every concrete term is taken instead.
    let returned: Vec<(String, f64)> = counted
        .iter()
        .filter(|(_, weight)| *weight >= 2.0)
        .cloned()
        .collect();
    if returned.is_empty() {
        // Nothing recurs, so the passage is too short to have a habit and concreteness has to
        // carry the decision alone. Borrowed vocabulary is dropped from the running: a word the
        // writer reached for once and never came back to, in a register they had to borrow, is
        // decoration. What is left is what the passage could not avoid naming.
        counted.retain(|(word, _)| word.len() <= PLAIN && !is_abstract(word));
    } else {
        counted = returned;
    }
    counted.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    counted.truncate(TERMS);
    counted
}

/// Whether a token can carry the point at all.
///
/// Function words cannot: they are the frame every passage shares. A word the lexicon knows as a
/// closed class word is therefore out, and everything else is in, which is exactly the open and
/// closed class split the lexicon already draws.
fn bears_content(token: &Token) -> bool {
    let word = token.key.as_str();
    if word.len() < 3 || !word.chars().all(char::is_alphabetic) {
        return false;
    }
    !lexicon::is_closed(word) && !style::is_empty(word)
}

/// Whether a word names an idea rather than a thing, read off its ending.
fn is_abstract(word: &str) -> bool {
    ABSTRACT
        .iter()
        .any(|ending| word.len() > ending.len() + 2 && word.ends_with(ending))
}

/// A word reduced to what it shares with its other forms.
///
/// Only the endings that leave the word meaning the same thing are taken off, so "strings" and
/// "string" are one term while "strain" and "string" stay two.
fn stem(word: &str) -> String {
    for ending in ["ing", "ed", "es", "s"] {
        if word.len() > ending.len() + 2 && word.ends_with(ending) {
            return word[..word.len() - ending.len()].to_owned();
        }
    }
    word.to_owned()
}
