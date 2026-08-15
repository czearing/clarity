//! Pulling the prose out of a source or documentation file.
//!
//! A file is not prose. It is prose with code around it, and the code is not English and must not
//! be judged as though it were. What is left after the code is taken out is what a reader of the
//! documentation actually reads, which is the only thing worth holding to a standard.

/// Prose from doc comments, skipping fenced code and hidden doctest lines.
#[must_use]
pub fn from_source(source: &str) -> String {
    let mut prose = String::new();
    let mut fenced = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        else {
            continue;
        };
        let rest = rest.trim();
        if rest.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || rest.starts_with('#') || rest.starts_with('|') || rest.is_empty() {
            continue;
        }
        prose.push_str(rest);
        prose.push(' ');
    }
    prose
}

/// Prose from markdown, skipping fenced code, headings, and tables.
#[must_use]
pub fn from_markdown(source: &str) -> String {
    let mut prose = String::new();
    let mut fenced = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.starts_with('#') || line.starts_with('|') || line.is_empty() {
            continue;
        }
        prose.push_str(line);
        prose.push(' ');
    }
    prose
}
