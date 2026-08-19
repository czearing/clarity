//! Everything the engine knows about a language, learned from the text it was given.
//!
//! No word in this file is written down. A corpus starts empty and knows nothing; every word it
//! can later use, every pair of words it believes may sit next to each other, and every mark of
//! punctuation it can place is something it saw in the input. Point it at a repository and it
//! learns how that repository's authors write. Point it at a book and it learns the book.
//!
//! That is the whole reason the observation happens before the composition. A generator that
//! could reach for a word the input never used would be reaching for a word somebody wrote into
//! the generator, and one word written into the generator is a template with one hole.

use std::collections::{BTreeMap, BTreeSet};

use fitkit::Span;

/// A word the corpus has seen, identified by its position in the vocabulary.
///
/// Words are compared by their folded form, so `Engine` and `engine` are one word with two
/// observed spellings. Which spelling is written out is decided later, from where in the sentence
/// the word lands, because the corpus recorded where each spelling was seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Word(usize);

impl Word {
    /// Its position in the vocabulary, for indexing the arrays a search builds over it.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// A named property of a thing being described, used to associate words with it.
///
/// A feature never carries text. It is an opaque key, so that the same engine can be handed
/// features meaning "returns an optional" by a code reader and features meaning "appears under a
/// heading" by a prose reader, and cannot tell the difference. What a feature *means* in words is
/// not stated anywhere; it is whatever the input's own authors wrote near it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Feature(u64);

impl Feature {
    /// A feature keyed by an arbitrary identifier of the caller's choosing.
    ///
    /// The number is a name, not a weight. Two callers that pick the same number are talking
    /// about the same property, and nothing else about it is fixed here.
    #[must_use]
    pub const fn keyed(key: u64) -> Self {
        Self(key)
    }

    /// A feature keyed by a label, folded to a number so the label itself cannot reach an output.
    ///
    /// The label is a key and never a word. It is hashed on the way in, and the hash is all the
    /// corpus keeps, so there is no route by which a caller's naming of a property becomes text a
    /// reader sees.
    #[must_use]
    pub fn of(label: &str) -> Self {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in label.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(hash)
    }
}

/// One token as it was observed: which word, how it was spelled, and how it was spaced.
#[derive(Clone, Copy, Debug)]
struct Seen {
    word: usize,
    /// Where in the input this token was read from.
    span: Span,
    /// Whether it was written with an initial capital.
    capital: bool,
    /// Whether it followed the previous token with no space between them.
    attached: bool,
    /// Whether every character in it is punctuation rather than letters or digits.
    marking: bool,
}

/// One occurrence of a word in the input, at the place it was read.
///
/// A composition moves through these rather than through word types, which is what makes it
/// impossible for a sentence to say the same thing twice: positions only ever increase, so no
/// occurrence can be visited after itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    word: Word,
    at: usize,
    span: Span,
}

impl Place {
    /// The word here.
    #[must_use]
    pub const fn word(self) -> Word {
        self.word
    }

    /// How far into the input it is, in tokens.
    #[must_use]
    pub const fn position(self) -> usize {
        self.at
    }

    /// The region of the input it was read from.
    #[must_use]
    pub const fn source(self) -> Span {
        self.span
    }
}

/// A word's record in the vocabulary.
#[derive(Clone, Debug)]
struct Entry {
    /// The most frequent spelling seen with a leading capital, if any was.
    capital: Option<String>,
    /// The most frequent spelling seen without one.
    plain: Option<String>,
    /// Where it was first seen, which is the span it cites when it is used.
    first: Span,
    /// How many times it was seen at all.
    seen: u32,
    /// How many times it opened a sentence.
    opened: u32,
    /// How many times it closed one.
    closed: u32,
    /// Whether it is punctuation.
    marking: bool,
    /// How often it was written attached to the token before it.
    attached: u32,
}

