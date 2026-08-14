//! What categories a word may hold, and when to admit knowing nothing.
//!
//! Closed classes are listed exhaustively, which is possible because English does not coin new
//! determiners or pronouns. Open classes are resolved by inflection, and a word that neither
//! settles is refused rather than guessed.

use fitkit::core::{Answer, Refusal, Reported};
use fitkit::{Citation, Law};

use crate::tag::{Case, Form, Number, Person, Tag};
use crate::token::Token;

/// Determiners, with the number each one permits.
const DETERMINERS: &[(&str, &[Number])] = &[
    ("a", &[Number::Singular]),
    ("an", &[Number::Singular]),
    ("another", &[Number::Singular]),
    ("each", &[Number::Singular]),
    ("every", &[Number::Singular]),
    ("this", &[Number::Singular]),
    ("that", &[Number::Singular]),
    ("these", &[Number::Plural]),
    ("those", &[Number::Plural]),
    ("both", &[Number::Plural]),
    ("many", &[Number::Plural]),
    ("several", &[Number::Plural]),
    ("few", &[Number::Plural]),
    ("the", &[Number::Singular, Number::Plural]),
    ("its", &[Number::Singular, Number::Plural]),
    ("my", &[Number::Singular, Number::Plural]),
    ("your", &[Number::Singular, Number::Plural]),
    ("his", &[Number::Singular, Number::Plural]),
    ("her", &[Number::Singular, Number::Plural]),
    ("our", &[Number::Singular, Number::Plural]),
    ("their", &[Number::Singular, Number::Plural]),
    ("some", &[Number::Singular, Number::Plural]),
    ("any", &[Number::Singular, Number::Plural]),
    ("no", &[Number::Singular, Number::Plural]),
    ("all", &[Number::Singular, Number::Plural]),
    ("most", &[Number::Singular, Number::Plural]),
];

/// Personal pronouns, exhaustive for the standard paradigm.
const PRONOUNS: &[(&str, Person, Number, Case)] = &[
    ("i", Person::First, Number::Singular, Case::Subject),
    ("me", Person::First, Number::Singular, Case::Object),
    ("we", Person::First, Number::Plural, Case::Subject),
    ("us", Person::First, Number::Plural, Case::Object),
    ("you", Person::Second, Number::Singular, Case::Either),
    ("he", Person::Third, Number::Singular, Case::Subject),
    ("she", Person::Third, Number::Singular, Case::Subject),
    ("it", Person::Third, Number::Singular, Case::Either),
    ("him", Person::Third, Number::Singular, Case::Object),
    ("them", Person::Third, Number::Plural, Case::Object),
    ("they", Person::Third, Number::Plural, Case::Subject),
    ("who", Person::Third, Number::Singular, Case::Subject),
    ("everyone", Person::Third, Number::Singular, Case::Either),
    ("everybody", Person::Third, Number::Singular, Case::Either),
    ("someone", Person::Third, Number::Singular, Case::Either),
    ("somebody", Person::Third, Number::Singular, Case::Either),
    ("anyone", Person::Third, Number::Singular, Case::Either),
    ("nobody", Person::Third, Number::Singular, Case::Either),
    ("something", Person::Third, Number::Singular, Case::Either),
    ("nothing", Person::Third, Number::Singular, Case::Either),
];

