//! The smallest change that makes a sentence grammatical.
//!
//! Only words at a fault are touched, and only by swapping one inflected form for another of the
//! same word. Nothing is added, removed, or reworded, so a repair never changes what was meant.

use crate::check::{check_in, Report};
use crate::grammar::{Rule, Sentence};
use crate::register::{Convention, Register};
use crate::tag::{Form, Tag};

/// A word replaced by another form of itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Which token.
    pub at: usize,
    /// What it becomes.
    pub word: String,
}

/// The fewest edits that leave no fault, if any set of single-word swaps does.
///
/// Returns nothing when the sentence is already clean, rests on an unknown word, or cannot be
/// fixed by inflection alone. Refusing is the right answer for the last case: a sentence needing
/// a rewrite is not a sentence needing a repair.
#[must_use]
pub fn repair(sentence: &Sentence) -> Option<Vec<Edit>> {
    repair_in(sentence, Register::STRICT)
}

/// The fewest edits that leave nothing `register` holds against the sentence.
///
/// Repairing under a register rather than under the strictest reading is what stops a summary line
/// being conjugated. "One pass of the trellis." is a heading, and a repair asked to find it a verb
/// answers "Ones", which is not a correction of anything. A unit is only worth repairing once it
/// has been read as the kind of thing it is.
#[must_use]
pub fn repair_in(sentence: &Sentence, register: Register) -> Option<Vec<Edit>> {
    // No swap of one form for another puts a capital at the front or a full stop at the end, so
    // holding a sentence to that convention here would only stop every repair the sentence did
    // have. Whether it is punctuated is reported where it can be acted on.
    let register = register.without(Convention::Marks);
    let report = check_in(sentence, register);
    if report.faults.is_empty() || !report.unknown.is_empty() {
        return None;
    }
    let sites = sites(&report);
    let reading = &report.tags;
    single(sentence, &sites, reading, register, &report)
        .or_else(|| pair(sentence, &sites, reading, register, &report))
}

/// Positions a fault touches, latest first.
///
/// The later word is tried first, so a verb is brought to its subject rather than a subject
/// rewritten to suit its verb. The subject is usually what the writer meant.
fn sites(report: &Report) -> Vec<usize> {
    let mut sites: Vec<usize> = report
        .faults
        .iter()
        .flat_map(|fault| [fault.at.start, fault.at.end.saturating_sub(1)])
        .collect();
    sites.sort_unstable();
    sites.dedup();
    sites.reverse();
    sites
}

/// Whether a rule broken over `at` asks for the plain form of a verb.
///
/// A repair offers the form the broken rule named. Two things have to hold before an ending comes
/// off, and each rules out a different way of inventing a stem.
///
/// The rule broken there must be one that asks for the plain form, so the sentence is actually
/// short of a form rather than merely holding a word that ends in those letters. The word must
/// also have been read as a participle or a preterite, because a word can end in those letters
/// without carrying the inflection. Both questions are answered by what the engine already
/// settled rather than by the spelling, which is what keeps the rule from being a guess about
/// letters.
fn strips(report: &Report, reading: &[Tag], at: usize) -> bool {
    let asked = report.faults.iter().any(|fault| {
        matches!(fault.rule, Rule::ModalTakesBase | Rule::ToTakesBase)
            && at >= fault.at.start
            && at < fault.at.end
    });
    asked
        && matches!(
            reading.get(at),
            Some(Tag::Verb(Form::Past | Form::Participle))
        )
}

/// Whether the word at `at` has other forms to offer, given how it was read.
///
/// Number is a feature of nouns, verbs and determiners. An adjective, an adverb or a preposition
/// does not have it, and inflecting one derives a word that does not exist because there was
/// never a form to reach: "pairwise" gives "pairwises", "finer" gives "finers", "out" gives
/// "outs". Asking what the word looks like cannot catch these, because they look exactly like
/// plurals. What catches them is the reading the engine already settled on.
fn inflects(reading: &[Tag], at: usize) -> bool {
    reading.get(at).is_none_or(|tag| {
        matches!(
            tag,
            Tag::Noun(_) | Tag::Verb(_) | Tag::Determiner(_) | Tag::Pronoun(..)
        )
    })
}

/// One swap that clears every fault.
fn single(
    sentence: &Sentence,
    sites: &[usize],
    reading: &[Tag],
    register: Register,
    report: &Report,
) -> Option<Vec<Edit>> {
    sites
        .iter()
        .filter(|&&at| inflects(reading, at))
        .find_map(|&at| {
            forms(&sentence.tokens[at].word, strips(report, reading, at))
                .into_iter()
                .find(|word| {
                    clears(
                        sentence,
                        &[Edit {
                            at,
                            word: word.clone(),
                        }],
                        register,
                    )
                })
                .map(|word| vec![Edit { at, word }])
        })
}

