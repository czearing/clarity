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
use syn::{FnArg, ImplItem, Pat, PatType, ReturnType, Signature, Stmt, Type, Visibility};

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
    /// The call can stop the program, with what the code gives as the cause.
    Halts(Halt),
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
    /// The doc comment it already carries, a line at a time, as the author wrote it.
    pub doc: Vec<String>,
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

/// An item whose whole story is its name, its keyword and whether it was commented.
///
/// Nothing here has a body to follow or a signature to read, so one arm serves all of them and the
/// longer reading is left to the items that earn it.
fn plain_item(item: &syn::Item) -> Option<Piece> {
    let mut out = Vec::new();
    match item {
        syn::Item::Enum(choice) => {
            out.push(named_item(
                &choice.ident,
                "enum",
                &choice.vis,
                choice.span(),
                doc_of(&choice.attrs),
            ));
        }
        syn::Item::Trait(contract) => {
            out.push(named_item(
                &contract.ident,
                "trait",
                &contract.vis,
                contract.span(),
                doc_of(&contract.attrs),
            ));
        }
        syn::Item::Const(fixed) => {
            out.push(named_item(
                &fixed.ident,
                "const",
                &fixed.vis,
                fixed.span(),
                doc_of(&fixed.attrs),
            ));
        }
        syn::Item::Static(fixed) => {
            out.push(named_item(
                &fixed.ident,
                "static",
                &fixed.vis,
                fixed.span(),
                doc_of(&fixed.attrs),
            ));
        }
        syn::Item::Type(alias) => {
            out.push(named_item(
                &alias.ident,
                "type",
                &alias.vis,
                alias.span(),
                doc_of(&alias.attrs),
            ));
        }
        _ => return None,
    }
    out.pop()
}

/// Read one item and everything nested inside it.
fn walk(item: &syn::Item, source: &str, out: &mut Vec<Piece>) {
    if let Some(piece) = plain_item(item) {
        out.push(piece);
        return;
    }
    match item {
        syn::Item::Fn(function) => {
            out.push(from_fn(
                &function.sig,
                &function.vis,
                function.span(),
                source,
                doc_of(&function.attrs),
                body_facts(&function.block.stmts),
            ));
        }
        syn::Item::Impl(block) => {
            for nested in &block.items {
                match nested {
                    ImplItem::Fn(function) => out.push(from_fn(
                        &function.sig,
                        &function.vis,
                        function.span(),
                        source,
                        doc_of(&function.attrs),
                        body_facts(&function.block.stmts),
                    )),
                    // A constant hung off a type is reachable through that type and is part of
                    // what a caller is offered, so it is counted where a free constant is.
                    ImplItem::Const(fixed) => out.push(named_item(
                        &fixed.ident,
                        "const",
                        &fixed.vis,
                        fixed.span(),
                        doc_of(&fixed.attrs),
                    )),
                    _ => {}
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
            let written = doc_of(&record.attrs);
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
                documented: !written.is_empty(),
                doc: written,
                facts,
            });
        }
        _ => {}
    }
}

/// An item that is named and declared, with nothing in its shape left to read.
///
/// Rust declares constants, aliases, traits and enumerations, and a caller can reach for each of
/// them. None of them has a body to follow or a signature to draw a warning from. What is
/// known about them is the name, the word Rust declares them with, and whether the author wrote a
/// comment, which is enough to count them and not enough to describe them.
fn named_item(
    ident: &syn::Ident,
    kind: &'static str,
    vis: &Visibility,
    span: Span,
    written: Vec<String>,
) -> Piece {
    Piece {
        name: ident.to_string(),
        kind,
        public: matches!(vis, Visibility::Public(_)),
        line: span.start().line,
        indent: span.start().column,
        documented: !written.is_empty(),
        doc: written,
        facts: Vec::new(),
    }
}

