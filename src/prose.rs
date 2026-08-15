//! Pulling the prose out of a source or documentation file.
//!
//! A file is not prose. It is prose with code around it, and the code is not English and must not
//! be judged as though it were. What is left after the code is taken out is what a reader of the
//! documentation actually reads, which is the only thing worth holding to a standard.

/// Prose from doc comments, skipping fenced code and hidden doctest lines.
#[must_use]
pub fn from_source(source: &str) -> String {
    joined(&lines_of_source(source))
}

/// Prose from markdown, skipping fenced code, headings, and tables.
#[must_use]
pub fn from_markdown(source: &str) -> String {
    joined(&lines_of_markdown(source))
}

/// Prose from `path`'s kind of file, with each line's place in the file kept.
///
/// Reading prose out of a file and reading it back in are the same act seen from two sides, so
/// they are answered by one function. A change that cannot be put back where it came from is a
/// suggestion rather than a correction, and the only thing that makes it one is knowing the byte
/// the words were taken from.
///
/// The lines come back grouped, because one doc comment is one passage and one paragraph is one
/// paragraph. Which sentences a passage can spare is a question about the passage, so a
/// condensation asked of a line at a time gets a different and wrong answer.
#[must_use]
pub fn placed(source: &str, rust: bool) -> Vec<Vec<(usize, String)>> {
    if rust {
        lines_of_source(source)
    } else {
        lines_of_markdown(source)
    }
}

/// The prose lines run together, which is what a reader of them meets.
fn joined(blocks: &[Vec<(usize, String)>]) -> String {
    let mut prose = String::new();
    for (_, line) in blocks.iter().flatten() {
        prose.push_str(line);
        prose.push(' ');
    }
    prose
}

/// Prose lines of a Rust file, each with the byte its text starts at.
fn lines_of_source(source: &str) -> Vec<Vec<(usize, String)>> {
    let mut found = Vec::new();
    let mut block: Vec<(usize, String)> = Vec::new();
    let mut fenced = false;
    for (start, line) in numbered(source) {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        else {
            fenced = false;
            found.extend(held(&mut block));
            continue;
        };
        if rest.trim().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let text = rest.trim();
        if fenced || text.starts_with('#') || text.starts_with('|') {
            continue;
        }
        if text.is_empty() {
            found.extend(held(&mut block));
            continue;
        }
        let at = start + (line.len() - trimmed.len()) + 3 + (rest.len() - rest.trim_start().len());
        block.push((at, text.to_owned()));
    }
    found.extend(held(&mut block));
    found
}

/// Prose lines of a markdown file, each with the byte its text starts at.
fn lines_of_markdown(source: &str) -> Vec<Vec<(usize, String)>> {
    let mut found = Vec::new();
    let mut block: Vec<(usize, String)> = Vec::new();
    let mut fenced = false;
    for (start, line) in numbered(source) {
        let text = line.trim();
        if text.starts_with("```") {
            fenced = !fenced;
            found.extend(held(&mut block));
            continue;
        }
        if fenced || text.starts_with('#') || text.starts_with('|') {
            continue;
        }
        if text.is_empty() {
            found.extend(held(&mut block));
            continue;
        }
        block.push((
            start + (line.len() - line.trim_start().len()),
            text.to_owned(),
        ));
    }
    found.extend(held(&mut block));
    found
}

/// The block being gathered, handed over and cleared, or nothing if it never started.
fn held(block: &mut Vec<(usize, String)>) -> Option<Vec<(usize, String)>> {
    (!block.is_empty()).then(|| std::mem::take(block))
}

/// Every line with the byte it starts at.
fn numbered(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut at = 0;
    source.lines().map(move |line| {
        let start = at;
        at += line.len() + 1;
        (start, line)
    })
}
