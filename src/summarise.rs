//! What a whole crate says about itself, and the summary that follows from it.
//!
//! The item pass reads one declaration and writes one comment. A reader arriving at a repository
//! asks a wider question, about how large the thing is, how it is laid out, and where it bites.
//! No single declaration answers that, and the tree answers all of it.
//!
//! So this pass reads the tree and counts. The engine builds every sentence from a count or from
//! a name the code gave. The register that grades the repository grades those sentences too, so
//! a summary cannot state what the code does not show. Where the evidence runs out, the pass
//! leaves the section unwritten rather than filling it.

use std::collections::BTreeMap;
use std::path::Path;

use crate::code::{Fact, Halt, Piece};

/// One module of a crate, and what it holds.
#[derive(Debug, Clone)]
pub struct Module {
    /// The path the author gave it, as Rust spells it.
    pub path: String,
    /// Public items declared in it, by the word Rust uses for each kind.
    pub kinds: BTreeMap<&'static str, usize>,
    /// Public items in it, of every kind.
    pub items: usize,
}

/// A call that can stop, and the condition the code states for it.
#[derive(Debug, Clone)]
pub struct Stop {
    /// The module it is declared in.
    pub module: String,
    /// The name the author gave it.
    pub name: String,
    /// What the code says about the stop, as a clause following the word "stops".
    ///
    /// A check states a condition that has to hold. A panic states the failure instead. Writing
    /// the second as the first states the condition backwards, so which it is decides the wording
    /// and the wording is settled where the fact is read.
    pub cause: String,
}

/// Everything a crate's source says about the crate.
///
/// Gathered rather than judged. Each field is a count or a name taken from a declaration, so a
/// sentence built from it can be checked by opening the file it came from.
#[derive(Debug, Default, Clone)]
pub struct Survey {
    /// The name the crate is published under.
    pub name: String,
    /// The modules, in the order Rust would read them.
    pub modules: Vec<Module>,
    /// Public items across the crate, by the word Rust uses for each kind.
    pub kinds: BTreeMap<&'static str, usize>,
    /// Public items across the crate, of every kind.
    pub items: usize,
    /// Files that parsed.
    pub files: usize,
    /// Calls that stop on a condition the code states.
    pub stops: Vec<Stop>,
    /// Public items already carrying a doc comment.
    pub documented: usize,
    /// What the crate is built on, as the manifest names them.
    pub built_on: Vec<String>,
}

/// The crate read out of a directory of source files.
///
/// A file that does not parse contributes nothing rather than a guess, which is the same rule the
/// item pass follows: a count taken from a file the compiler would reject is a count of nothing.
#[must_use]
pub fn surveyed(name: &str, files: &[(String, String)]) -> Survey {
    let mut survey = Survey {
        name: name.to_owned(),
        ..Survey::default()
    };
    for (path, source) in files {
        let pieces = crate::code::findings(source);
        if pieces.is_empty() && source.trim().is_empty() {
            continue;
        }
        survey.files += 1;
        let module = module_of(path);
        let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut items = 0;
        for piece in pieces.iter().filter(|piece| piece.public) {
            *kinds.entry(piece.kind).or_default() += 1;
            *survey.kinds.entry(piece.kind).or_default() += 1;
            items += 1;
            survey.items += 1;
            if piece.documented {
                survey.documented += 1;
            }
            if let Some(cause) = halting(piece) {
                survey.stops.push(Stop {
                    module: module.clone(),
                    name: piece.name.clone(),
                    cause,
                });
            }
        }
        if items == 0 {
            continue;
        }
        match survey.modules.iter_mut().find(|held| held.path == module) {
            Some(held) => {
                held.items += items;
                for (kind, count) in kinds {
                    *held.kinds.entry(kind).or_default() += count;
                }
            }
            None => survey.modules.push(Module {
                path: module,
                kinds,
                items,
            }),
        }
    }
    survey
        .modules
        .sort_by_key(|module| std::cmp::Reverse(module.items));
    survey
}

