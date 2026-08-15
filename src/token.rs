//! Splitting text into what the tagger reads.

use fitkit::Span;

/// One word or mark, with where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The text as written.
    pub word: String,
    /// Lowercased, for lexicon lookup.
    pub key: String,
    /// Byte range in the source.
    pub at: Span,
    /// Whether the first letter was capitalised as written.
    pub capitalised: bool,
    /// Whether this token is a term being named rather than a word being used.
    pub mention: bool,
}

impl Token {
    /// Whether this token ends a sentence.
    #[must_use]
    pub fn ends_sentence(&self) -> bool {
        matches!(self.word.as_str(), "." | "!" | "?")
    }
}

/// Split text into words and marks.
///
/// Marks are separate tokens because they carry syntax. A contraction is split at its apostrophe,
/// so "she's" is a pronoun and a verb rather than one word that is neither.
#[must_use]
pub fn tokenise(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut start = None;
    let spans = names(text);
    let mut skip_to = 0;
    for (at, character) in text.char_indices() {
        if at < skip_to {
            continue;
        }
        if let Some(span) = spans.iter().find(|span| span.start == at) {
            if let Some(from) = start.take() {
                push(&mut tokens, text, from, at);
            }
            push_address(&mut tokens, text, span.start, span.end);
            skip_to = span.end;
            continue;
        }
        let part_of_word = character.is_alphanumeric() || character == '\'' || character == '-';
        match (part_of_word, start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                push(&mut tokens, text, from, at);
                start = None;
            }
            _ => {}
        }
        // Emphasis, escapes and headings are how a medium is written down, not how English is.
        // A reader sees the word inside them and no mark at all, so nothing is emitted for them.
        if !part_of_word && !character.is_whitespace() && !FORMATTING.contains(character) {
            push(&mut tokens, text, at, at + character.len_utf8());
        }
    }
    if let Some(from) = start {
        push(&mut tokens, text, from, text.len());
    }
    fold_mentions(tokens)
}

/// Where the names are in `text`.
///
/// A web address, a file path, or an identifier is one name, not a sentence. Split on its
/// punctuation it becomes a string of unknown words joined by marks, and every rule about what may
/// follow a mark then fires inside something that was never prose.
///
/// A name is found by its separators rather than by a list of hosts, extensions or namespaces, so
/// a name the author invented reads like any one already known. English does not join words with a
/// slash, does not put a stop in the middle of a word, and does not use an underscore at all, so a
/// run of non-space characters that does any of those is not English and is read as the one thing
/// it names.
fn names(text: &str) -> Vec<Span> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some(offset) = text[at..].find(|character: char| !character.is_whitespace()) else {
            break;
        };
        let from = at + offset;
        let to = text[from..]
            .find(char::is_whitespace)
            .map_or(text.len(), |end| from + end);
        let run = text[from..to].trim_end_matches(['.', ',', ';', ':', '!', '?', ')']);
        if named(run) {
            found.push(Span::new(from, from + run.len()));
        }
        at = to;
    }
    found
}

/// Whether a run of non-space characters is a name rather than a word.
fn named(run: &str) -> bool {
    if run.is_empty() {
        return false;
    }
    if run.contains('/') || run.contains('_') || run.contains("::") {
        return true;
    }
    // A stop with a letter or a digit straight after it is not the end of a sentence, because a
    // sentence never resumes without a space. It is the separator inside a name.
    run.char_indices().any(|(at, character)| {
        character == '.' && run[at + 1..].starts_with(|next: char| next.is_alphanumeric())
    })
}

/// Push a whole run as one name.
fn push_address(tokens: &mut Vec<Token>, text: &str, from: usize, to: usize) {
    let word = text[from..to].to_owned();
    tokens.push(Token {
        key: word.to_lowercase(),
        word,
        at: Span::new(from, to),
        capitalised: true,
        mention: true,
    });
}

/// Characters that mark up a medium rather than punctuating a sentence.
const FORMATTING: &str = "*\\#~|";

/// Marks that open and close a term being named rather than used.
const QUOTES: &[&str] = &["\"", "`", "\u{201c}", "\u{201d}"];

