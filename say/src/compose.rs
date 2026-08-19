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

use crate::corpus::{Corpus, Feature, Place};
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
    /// Where the part describes itself, when the input keeps that apart from what it contains.
    describing: Option<Feature>,
}

impl Claim {
    /// State where this part describes itself, so that a note about something inside it cannot
    /// stand in for a description of it.
    #[must_use]
    pub const fn described_by(mut self, describing: Feature) -> Self {
        self.describing = Some(describing);
        self
    }

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
        Ok(Self {
            feature,
            weight,
            describing: None,
        })
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

/// The shortest clause worth composing, in positions offered to the search.
const SHORTEST: usize = 3;

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

    // A part the input never wrote a finished sentence about is a part there is nothing to report
    // about, and it is dropped here rather than refused later. Requiring more than one sentence
    // was tried and was wrong: the part of a tree an author describes in exactly one line is the
    // one they wrote a summary line for, which is the sentence a description wants most. Refusing later would throw away the
    // whole document over one heading; reporting it anyway would mean writing the sentence, which
    // is the one thing this cannot do.
    let sayable: Vec<Claim> = claims
        .iter()
        .filter(|claim| clause(corpus, **claim).is_ok())
        .copied()
        .collect();
    if sayable.is_empty() {
        return Err(Refusal::unreported(
            "the input never wrote a finished sentence about any of its parts",
        ));
    }

    let terms = declare(corpus, &sayable, most)?;
    // The order the search states its choices in, recorded as it builds them, because where one
    // passage ends and the next begins is a question about that order and nothing else can see it.
    let stated: core::cell::RefCell<Vec<Feature>> = core::cell::RefCell::new(Vec::new());
    let chosen = optimise_subset_parts(&terms, AFFORDABLE, 64, |item| {
        stated.borrow_mut().push(sayable[item].feature);
        clause(corpus, sayable[item])
    })?;
    let breaks = parted(corpus, &stated.into_inner());
    Ok(Said::from_search(chosen, breaks))
}

/// Where the run of stated parts breaks into passages.
///
/// Two parts an author discusses in the same words belong together and two they never discuss
/// together do not, which is the measurement the selection already rests on, asked here of
/// neighbours in the order they will be read. The break falls at a valley: a join weaker than the
/// one before it and no stronger than the one after. That is a fact about the shape of this
/// input's own cohesion, so there is no cutoff to pick and none was picked. A fixed cutoff cannot
/// work: any level low enough to part an encyclopedia into paragraphs parts a set of modules into
/// a line apiece, and any level high enough to hold the modules together welds the encyclopedia
/// into one wall of text.
fn parted(corpus: &Corpus, stated: &[Feature]) -> Vec<usize> {
    let links: Vec<f64> = stated
        .windows(2)
        .map(|pair| shared_vocabulary(corpus, pair[0], pair[1]))
        .collect();
    (0..links.len())
        .filter(|&join| {
            let before = if join == 0 { f64::MAX } else { links[join - 1] };
            let after = links.get(join + 1).copied().unwrap_or(f64::MAX);
            links[join] < before && links[join] <= after
        })
        .map(|join| join + 1)
        .collect()
}