/// A language, and a set of properties, learned from observed text.
///
/// Built by observation and then settled. Nothing can be composed from an unsettled corpus,
/// because the statistics a composition needs — which words open sentences, which punctuation
/// ends them, which words are specific to a property and which are the connective tissue every
/// property shares — are all relationships across the whole input, and none of them can be known
/// while text is still arriving.
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    vocabulary: Vec<Entry>,
    index: BTreeMap<String, usize>,
    stream: Vec<Seen>,
    /// Where each observation began in the stream, and what features it was attached to.
    passages: Vec<(usize, usize, Vec<Feature>)>,
    /// Learned: how often word b directly followed word a.
    follows: BTreeMap<(usize, usize), u32>,
    /// How many different words have been seen after each word.
    varied: Vec<u32>,
    /// Learned: how often a word was seen in text attached to a feature.
    marks: BTreeMap<(Feature, usize), u32>,
    /// Learned: how many tokens were seen attached to each feature.
    weight: BTreeMap<Feature, u32>,
    /// Learned: the punctuation that ends a sentence, most convincing first.
    terminators: Vec<usize>,
    settled: bool,
}

impl Corpus {
    /// An empty corpus, which knows no words at all.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read text, learning its words and their order, without attaching it to any property.
    ///
    /// Use this for text that shows how the input's authors write but says nothing in particular
    /// about the things being described: prose around the subject, headings, surrounding
    /// paragraphs. It teaches the corpus grammar without teaching it claims.
    pub fn observe(&mut self, text: &str, at: Span) {
        self.attach(&[], text, at);
    }

    /// Read text that is evidence for these properties, learning both its words and that link.
    ///
    /// This is how the engine comes to know what a property sounds like. Give it the sentence an
    /// author wrote about an item that returns an optional value, tagged with the feature meaning
    /// "returns an optional", and it learns which words that author reaches for. Give it enough
    /// of them and it can write about an item whose author wrote nothing at all.
    pub fn attach(&mut self, features: &[Feature], text: &str, at: Span) {
        self.settled = false;
        let start = self.stream.len();
        let mut offset = at.start;
        let mut attached = false;
        for (token, gap) in tokens(text) {
            let span = Span::new(offset + gap, offset + gap + token.len());
            let word = self.intern(token, span);
            self.stream.push(Seen {
                word,
                span,
                capital: token.chars().next().is_some_and(char::is_uppercase),
                attached: attached && gap == 0,
                marking: self.vocabulary[word].marking,
            });
            offset += gap + token.len();
            attached = true;
        }
        let end = self.stream.len();
        if end > start {
            self.passages.push((start, end, features.to_vec()));
        }
    }

    /// Work out everything that can only be known once all the text has arrived.
    ///
    /// Sentence boundaries are found here rather than assumed, by looking for punctuation that is
    /// usually followed by a capitalised word. That is a claim about the observed text and not
    /// about English, so a corpus of text that marks its sentences some other way is read on its
    /// own terms.
    pub fn settle(&mut self) {
        if self.settled {
            return;
        }
        self.terminators = self.learn_terminators();
        self.count_transitions();
        self.count_marks();
        self.settled = true;
    }

    /// Whether the corpus has been settled and can be composed from.
    #[must_use]
    pub const fn is_settled(&self) -> bool {
        self.settled
    }

    /// How many distinct words it knows.
    #[must_use]
    pub fn vocabulary(&self) -> usize {
        self.vocabulary.len()
    }

    /// How many tokens it has read.
    #[must_use]
    pub fn tokens(&self) -> usize {
        self.stream.len()
    }

