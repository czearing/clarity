//! Write and keep the doc comments of a repository, and never touch the code.
//!
//! Point it at files. It reports what it would write. Given `--write` it writes. What it writes is
//! only ever a run of doc comment lines above an item that had none. A line of code is never read
//! as prose, never rewritten, and never moved: the only edit the pass can express is inserting doc
//! comment lines at the start of a line, which is what makes touching code impossible rather than
//! merely discouraged.
//!
//! Given `--names` it reports names whose number disagrees with the type they are given, which is
//! the one thing here that is a claim about the code being wrong rather than about what it does.
//!
//! Given `--noise` it reports the doc comments already written that say nothing the declaration
//! under them does not. Nothing is deleted, because a comment is the author's and the pass can
//! only be sure it found no word of theirs in it, which is a reason to look and not a verdict.

use clarity::code::{findings, Fact};
use clarity::document::{says_nothing, written};
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let writing = args.iter().any(|arg| arg == "--write");
    let naming = args.iter().any(|arg| arg == "--names");
    let noise = args.iter().any(|arg| arg == "--noise");
    let paths: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();

    let mut items = 0usize;
    let mut proposed = 0usize;
    let mut names = 0usize;
    let mut empty = 0usize;
    let mut touched = 0usize;

    for path in paths {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let pieces = findings(&source);
        if pieces.is_empty() {
            continue;
        }
        let mut edits: Vec<(usize, String)> = Vec::new();
        for piece in &pieces {
            items += 1;
            if naming {
                for found in &piece.facts {
                    if let Fact::Misnumbered(name, had, wanted) = &found.fact {
                        names += 1;
                        clarity::say!(
                            "{path}:{} {name} is {} and its type is {}",
                            piece.line,
                            had.says(),
                            wanted.says()
                        );
                    }
                }
            }
            if noise {
                if piece.public && says_nothing(piece) {
                    empty += 1;
                    clarity::say!(
                        "{path}:{} {} says nothing the code does not",
                        piece.line,
                        piece.name
                    );
                    for line in &piece.doc {
                        clarity::say!("    /// {line}");
                    }
                }
                continue;
            }
            if naming || !piece.public {
                continue;
            }
            let Some(comment) = written(piece) else {
                continue;
            };
            proposed += 1;
            let at = under_doc(&source, piece.line);
            if writing {
                edits.push((at, marked(&comment, piece.indent)));
            } else {
                clarity::say!("{path}:{at}");
                clarity::put!("{}", marked(&comment, 4));
            }
        }
        if writing && !edits.is_empty() {
            edits.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
            let mut lines: Vec<String> = source.split_inclusive('\n').map(str::to_owned).collect();
            for (line, block) in edits {
                if line >= 1 && line <= lines.len() + 1 {
                    lines.insert(line - 1, block);
                }
            }
            if fs::write(path, lines.concat()).is_ok() {
                touched += 1;
            }
        }
    }

    if noise {
        clarity::say!("items {items}, comments saying nothing {empty}");
    } else if naming {
        clarity::say!("items {items}, names disagreeing with their type {names}");
    } else if writing {
        clarity::say!("items {items}, comments written {proposed}, files touched {touched}");
    } else {
        clarity::say!("items {items}, comments proposed {proposed}");
    }
}

/// The line a new section goes on, which is under whatever comment is already there.
///
/// An item's line is the first line of its comment when it has one, so writing there puts the new
/// section above the author's summary and takes the summary's place. What is being added belongs
/// under what they wrote, so the doc lines already there are stepped over first. Attributes are
/// not stepped over: a doc comment has to stay next to what it documents, and only the run of
/// comment lines starting the item is part of it.
fn under_doc(source: &str, line: usize) -> usize {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut at = line;
    while at >= 1 && at <= lines.len() && lines[at - 1].trim_start().starts_with("///") {
        at += 1;
    }
    at
}

/// A comment written out as the doc comment lines that go above an item.
///
/// A line with nothing on it is written without the space that would follow the marker, because a
/// space at the end of a line is what `cargo fmt` removes and what a reviewer sees as a change
/// nobody made.
fn marked(comment: &str, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut block = String::new();
    for line in comment.lines() {
        block.push_str(&pad);
        if line.is_empty() {
            block.push_str("///\n");
        } else {
            block.push_str("/// ");
            block.push_str(line);
            block.push('\n');
        }
    }
    block
}

#[cfg(test)]
mod tests {
    use super::under_doc;

    #[test]
    fn a_new_section_goes_under_the_comment_that_is_already_there() {
        // An item's line is the first line of its comment, so writing there displaced the
        // author's summary and put a panics section in its place.
        let source = "/// Solves it.\n///\n/// More about it.\npub fn solve() {}\n";
        assert_eq!(under_doc(source, 1), 4, "under the three comment lines");
    }

    #[test]
    fn an_item_with_no_comment_is_written_straight_above() {
        assert_eq!(under_doc("pub fn solve() {}\n", 1), 1);
    }

    #[test]
    fn an_attribute_is_not_stepped_over() {
        // A doc comment has to stay next to what it documents, so only the run of comment lines
        // starting the item is part of it.
        let source = "/// Solves it.\n#[inline]\npub fn solve() {}\n";
        assert_eq!(under_doc(source, 1), 2);
    }
}