/// Fold each quoted or backticked run into one token standing for the term it names.
///
/// A named term is a noun whatever it contains. Without this, "a determiner and its noun must
/// share number, as in \"a dog\" but not \"a dogs\"" is read as though the writer had written
/// "a dogs", and the sentence is blamed for the mistake it is describing.
fn fold_mentions(tokens: Vec<Token>) -> Vec<Token> {
    let opener = |token: &Token| QUOTES.contains(&token.word.as_str());
    let mut folded: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut inside: Option<usize> = None;
    for token in tokens {
        match inside {
            Some(from) if opener(&token) => {
                let end = token.at.end;
                let held: Vec<Token> = folded.drain(from..).collect();
                let word: String = held
                    .iter()
                    .map(|held| held.word.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let at = Span::new(held.first().map_or(end, |first| first.at.start), end);
                folded.push(Token {
                    key: word.to_lowercase(),
                    word,
                    at,
                    // A quoted or backticked name is spelled the way the thing is spelled. An
                    // identifier that begins a sentence in lower case has not failed to open with
                    // a capital: it has no capital to open with, and demanding one would ask the
                    // writer to misname the thing they are naming.
                    capitalised: true,
                    mention: true,
                });
                inside = None;
            }
            None if opener(&token) => inside = Some(folded.len()),
            Some(_) | None => folded.push(token),
        }
    }
    folded
}

/// Endings that are words of their own, each with the spelling left when the apostrophe is dropped.
pub(crate) const CLITICS: &[(&str, &str)] = &[
    ("n't", "nt"),
    ("'s", "s"),
    ("'re", "re"),
    ("'ve", "ve"),
    ("'ll", "ll"),
    ("'d", "d"),
    ("'m", "m"),
];

fn push(tokens: &mut Vec<Token>, text: &str, from: usize, to: usize) {
    let word = &text[from..to];
    let lower = word.to_lowercase();
    for (clitic, _) in CLITICS {
        if let Some(stem) = lower.strip_suffix(clitic) {
            if !stem.is_empty() {
                let cut = from + stem.len();
                push(tokens, text, from, cut);
                one(tokens, text, cut, to);
                return;
            }
        }
    }
    one(tokens, text, from, to);
}

fn one(tokens: &mut Vec<Token>, text: &str, from: usize, to: usize) {
    let word = &text[from..to];
    tokens.push(Token {
        word: word.to_string(),
        key: word.to_lowercase(),
        at: Span::new(from, to),
        capitalised: word.chars().next().is_some_and(char::is_uppercase),
        mention: false,
    });
}

/// `token` with a different word, keeping its position and capitalisation.
#[must_use]
pub fn retype(token: &Token, word: &str) -> Token {
    let word = if token.capitalised {
        capitalise(word)
    } else {
        word.to_string()
    };
    Token {
        key: word.to_lowercase(),
        word,
        at: token.at,
        capitalised: token.capitalised,
        mention: token.mention,
    }
}

/// `word` with its first letter in upper case.
fn capitalise(word: &str) -> String {
    let mut letters = word.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// Split any word that a missing apostrophe left fused, so "dont" becomes "do" and "n't".
///
/// A word is only split where the stem is listed outright, so "dont" becomes "do" and "n't" while
/// "dogs" stays whole: "dog" is known by its shape, not by being listed, and that is not enough.
#[must_use]
pub fn mend(tokens: &[Token]) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        match split(token) {
            Some((stem, clitic)) => out.extend([stem, clitic]),
            None => out.push(token.clone()),
        }
    }
    out
}

/// Positions of words that a missing apostrophe left fused.
pub fn fused(tokens: &[Token]) -> impl Iterator<Item = usize> + '_ {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| split(token).is_some())
        .map(|(at, _)| at)
}

/// The two words a fused one holds, if it holds two.
/// Whether a clitic can attach to a stem, which decides whether a missing apostrophe is inferred.
///
/// "n't" attaches to an auxiliary and nothing else. The rest attach to a pronoun and nothing else.
/// Without this, "call" reads as "ca" and "'ll", and "form" as "for" and "'m".
fn attaches(stem: &str, clitic: &str) -> bool {
    if clitic == "n't" {
        return crate::lexicon::is_auxiliary(stem);
    }
    crate::lexicon::is_pronoun(stem)
}

fn split(token: &Token) -> Option<(Token, Token)> {
    if crate::lexicon::places(&token.key) {
        return None;
    }
    CLITICS.iter().find_map(|(clitic, bare)| {
        let stem = token.key.strip_suffix(bare)?;
        if !attaches(stem, clitic) {
            return None;
        }
        let at = stem.len();
        (!stem.is_empty() && crate::lexicon::places(stem) && crate::lexicon::places(clitic)).then(
            || {
                let cut = token.at.start + at;
                (
                    Token {
                        word: token.word[..at].to_string(),
                        key: stem.to_string(),
                        at: Span::new(token.at.start, cut),
                        capitalised: token.capitalised,
                        mention: false,
                    },
                    Token {
                        word: (*clitic).to_string(),
                        key: (*clitic).to_string(),
                        at: Span::new(cut, token.at.end),
                        capitalised: false,
                        mention: false,
                    },
                )
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::tokenise;

    #[test]
    fn a_run_joined_by_a_separator_english_never_uses_is_one_name() {
        for run in [
            "tests/corpus.rs",
            "Frame::every",
            "read_to_string",
            "docs/LIMITS.md",
        ] {
            let text = format!("See {run} for this.");
            let tokens = tokenise(&text);
            let words: Vec<&str> = tokens.iter().map(|t| t.word.as_str()).collect();
            assert_eq!(words, ["See", run, "for", "this", "."], "{run}");
        }
    }

    #[test]
    fn a_stop_inside_a_name_does_not_end_the_sentence() {
        let tokens = tokenise("tests/corpus.rs is labelled.");
        assert_eq!(
            tokens.iter().filter(|t| t.ends_sentence()).count(),
            1,
            "only the final stop ends the sentence"
        );
    }

    #[test]
    fn how_a_medium_is_written_down_is_not_read_as_punctuation() {
        let tokens = tokenise("What is wrong is the *selection*.");
        assert_eq!(
            tokens.iter().map(|t| t.word.as_str()).collect::<Vec<_>>(),
            ["What", "is", "wrong", "is", "the", "selection", "."]
        );
    }

    #[test]
    fn marks_are_their_own_tokens() {
        let tokens = tokenise("The dog runs.");
        assert_eq!(
            tokens.iter().map(|t| t.word.as_str()).collect::<Vec<_>>(),
            ["The", "dog", "runs", "."]
        );
    }

    #[test]
    fn a_contraction_splits_into_its_two_words() {
        let tokens = tokenise("she's here");
        assert_eq!(tokens[0].word, "she");
        assert_eq!(tokens[1].word, "'s");
        assert_eq!(tokenise("don't go")[1].word, "n't");
    }

    #[test]
    fn a_token_points_back_at_the_text() {
        let text = "the cat sat";
        let tokens = tokenise(text);
        assert_eq!(&text[tokens[1].at.start..tokens[1].at.end], "cat");
    }
}