/// The top-level module a file belongs to, as Rust names it.
///
/// A directory is a module and so is a file, and `mod.rs` names the directory rather than itself.
/// The top level is what a reader is given to navigate by, so a file nested deeper is counted
/// under the module it hangs from rather than listed on its own.
fn module_of(path: &str) -> String {
    let trimmed = Path::new(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let from = trimmed
        .iter()
        .position(|part| part == "src")
        .map_or(0, |at| at + 1);
    let rest: Vec<&String> = trimmed.iter().skip(from).collect();
    let Some(first) = rest.first() else {
        return String::new();
    };
    let head = first.trim_end_matches(".rs");
    if head == "lib" || head == "main" || head == "mod" {
        return String::new();
    }
    head.to_owned()
}

/// The condition a piece states for stopping, where it states one.
///
/// A stop whose condition is written down is worth reporting. A bare unwrap states nothing, and
/// the reading pass records no cause for it, so it is absent here rather than invented.
fn halting(piece: &Piece) -> Option<String> {
    piece.facts.iter().find_map(|found| match &found.fact {
        Fact::Halts(Halt::Unless(what) | Halt::Expects(what)) => {
            Some(format!("unless `{what}` holds"))
        }
        Fact::Halts(Halt::Says(what)) => Some(format!("and reports `{what}`")),
        Fact::Halts(Halt::Equal(a, b)) => Some(format!("unless `{a}` equals `{b}`")),
        Fact::Halts(Halt::Differ(a, b)) => Some(format!("unless `{a}` differs from `{b}`")),
        _ => None,
    })
}

/// A sentence the summary may take, with what it costs and what it is about.
struct Line {
    text: String,
    /// Whether it says anything a reader could not get by looking at the declaration list.
    told: bool,
    /// What it is about, so two sentences saying the same thing can be compared.
    about: &'static str,
}

/// What a count is worth saying, in the same units the item pass prices findings in.
const COUNTED: f64 = 12.0;

/// One section of the summary, which holds a heading, the sentences its paragraph may take, and
/// any table the counts license.
struct Section {
    head: Option<&'static str>,
    lines: Vec<Line>,
    rows: Vec<(String, String)>,
    columns: Option<(&'static str, &'static str)>,
    listed: Vec<Line>,
}

/// The whole summary, as markdown, or nothing where the crate licensed nothing.
///
/// A reader needs the sections in one order, so the pass writes them in it. The crate is named,
/// then measured, then broken into parts, then laid out, and what can stop a caller comes last.
/// The pass drops a section with no evidence behind it rather than writing an empty one, so the
/// shape of the document is itself a report.
#[must_use]
pub fn summarised(survey: &Survey) -> Option<String> {
    if survey.items == 0 {
        return None;
    }
    let mut out = Vec::new();
    let words = crate::document::spelled(&survey.name);
    out.push(format!("# {}", titled(&words)));
    let mut written = 0;
    for section in sections(survey) {
        let paragraph = paragraph(&section.lines);
        if paragraph.is_empty() && section.rows.is_empty() && section.listed.is_empty() {
            continue;
        }
        written += 1;
        if let Some(head) = section.head {
            out.push(String::new());
            out.push(format!("## {head}"));
        }
        if !paragraph.is_empty() {
            out.push(String::new());
            out.push(paragraph);
        }
        if let (Some((left, right)), false) = (section.columns, section.rows.is_empty()) {
            out.push(String::new());
            out.push(format!("| {left} | {right} |"));
            out.push("| --- | ---: |".to_owned());
            for (name, value) in &section.rows {
                out.push(format!("| {name} | {value} |"));
            }
        }
        let bullets = chosen(&section.listed);
        if !bullets.is_empty() {
            out.push(String::new());
            for bullet in bullets {
                out.push(format!("- {bullet}"));
            }
        }
    }
    if written == 0 {
        return None;
    }
    out.push(String::new());
    out.push(provenance(survey));
    Some(out.join("\n"))
}

/// A paragraph built one sentence at a time, kept readable at every step.
///
/// A sentence that reads on its own can still spoil the passage it joins, because a convention
/// like not repeating a word is about the run of prose and not about any one sentence in it. So
/// the pass tries each sentence against the paragraph as it stands, and keeps it only where the
/// whole still reads. That is the same take-or-leave decision the item pass makes, asked of a
/// larger unit.
fn paragraph(offered: &[Line]) -> String {
    let mut held = String::new();
    for sentence in chosen(offered) {
        let tried = if held.is_empty() {
            sentence.clone()
        } else {
            format!("{held} {sentence}")
        };
        if crate::document::readable(&tried).is_some() {
            held = tried;
        }
    }
    held
}

/// The sentences kept, by the same arithmetic the item pass uses.
///
/// A sentence earns its place by telling a reader something the source listing does not already
/// show, and pays for the words it takes. One that only names what the reader can see costs its
/// words and earns nothing, so it is left. Two sentences about the same thing compete, and the
/// shorter is the one that survives.
fn chosen(offered: &[Line]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for candidate in offered {
        let fewest = offered
            .iter()
            .filter(|rival| rival.about == candidate.about)
            .map(|rival| counted(&rival.text))
            .fold(f64::INFINITY, f64::min);
        let words = counted(&candidate.text) - fewest;
        let earned = if candidate.told { COUNTED } else { 0.0 };
        if words - earned < 0.0 && !kept.contains(&candidate.text) {
            kept.push(candidate.text.clone());
        }
    }
    kept
}

/// How long a sentence is, counted in words.
fn counted(text: &str) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let words = text.split_whitespace().count() as f64;
    words
}