/// Two swaps, for a fault no single word can settle.
fn pair(
    sentence: &Sentence,
    sites: &[usize],
    reading: &[Tag],
    register: Register,
    report: &Report,
) -> Option<Vec<Edit>> {
    for (index, &first) in sites.iter().enumerate() {
        if !inflects(reading, first) {
            continue;
        }
        for &second in &sites[index + 1..] {
            if !inflects(reading, second) {
                continue;
            }
            for left in forms(&sentence.tokens[first].word, strips(report, reading, first)) {
                for right in forms(
                    &sentence.tokens[second].word,
                    strips(report, reading, second),
                ) {
                    let edits = vec![
                        Edit {
                            at: first,
                            word: left.clone(),
                        },
                        Edit {
                            at: second,
                            word: right,
                        },
                    ];
                    if clears(sentence, &edits, register) {
                        return Some(edits);
                    }
                }
            }
        }
    }
    None
}

/// Whether applying `edits` leaves a sentence with no fault and nothing unknown.
fn clears(sentence: &Sentence, edits: &[Edit], register: Register) -> bool {
    let report = check_in(&apply(sentence, edits), register);
    report.is_clean()
}

/// `sentence` with `edits` applied.
#[must_use]
pub fn apply(sentence: &Sentence, edits: &[Edit]) -> Sentence {
    let mut out = sentence.clone();
    for edit in edits {
        if let Some(token) = out.tokens.get_mut(edit.at) {
            *token = crate::token::retype(token, &edit.word);
        }
    }
    out
}

/// Other forms of the same word, and other determiners of the same kind.
///
/// Regular inflection only. An irregular form the lexicon knows is reachable because the lexicon
/// lists both members of the pair, and one that it does not know is out of reach, which the
/// caller reports as a refusal to repair rather than a guess.
///
/// Inflection is a way of proposing a word, not a way of making one: applied to a full stop it
/// proposes ".es" and applied to a numeral it proposes "2es", and a repair that writes either has
/// done more damage than the fault it was called to mend. Three things bound it, and each answers
/// a different way of inventing a word.
///
/// `wants_base` says whether the rule this word broke asked for the plain form of a verb. An
/// ending is taken off only then, because removing one invents a stem and adding one derives a
/// form from a word already in hand.
fn forms(word: &str, wants_base: bool) -> Vec<String> {
    let lower = word.to_lowercase();
    if !lower.chars().all(char::is_alphabetic) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (from, to) in PAIRS {
        if lower == *from {
            out.push((*to).to_string());
        }
        if lower == *to {
            out.push((*from).to_string());
        }
    }
    // A closed class word has the forms it has and no others. Inflecting one derives a word that
    // exists and means something else: "a" gives "as", "one" gives "ones", "it" gives "its", and
    // each is a real entry in the lexicon, so asking whether the result is a word does not catch
    // any of them. What catches them is that English does not make new determiners or pronouns.
    if !crate::lexicon::is_closed(&lower) {
        // Taking a plural ending off invents a stem in the same way taking a past ending off
        // does, and the round trip does not always catch it: "citru" plus "s" spells "citrus",
        // so the spelling licenses a word that is not one. What settles it is that the lexicon
        // already reads "citrus" as a singular, because English does not spell a plural by
        // adding a bare s to a stem ending in u. A rule asking for a singular and a word that
        // already offers one disagree about the reading, not about the spelling, so there is
        // nothing here to respell. "dogs" offers no singular and still reaches "dog".
        let singular = crate::lexicon::offers(&lower, Tag::Noun(crate::tag::Number::Singular));
        match plain(&lower) {
            Some(_) if singular => {}
            Some(stem) => out.push(stem),
            // English does not inflect an adverb for number. Nothing in the spelling says so, and
            // the plural rule applied to one invents a word: "correctly" gave "correctlies" and
            // "everywhere" gave "everywheres", both offered as repairs on a real repository. A
            // word that is only ever an adverb has no plural to reach for, so the rule is not
            // asked. A word that is also a noun keeps its plural, which is why "times" survives.
            None if crate::lexicon::offers(&lower, Tag::Adverb)
                && !crate::lexicon::offers(&lower, Tag::Noun(crate::tag::Number::Plural)) => {}
            // English does not put one inflection on top of another. A word already carrying a
            // verbal ending has no plural to offer, and marking it stacks a second: "summed"
            // gives "summeds", which has the shape of a plural and so passes every test that
            // asks what a word looks like rather than what it already is.
            None if !lower.ends_with("ed") && !lower.ends_with("ing") => {
                out.extend(marked(&lower));
            }
            None => {}
        }
        // Taking the past ending off is offered only where a rule has asked for the plain form.
        // English spells the past of a stem ending in "e" by adding "d" and of any other stem by
        // adding "ed", so a word ending in "ed" has two decompositions and its spelling does not
        // say which: "measured" is "measur" plus "ed" or "measure" plus "d", and both put the
        // word back together again. A round trip settles a plural because "categorys" is not how
        // English spells it, and it settles nothing here, so spelling alone cannot license the
        // strip. Over an unseen repository, offered wherever a word merely ended in "ed", it was
        // wrong nineteen times out of nineteen: "defin", "hydrat", "dissolv", "measur", "stat",
        // "ag" and "ne", and its two real words were both correct participles it would have
        // broken. None of those sat under a rule asking for a plain form. What licenses it is
        // that the sentence is being held to one: "he must walked" breaks a rule that names the
        // form it wants, and the stem is then a form the grammar asked for rather than a word
        // invented from a spelling.
        if wants_base {
            out.extend(bare(&lower));
        }
    }
    // What the lexicon lists, it has an opinion about, and a rule may not overrule it. "written"
    // is listed as a participle and "be" as a plain verb, so deriving "writtens" and "bes" is a
    // rule contradicting the entry it was applied to. Where the lexicon is silent the rule is all
    // there is and is trusted, which is how "file" still reaches "files".
    let listed = crate::lexicon::places(&lower);
    out.retain(|form| {
        *form != lower
            && !form.is_empty()
            && if listed {
                crate::lexicon::places(form)
            } else {
                crate::lexicon::knows(form)
            }
    });
    out.dedup();
    out
}

