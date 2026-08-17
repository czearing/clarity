//! What a piece of code says about itself, read off the code and nothing else.
//!
//! A doc comment is a claim about a function, and a claim is worth writing only where something
//! in the code stands behind it. So the code is read first and turned into findings, each one a
//! fact together with what it cost to be sure of it. Nothing here knows what any repository is
//! about: a finding is a shape in the syntax, and the words it is later written with come from
//! the identifiers the author already chose.
//!
//! The confidence is a price, in the same currency the grammar uses. A finding read straight off
//! a signature is cheap, because a signature cannot be wrong about itself. A finding read out of
//! a body costs more, because a body may do a thing on one path and not another. What is written
//! is then whatever the search can afford, which is how a low confidence turns into a shorter
//! comment rather than a wrong one.
//!
//! ```
//! use clarity::code::{findings, Fact};
//!
//! let found = findings("fn head(list: &[u8]) -> Option<u8> { list.first().copied() }");
//! let item = &found[0];
//! assert_eq!(item.name, "head");
//! assert!(item.facts.iter().any(|finding| finding.fact == Fact::MayBeAbsent));
//! ```

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{Expr, FnArg, ImplItem, Pat, PatType, ReturnType, Signature, Stmt, Type, Visibility};

/// What the code was found to say.
///
/// Each one is a shape in the syntax and not a sentence. Holding them apart from their wording is
/// what lets the same finding be written in a summary or left out of one, and what stops a
/// judgement about the code being mixed up with a judgement about English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// The answer may be missing, because the return type admits a missing one.
    MayBeAbsent,
    /// The call may fail, because the return type admits a failure.
    MayFail,
    /// The answer is many of something, because the return type is a collection.
    Many,
    /// The answer is true or false.
    YesOrNo,
    /// The answer is a count or a measure.
    Number,
    /// The call changes what it is called on.
    Alters,
    /// The call reads what it is called on and changes nothing.
    Reads,
    /// The call answers with nothing.
    Silent,
    /// The call can stop the program, with the words that name the place it stops.
    Halts,
    /// A name is singular where the thing it names is many, or the other way about.
    ///
    /// The name, the number it carries, and the number its type asks for. This is
    /// the one finding that is about the code being wrong rather than about what the code does.
    Misnumbered(String, Number, Number),
    /// The call takes each of these, named as the author named them.
    Takes(Vec<String>),
    /// The call answers with this, named by the type the author wrote.
    Answers(String),
}

/// Whether a thing is one or many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Number {
    /// This holds one thing.
    One,
    /// More than one of it.
    Many,
}

impl Number {
    /// The word for this number, used where a finding has to be written out.
    #[must_use]
    pub const fn says(self) -> &'static str {
        match self {
            Self::One => "singular",
            Self::Many => "plural",
        }
    }
}

/// A fact together with what it cost to be sure of it.
///
/// The price is the whole of the confidence. A signature is worth more than a body because a
/// signature cannot be wrong about itself, and a body may take one path today and another
/// tomorrow. Nothing else is graded, because grading anything else would be an opinion.
#[derive(Clone, Debug)]
pub struct Finding {
    /// What was found.
    pub fact: Fact,
    /// What it cost to find it. Lower is surer.
    pub price: f64,
}

/// What a signature says, which it cannot be wrong about.
pub const SIGNED: f64 = 1.0;

/// What a body says on every path through it.
pub const ALWAYS: f64 = 2.0;

/// What a body says on some path through it.
pub const SOMETIMES: f64 = 4.0;

/// A documentable item, with what the code says about it.
#[derive(Clone, Debug)]
pub struct Piece {
    /// The name the author gave it.
    pub name: String,
    /// What kind of thing it is, in the word Rust uses.
    pub kind: &'static str,
    /// Whether anything outside the file can see it.
    pub public: bool,
    /// The line the item starts on, counting from one.
    pub line: usize,
    /// How far the item is indented, in characters.
    pub indent: usize,
    /// Whether the item already carries a doc comment.
    pub documented: bool,
    /// What the code says about it.
    pub facts: Vec<Finding>,
}

/// Everything a source file says about the items in it.
///
/// A file that does not parse yields nothing rather than a guess, because a finding read out of a
/// file the compiler would reject is a finding about nothing.
#[must_use]
pub fn findings(source: &str) -> Vec<Piece> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for item in &file.items {
        walk(item, source, &mut found);
    }
    found
}