/// Every section the evidence licenses, each with the sentences and rows behind it.
fn sections(survey: &Survey) -> Vec<Section> {
    vec![
        Section {
            head: None,
            lines: opening(survey),
            rows: Vec::new(),
            columns: None,
            listed: Vec::new(),
        },
        Section {
            head: Some("What it is made of"),
            lines: composition(survey),
            rows: kind_rows(survey),
            columns: Some(("Kind", "Public items")),
            listed: Vec::new(),
        },
        Section {
            head: Some("How it is laid out"),
            lines: layout(survey),
            rows: module_rows(survey),
            columns: Some(("Module", "Public items")),
            listed: Vec::new(),
        },
        Section {
            head: Some("What can stop"),
            lines: stopping(survey),
            rows: Vec::new(),
            columns: None,
            listed: stop_lines(survey),
        },
        Section {
            head: Some("What it is built on"),
            lines: foundation(survey),
            rows: Vec::new(),
            columns: None,
            listed: Vec::new(),
        },
    ]
}

/// One row per kind of item the crate declares, largest first.
fn kind_rows(survey: &Survey) -> Vec<(String, String)> {
    let mut ranked: Vec<(&&str, &usize)> = survey.kinds.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    ranked
        .iter()
        .map(|(kind, count)| (capitalised(english(kind)), count.to_string()))
        .collect()
}

/// One row per module, largest first, for the modules a reader would navigate by.
fn module_rows(survey: &Survey) -> Vec<(String, String)> {
    survey
        .modules
        .iter()
        .filter(|module| !module.path.is_empty())
        .take(12)
        .map(|module| (format!("`{}`", module.path), module.items.to_string()))
        .collect()
}

/// One line per call that states a condition on which it stops.
fn stop_lines(survey: &Survey) -> Vec<Line> {
    survey
        .stops
        .iter()
        .take(12)
        .filter_map(|stop| {
            let sentence = format!(
                "The call `{}` in `{}` stops {}.",
                stop.name, stop.module, stop.cause
            );
            graded(&sentence).map(|text| Line {
                text,
                told: true,
                about: "stop",
            })
        })
        .collect()
}

/// The word with its first letter capitalised.
fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    }
}

/// The opening, which says what the crate is and how much of it there is.
fn opening(survey: &Survey) -> Vec<Line> {
    let mut lines = Vec::new();
    let words = crate::document::spelled(&survey.name);
    if let Some(text) = crate::document::nameable(&format!("The {}.", words.join(" "))) {
        lines.push(Line {
            text,
            told: false,
            about: "name",
        });
    }
    let title = titled(&words);
    let modules = survey.modules.len();
    if survey.items > 0 && modules > 0 {
        let sentence = format!(
            "{title} declares {} across {}, in {}.",
            counting(survey.items, "public item"),
            counting(modules, "module"),
            counting(survey.files, "file")
        );
        if let Some(text) = graded(&sentence) {
            lines.push(Line {
                text,
                told: true,
                about: "size",
            });
        }
    }
    if survey.documented > 0 && survey.items > 0 {
        let bare = survey.items - survey.documented;
        let sentence = if bare == 0 {
            "Every one of them carries a comment.".to_owned()
        } else {
            format!(
                "Comments cover {} of them, and {} carry none.",
                survey.documented, bare
            )
        };
        if let Some(text) = graded(&sentence) {
            lines.push(Line {
                text,
                told: true,
                about: "documented",
            });
        }
    }
    lines
}