/// Verb forms that no rule can derive, listed by form.
const IRREGULAR_VERBS: &[(&str, &[Form])] = &[
    ("be", &[Form::Base]),
    ("am", &[Form::Base]),
    ("is", &[Form::ThirdSingular]),
    ("are", &[Form::Base]),
    ("was", &[Form::PastSingular]),
    ("were", &[Form::PastPlural]),
    ("been", &[Form::Participle]),
    ("being", &[Form::Gerund]),
    ("have", &[Form::Base]),
    ("has", &[Form::ThirdSingular]),
    ("had", &[Form::Past, Form::Participle]),
    ("having", &[Form::Gerund]),
    ("do", &[Form::Base]),
    ("does", &[Form::ThirdSingular]),
    ("did", &[Form::Past]),
    ("done", &[Form::Participle]),
    ("doing", &[Form::Gerund]),
    ("go", &[Form::Base]),
    ("goes", &[Form::ThirdSingular]),
    ("went", &[Form::Past]),
    ("gone", &[Form::Participle]),
    ("say", &[Form::Base]),
    ("says", &[Form::ThirdSingular]),
    ("said", &[Form::Past, Form::Participle]),
    ("make", &[Form::Base]),
    ("makes", &[Form::ThirdSingular]),
    ("made", &[Form::Past, Form::Participle]),
    ("take", &[Form::Base]),
    ("takes", &[Form::ThirdSingular]),
    ("took", &[Form::Past]),
    ("taken", &[Form::Participle]),
    ("come", &[Form::Base, Form::Participle]),
    ("comes", &[Form::ThirdSingular]),
    ("came", &[Form::Past]),
    ("see", &[Form::Base]),
    ("sees", &[Form::ThirdSingular]),
    ("saw", &[Form::Past]),
    ("seen", &[Form::Participle]),
    ("know", &[Form::Base]),
    ("knows", &[Form::ThirdSingular]),
    ("knew", &[Form::Past]),
    ("known", &[Form::Participle]),
    ("give", &[Form::Base]),
    ("gives", &[Form::ThirdSingular]),
    ("gave", &[Form::Past]),
    ("given", &[Form::Participle]),
    ("find", &[Form::Base]),
    ("finds", &[Form::ThirdSingular]),
    ("found", &[Form::Past, Form::Participle]),
    ("think", &[Form::Base]),
    ("thinks", &[Form::ThirdSingular]),
    ("thought", &[Form::Past, Form::Participle]),
    ("write", &[Form::Base]),
    ("writes", &[Form::ThirdSingular]),
    ("wrote", &[Form::Past]),
    ("written", &[Form::Participle]),
    ("run", &[Form::Base, Form::Participle]),
    ("runs", &[Form::ThirdSingular]),
    ("ran", &[Form::Past]),
    ("eat", &[Form::Base]),
    ("eats", &[Form::ThirdSingular]),
    ("ate", &[Form::Past]),
    ("eaten", &[Form::Participle]),
    ("get", &[Form::Base]),
    ("gets", &[Form::ThirdSingular]),
    ("got", &[Form::Past, Form::Participle]),
    ("put", &[Form::Base, Form::Past, Form::Participle]),
    ("puts", &[Form::ThirdSingular]),
    ("ought", &[Form::Base, Form::ThirdSingular]),
    ("'s", &[Form::ThirdSingular]),
    ("'re", &[Form::Base]),
    ("'ve", &[Form::Base]),
    ("'m", &[Form::Base]),
    ("'d", &[Form::Past]),
    ("lie", &[Form::Base]),
    ("lies", &[Form::ThirdSingular]),
    ("lay", &[Form::Base, Form::Past]),
    ("lain", &[Form::Participle]),
];

/// Nouns whose plural no rule derives.
const IRREGULAR_NOUNS: &[(&str, Number)] = &[
    ("man", Number::Singular),
    ("men", Number::Plural),
    ("woman", Number::Singular),
    ("women", Number::Plural),
    ("child", Number::Singular),
    ("children", Number::Plural),
    ("person", Number::Singular),
    ("people", Number::Plural),
    ("foot", Number::Singular),
    ("feet", Number::Plural),
    ("tooth", Number::Singular),
    ("teeth", Number::Plural),
    ("mouse", Number::Singular),
    ("mice", Number::Plural),
    ("goose", Number::Singular),
    ("geese", Number::Plural),
    ("datum", Number::Singular),
    ("criterion", Number::Singular),
    ("criteria", Number::Plural),
    ("analysis", Number::Singular),
    ("analyses", Number::Plural),
];