/// What a function's signature and body say about it.
fn from_fn(
    sig: &Signature,
    vis: &Visibility,
    span: Span,
    _source: &str,
    written: Vec<String>,
    mut facts: Vec<Finding>,
) -> Piece {
    facts.extend(signature_facts(sig));
    Piece {
        name: sig.ident.to_string(),
        kind: "fn",
        public: matches!(vis, Visibility::Public(_)),
        line: span.start().line,
        indent: span.start().column,
        documented: !written.is_empty(),
        doc: written,
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

/// The doc comment these attributes carry, a line at a time.
///
/// The text is taken as the author wrote it, because what is being asked later is whether it says
/// anything, and rewording it first would be answering about something the author did not write.
fn doc_of(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(pair) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) = &pair.value
            {
                lines.push(text.value().trim().to_owned());
            }
        }
    }
    lines.retain(|line| !line.is_empty());
    lines
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

/// Why a call stops, in the words the code itself gives for it.
///
/// A stop is only worth reporting when the code says what would cause it. An assertion carries its
/// check, and a stop written by hand usually carries a message; both are literal tokens, so
/// repeating them invents nothing. Where neither is there, as with a bare unwrap, the code names no
/// cause, and nothing here can name one either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Halt {
    /// A check that has to hold, written as the code writes it.
    Unless(String),
    /// Two things a check requires to be equal.
    Equal(String, String),
    /// Two things a check requires to differ.
    Differ(String, String),
    /// What the code prints when it stops.
    Says(String),
    /// What `expect` was promised, which is the opposite of the reason it stops.
    ///
    /// The message handed to `panic!` describes the failure, so it is quoted as the failure. The
    /// message handed to `expect` describes what the caller was relying on instead — the language
    /// asks for the reason the value is expected to be there — so quoting it as the failure states
    /// the condition backwards. It is reported as the thing that has to hold.
    Expects(String),
}

/// What a body says, found by looking at what it does rather than at what it is called.
fn body_facts(stmts: &[Stmt]) -> Vec<Finding> {
    let mut stopping = Stopping { why: None };
    for stmt in stmts {
        syn::visit::Visit::visit_stmt(&mut stopping, stmt);
    }
    let mut facts = Vec::new();
    if let Some(why) = stopping.why {
        facts.push(Finding {
            fact: Fact::Halts(why),
            price: SOMETIMES,
        });
    }
    facts
}

/// The first cause of a stop found anywhere in a body, however deeply it is written.
///
/// This walks the tree rather than reading statements as text. A stop is very often not a statement
/// at all: it is the tail of a chain, or the one line of a closure handed to another call, and a
/// pass that only reads whole statements as strings can see that something stops without being able
/// to say what would cause it. Walking also settles by construction the case a string search could
/// never settle, of a name written inside a longer one.
struct Stopping {
    /// The first cause found, since one warning helps a caller and further ones do not.
    why: Option<Halt>,
}

impl Stopping {
    /// Keeps a cause unless one is already held.
    fn keep(&mut self, why: Halt) {
        if self.why.is_none() {
            self.why = Some(why);
        }
    }