/// What the crate is made of, counted by the word Rust uses for each kind.
fn composition(survey: &Survey) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut ranked: Vec<(&&str, &usize)> = survey.kinds.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    let listed: Vec<String> = ranked
        .iter()
        .map(|(kind, count)| counting(**count, english(kind)))
        .collect();
    if !listed.is_empty() {
        let sentence = format!("The crate declares {}.", joined(&listed));
        if let Some(text) = graded(&sentence) {
            lines.push(Line {
                text,
                told: true,
                about: "kinds",
            });
        }
    }
    if let Some((kind, count)) = ranked.first() {
        if survey.items > 0 {
            let share = percent(**count, survey.items);
            let sentence = format!(
                "The largest share of that, {share}, is {}.",
                crate::repair::pluralised(english(kind))
            );
            if let Some(text) = graded(&sentence) {
                lines.push(Line {
                    text,
                    told: true,
                    about: "dominant",
                });
            }
        }
    }
    lines
}

/// How the crate is laid out, named by its largest modules.
fn layout(survey: &Survey) -> Vec<Line> {
    let mut lines = Vec::new();
    let named: Vec<&Module> = survey
        .modules
        .iter()
        .filter(|module| !module.path.is_empty())
        .take(5)
        .collect();
    if named.is_empty() {
        return lines;
    }
    let listed: Vec<String> = named
        .iter()
        .map(|module| format!("`{}`", module.path))
        .collect();
    let sentence = format!("The heaviest modules are {}.", joined(&listed));
    if let Some(text) = graded(&sentence) {
        lines.push(Line {
            text,
            told: true,
            about: "largest",
        });
    }
    if survey.items > 0 && named.len() > 1 {
        let share = percent(named[0].items, survey.items);
        let sentence = format!(
            "A share of {share} sits in the first of them alone, which holds {}.",
            counting(named[0].items, "declaration")
        );
        if let Some(text) = graded(&sentence) {
            lines.push(Line {
                text,
                told: true,
                about: "concentration",
            });
        }
    }
    lines
}

/// What can stop a caller, and the condition the code states for it.
fn stopping(survey: &Survey) -> Vec<Line> {
    let mut lines = Vec::new();
    if survey.stops.is_empty() {
        return lines;
    }
    let sentence = format!(
        "The source states a condition on which {} can stop.",
        counting(survey.stops.len(), "public call")
    );
    if let Some(text) = graded(&sentence) {
        lines.push(Line {
            text,
            told: true,
            about: "stops",
        });
    }
    let mut homes: Vec<&str> = survey
        .stops
        .iter()
        .map(|stop| stop.module.as_str())
        .filter(|home| !home.is_empty())
        .collect();
    homes.sort_unstable();
    homes.dedup();
    if !homes.is_empty() {
        let listed: Vec<String> = homes.iter().map(|home| format!("`{home}`")).collect();
        let sentence = format!("Those warnings come from {}.", joined(&listed));
        if let Some(text) = graded(&sentence) {
            lines.push(Line {
                text,
                told: true,
                about: "homes",
            });
        }
    }
    lines
}

/// What the crate rests on, as its manifest names them.
fn foundation(survey: &Survey) -> Vec<Line> {
    let mut lines = Vec::new();
    if survey.built_on.is_empty() {
        return lines;
    }
    let listed: Vec<String> = survey
        .built_on
        .iter()
        .map(|name| format!("`{name}`"))
        .collect();
    let sentence = format!("The crate is built on {}.", joined(&listed));
    if let Some(text) = graded(&sentence) {
        lines.push(Line {
            text,
            told: true,
            about: "deps",
        });
    }
    lines
}

/// The line that says where every number above came from.
///
/// A generated document that does not say it was generated invites a reader to argue with a
/// person about a count, so it says so and names what it read.
fn provenance(survey: &Survey) -> String {
    format!(
        "Generated by reading {} of source. Every count above is of public items only.",
        counting(survey.files, "file")
    )
}

