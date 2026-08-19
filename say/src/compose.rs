//! Composing a passage, which is two searches and no decisions of its own.
//!
//! The first search decides what is worth saying: a subset of the claims, priced by the evidence
//! behind each and by how well any two of them sit together in the text that was read. The second
//! decides how to say each one: a path through the corpus's vocabulary, priced by how strongly a
//! word belongs to the claim and how readily the input's authors put one word after another.
//!
//! Neither search is shown the answer it is scoring, and nothing between them holds a sentence.
//! There is no branch here on what a claim means, because nothing here knows what a claim means.

use fitkit::{
    decode_path_parts, optimise_subset_parts, Answer, Confidence, Evidence, Refusal, Span, Terms,
};

use crate::corpus::{Corpus, Feature};
use crate::said::{Clause, Said, Slot};

/// Something worth saying, and what stands behind saying it.
///
/// A claim names a property and nothing else. It carries no words: what it will be written with
/// is whatever the corpus associates with that property, so the same claim reads differently in a
/// terse repository and a discursive one, and neither reading was written here.
#[derive(Clone, Copy, Debug)]
pub struct Claim {
    feature: Feature,
    weight: Evidence<f64>,
}

impl Claim {
    /// A claim about a property, worth stating to the degree its evidence supports it.
    ///
    /// # Errors
    ///
    /// Refuses evidence that cannot support a decision: an empty span, which cites nothing, or
    /// zero confidence, which is an admission that the claim is not held at all.
    pub fn new(feature: Feature, weight: Evidence<f64>) -> Answer<Self> {
        if !weight.is_informative() {
            return Err(Refusal::uninformative(
                "a claim needs a span it speaks for and some trust",
            ));
        }
        if !weight.value.is_finite() {
            return Err(Refusal::incoherent(
                "a claim's worth must be a finite quantity",
            ));
        }
        Ok(Self { feature, weight })
    }

    /// The property claimed.
    #[must_use]
    pub const fn feature(self) -> Feature {
        self.feature
    }

    /// The region of the input that supports it.
    #[must_use]
    pub const fn source(self) -> Span {
        self.weight.span
    }

    /// How far it is trusted.
    #[must_use]
    pub const fn trust(self) -> Confidence {
        self.weight.confidence
    }
}

/// The most claims a single passage can weigh, fixed by the width of a subset mask.
pub const MOST_CLAIMS: usize = 64;

/// How many candidate words a claim is allowed to draw from what it is characteristic of.
const CHARACTERISTIC: usize = 24;

/// What a word that says nothing in particular about a claim is worth: exactly nothing either way.
const NEUTRAL: f64 = 1.0;

/// The cost of a step the corpus gives no reason to take.
/// How many claims can be weighed by looking at every combination of them.
///
/// Beyond this the count of combinations doubles per claim, so the exact answer stops being an
/// answer at all: it is the same result arriving after the reader has gone. A search that reports
/// which method it used is telling the caller how much to trust it, which is the whole reason to
/// have the choice.
const AFFORDABLE: usize = 20;

const IMPOSSIBLE: f64 = 1.0e6;

/// How many places in the input one clause may pick its words from.
const PLACES: usize = 240;

/// The shortest clause worth composing, in positions offered to the search.
const SHORTEST: usize = 3;

/// The longest, however long the input's own sentences run.
const LONGEST: usize = 24;

/// Say what is worth saying about these claims, in the words of the corpus.
///
/// The count of clauses is the count of claims the subset search kept, so a passage cannot have a
/// section the evidence does not fill. Where the corpus has nothing to say about a claim, that
/// clause refuses and the refusal is carried out rather than papered over.
///
/// # Errors
///
/// Refuses an unsettled corpus, an empty claim list, more claims than a mask can hold, a corpus
/// with no words, and any claim set the declarations leave unsatisfiable.
pub fn compose(corpus: &Corpus, claims: &[Claim], most: usize) -> Answer<Said> {
    if !corpus.is_settled() {
        return Err(Refusal::incoherent(
            "a corpus must be settled before it can be composed from",
        ));
    }
    if corpus.vocabulary() == 0 {
        return Err(Refusal::unreported(
            "the corpus holds no words to say anything with",
        ));
    }
    if claims.is_empty() {
        return Err(Refusal::unreported("there is nothing claimed to say"));
    }
    if claims.len() > MOST_CLAIMS {
        return Err(Refusal::incoherent(
            "more claims than one selection can weigh at once",
        ));
    }

    let terms = declare(corpus, claims, most)?;
    let chosen =
        optimise_subset_parts(&terms, AFFORDABLE, 64, |item| clause(corpus, claims[item]))?;
    Ok(Said::from_search(chosen))
}

/// State what each claim is worth and how any two of them sit together.
///
/// Every number here arrives as evidence. The worth of a claim is the caller's, discounted by the
/// trust the caller holds it with; the worth of a pair is measured in the input, from how much
/// vocabulary the two properties share. Nothing is tuned, because there is nothing here to tune.
fn declare(corpus: &Corpus, claims: &[Claim], most: usize) -> Answer<Terms> {
    let mut terms = Terms::over(claims.len())?;
    for (position, claim) in claims.iter().enumerate() {
        terms = terms.worth(position, claim.weight)?;
    }
    for (a, first) in claims.iter().enumerate() {
        for (b, second) in claims.iter().enumerate().skip(a + 1) {
            let shared = shared_vocabulary(corpus, first.feature, second.feature);
            if shared <= 0.0 {
                continue;
            }
            let trust = first.weight.confidence.and(second.weight.confidence);
            let span = reach(first.weight.span, second.weight.span);
            if span.is_empty() || trust.is_zero() {
                continue;
            }
            terms = terms.together(a, b, Evidence::new(span, trust, shared))?;
        }
    }
    if most > 0 && most < claims.len() {
        terms = terms.at_most(most)?;
    }
    Ok(terms)
}