/// Read one item and everything nested inside it.
fn walk(item: &syn::Item, source: &str, out: &mut Vec<Piece>) {
    match item {
        syn::Item::Fn(function) => {
            out.push(from_fn(
                &function.sig,
                &function.vis,
                function.span(),
                source,
                has_doc(&function.attrs),
                body_facts(&function.block.stmts),
            ));
        }
        syn::Item::Impl(block) => {
            for nested in &block.items {
                if let ImplItem::Fn(function) = nested {
                    out.push(from_fn(
                        &function.sig,
                        &function.vis,
                        function.span(),
                        source,
                        has_doc(&function.attrs),
                        body_facts(&function.block.stmts),
                    ));
                }
            }
        }
        syn::Item::Mod(module) => {
            if let Some((_, items)) = &module.content {
                for nested in items {
                    walk(nested, source, out);
                }
            }
        }
        syn::Item::Struct(record) => {
            let mut facts = Vec::new();
            for field in &record.fields {
                let Some(name) = field.ident.as_ref().map(ToString::to_string) else {
                    continue;
                };
                let Some(number) = number_of(&field.ty) else {
                    continue;
                };
                if let Some(had) = reads_as(&name) {
                    if had != number {
                        facts.push(Finding {
                            fact: Fact::Misnumbered(name, had, number),
                            price: SIGNED,
                        });
                    }
                }
            }
            out.push(Piece {
                name: record.ident.to_string(),
                kind: "struct",
                public: matches!(record.vis, Visibility::Public(_)),
                line: record.span().start().line,
                indent: record.span().start().column,
                documented: has_doc(&record.attrs),
                facts,
            });
        }
        _ => {}
    }
}

/// What a function's signature and body say about it.
fn from_fn(
    sig: &Signature,
    vis: &Visibility,
    span: Span,
    _source: &str,
    documented: bool,
    mut facts: Vec<Finding>,
) -> Piece {
    facts.extend(signature_facts(sig));
    Piece {
        name: sig.ident.to_string(),
        kind: "fn",
        public: matches!(vis, Visibility::Public(_)),
        line: span.start().line,
        indent: span.start().column,
        documented,
        facts,
    }
}

/// Whether a method changes what it is called on.
///
/// A method takes its receiver by value, by reference, or by a type it writes out. Only the
/// reference form can be plain or mutable, so that is the one asked, and taking a thing by value
/// is a change to a copy rather than to what the caller holds.
fn alters(receiver: &syn::Receiver) -> bool {
    match &receiver.kind {
        syn::ReceiverKind::Reference(_, _, mutable) => mutable.is_some(),
        _ => false,
    }
}

/// Whether any of these attributes is a doc comment.
fn has_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}

/// What a signature says, which it cannot be wrong about.
fn signature_facts(sig: &Signature) -> Vec<Finding> {
    let mut facts = Vec::new();
    let mut takes = Vec::new();
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(receiver) => facts.push(Finding {
                fact: if alters(receiver) {
                    Fact::Alters
                } else {
                    Fact::Reads
                },
                price: SIGNED,
            }),
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(named) = &**pat {
                    let name = named.ident.to_string();
                    if let (Some(number), Some(had)) = (number_of(ty), reads_as(&name)) {
                        if had != number {
                            facts.push(Finding {
                                fact: Fact::Misnumbered(name.clone(), had, number),
                                price: SIGNED,
                            });
                        }
                    }
                    takes.push(name);
                }
            }
        }
    }
    if !takes.is_empty() {
        facts.push(Finding {
            fact: Fact::Takes(takes),
            price: SIGNED,
        });
    }
    match &sig.output {
        ReturnType::Default => facts.push(Finding {
            fact: Fact::Silent,
            price: SIGNED,
        }),
        ReturnType::Type(_, ty) => {
            let head = outer(ty);
            match head.as_str() {
                "Option" => facts.push(Finding {
                    fact: Fact::MayBeAbsent,
                    price: SIGNED,
                }),
                "Result" => facts.push(Finding {
                    fact: Fact::MayFail,
                    price: SIGNED,
                }),
                "bool" => facts.push(Finding {
                    fact: Fact::YesOrNo,
                    price: SIGNED,
                }),
                _ => {}
            }
            if number_of(ty) == Some(Number::Many) {
                facts.push(Finding {
                    fact: Fact::Many,
                    price: SIGNED,
                });
            }
            if counts(&head) {
                facts.push(Finding {
                    fact: Fact::Number,
                    price: SIGNED,
                });
            }
            facts.push(Finding {
                fact: Fact::Answers(head),
                price: SIGNED,
            });
        }
    }
    facts
}

/// What a body says, found by looking at what it does rather than at what it is called.
fn body_facts(stmts: &[Stmt]) -> Vec<Finding> {
    let mut halts = false;
    for stmt in stmts {
        walk_stmt(stmt, &mut halts);
    }
    let mut facts = Vec::new();
    if halts {
        facts.push(Finding {
            fact: Fact::Halts,
            price: SOMETIMES,
        });
    }
    facts
}

/// Whether anything in this statement can stop the program.
fn walk_stmt(stmt: &Stmt, halts: &mut bool) {
    let text = match stmt {
        Stmt::Expr(expr, _) => quote_of(expr),
        Stmt::Local(local) => local
            .init
            .as_ref()
            .map(|init| quote_of(&init.expr))
            .unwrap_or_default(),
        _ => String::new(),
    };
    if stops(&text) {
        *halts = true;
    }
}

/// The text of an expression, used to ask what it does rather than to rewrite it.
fn quote_of(expr: &Expr) -> String {
    use quote::ToTokens;
    expr.to_token_stream().to_string()
}