    /// The words it knows, in vocabulary order.
    pub fn words(&self) -> impl Iterator<Item = Word> + '_ {
        (0..self.vocabulary.len()).map(Word)
    }

    /// The spelling of a word appropriate to where it sits, as somebody wrote it.
    ///
    /// A word that was only ever seen capitalised is written capitalised wherever it lands,
    /// because that is the only spelling anybody used for it. A word seen both ways is written
    /// the way it was most often written in that position.
    #[must_use]
    pub fn spelling(&self, word: Word, opening: bool) -> &str {
        let entry = &self.vocabulary[word.0];
        let first = if opening {
            entry.capital.as_ref()
        } else {
            entry.plain.as_ref()
        };
        first
            .or(entry.plain.as_ref())
            .or(entry.capital.as_ref())
            .map_or("", String::as_str)
    }

    /// Where the word was first seen, which is the region it can cite.
    #[must_use]
    pub fn source(&self, word: Word) -> Span {
        self.vocabulary[word.0].first
    }

    /// Whether the word is punctuation rather than letters.
    #[must_use]
    pub fn is_marking(&self, word: Word) -> bool {
        self.vocabulary[word.0].marking
    }

    /// Whether the word is usually written with no space before it.
    #[must_use]
    pub fn is_attached(&self, word: Word) -> bool {
        let entry = &self.vocabulary[word.0];
        entry.seen > 0 && entry.attached * 2 > entry.seen
    }

    /// The punctuation this text uses to end a sentence, if it ends them with any.
    #[must_use]
    pub fn terminator(&self) -> Option<Word> {
        self.terminators.first().copied().map(Word)
    }

    /// How likely this word is to follow that one, as a share of everything seen after it.
    ///
    /// Smoothed, so a pair never seen is unlikely rather than impossible. A generator that
    /// refused every unseen pair could only ever reproduce sentences it had already read.
    #[must_use]
    // A count of words in a text. A count large enough to lose a bit here is a text nobody
    // has, and a rate taken from one reads the same either way.
    #[allow(clippy::cast_precision_loss)]
    pub fn follows(&self, before: Word, after: Word) -> f64 {
        let total = f64::from(self.vocabulary[before.0].seen);
        let joint = f64::from(self.follows.get(&(before.0, after.0)).copied().unwrap_or(0));
        // How much weight to give a pair never seen is decided by how varied this word's company
        // has been. A word that has been followed by fifty different words is a word whose next
        // word is not settled, so an unseen successor is unremarkable; a word that has only ever
        // been followed by one is a word whose company is fixed, and departing from it is news.
        // The count of distinct successors is that measurement, and it is the text's, not a
        // number chosen here.
        let varied = f64::from(self.varied[before.0]);
        (joint + varied * self.commonness(after)) / (total + varied)
    }

    /// How likely this word is to open a sentence.
    #[must_use]
    pub fn opens(&self, word: Word) -> f64 {
        let entry = &self.vocabulary[word.0];
        (f64::from(entry.opened) + SMOOTHING) / (f64::from(entry.seen) + SMOOTHING * 2.0)
    }

    /// How likely this word is to close one.
    #[must_use]
    pub fn closes(&self, word: Word) -> f64 {
        let entry = &self.vocabulary[word.0];
        (f64::from(entry.closed) + SMOOTHING) / (f64::from(entry.seen) + SMOOTHING * 2.0)
    }

    /// How strongly this word belongs to this property, against how common it is generally.
    ///
    /// A word that appears wherever this property does and rarely elsewhere is what the property
    /// sounds like. A word that appears everywhere carries no information about it, and this
    /// returns near nothing for such a word however often it was seen beside one.
    #[must_use]
    // A count of words in a text. A count large enough to lose a bit here is a text nobody
    // has, and a rate taken from one reads the same either way.
    #[allow(clippy::cast_precision_loss)]
    pub fn affinity(&self, feature: Feature, word: Word) -> f64 {
        let together = self.marks.get(&(feature, word.0)).copied().unwrap_or(0);
        if together == 0 {
            return 0.0;
        }
        let feature_total = self.weight.get(&feature).copied().unwrap_or(0);
        if feature_total == 0 {
            return 0.0;
        }
        let within = f64::from(together) / f64::from(feature_total);
        let seen = self.vocabulary[word.0].seen;
        let overall = f64::from(seen) / self.stream.len().max(1) as f64;
        if overall <= 0.0 {
            return 0.0;
        }
        let ratio = (within / overall).max(0.0);
        ratio.powf(discount(together, seen, feature_total))
    }

    /// How much likelier one word is after another than it is anywhere, discounted the same way.
    ///
    /// This is what a clause pays to put one word after another, and it is a ratio rather than a
    /// probability so that a word which belongs where it is put is free. Charging the probability
    /// outright would charge every word its own surprise, and the cheapest reading of any text
    /// would be the shortest one it could end.
    #[must_use]
    pub fn association(&self, before: Word, after: Word) -> f64 {
        let joint = self.follows.get(&(before.0, after.0)).copied().unwrap_or(0);
        let anywhere = self.commonness(after).max(f64::MIN_POSITIVE);
        let ratio = self.follows(before, after) / anywhere;
        let left = self.vocabulary[before.0].seen;
        let right = self.vocabulary[after.0].seen;
        ratio.powf(discount(joint, left, right))
    }

    /// How commonly the word is used at all, as a share of every token read.
    ///
    /// This is what makes a word available as connective tissue. The words that hold a sentence
    /// together are the ones that turn up everywhere and mean little on their own, and that is a
    /// measurable property rather than a list somebody typed.
    #[must_use]
    // A count of words in a text. A count large enough to lose a bit here is a text nobody
    // has, and a rate taken from one reads the same either way.
    #[allow(clippy::cast_precision_loss)]
    pub fn commonness(&self, word: Word) -> f64 {
        f64::from(self.vocabulary[word.0].seen) / self.stream.len().max(1) as f64
    }

    /// The words seen attached to this property, most characteristic first.
    #[must_use]
    pub fn characteristic(&self, feature: Feature, most: usize) -> Vec<Word> {
        let mut scored: Vec<(usize, f64)> = self
            .marks
            .range((feature, 0)..=(feature, usize::MAX))
            .map(|(&(_, word), _)| (word, self.affinity(feature, Word(word))))
            .filter(|&(_, score)| score > 1.0)
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        scored.truncate(most);
        scored.into_iter().map(|(word, _)| Word(word)).collect()
    }

    /// The most common words that belong to no property in particular.
    ///
    /// Discovered by distribution rather than declared: these are the words the text uses
    /// constantly and that say nothing about any one subject, which is what a function word is.
    #[must_use]
    pub fn connective(&self, most: usize) -> Vec<Word> {
        let mut scored: Vec<(usize, f64)> = (0..self.vocabulary.len())
            .filter(|&word| !self.vocabulary[word].marking)
            .map(|word| (word, self.commonness(Word(word))))
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        scored.truncate(most);
        scored.into_iter().map(|(word, _)| Word(word)).collect()
    }

    /// Every place these words were seen, in the order the input presents them.
    ///
    /// Ranked by the order the words were given, so that when there are more occurrences than a
    /// search can afford, the ones kept are the ones the caller thought most characteristic. The
    /// result is always in input order, because that is what lets a composition be a subsequence
    /// of the text rather than a rearrangement of it.
    #[must_use]
    pub fn places(&self, words: &[Word], most: usize) -> Vec<Place> {
        let mut rank: BTreeMap<usize, usize> = BTreeMap::new();
        for (position, word) in words.iter().enumerate() {
            rank.insert(word.index(), position);
        }
        let mut found: Vec<(usize, Place)> = self
            .stream
            .iter()
            .enumerate()
            .filter_map(|(at, token)| {
                rank.get(&token.word).map(|&order| {
                    (
                        order,
                        Place {
                            word: Word(token.word),
                            at,
                            span: token.span,
                        },
                    )
                })
            })
            .collect();
        if found.len() > most {
            found.sort_by_key(|&(order, place)| (order, place.at));
            found.truncate(most);
        }
        found.sort_by_key(|&(_, place)| place.at);
        found.into_iter().map(|(_, place)| place).collect()
    }

    /// Every place inside the text that was attached to this property, in input order.
    ///
    /// A clause about a part of the input is built out of what the input said about that part,
    /// not out of everything it ever said. Without that, a composition splices words from
    /// unrelated passages that happen to share a neighbour, and the result is fluent nowhere.
    /// Ranked by how much each word says about the property when there are more places than a
    /// search can afford, with the marks that end sentences always kept so a clause can finish.
    #[must_use]
    pub fn places_in(&self, feature: Feature, most: usize) -> Vec<Place> {
        let mut found: Vec<(f64, Place)> = Vec::new();
        let mut said: BTreeSet<usize> = BTreeSet::new();
        for (start, end, features) in &self.passages {
            if !features.contains(&feature) {
                continue;
            }
            for (offset, token) in self.stream[*start..*end].iter().enumerate() {
                let word = Word(token.word);
                let ends = self.terminators.contains(&token.word);
                // Punctuation is not something to say. The one mark that can be chosen is the one
                // that finishes a sentence, because finishing is a decision; the rest are how the
                // input spaced its own words and carry nothing to report.
                if token.marking && !ends {
                    continue;
                }
                let rank = if ends {
                    f64::INFINITY
                } else {
                    self.affinity(feature, word) + self.commonness(word)
                };
                // A word that says something about the property is given one place and not
                // every place. Saying it a second time says nothing the first did not, and where
                // the input repeats itself it is listing or headings rather than sentences. The
                // words that carry no information are left with all of theirs, because those are
                // what holds a sentence together and a sentence needs them wherever it needs
                // them. So repetition is not made expensive, it is made unavailable.
                if rank.is_finite() && self.affinity(feature, word) > 1.0 {
                    if said.contains(&token.word) {
                        continue;
                    }
                    said.insert(token.word);
                }
                found.push((
                    rank,
                    Place {
                        word,
                        at: start + offset,
                        span: token.span,
                    },
                ));
            }
        }
        if found.len() > most {
            found.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.at.cmp(&b.1.at)));
            found.truncate(most);
        }
        found.sort_by_key(|&(_, place)| place.at);
        found.into_iter().map(|(_, place)| place).collect()
    }

    /// The lengths of the sentences it read, in tokens, shortest first.
    ///
    /// A generator needs these because a model that treats stopping as free will always stop at
    /// once: every further word multiplies in another probability below one, so the shortest
    /// reading wins whatever it says. Knowing how long this text's sentences actually run turns
    /// stopping into an event that has to compete with continuing.
    #[must_use]
    pub fn lengths(&self) -> Vec<usize> {
        let mut lengths = Vec::new();
        let mut run = 0usize;
        for (index, _) in self.stream.iter().enumerate() {
            if self.ends_sentence(index) {
                if run > 0 {
                    // Counted with its terminator, because a sentence has to have room to reach
                    // the mark that ends it.
                    lengths.push(run + 1);
                }
                run = 0;
            } else {
                run += 1;
            }
        }
        if run > 0 {
            lengths.push(run);
        }
        lengths.sort_unstable();
        lengths
    }

    /// The typical length of a sentence in this text, in tokens.
    ///
    /// The engine writes to the length the input writes to. Nothing here prefers a short sentence
    /// or a long one; a terse repository gets terse prose back.
    #[must_use]
    pub fn typical_length(&self) -> usize {
        let lengths = self.lengths();
        if lengths.is_empty() {
            return 0;
        }
        lengths[lengths.len() / 2]
    }

    /// Add a word to the vocabulary, or find it, and record how it was spelled.
    fn intern(&mut self, token: &str, at: Span) -> usize {
        let folded = token.to_lowercase();
        let marking = token.chars().all(|c| !c.is_alphanumeric());
        let position = if let Some(&found) = self.index.get(&folded) {
            found
        } else {
            let fresh = self.vocabulary.len();
            self.vocabulary.push(Entry {
                capital: None,
                plain: None,
                first: at,
                seen: 0,
                opened: 0,
                closed: 0,
                marking,
                attached: 0,
            });
            self.index.insert(folded, fresh);
            fresh
        };
        let entry = &mut self.vocabulary[position];
        entry.seen += 1;
        if token.chars().next().is_some_and(char::is_uppercase) {
            entry.capital.get_or_insert_with(|| token.to_owned());
        } else {
            entry.plain.get_or_insert_with(|| token.to_owned());
        }
        position
    }

    /// Find the punctuation that ends sentences, by how often a capital follows it.
    // A count of words in a text. A count large enough to lose a bit here is a text nobody
    // has, and a rate taken from one reads the same either way.
    #[allow(clippy::cast_precision_loss)]
    fn learn_terminators(&self) -> Vec<usize> {
        let mut after: BTreeMap<usize, (u32, u32, u32, u32)> = BTreeMap::new();
        let mut finals: BTreeMap<usize, u32> = BTreeMap::new();
        for &(_, end, _) in &self.passages {
            if let Some(last) = self.stream.get(end - 1) {
                *finals.entry(last.word).or_insert(0) += 1;
            }
        }
        for (index, token) in self.stream.iter().enumerate() {
            if !token.marking {
                continue;
            }
            let tally = after.entry(token.word).or_insert((0, 0, 0, 0));
            tally.0 += 1;
            if self.stream.get(index + 1).is_none_or(|next| next.capital) {
                tally.1 += 1;
            }
            if token.attached {
                tally.2 += 1;
            }
            tally.3 += finals.get(&token.word).copied().unwrap_or(0).min(1);
        }
        let closing_rate = if self.stream.is_empty() {
            0.0
        } else {
            self.passages.len() as f64 / self.stream.len() as f64
        };
        let mut ranked: Vec<(usize, f64, u32)> = after
            .into_iter()
            .filter(|&(_, (total, _, _, _))| total >= 2)
            // A mark that ends a sentence is written onto the end of the word it finishes, and is
            // followed by the start of the next one. A mark that opens something instead, like the
            // one heading a section, stands apart from what follows it and fails this. Both halves
            // are needed: either alone admits the wrong marks.
            .filter(|&(_, (total, _, attached, _))| attached * 2 > total)
            .map(|(word, (total, capitals, _, _))| {
                let last = finals.get(&word).copied().unwrap_or(0);
                (
                    word,
                    f64::from(capitals) / f64::from(total),
                    total,
                    f64::from(last) / f64::from(total),
                )
            })
            // Measured against how often a capital turns up at all, rather than against a figure
            // set down here. A mark that is followed by the start of a sentence more often than
            // chance would explain is doing the ending; every other mark is doing something else.
            .filter(|&(_, share, _, _)| share > self.capital_rate())
            // The last thing a writer puts in a passage is the mark that finishes the last
            // sentence. A mark that wraps a name, or introduces a list, lands there no more often
            // than chance would put it there, and this is what tells the two kinds apart when
            // both sit against a word and both are followed by a capitalised name.
            .filter(|&(_, _, _, ending)| ending > closing_rate)
            .map(|(word, share, total, _)| (word, share, total))
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.cmp(&a.2)));
        ranked.into_iter().map(|(word, _, _)| word).collect()
    }

    /// Whether the token at this place actually finishes a sentence.
    ///
    /// Being the right mark is not enough. The dot in a decimal is the same character as the one
    /// that ends a sentence, and only what follows tells them apart: a sentence is followed by
    /// the start of another, or by nothing at all.
    fn ends_sentence(&self, index: usize) -> bool {
        let Some(token) = self.stream.get(index) else {
            return false;
        };
        if !self.terminators.contains(&token.word) {
            return false;
        }
        self.stream.get(index + 1).is_none_or(|next| next.capital)
    }

    /// How often a token in this text is written with a leading capital.
    // A count of words in a text. A count large enough to lose a bit here is a text nobody
    // has, and a rate taken from one reads the same either way.
    #[allow(clippy::cast_precision_loss)]
    fn capital_rate(&self) -> f64 {
        if self.stream.is_empty() {
            return 1.0;
        }
        let capitals = self.stream.iter().filter(|token| token.capital).count();
        capitals as f64 / self.stream.len() as f64
    }

    /// Count which word follows which, and which words open and close sentences.
    fn count_transitions(&mut self) {
        let terminators = self.terminators.clone();
        let mut opening = true;
        let mut previous: Option<usize> = None;
        let mut counts: BTreeMap<(usize, usize), u32> = BTreeMap::new();
        let mut attached: Vec<u32> = vec![0; self.vocabulary.len()];
        let mut opened: Vec<u32> = vec![0; self.vocabulary.len()];
        let mut closed: Vec<u32> = vec![0; self.vocabulary.len()];
        for (index, token) in self.stream.iter().enumerate() {
            let ends = self.ends_sentence(index);
            let _ = &terminators;
            if token.attached {
                attached[token.word] += 1;
            }
            if opening && !ends {
                opened[token.word] += 1;
                opening = false;
            }
            if let Some(last) = previous {
                *counts.entry((last, token.word)).or_insert(0) += 1;
            }
            if ends {
                closed[token.word] += 1;
                opening = true;
                previous = None;
            } else {
                previous = Some(token.word);
            }
        }
        let mut varied: Vec<u32> = vec![0; self.vocabulary.len()];
        for (before, _) in counts.keys() {
            varied[*before] += 1;
        }
        self.varied = varied;
        if let Some(last) = previous {
            closed[last] += 1;
        }
        for (position, entry) in self.vocabulary.iter_mut().enumerate() {
            entry.opened = opened[position];
            entry.closed = closed[position];
            entry.attached = attached[position];
        }
        self.follows = counts;
    }

    /// Count which words were seen in text attached to which property.
    fn count_marks(&mut self) {
        let mut marks: BTreeMap<(Feature, usize), u32> = BTreeMap::new();
        let mut weight: BTreeMap<Feature, u32> = BTreeMap::new();
        for (start, end, features) in &self.passages {
            for token in &self.stream[*start..*end] {
                if token.marking {
                    continue;
                }
                for feature in features {
                    *marks.entry((*feature, token.word)).or_insert(0) += 1;
                    *weight.entry(*feature).or_insert(0) += 1;
                }
            }
        }
        self.marks = marks;
        self.weight = weight;
    }
}