/// Nouns that take the same form in both numbers, so only context decides.
///
/// "data" is here rather than among the irregulars because both "this data is" and "these data
/// are" are current, and an engine that cannot cite a rule against one must accept both.
const INVARIANT_NOUNS: &[&str] = &[
    "sheep",
    "fish",
    "deer",
    "series",
    "species",
    "aircraft",
    "means",
    "offspring",
    "data",
];

/// Modals, exhaustive. "ought" is absent because it takes a to-infinitive, which no modal does,
/// so it is listed among the verbs instead. "wo", "ca", and "sha" are what splitting "won't",
/// "can't", and "shan't" leaves behind.
const MODALS: &[&str] = &[
    "can", "could", "may", "might", "shall", "should", "will", "would", "must", "'ll", "wo", "ca",
    "sha",
];

/// Prepositions, the common core.
const PREPOSITIONS: &[&str] = &[
    "about",
    "above",
    "across",
    "after",
    "against",
    "along",
    "among",
    "around",
    "at",
    "before",
    "behind",
    "below",
    "beneath",
    "beside",
    "between",
    "beyond",
    "by",
    "despite",
    "down",
    "during",
    "except",
    "for",
    "from",
    "in",
    "inside",
    "into",
    "like",
    "near",
    "of",
    "off",
    "on",
    "onto",
    "outside",
    "over",
    "past",
    "since",
    "through",
    "throughout",
    "toward",
    "towards",
    "under",
    "until",
    "up",
    "upon",
    "with",
    "within",
    "without",
];

/// Coordinators, exhaustive for the central class.
const COORDINATORS: &[&str] = &["and", "or", "but", "nor", "yet", "so"];

/// Subordinators, the common core.
const SUBORDINATORS: &[&str] = &[
    "although", "because", "before", "if", "once", "since", "though", "unless", "until", "when",
    "whenever", "where", "whereas", "wherever", "while", "whether", "that", "than",
];

/// Adverbs that no suffix reveals.
const BARE_ADVERBS: &[&str] = &[
    "not",
    "never",
    "always",
    "often",
    "sometimes",
    "rarely",
    "seldom",
    "very",
    "quite",
    "too",
    "also",
    "just",
    "only",
    "still",
    "already",
    "soon",
    "now",
    "then",
    "here",
    "there",
    "well",
    "almost",
    "even",
    "rather",
    "perhaps",
    "however",
    "therefore",
    "thus",
    "instead",
    "again",
    "n't",
];

/// Verbs that take a to-infinitive, which is what tells "to walk" from "to the store".
///
/// Subcategorization is a property of the individual verb and cannot be derived, so the common
/// ones are listed.
pub const TAKES_INFINITIVE: &[&str] = &[
    "want",
    "wants",
    "wanted",
    "need",
    "needs",
    "needed",
    "try",
    "tries",
    "tried",
    "hope",
    "hopes",
    "hoped",
    "plan",
    "plans",
    "planned",
    "decide",
    "decides",
    "decided",
    "like",
    "likes",
    "liked",
    "love",
    "loves",
    "loved",
    "begin",
    "begins",
    "began",
    "start",
    "starts",
    "started",
    "continue",
    "continues",
    "continued",
    "seem",
    "seems",
    "seemed",
    "appear",
    "appears",
    "appeared",
    "refuse",
    "refuses",
    "refused",
    "agree",
    "agrees",
    "agreed",
    "learn",
    "learns",
    "learned",
    "expect",
    "expects",
    "expected",
    "intend",
    "intends",
    "intended",
    "manage",
    "manages",
    "managed",
    "offer",
    "offers",
    "offered",
    "promise",
    "promises",
    "promised",
    "ought",
    "used",
    "failed",
    "fails",
    "fail",
];