/// The verb with its third person singular ending on it.
///
/// English spells a plural noun and a third person singular verb the same way, so this is the
/// same spelling rule read for the other part of speech rather than a second rule to keep.
#[must_use]
pub fn third(word: &str) -> Option<String> {
    marked(&word.to_lowercase())
}

/// Whether a word is written as a plural, judged by the spelling English would have used.
///
/// A name in code is a noun phrase and the head of a noun phrase is its last word, so this is
/// what tells "ids" from "id" without either being listed anywhere. Spelling the singular back
/// out is what makes it an inflection rather than a coincidence: "addres" marked is "addreses",
/// which is not "address". A word the lexicon already reads as a singular is not a plural
/// however it is spelled, which is what keeps "status" and "citrus" out.
#[must_use]
pub fn is_plural(word: &str) -> bool {
    let lower = word.to_lowercase();
    plain(&lower).is_some()
        && !crate::lexicon::offers(&lower, Tag::Noun(crate::tag::Number::Singular))
}

/// The word with the past inflection taken off, where taking it off is not a guess.
///
/// English doubles a final consonant before "ed" sometimes and not others, and nothing in the
/// written word says which: "summed" comes from "sum" and "filled" comes from "fill". Stripping
/// the ending alone gives "summ", which no English word looks like, because no stem ends in a
/// doubled consonant. That is the tell, and where it shows the answer is refused rather than
/// guessed at.
fn bare(word: &str) -> Option<String> {
    let stem = word.strip_suffix("ed")?;
    let mut letters = stem.chars().rev();
    let last = letters.next()?;
    if letters.next() == Some(last) && !VOWELS.contains(last) {
        return None;
    }
    Some(stem.to_owned())
}

/// The word with the plural inflection on it, spelled the way English spells it.
///
/// One spelling, not every spelling. Offering both "{word}s" and "{word}es" and letting the
/// lexicon choose asks a question the lexicon cannot answer, because it places a word by its shape
/// and "categorys" has the shape of a plural. English decides this by the end of the stem and
/// leaves no choice. A sibilant takes \"es\". A consonant before a final \"y\" turns it into
/// \"ies\", and every other stem adds \"s\".
fn marked(word: &str) -> Option<String> {
    if word.is_empty() {
        return None;
    }
    if word.ends_with(['s', 'x', 'z']) || word.ends_with("ch") || word.ends_with("sh") {
        return Some(format!("{word}es"));
    }
    let mut letters = word.chars().rev();
    let last = letters.next()?;
    let before = letters.next();
    if last == 'y' && before.is_some_and(|before| !VOWELS.contains(before)) {
        return Some(format!("{}ies", &word[..word.len() - 1]));
    }
    if last == 'o' && before.is_some_and(|before| !VOWELS.contains(before)) {
        return Some(format!("{word}es"));
    }
    Some(format!("{word}s"))
}