/// State what each claim is worth and how any two of them sit together.
///
/// Every number here arrives as evidence. The worth of a claim is the caller's, discounted by the
/// trust the caller holds it with; the worth of a pair is measured in the input, from how much
/// vocabulary the two properties share. Nothing is tuned, because there is nothing here to tune.
///
/// A pair SUBTRACTS. Saying a thing twice in the same words is worth less than saying it once,
/// so two claims drawn from the same vocabulary cost the search what the second of them repeats.
/// That is the whole reason a subset is searched for at all: with a positive pair term every
/// claim is worth taking, the best subset is every claim, and the search decodes a decision that
/// was never in doubt. It was positive here, and the document it produced was an index — thirty
/// three near-identical lines, because an index is exactly what a monotone objective asks for.
/// The penalty is the shared fraction of the smaller worth, so two claims that repeat each other
/// entirely are worth one of them, and a claim that repeats nothing pays nothing.
fn declare(corpus: &Corpus, claims: &[Claim], most: usize) -> Answer<Terms> {
    let mut terms = Terms::over(claims.len())?;
    for (position, claim) in claims.iter().enumerate() {
        terms = terms.worth(position, claim.weight)?;
    }
    // What each claim would say, so that what it costs to say two of them is measured over the
    // sentences themselves rather than over a summary of the parts they came from.
    let leading: Vec<Vec<Place>> = claims
        .iter()
        .map(|claim| {
            let sentences =
                corpus.sentences_in(claim.describing.unwrap_or(claim.feature), CONSIDERED);
            ordered(corpus, claim.feature, &sentences)
                .first()
                .map_or_else(Vec::new, |sentence| (*sentence).clone())
        })
        .collect();
    for (a, first) in claims.iter().enumerate() {
        for (b, second) in claims.iter().enumerate().skip(a + 1) {
            let shared = repetition(corpus, &leading[a], &leading[b]);
            if shared <= 0.0 {
                continue;
            }
            let trust = first.weight.confidence.and(second.weight.confidence);
            let span = reach(first.weight.span, second.weight.span);
            if span.is_empty() || trust.is_zero() {
                continue;
            }
            // Cubed, because what is being priced is a sentence that is ALREADY SAID, not one
            // that happens to share some words. A claim sits in as many pairs as there are other
            // claims, so a penalty linear in the overlap grows with the square of the pool while
            // worth grows with the pool: at thirty-eight claims, an unremarkable fifth of the
            // words in common between every pair outweighed every worth there was and the best
            // subset was the empty one -- a document of no lines. Spreading the penalty over the
            // pool instead was worse in the other direction: twelve modules with the SAME summary
            // line came to exactly break even, and the document stated it twelve times over. A
            // fifth in common is not a repetition and prices at a thousandth; all of it in common
            // is, and prices at all of it.
            // Against the LARGER of the two worths, so that two claims that repeat each other
            // entirely are worth strictly LESS than either alone. Priced against the smaller,
            // stating both came to exactly what stating one came to, and which of the two the
            // search returned was then decided by the order the sums happened to be added in:
            // the same input described one way in a release build and another way in a debug one.
            let repeated = shared * shared * shared * first.weight.value.max(second.weight.value);
            terms = terms.together(a, b, Evidence::new(span, trust, -repeated))?;
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

/// How many of the input's own sentences about one claim are weighed before one is chosen.
const CONSIDERED: usize = 400;

/// How many of the best are actually decoded, when the best cannot be shortened into a clause.
const TRIED: usize = 12;

/// The longest sentence worth reporting: past this a reader is being handed a paragraph.
const WHOLE: usize = 40;

/// Compose one clause about one claim out of a sentence the input wrote about it.
///
/// The states are the places of one of the input's own sentences, and one silence. A step to the
/// next place keeps what was written; a step past a place leaves it out, and is allowed only
/// where the input itself has written the two words that would then meet. So a clause is either
/// a sentence somebody wrote or that sentence shortened at joins the same text vouches for, and
/// producing a string of words nobody ever put together is not something this can express.
///
/// Nothing here holds a sentence, and no grammar is written down. Which sentence to say is a
/// measurement of how much the sentence says about the claim; how much of it to say is the path
/// search, priced against how long this text's sentences run.
fn clause(corpus: &Corpus, claim: Claim) -> Answer<Clause> {
    let sentences = corpus.sentences_in(claim.describing.unwrap_or(claim.feature), CONSIDERED);
    // Two structural facts about where a sentence was written, and no judgement about what it
    // says. An author opens a paragraph with the sentence that says what the paragraph is about,
    // and writes the sentence that says what the whole part is about before the ones that go into
    // it: the summary line of a doc comment, the lead of an article, the opening of a chapter.
    // What was tried before this and abandoned was ranking by how characteristic a sentence's
    // words are, which selects the most unusual sentence in a part. That is the opposite of an
    // orienting one, and it produced documents made of interior detail.
    let considered = ordered(corpus, claim.feature, &sentences);
    for sentence in considered.iter().take(TRIED) {
        if let Ok(said) = say(corpus, claim.feature, sentence) {
            return Ok(said);
        }
    }
    Err(Refusal::unreported(
        "the input says nothing about this in a sentence it finished",
    ))
}

/// The sentences that could stand for a part, best first.
///
/// Split out so that what a part would say is known before the search decides whether to say it.
/// The cost of stating two parts together is measured over these very sentences, so the evidence
/// the search is given and the text it later decodes cannot drift apart.
fn ordered<'a>(
    corpus: &'a Corpus,
    feature: Feature,
    sentences: &'a [Vec<Place>],
) -> Vec<&'a Vec<Place>> {
    let about: Vec<&Vec<Place>> = candidates_of(corpus, sentences)
        .filter(|sentence| about_it(corpus, feature, sentence))
        .collect();
    let opening: Vec<&Vec<Place>> = about
        .iter()
        .copied()
        .filter(|sentence| {
            sentence
                .first()
                .is_some_and(|place| corpus.opens_passage(*place))
        })
        .collect();
    let mut considered: Vec<&Vec<Place>> = if opening.is_empty() {
        if about.is_empty() {
            candidates_of(corpus, sentences).collect()
        } else {
            about
        }
    } else {
        opening
    };
    considered.sort_by_key(|sentence| {
        sentence
            .first()
            .map_or(usize::MAX, |place| place.position())
    });
    considered
}

/// How much of what one sentence says has already been said by another.
///
/// Measured over the words themselves, each carrying as much weight as it is rare in this text,
/// so a word both sentences share because everyone shares it counts for almost nothing and a word
/// they share because they are about the same thing counts for a great deal. No list of common
/// words exists here: how common a word is is a rate this text was measured for.
///
/// This is deliberately not the characteristic vocabulary the passages are grouped by. That set
/// keeps only words rare elsewhere, which excludes precisely the words a formulaic repository
/// repeats in every module, so it reports two near-identical lines as unrelated. A reader sees
/// the repetition whether or not it is statistically distinctive.
// Counts of words in a text, and a rate taken from one.
#[allow(clippy::cast_precision_loss)]
fn repetition(corpus: &Corpus, first: &[Place], second: &[Place]) -> f64 {
    let weigh = |place: &Place| {
        if corpus.is_symbolic(*place) {
            return 0.0;
        }
        let rate = corpus.commonness(place.word());
        if rate <= 0.0 {
            0.0
        } else {
            -rate.ln()
        }
    };
    let carried = |sentence: &[Place]| sentence.iter().map(weigh).sum::<f64>();
    let left = carried(first);
    let right = carried(second);
    if left <= 0.0 || right <= 0.0 {
        return 0.0;
    }
    let held: Vec<crate::Word> = second.iter().map(|place| place.word()).collect();
    let common: f64 = first
        .iter()
        .filter(|place| held.contains(&place.word()))
        .map(weigh)
        .sum();
    (common / left.min(right)).min(1.0)
}

/// Whether a sentence is about the part it was found in, rather than merely inside it.
///
/// A sentence that uses none of the words this part is characteristic of describes something the
/// part happens to contain, not the part: "No information." was written in a module about
/// confidence, and says nothing about confidence. The test is the same affinity the selection
/// already runs on, so no list of weak sentences exists anywhere here and none could.
fn about_it(corpus: &Corpus, feature: Feature, sentence: &[Place]) -> bool {
    let characteristic = corpus.characteristic(feature, CHARACTERISTIC);
    if characteristic.is_empty() {
        return true;
    }
    sentence
        .iter()
        .any(|place| characteristic.contains(&place.word()))
}

/// The same test, over sentences already read.
fn candidates_of<'a>(
    corpus: &'a Corpus,
    sentences: &'a [Vec<Place>],
) -> impl Iterator<Item = &'a Vec<Place>> {
    sentences
        .iter()
        .filter(|sentence| worth_reading(corpus, sentence))
}