/// The English noun for a word Rust declares things with.
///
/// A summary is prose, and a keyword is not a word of English: "fn" has no plural and no reading,
/// so a sentence built on it is rejected by the grammar and the section is lost. The mapping is of
/// the language rather than of any repository, so it is written once here and holds everywhere.
fn english(kind: &str) -> &'static str {
    match kind {
        "fn" => "function",
        "struct" => "structure",
        "enum" => "enumeration",
        "const" => "constant",
        "static" => "static value",
        "type" => "type alias",
        _ => "trait",
    }
}

/// A count and the thing counted, with the thing spelled as English spells it for that number.
///
/// The plural is spelled by the same rule the repair pass uses, so a word the engine has never
/// seen is still written correctly and nothing is listed here.
fn counting(count: usize, thing: &str) -> String {
    if count == 1 {
        return format!("1 {thing}");
    }
    let plural = crate::repair::pluralised(thing);
    format!("{count} {plural}")
}

/// A share of a whole, written as English writes one.
fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "None".to_owned();
    }
    #[allow(clippy::cast_precision_loss)]
    let share = (part as f64 / whole as f64) * 100.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rounded = share.round() as usize;
    format!("{rounded} percent")
}

/// A list written the way English writes one, with the last item joined by "and".
fn joined(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// The crate name written as a title, each word capitalised as English capitalises a title.
fn titled(words: &[String]) -> String {
    words
        .iter()
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A sentence the engine can read, or nothing.
///
/// The same gate the item pass writes under, so a sentence this composes badly is not written.
/// A summary sentence is a full sentence here rather than a noun phrase, with one exception: the
/// opening names the crate and has no predicate to give, which is the convention a summary line
/// already follows.
fn graded(text: &str) -> Option<String> {
    crate::document::readable(text)
}

#[cfg(test)]
mod tests {
    use super::{counting, joined, module_of, percent, summarised, surveyed};

    #[test]
    fn a_count_is_written_with_the_word_english_spells_for_it() {
        assert_eq!(counting(1, "module"), "1 module");
        assert_eq!(counting(3, "module"), "3 modules");
        assert_eq!(counting(2, "public item"), "2 public items");
    }

    #[test]
    fn a_list_is_joined_the_way_english_joins_one() {
        assert_eq!(joined(&["a".to_owned()]), "a");
        assert_eq!(joined(&["a".to_owned(), "b".to_owned()]), "a and b");
        assert_eq!(
            joined(&["a".to_owned(), "b".to_owned(), "c".to_owned()]),
            "a, b, and c"
        );
    }

    #[test]
    fn a_file_is_counted_under_the_module_it_hangs_from() {
        assert_eq!(module_of("src/dp.rs"), "dp");
        assert_eq!(module_of("src/data/sugar/mod.rs"), "data");
        assert_eq!(module_of("src/lib.rs"), "");
    }

    #[test]
    fn a_share_is_written_as_a_share() {
        assert_eq!(percent(1, 2), "50 percent");
        assert_eq!(percent(0, 0), "None");
    }

    #[test]
    fn a_crate_with_nothing_public_earns_no_summary() {
        let files = vec![("src/lib.rs".to_owned(), "fn hidden() {}".to_owned())];
        assert!(summarised(&surveyed("thing", &files)).is_none());
    }

    #[test]
    fn a_crate_is_summarised_from_what_its_code_declares() {
        let files = vec![
            (
                "src/acid.rs".to_owned(),
                "pub struct AcidSystem { pub ph: f64 }\npub fn holds(n: usize) -> bool { assert!(n > 0); true }".to_owned(),
            ),
            (
                "src/salt.rs".to_owned(),
                "pub struct SaltSystem { pub mass: f64 }".to_owned(),
            ),
        ];
        let survey = surveyed("acid_kit", &files);
        assert_eq!(survey.items, 3);
        assert_eq!(survey.modules.len(), 2);
        assert_eq!(survey.stops.len(), 1);
        let text = summarised(&survey).expect("a crate that declares things earns a summary");
        assert!(text.starts_with("# Acid Kit"));
        assert!(text.contains("3 public items"));
        assert!(text.contains("1 public call can stop"));
        assert!(text.contains("The call `holds` in `acid` stops unless `n > 0` holds."));
    }
}