/// Whether a piece of code can stop the program where it stands.
///
/// This looks only for the names the standard library uses when it stops. A call that stops is
/// found by what it calls, and no list has to be kept.
fn stops(text: &str) -> bool {
    ["panic !", "unreachable !", ". unwrap ()", ". expect ("]
        .iter()
        .any(|mark| text.contains(mark))
}

/// The outermost name in a type, which is what the type is.
fn outer(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default(),
        Type::Reference(reference) => outer(&reference.elem),
        Type::Slice(_) => "slice".to_owned(),
        Type::Tuple(items) if items.elems.is_empty() => "nothing".to_owned(),
        _ => String::new(),
    }
}

/// Whether a type holds one thing or many, where the type says so.
///
/// A type says so by being a collection or by being one thing. Where it says neither, the answer
/// is that nothing was found, which is not the same as finding that it holds one.
#[must_use]
pub fn number_of(ty: &Type) -> Option<Number> {
    match ty {
        Type::Reference(reference) => number_of(&reference.elem),
        Type::Slice(_) | Type::Array(_) => Some(Number::Many),
        Type::Path(path) => {
            let head = path.path.segments.last()?.ident.to_string();
            if collects(&head) {
                Some(Number::Many)
            } else if counts(&head) {
                // A number can be a count of many things or a measure in many units, so a plural
                // name over it is good English rather than a disagreement. "duration_minutes" and
                // "water_determinations" are both right, and neither holds more than one number.
                None
            } else if scalar(&head) {
                Some(Number::One)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether a type name is a collection, so what it holds is many.
fn collects(head: &str) -> bool {
    matches!(
        head,
        "Vec" | "VecDeque" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "Iterator"
    ) || head.ends_with("Map")
        || head.ends_with("Set")
        || head.ends_with("List")
}

/// Whether a type name is one thing that cannot be many.
fn scalar(head: &str) -> bool {
    matches!(
        head,
        "bool"
            | "char"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "f32"
            | "f64"
    )
}

/// Whether a type name is a count or a measure.
fn counts(head: &str) -> bool {
    matches!(
        head,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "f32"
            | "f64"
    )
}

/// The number a name is written in.
///
/// The last word of the name carries it, because a name is a noun phrase and the head of a noun
/// phrase is its last word. What answers is the same morphology a repair uses to spell a plural, so
/// a coined name is read without being listed anywhere. A name with no letters in its last word
/// says nothing.
#[must_use]
pub fn reads_as(name: &str) -> Option<Number> {
    let last = name.rsplit('_').next()?;
    if last.len() < 2 || !last.chars().all(char::is_alphabetic) {
        return None;
    }
    // A word English spells the same either way says nothing about number, and "species" over a
    // list is not a mistake. The lexicon offering both readings is what that looks like, so the
    // question is dropped rather than answered wrongly. Missing a finding costs a line left
    // alone; a false one costs a name that was already right.
    if crate::lexicon::offers(last, crate::tag::Tag::Noun(crate::tag::Number::Plural))
        && crate::lexicon::offers(last, crate::tag::Tag::Noun(crate::tag::Number::Singular))
    {
        return None;
    }
    if crate::repair::is_plural(last) {
        Some(Number::Many)
    } else {
        Some(Number::One)
    }
}

#[cfg(test)]
mod tests {
    use super::{findings, Fact, Number};

    #[test]
    fn a_signature_says_whether_the_answer_can_be_missing() {
        let found = findings("fn first(of: &[u8]) -> Option<u8> { of.first().copied() }");
        assert!(found[0].facts.iter().any(|f| f.fact == Fact::MayBeAbsent));
    }

    #[test]
    fn a_signature_says_whether_the_call_changes_what_it_is_called_on() {
        let found =
            findings("impl T { fn push(&mut self, one: u8) {} fn len(&self) -> usize { 0 } }");
        assert!(found[0].facts.iter().any(|f| f.fact == Fact::Alters));
        assert!(found[1].facts.iter().any(|f| f.fact == Fact::Reads));
        assert!(found[1].facts.iter().any(|f| f.fact == Fact::Number));
    }

    #[test]
    fn a_name_is_read_against_the_number_its_type_asks_for() {
        let found = findings("fn take(id: Vec<u8>) {}");
        assert!(found[0]
            .facts
            .iter()
            .any(|f| f.fact == Fact::Misnumbered("id".to_owned(), Number::One, Number::Many)));
    }

    #[test]
    fn a_name_that_already_agrees_with_its_type_is_left_alone() {
        let found = findings("fn take(ids: Vec<u8>, id: u8) {}");
        assert!(!found[0]
            .facts
            .iter()
            .any(|f| matches!(f.fact, Fact::Misnumbered(..))));
    }

    #[test]
    fn a_body_that_can_stop_the_program_says_so() {
        let found = findings("fn get(of: &[u8]) -> u8 { *of.first().unwrap() }");
        assert!(found[0].facts.iter().any(|f| f.fact == Fact::Halts));
    }

    #[test]
    fn a_file_that_does_not_parse_yields_nothing_rather_than_a_guess() {
        assert!(findings("fn (((").is_empty());
    }
}