/// Adjectives that no suffix reveals.
const BARE_ADJECTIVES: &[&str] = &[
    "good",
    "bad",
    "big",
    "small",
    "large",
    "little",
    "long",
    "short",
    "high",
    "low",
    "old",
    "new",
    "young",
    "great",
    "own",
    "same",
    "different",
    "important",
    "able",
    "sure",
    "clear",
    "hard",
    "easy",
    "early",
    "late",
    "hot",
    "cold",
    "warm",
    "cool",
    "fast",
    "slow",
    "quick",
    "red",
    "green",
    "blue",
    "black",
    "white",
    "brown",
    "grey",
    "gray",
    "wet",
    "dry",
    "full",
    "empty",
    "heavy",
    "light",
    "strong",
    "weak",
    "loud",
    "quiet",
    "clean",
    "dirty",
    "rich",
    "poor",
    "safe",
    "free",
    "true",
    "false",
    "real",
    "main",
    "next",
    "last",
    "first",
    "second",
];

/// Whether the lexicon lists `key` outright.
#[must_use]
pub fn places(key: &str) -> bool {
    listed(key).is_some()
}

/// Every pronoun tag the lexicon can produce, without repeats.
#[must_use]
pub fn pronouns() -> Vec<Tag> {
    let mut tags: Vec<Tag> = Vec::new();
    for (_, person, number, case) in PRONOUNS {
        let tag = Tag::Pronoun(*person, *number, *case);
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// The categories a word may hold.
///
/// Answers only from the listed classes and from inflection. A word it cannot place is
/// [`Reported::Unreported`], not a guess, so the tagger refuses rather than inventing a reading.
pub struct Lexicon;

impl Law for Lexicon {
    type Input = Token;
    type Output = Reported<Vec<Tag>>;

    fn citation(&self) -> Citation {
        Citation {
            key: "HuddlestonPullum2002",
            source: "Huddleston & Pullum, The Cambridge Grammar of the English Language, \
                     Cambridge University Press, 2002, chapters 3 to 5",
        }
    }

    fn admits(&self, input: &Self::Input) -> Answer<()> {
        if input.key.is_empty() {
            return Err(Refusal::incoherent("an empty token"));
        }
        Ok(())
    }

    fn derive(&self, input: &Self::Input) -> Answer<Self::Output> {
        let tags = listed(&input.key).unwrap_or_else(|| inflected(&input.key, input.capitalised));
        Ok(if tags.is_empty() {
            Reported::Unreported
        } else {
            Reported::Known(tags)
        })
    }
}

/// Categories from the closed classes, which are listed rather than derived.
fn listed(key: &str) -> Option<Vec<Tag>> {
    let mut tags = Vec::new();
    if key.chars().all(|c| c.is_ascii_digit() || c == '-')
        && key.chars().any(|c| c.is_ascii_digit())
    {
        return Some(vec![Tag::Numeral]);
    }
    if key.chars().all(|c| !c.is_alphanumeric()) {
        return Some(vec![Tag::Mark]);
    }
    for (word, numbers) in DETERMINERS {
        if *word == key {
            tags.extend(numbers.iter().map(|n| Tag::Determiner(*n)));
        }
    }
    for (word, person, number, case) in PRONOUNS {
        if *word == key {
            tags.push(Tag::Pronoun(*person, *number, *case));
        }
    }
    for (word, forms) in IRREGULAR_VERBS {
        if *word == key {
            tags.extend(forms.iter().map(|f| Tag::Verb(*f)));
        }
    }
    for (word, number) in IRREGULAR_NOUNS {
        if *word == key {
            tags.push(Tag::Noun(*number));
        }
    }
    if INVARIANT_NOUNS.contains(&key) {
        tags.push(Tag::Noun(Number::Singular));
        tags.push(Tag::Noun(Number::Plural));
    }
    if MODALS.contains(&key) {
        tags.push(Tag::Modal);
    }
    if PREPOSITIONS.contains(&key) {
        tags.push(Tag::Preposition);
    }
    if COORDINATORS.contains(&key) {
        tags.push(Tag::Coordinator);
    }
    if SUBORDINATORS.contains(&key) {
        tags.push(Tag::Subordinator);
    }
    if BARE_ADVERBS.contains(&key) {
        tags.push(Tag::Adverb);
    }
    if BARE_ADJECTIVES.contains(&key) {
        tags.push(Tag::Adjective);
    }
    if key == "to" {
        tags.push(Tag::To);
        tags.push(Tag::Preposition);
    }
    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}

/// Categories an ending reveals. Every reading the ending permits is offered, most likely first,
/// and the search decides between them, since English marks plural nouns and present tense verbs
/// alike.
///
/// A string with no vowel letter is not an English word, so it yields nothing and the caller
/// reports it unknown.
fn inflected(key: &str, capitalised: bool) -> Vec<Tag> {
    let mut tags = Vec::new();
    if !key.contains(['a', 'e', 'i', 'o', 'u', 'y']) {
        return tags;
    }
    if capitalised {
        tags.push(Tag::Proper(Number::Singular));
    }
    if let Some(stem) = key.strip_suffix("ing") {
        if stem.len() >= 3 {
            tags.push(Tag::Verb(Form::Gerund));
        }
    }
    if let Some(stem) = key.strip_suffix("ed") {
        if stem.len() >= 2 {
            tags.push(Tag::Verb(Form::Past));
            tags.push(Tag::Verb(Form::Participle));
        }
    }
    if let Some(stem) = key.strip_suffix("ly") {
        if stem.len() >= 3 {
            tags.push(Tag::Adverb);
        }
    }
    if key.ends_with("ness")
        || key.ends_with("tion")
        || key.ends_with("ment")
        || key.ends_with("ity")
    {
        tags.push(Tag::Noun(Number::Singular));
    }
    if key.ends_with("ous") || key.ends_with("ful") || key.ends_with("ive") || key.ends_with("able")
    {
        tags.push(Tag::Adjective);
    }
    if let Some(stem) = key.strip_suffix('s') {
        if stem.len() >= 2 && !stem.ends_with('s') {
            tags.push(Tag::Verb(Form::ThirdSingular));
            tags.push(Tag::Noun(Number::Plural));
        }
    }
    if capitalised {
        tags.retain(|tag| !matches!(tag, Tag::Noun(_)));
    }
    if tags.is_empty() && key.len() >= 2 {
        tags.push(Tag::Noun(Number::Singular));
        tags.push(Tag::Verb(Form::Base));
    }
    let mut seen = Vec::with_capacity(tags.len());
    tags.retain(|tag| {
        seen.iter().any(|kept| kept == tag) || {
            seen.push(*tag);
            true
        }
    });
    tags
}

#[cfg(test)]
mod tests {
    use fitkit::{ask, Reported};

    use super::Lexicon;
    use crate::tag::{Form, Number, Tag};
    use crate::token::tokenise;

    fn tags(word: &str) -> Reported<Vec<Tag>> {
        ask(&Lexicon, &tokenise(word).remove(0)).expect("a word is a coherent question")
    }

    #[test]
    fn a_determiner_carries_only_the_numbers_it_permits() {
        let Reported::Known(a) = tags("a") else {
            panic!("a is listed")
        };
        assert_eq!(a, [Tag::Determiner(Number::Singular)]);
        let Reported::Known(these) = tags("these") else {
            panic!("these is listed")
        };
        assert_eq!(these, [Tag::Determiner(Number::Plural)]);
    }

    #[test]
    fn a_word_ending_in_s_offers_both_readings() {
        let Reported::Known(found) = tags("walks") else {
            panic!("walks inflects")
        };
        assert!(found.contains(&Tag::Noun(Number::Plural)));
        assert!(found.contains(&Tag::Verb(Form::ThirdSingular)));
    }

    #[test]
    fn an_unplaceable_word_is_unreported_rather_than_guessed() {
        assert!(
            matches!(tags("qq"), Reported::Unreported),
            "no vowel, so not a word"
        );
    }

    #[test]
    fn an_unlisted_open_class_word_offers_the_readings_english_allows() {
        let Reported::Known(found) = tags("kestrel") else {
            panic!("it looks like a word")
        };
        assert!(found.contains(&Tag::Noun(Number::Singular)));
        assert!(found.contains(&Tag::Verb(Form::Base)));
    }
}