/// Whether a sentence is one this text wrote as prose rather than as a table or a reference.
///
/// Long enough to say something, short enough to read, and written in no more marks and lone
/// characters than this author writes in. That last is what separates a sentence from a row of a
/// table, a line of mathematics and an entry in a bibliography, and the line between them is the
/// author's own rate rather than any judgement made here about what those things look like.
// A count of words in a sentence.
#[allow(clippy::cast_precision_loss)]
fn worth_reading(corpus: &Corpus, sentence: &[Place]) -> bool {
    if sentence.len() < SHORTEST || sentence.len() > WHOLE {
        return false;
    }
    // Measured over everything but the terminator, on both sides of the comparison, because a
    // sentence carrying its own full stop is not evidence of anything about the sentence.
    let body = match sentence.split_last() {
        Some((last, rest)) if corpus.is_symbolic(*last) => rest,
        _ => sentence,
    };
    if body.is_empty() {
        return false;
    }
    let marks = body
        .iter()
        .filter(|place| corpus.is_symbolic(**place))
        .count();
    marks as f64 / body.len() as f64 <= corpus.marking_rate()
}

/// Say one sentence of the input, as the input wrote it.
///
/// The path runs through the places of that sentence and may not leave any of them out. Leaving
/// one out was tried and abandoned: a join both words have been written with elsewhere is not a
/// join that holds here, and shortening what somebody wrote changes what they said while keeping
/// their name on it. What is decided here is which sentence, and that decision is a measurement.
fn say(corpus: &Corpus, feature: Feature, places: &[Place]) -> Answer<Clause> {
    let last = places.len() - 1;
    let steps = places.len();
    let silence = places.len();
    let states = silence + 1;
    let expressive: Vec<f64> = places
        .iter()
        .map(|place| corpus.affinity(feature, place.word()).max(NEUTRAL))
        .collect();

    let emission = |step: usize, state: usize| -> f64 {
        if state == silence {
            // Silence is where a clause has finished, never an alternative to a sentence: the
            // sentence chosen has a beginning and an end of its own, both put there by whoever
            // wrote it.
            return IMPOSSIBLE;
        }
        if state != step {
            return IMPOSSIBLE;
        }
        if step + 1 == steps && state != last {
            return IMPOSSIBLE;
        }
        -expressive[state].ln()
    };

    let transition = |from: usize, to: usize| -> f64 {
        if from == silence || to == silence || to != from + 1 {
            return IMPOSSIBLE;
        }
        -corpus
            .association(places[from].word(), places[to].word())
            .ln()
    };

    let part = |step: usize, state: usize| -> Slot {
        if state == silence {
            return Slot::silent();
        }
        let place = places[state];
        Slot::spoken(
            corpus.written(place).to_owned(),
            place.source(),
            place.is_glued() && step > 0,
        )
    };

    let chosen = decode_path_parts(steps, states, 1.0, emission, transition, part)?;
    Ok(Clause::from_search(chosen))
}
