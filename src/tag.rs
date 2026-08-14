//! The categories a word can hold.

/// Grammatical number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Number {
    /// One.
    Singular,
    /// More than one.
    Plural,
}

/// Grammatical person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Person {
    /// Speaker.
    First,
    /// Addressee.
    Second,
    /// Neither.
    Third,
}

/// Whether a pronoun can head a subject, an object, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Case {
    /// Subject only, such as "he".
    Subject,
    /// Object only, such as "him".
    Object,
    /// Either, such as "you".
    Either,
}

/// The form of a verb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Form {
    /// Plain form, as in "they walk" or "to walk".
    Base,
    /// Third person singular present, as in "she walks".
    ThirdSingular,
    /// Preterite, as in "she walked". Number neutral, which is true of every verb but "be".
    Past,
    /// Preterite that demands a singular subject, which only "was" does.
    PastSingular,
    /// Preterite that demands a plural subject, which only "were" does.
    PastPlural,
    /// Past participle, as in "has walked".
    Participle,
    /// Present participle or gerund, as in "is walking".
    Gerund,
}

/// A word category, carrying the features agreement needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Article or other determiner.
    Determiner(Number),
    /// Common noun.
    Noun(Number),
    /// Name.
    Proper(Number),
    /// Pronoun.
    Pronoun(Person, Number, Case),
    /// Lexical or auxiliary verb.
    Verb(Form),
    /// Modal auxiliary, which never inflects.
    Modal,
    /// Preposition.
    Preposition,
    /// Adjective.
    Adjective,
    /// Adverb.
    Adverb,
    /// Coordinator, such as "and".
    Coordinator,
    /// Subordinator, such as "because".
    Subordinator,
    /// Infinitival "to".
    To,
    /// Number word.
    Numeral,
    /// Any mark.
    Mark,
}

impl Tag {
    /// Every tag the tagger may choose between.
    ///
    /// The search costs the square of this, so it holds only combinations English has. Pronoun
    /// features come from the lexicon rather than the product of the feature sets, which would
    /// name a dozen pronouns that do not exist.
    #[must_use]
    pub fn every() -> Vec<Self> {
        let mut tags = vec![
            Self::Modal,
            Self::Preposition,
            Self::Adjective,
            Self::Adverb,
            Self::Coordinator,
            Self::Subordinator,
            Self::To,
            Self::Numeral,
            Self::Mark,
        ];
        for number in [Number::Singular, Number::Plural] {
            tags.push(Self::Determiner(number));
            tags.push(Self::Noun(number));
            tags.push(Self::Proper(number));
        }
        tags.extend(crate::lexicon::pronouns());
        for form in [
            Form::Base,
            Form::ThirdSingular,
            Form::Past,
            Form::PastSingular,
            Form::PastPlural,
            Form::Participle,
            Form::Gerund,
        ] {
            tags.push(Self::Verb(form));
        }
        tags
    }

    /// Whether this tag can head a noun phrase.
    #[must_use]
    pub fn is_nominal(self) -> bool {
        matches!(self, Self::Noun(_) | Self::Proper(_) | Self::Pronoun(..))
    }

    /// The number this tag carries, if it carries one.
    #[must_use]
    pub fn number(self) -> Option<Number> {
        match self {
            Self::Determiner(number)
            | Self::Noun(number)
            | Self::Proper(number)
            | Self::Pronoun(_, number, _) => Some(number),
            _ => None,
        }
    }

    /// Whether this tag is a verb carrying tense, which is what agrees with a subject.
    #[must_use]
    pub fn is_finite_verb(self) -> bool {
        matches!(
            self,
            Self::Verb(
                Form::Base
                    | Form::ThirdSingular
                    | Form::Past
                    | Form::PastSingular
                    | Form::PastPlural
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Case, Number, Person, Tag};

    #[test]
    fn every_tag_is_listed_once() {
        let all = Tag::every();
        let mut seen = all.clone();
        seen.sort_by_key(|tag| format!("{tag:?}"));
        seen.dedup();
        assert_eq!(
            seen.len(),
            all.len(),
            "a duplicate candidate wastes a state"
        );
    }

    #[test]
    fn a_pronoun_carries_its_number() {
        let tag = Tag::Pronoun(Person::Third, Number::Plural, Case::Subject);
        assert_eq!(tag.number(), Some(Number::Plural));
        assert!(tag.is_nominal());
    }
}
