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

use clarity::code::{findings, Fact};
use clarity::document::written;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let writing = args.iter().any(|arg| arg == "--write");
    let naming = args.iter().any(|arg| arg == "--names");
    let paths: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();

    let mut items = 0usize;
    let mut proposed = 0usize;
    let mut names = 0usize;
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
                        println!(
                            "{path}:{} {name} is {} and its type is {}",
                            piece.line,
                            had.says(),
                            wanted.says()
                        );
                    }
                }
            }
            if naming || piece.documented || !piece.public {
                continue;
            }
            let Some(comment) = written(piece) else {
                continue;
            };
            proposed += 1;
            let pad = " ".repeat(piece.indent);
            let mut block = String::new();
            for line in comment.lines() {
                block.push_str(&pad);
                block.push_str("/// ");
                block.push_str(line);
                block.push('\n');
            }
            if writing {
                edits.push((piece.line, block));
            } else {
                println!("{path}:{}", piece.line);
                for line in comment.lines() {
                    println!("    /// {line}");
                }
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

    if naming {
        println!("items {items}, names disagreeing with their type {names}");
    } else if writing {
        println!("items {items}, comments written {proposed}, files touched {touched}");
    } else {
        println!("items {items}, comments proposed {proposed}");
    }
}
