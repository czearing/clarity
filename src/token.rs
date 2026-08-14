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
    for (at, character) in text.char_indices() {
        let part_of_word = character.is_alphanumeric() || character == '\'' || character == '-';
        match (part_of_word, start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                push(&mut tokens, text, from, at);
                start = None;
            }
            _ => {}
        }
        if !part_of_word && !character.is_whitespace() {
            push(&mut tokens, text, at, at + character.len_utf8());
        }
    }
    if let Some(from) = start {
        push(&mut tokens, text, from, text.len());
    }
    tokens
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
fn split(token: &Token) -> Option<(Token, Token)> {
    if crate::lexicon::places(&token.key) {
        return None;
    }
    CLITICS.iter().find_map(|(clitic, bare)| {
        let stem = token.key.strip_suffix(bare)?;
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
                    },
                    Token {
                        word: (*clitic).to_string(),
                        key: (*clitic).to_string(),
                        at: Span::new(cut, token.at.end),
                        capitalised: false,
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