/// How much of a count to assume for something never seen.
const SMOOTHING: f64 = 0.5;

/// Split text into words and marks, reporting the gap that preceded each.
///
/// This is segmentation, not reading: it decides where one token stops and the next starts using
/// nothing but the character classes Unicode already defines. It never looks at what a token
/// says, so it carries no opinion about any language, subject or spelling into what follows.
fn tokens(text: &str) -> Vec<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        let gap_start = position;
        while position < bytes.len() && (bytes[position] as char).is_whitespace() {
            position += 1;
        }
        let gap = position - gap_start;
        if position >= bytes.len() {
            break;
        }
        let start = position;
        let head = text[position..].chars().next().unwrap_or(' ');
        if head.is_alphanumeric() {
            while position < bytes.len() {
                let Some(next) = text[position..].chars().next() else {
                    break;
                };
                if next.is_alphanumeric() || next == '\'' || next == '_' {
                    position += next.len_utf8();
                } else {
                    break;
                }
            }
        } else {
            position += head.len_utf8();
        }
        found.push((&text[start..position], gap));
    }
    found
}

/// How far to trust a ratio taken from this many observations.
///
/// A ratio of rates is at its wildest where it rests on least: a word written once, in one place,
/// looks infinitely characteristic of that place, and a measure that believes it will write a
/// sentence out of whatever the input said only once. The correction is the one Pantel and Lin
/// give for the same fault in pointwise mutual information: weigh the ratio by how many times the
/// pair was actually seen, and by how often the rarer of the two was seen at all. A ratio resting
/// on one observation is pulled most of the way back to saying nothing; a ratio resting on twenty
/// is left almost as it was found. Both numbers are counts of the text, so there is nothing here
/// to tune.
fn discount(joint: u32, left: u32, right: u32) -> f64 {
    let joint = f64::from(joint);
    let rarer = f64::from(left.min(right));
    (joint / (joint + 1.0)) * (rarer / (rarer + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corpus_knows_nothing_before_it_reads() {
        let corpus = Corpus::new();
        assert_eq!(corpus.vocabulary(), 0);
        assert_eq!(corpus.terminator(), None);
    }

    #[test]
    fn it_learns_which_mark_ends_a_sentence() {
        let mut corpus = Corpus::new();
        corpus.observe(
            "The engine reads. The engine writes. A reader checks it. Nothing else does.",
            Span::new(0, 74),
        );
        corpus.settle();
        let terminator = corpus.terminator().expect("a terminator was observed");
        assert_eq!(corpus.spelling(terminator, false), ".");
    }

    #[test]
    fn a_word_only_ever_capitalised_stays_capitalised() {
        let mut corpus = Corpus::new();
        corpus.observe("Rust holds. Rust binds.", Span::new(0, 23));
        corpus.settle();
        let word = corpus
            .words()
            .find(|&w| corpus.spelling(w, true) == "Rust")
            .unwrap();
        assert_eq!(corpus.spelling(word, false), "Rust");
    }

    #[test]
    fn a_word_specific_to_a_feature_beats_one_used_everywhere() {
        let mut corpus = Corpus::new();
        let optional = Feature::of("optional");
        corpus.attach(&[optional], "the answer may be missing", Span::new(0, 25));
        corpus.observe("the answer is here the answer is there", Span::new(25, 63));
        corpus.settle();
        let missing = corpus
            .words()
            .find(|&w| corpus.spelling(w, false) == "missing")
            .unwrap();
        let the = corpus
            .words()
            .find(|&w| corpus.spelling(w, false) == "the")
            .unwrap();
        assert!(corpus.affinity(optional, missing) > corpus.affinity(optional, the));
    }

    #[test]
    fn punctuation_is_written_without_a_space_before_it() {
        let mut corpus = Corpus::new();
        corpus.observe("one, two, three. Four, five, six.", Span::new(0, 32));
        corpus.settle();
        let comma = corpus
            .words()
            .find(|&w| corpus.spelling(w, false) == ",")
            .unwrap();
        assert!(corpus.is_attached(comma));
        assert!(corpus.is_marking(comma));
    }
}