/// The word with the plural inflection taken off, if that inflection is what put it there.
///
/// The same rule read backwards, and checked by running it forwards again. Stripping "es" from
/// "address" gives "addres", and only spelling the answer back out catches it: "addres" marked is
/// "addreses", which is not the word we started with, so the strip was never an inflection.
pub fn plain(word: &str) -> Option<String> {
    let stems = [
        word.strip_suffix("ies").map(|stem| format!("{stem}y")),
        word.strip_suffix("es").map(str::to_owned),
        word.strip_suffix('s').map(str::to_owned),
    ];
    stems
        .into_iter()
        .flatten()
        .find(|stem| marked(stem).as_deref() == Some(word))
}

/// The letters that make a preceding consonant matter to how a word is spelled.
const VOWELS: &str = "aeiou";

/// Words whose counterpart no rule derives.
const PAIRS: &[(&str, &str)] = &[
    ("a", "the"),
    ("an", "the"),
    ("this", "these"),
    ("that", "those"),
    ("every", "all"),
    ("is", "are"),
    ("was", "were"),
    ("has", "have"),
    ("does", "do"),
    ("man", "men"),
    ("woman", "women"),
    ("child", "children"),
    ("person", "people"),
    ("mouse", "mice"),
    ("foot", "feet"),
    ("tooth", "teeth"),
    ("criterion", "criteria"),
    ("analysis", "analyses"),
    ("datum", "data"),
];

#[cfg(test)]
mod tests {
    use super::{apply, forms, repair};
    use crate::check::check;
    use crate::grammar::Sentence;

    #[test]
    fn an_adverb_is_not_given_a_plural() {
        // "correctly" gave "correctlies" and "everywhere" gave "everywheres", both offered as
        // repairs on a real repository. Neither is a word.
        for word in ["correctly", "everywhere", "elsewhere", "quickly"] {
            assert!(
                !forms(word, false).iter().any(|form| form.ends_with('s')),
                "{word} was given a plural"
            );
        }
        assert!(forms("file", false).contains(&"files".to_owned()));
    }

    #[test]
    fn no_word_is_offered_a_second_inflection() {
        // "summed" already carries one. Marking it again gives "summeds", which has the shape of
        // a plural and passes every test that asks what a word looks like.
        assert!(!forms("summed", false)
            .iter()
            .any(|form| form.ends_with('s')));
        assert!(forms("file", false).contains(&"files".to_owned()));
    }

    #[test]
    fn a_stem_is_only_taken_off_where_a_rule_asked_for_the_plain_form() {
        // "measured" is "measur" plus "ed" or "measure" plus "d" and nothing in the spelling says
        // which. Where nothing asked for a plain form the question is left alone rather than
        // guessed at, which is what keeps "defin", "ag" and "ne" out of an unseen repository.
        for word in ["measured", "walked", "summed", "defined", "aged", "need"] {
            assert!(
                !forms(word, false)
                    .iter()
                    .any(|form| word.starts_with(form.as_str())),
                "a stem was guessed at for {word}: {:?}",
                forms(word, false)
            );
        }
        // Asked for, it is offered, and still refused where the spelling cannot say what the
        // stem was: "summed" would give "summ", which no English stem looks like.
        assert!(forms("walked", true).contains(&"walk".to_owned()));
        assert!(!forms("summed", true).contains(&"summ".to_owned()));
    }

    #[test]
    fn a_modal_left_with_a_past_verb_is_given_the_plain_form() {
        assert_eq!(fixed("he must walked"), "he must walk");
    }

    fn fixed(text: &str) -> String {
        let sentence = Sentence::read(text);
        let edits = repair(&sentence).expect("a fault this small has a repair");
        apply(&sentence, &edits)
            .tokens
            .iter()
            .map(|token| token.word.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn a_verb_is_brought_into_agreement() {
        assert_eq!(fixed("the dogs runs"), "the dogs run");
        assert_eq!(fixed("the dog run"), "the dog runs");
    }

    #[test]
    fn a_determiner_is_brought_into_agreement() {
        assert_eq!(fixed("a dogs run"), "the dogs run");
        assert_eq!(fixed("every dogs run"), "all dogs run");
    }

    #[test]
    fn an_irregular_pair_is_reachable() {
        assert_eq!(fixed("the man were here"), "the man was here");
    }

    #[test]
    fn a_repair_leaves_nothing_to_report() {
        for text in [
            "she can walks",
            "the criteria is clear",
            "the key to the cabinets are here",
        ] {
            let sentence = Sentence::read(text);
            let edits = repair(&sentence).expect("a repair exists");
            assert!(
                check(&apply(&sentence, &edits)).is_clean(),
                "{text} left a fault"
            );
        }
    }

    #[test]
    fn a_clean_sentence_needs_no_repair() {
        assert_eq!(repair(&Sentence::read("the dog runs")), None);
    }
}