    /// What this macro would stop for, where it is one of the macros that stop.
    ///
    /// The debug forms are absent, because a release build compiles them away, so a comment saying
    /// the program can stop there would be wrong wherever it matters. A todo is absent for the
    /// opposite reason: it always stops, so a caller is not being warned of a case but told the
    /// work is unfinished, which the name already says to anyone who opens it.
    fn macro_stop(&mut self, name: &str, tokens: &proc_macro2::TokenStream) {
        let parts = parted(tokens);
        match name {
            "assert" => {
                if let Some(check) = parts.first() {
                    self.keep(Halt::Unless(tight(check.clone())));
                }
            }
            "assert_eq" | "assert_ne" => {
                if let [left, right, ..] = parts.as_slice() {
                    let (left, right) = (tight(left.clone()), tight(right.clone()));
                    self.keep(if name == "assert_eq" {
                        Halt::Equal(left, right)
                    } else {
                        Halt::Differ(left, right)
                    });
                }
            }
            "panic" | "unreachable" => {
                if let Some(words) = said(parts.first()) {
                    self.keep(Halt::Says(words));
                }
            }
            _ => {}
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for Stopping {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.why.is_some() {
            return;
        }
        if let Some(name) = node.path.segments.last() {
            self.macro_stop(&name.ident.to_string(), &node.tokens);
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.why.is_some() {
            return;
        }
        if node.method == "expect" {
            if let Some(syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(words),
                ..
            })) = node.args.first()
            {
                // The words are not cut where a value would go, as a message written by hand is.
                // What is handed to `expect` is not a format, so a brace in it is a brace, and it
                // is what the program will really print.
                if let Some(words) = quotable(words.value().trim()) {
                    self.keep(Halt::Expects(words));
                }
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// The words of a string written straight into the code, where that is what is there.
fn said(part: Option<&proc_macro2::TokenStream>) -> Option<String> {
    let text = part?.clone().into_iter().next()?;
    let proc_macro2::TokenTree::Literal(written) = text else {
        return None;
    };
    let quoted = written.to_string();
    let inner = quoted.strip_prefix('"')?.strip_suffix('"')?;
    // A message written for a person often ends where a value is put into it. What comes before
    // that is the part the author wrote, and the part a caller can be told. The cut is made at the
    // brace rather than at an empty pair of them, because a value can be put in by name or with a
    // format of its own, and neither of those is anything to show a reader.
    let words = inner.split('{').next().unwrap_or(inner).trim();
    let words = words.trim_end_matches([':', ',', '-', ';']).trim();
    quotable(words)
}

/// Words fit to go inside a code span, or nothing.
///
/// The words are written back between backticks, so words carrying a backtick of their own would
/// close the span early and leave the rest as prose. There is nothing to be gained by repairing
/// that, and a comment is never worth a wrong one, so such words are left unsaid.
fn quotable(words: &str) -> Option<String> {
    (!words.is_empty() && !words.contains('`')).then(|| words.to_string())
}

/// The arguments of a macro, split where the commas between them are.
fn parted(tokens: &proc_macro2::TokenStream) -> Vec<proc_macro2::TokenStream> {
    let mut parts = vec![proc_macro2::TokenStream::new()];
    for tree in tokens.clone() {
        if let proc_macro2::TokenTree::Punct(mark) = &tree {
            if mark.as_char() == ',' {
                parts.push(proc_macro2::TokenStream::new());
                continue;
            }
        }
        parts
            .last_mut()
            .expect("a part is pushed before anything is put in one")
            .extend(std::iter::once(tree));
    }
    parts.retain(|part| !part.is_empty());
    parts
}

/// Code written back the way a person writes it, rather than the way tokens print.
///
/// Tokens print with a space between every one of them, so a call reads as `values . len ()`. That
/// is not what the author wrote and not what a reader will search for, so the spaces that only
/// exist because the tokens were taken apart are closed up again.
fn tight(tokens: proc_macro2::TokenStream) -> String {
    let mut out = String::new();
    // Whether the next thing follows straight on, what came last was something an operator can
    // work on, and whether the run of marks being written started after one of those.
    let mut joined = true;
    let mut operand = false;
    let mut ran_after_operand = false;
    let mut ran_on = false;
    for tree in tokens {
        match tree {
            proc_macro2::TokenTree::Group(inner) => {
                let (open, close) = match inner.delimiter() {
                    proc_macro2::Delimiter::Parenthesis => ("(", ")"),
                    proc_macro2::Delimiter::Bracket => ("[", "]"),
                    proc_macro2::Delimiter::Brace => ("{", "}"),
                    proc_macro2::Delimiter::None => ("", ""),
                };
                // Brackets straight after a name are a call or an index and belong to the name.
                // Brackets after an operator are what it works on, and stand apart from it.
                if !operand && !joined && !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(open);
                out.push_str(&tight(inner.stream()));
                out.push_str(close);
                joined = false;
                operand = true;
                ran_on = false;
            }
            proc_macro2::TokenTree::Punct(mark) => {
                let letter = mark.as_char();
                let alone = mark.spacing() == proc_macro2::Spacing::Alone;
                // A bang straight after a name calls a macro, and one that carries on into another
                // mark is half of an operator. Which one this is can be read off the mark itself.
                let calling = letter == '!'
                    && alone
                    && out.ends_with(|letter: char| letter.is_alphanumeric() || letter == '_');
                let held = calling || matches!(letter, '.' | ',' | ';' | '?' | ':');
                // A run of marks is one operator. Whether it works on what follows it alone is
                // settled by what stood before the run, not by what stands before this mark, so
                // the minus in "a == -1" is read as a sign and the one in "a - 1" is not.
                if !ran_on {
                    ran_after_operand = operand;
                }
                if !held && !joined && !out.is_empty() && !out.ends_with(['(', '[', '{']) {
                    out.push(' ');
                }
                out.push(letter);
                joined = calling || matches!(letter, '.' | ':') || !alone || !ran_after_operand;
                if matches!(letter, ',' | ';') {
                    out.push(' ');
                    joined = true;
                }
                operand = false;
                ran_on = !alone;
            }
            other => {
                if !joined && !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&other.to_string());
                joined = false;
                operand = true;
                ran_on = false;
            }
        }
    }
    out
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
    let plural = crate::lexicon::offers(last, crate::tag::Tag::Noun(crate::tag::Number::Plural));
    let singular =
        crate::lexicon::offers(last, crate::tag::Tag::Noun(crate::tag::Number::Singular));
    if plural && singular {
        return None;
    }
    // A word the lexicon knows is answered by the lexicon, because spelling is only a guess at
    // number and it guesses badly on a singular noun that ends in s. "gas" over a `bool` was
    // reported as a plural name given a singular type; so were "bias", "lens" and "status". The
    // morphology is still what reads a coined name, which is the case no lexicon can cover.
    if singular {
        return Some(Number::One);
    }
    if plural {
        return Some(Number::Many);
    }
    if crate::repair::is_plural(last) {
        Some(Number::Many)
    } else {
        Some(Number::One)
    }
}

#[cfg(test)]
mod tests {
    use super::{findings, tight, Fact, Halt, Number};

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
    fn a_stop_is_reported_with_the_cause_the_code_gives_for_it() {
        let found = findings("fn take(of: &[u8]) -> u8 { assert!(!of.is_empty()); of[0] }");
        assert!(found[0]
            .facts
            .iter()
            .any(|f| f.fact == Fact::Halts(Halt::Unless("!of.is_empty()".to_string()))));
    }

    #[test]
    fn a_check_of_two_things_is_reported_as_both_of_them() {
        let found = findings("fn take(of: &[u8], n: usize) { assert_eq!(of.len(), n * n); }");
        assert!(found[0].facts.iter().any(
            |f| f.fact == Fact::Halts(Halt::Equal("of.len()".to_string(), "n * n".to_string()))
        ));
    }

    #[test]
    fn a_stop_written_where_no_statement_reaches_is_still_found() {
        let found = findings(
            r#"fn take(of: &[u8]) -> u8 { of.first().copied().unwrap_or_else(|| panic!("no bytes: {}", 1)) }"#,
        );
        assert!(found[0]
            .facts
            .iter()
            .any(|f| f.fact == Fact::Halts(Halt::Says("no bytes".to_string()))));
    }

    #[test]
    fn a_stop_the_code_gives_no_cause_for_is_not_reported() {
        let found = findings("fn get(of: &[u8]) -> u8 { *of.first().unwrap() }");
        assert!(!found[0]
            .facts
            .iter()
            .any(|f| matches!(f.fact, Fact::Halts(_))));
    }

    #[test]
    fn a_stop_asked_for_words_it_was_given_none_of_is_read_without_stopping() {
        // `expect` with nothing handed to it still parses, so the pass meets it and must not
        // reach for an argument that is not there.
        let found = findings("fn get(of: Option<u8>) -> u8 { of.expect() }");
        assert!(!found[0]
            .facts
            .iter()
            .any(|f| matches!(f.fact, Fact::Halts(_))));
    }

    #[test]
    fn a_message_is_cut_where_the_first_value_is_put_into_it() {
        for source in [
            r#"fn get(of: &[u8]) -> u8 { panic!("no bytes: {}", 1) }"#,
            r#"fn get(of: &[u8]) -> u8 { panic!("no bytes: {:?}", of) }"#,
            r#"fn get(of: &[u8]) -> u8 { panic!("no bytes: {count}") }"#,
        ] {
            let found = findings(source);
            assert!(
                found[0]
                    .facts
                    .iter()
                    .any(|f| f.fact == Fact::Halts(Halt::Says("no bytes".to_string()))),
                "{source}"
            );
        }
    }

    #[test]
    fn words_that_would_close_the_span_they_are_written_in_are_left_unsaid() {
        let found = findings(r#"fn get(of: &[u8]) -> u8 { panic!("no `bytes` here") }"#);
        assert!(!found[0]
            .facts
            .iter()
            .any(|f| matches!(f.fact, Fact::Halts(_))));
    }

    #[test]
    fn a_check_is_written_back_the_way_it_was_written() {
        for (source, want) in [
            ("fn f(a: u8, b: u8) { assert!(a != b); }", "a != b"),
            (
                "fn f(of: &[u8]) { assert!(!of.is_empty()); }",
                "!of.is_empty()",
            ),
            (
                "fn f(n: usize) { assert!(n < std::usize::MAX); }",
                "n < std::usize::MAX",
            ),
            (
                "fn f(x: u8) { assert!(matches!(x, 1 | 2)); }",
                "matches!(x, 1 | 2)",
            ),
            ("fn f(a: u8) { assert!(a == -1); }", "a == -1"),
        ] {
            let found = findings(source);
            assert!(
                found[0]
                    .facts
                    .iter()
                    .any(|f| f.fact == Fact::Halts(Halt::Unless(want.to_string()))),
                "{source} gave {:?}",
                found[0].facts
            );
        }
    }

    #[test]
    fn what_is_written_back_is_the_check_that_was_read() {
        // Closing up the spaces a tokeniser leaves is only safe if it changes nothing but spacing.
        // Rust settles that: what is written back is handed to the parser again, and it has to
        // come out as the same expression.
        for check in [
            "a != b",
            "!of.is_empty()",
            "n < std::usize::MAX",
            "matches!(x, 1 | 2)",
            "a == -1",
            "values.len() == n * n",
            "(a + b) * c",
            "a && b || !c",
            "v[0] >= v[1]",
            "self.mass.unwrap_or(0.0) > 0.0",
            "x as u8 == 1",
            "&a == &b",
            "Some(x) != None",
            "a.b().c(d, e)",
            "f(-1, 2)",
            "1.0_f64 / 2.0",
            "*p == 1",
        ] {
            let read: syn::Expr = syn::parse_str(check).expect("the check is written in Rust");
            let written = tight(quote::ToTokens::to_token_stream(&read));
            let again: syn::Expr =
                syn::parse_str(&written).unwrap_or_else(|_| panic!("{check} became {written}"));
            assert_eq!(
                quote::ToTokens::to_token_stream(&read).to_string(),
                quote::ToTokens::to_token_stream(&again).to_string(),
                "{check} became {written}"
            );
        }
    }

    #[test]
    fn a_check_a_release_build_deletes_is_not_reported() {
        let found = findings("fn take(of: &[u8]) { debug_assert!(!of.is_empty()); }");
        assert!(!found[0]
            .facts
            .iter()
            .any(|f| matches!(f.fact, Fact::Halts(_))));
    }

    #[test]
    fn a_file_that_does_not_parse_yields_nothing_rather_than_a_guess() {
        assert!(findings("fn (((").is_empty());
    }
}