/// How much of their characteristic vocabulary two properties hold in common.
///
/// Two properties the input discusses in the same words belong in the same passage; two it never
/// discusses together do not. That is a measurement of the input, so it transfers to any input.
// A count of words in a text. A count large enough to lose a bit here is a text nobody
// has, and a rate taken from one reads the same either way.
#[allow(clippy::cast_precision_loss)]
fn shared_vocabulary(corpus: &Corpus, first: Feature, second: Feature) -> f64 {
    if first == second {
        return 0.0;
    }
    let left = corpus.characteristic(first, CHARACTERISTIC);
    if left.is_empty() {
        return 0.0;
    }
    let right = corpus.characteristic(second, CHARACTERISTIC);
    if right.is_empty() {
        return 0.0;
    }
    let common = left.iter().filter(|word| right.contains(word)).count();
    if common == 0 {
        return 0.0;
    }
    common as f64 / left.len().min(right.len()) as f64
}

/// The region running from the first of two spans to the last.
fn reach(first: Span, second: Span) -> Span {
    Span::new(first.start.min(second.start), first.end.max(second.end))
}

/// Compose one clause about one claim, by decoding a path through the input's own text.
///
/// The states are places in the input where a word bearing on the claim was read, and one
/// silence. A step may only move forward through the input, so a clause is a subsequence of what
/// somebody actually wrote: it can leave words out and it can splice one passage to another, but
/// it can never come back to a place it has already used. That is what makes saying the same
/// thing twice unrepresentable rather than merely expensive.
///
/// Silence is absorbing, so where a clause stops is a decision the search made.
fn clause(corpus: &Corpus, claim: Claim) -> Answer<Clause> {
    let places = corpus.places_in(claim.feature, PLACES);
    if places.is_empty() {
        return Err(Refusal::unreported(
            "the input never says anything about this",
        ));
    }
    let lengths = corpus.lengths();
    if lengths.is_empty() {
        return Err(Refusal::unreported(
            "the input holds no sentence to learn a length from",
        ));
    }
    let silence = places.len();
    let states = silence + 1;
    let steps = corpus.typical_length().clamp(SHORTEST, LONGEST);
    let ending: Vec<bool> = places
        .iter()
        .map(|place| Some(place.word()) == corpus.terminator())
        .collect();

    // How much each place says about this claim, measured against how common its word is
    // anyway. Above one means the text uses this word when this property holds and not
    // otherwise; at one the word carries no information about it either way.
    let expressive: Vec<f64> = places
        .iter()
        .map(|place| corpus.affinity(claim.feature, place.word()).max(NEUTRAL))
        .collect();
    let opens: Vec<f64> = places
        .iter()
        .map(|place| corpus.opens(place.word()))
        .collect();
    let closes: Vec<f64> = places
        .iter()
        .map(|place| corpus.closes(place.word()))
        .collect();

    let emission = |step: usize, state: usize| -> f64 {
        if state == silence {
            // A clause that says nothing at all is not a shorter clause, it is the absence of
            // one. Silence has to be reached from a finished sentence, never chosen instead of
            // starting one.
            return if step == 0 { IMPOSSIBLE } else { 0.0 };
        }
        // The last position a clause has must finish it. Everything else is unreachable there, so
        // a clause cannot simply run out of room mid-sentence: it either reaches the mark this
        // text ends sentences with, or it has already stopped.
        if step + 1 == steps && !ending[state] {
            return IMPOSSIBLE;
        }
        let mut cost = -expressive[state].ln();
        if step == 0 {
            cost -= opens[state].ln();
        }
        cost
    };

    let transition = |from: usize, to: usize| -> f64 {
        match (from == silence, to == silence) {
            (true, true) => 0.0,
            (true, false) => IMPOSSIBLE,
            // A clause ends where the text's own sentences end: after the mark this text uses to
            // finish one. Where that mark falls is the search's decision, so the length of a
            // clause is decided by the search and not by anything set down here.
            (false, true) => {
                if ending[from] {
                    -closes[from].ln()
                } else {
                    IMPOSSIBLE
                }
            }
            (false, false) => {
                if places[to].position() <= places[from].position() {
                    return IMPOSSIBLE;
                }
                // Saying a word and then saying it again says nothing the first one did not.
                // Where the input does this it is a table or a listing rather than a sentence,
                // and lifting that shape into a sentence reports a repetition as if it were
                // language.
                if places[to].word() == places[from].word() {
                    return IMPOSSIBLE;
                }
                // How much likelier this word is after that one than it is anywhere. A pair the
                // text actually uses is worth taking; a pair it never uses is not, and neither
                // judgement needed a number chosen here.
                let after = corpus.follows(places[from].word(), places[to].word());
                let anywhere = corpus.commonness(places[to].word()).max(f64::MIN_POSITIVE);
                -(after / anywhere).ln()
            }
        }
    };

    let part = |step: usize, state: usize| -> Slot {
        if state == silence {
            return Slot::silent();
        }
        let place = places[state];
        Slot::spoken(
            corpus.spelling(place.word(), step == 0).to_owned(),
            place.source(),
            corpus.is_attached(place.word()) && step > 0,
        )
    };

    let chosen = decode_path_parts(steps, states, 1.0, emission, transition, part)?;
    Ok(Clause::from_search(chosen))
}
