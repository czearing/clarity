//! Write the summary of a whole repository from what its source declares.
//!
//! Point it at a crate directory. It reads the manifest for the name and what the crate is built
//! on, reads every source file under it, and writes markdown to standard output. Given `--write`
//! it puts that markdown in `SUMMARY.md` instead, and `--out=<path>` names another file.
//!
//! Nothing here is written about any particular repository. The engine assembles every sentence
//! from a count or from a name it read in the source, and the grammar that grades a repository
//! grades that sentence as well. One the grammar charges a fault for is dropped, so a section
//! with no evidence behind it comes out missing rather than empty.

use clarity::summarise::{summarised, surveyed, Survey};
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let writing = args.iter().any(|arg| arg == "--write");
    let root = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);

    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    let name = named(&manifest);
    if name.is_empty() {
        clarity::say!(
            "no crate name found in {}",
            root.join("Cargo.toml").display()
        );
        std::process::exit(1);
    }

    let mut files = Vec::new();
    gather(&root.join("src"), &mut files);
    if files.is_empty() {
        clarity::say!("no source found under {}", root.join("src").display());
        std::process::exit(1);
    }

    let mut survey: Survey = surveyed(&name, &files);
    survey.built_on = depended(&manifest);

    let Some(text) = summarised(&survey) else {
        clarity::say!("{name} declares nothing public, so there is nothing to summarise");
        std::process::exit(1);
    };

    let out = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--out="))
        .map(PathBuf::from);
    if writing || out.is_some() {
        let at = out.unwrap_or_else(|| root.join("SUMMARY.md"));
        match fs::write(&at, format!("{text}\n")) {
            Ok(()) => clarity::say!("wrote {}", at.display()),
            Err(why) => {
                clarity::say!("could not write {}: {why}", at.display());
                std::process::exit(1);
            }
        }
    } else {
        clarity::say!("{text}");
    }
}

/// Every source file under a directory, paired with what it holds.
fn gather(at: &Path, into: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(at) else {
        return;
    };
    let mut sorted: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    sorted.sort();
    for path in sorted {
        if path.is_dir() {
            gather(&path, into);
        } else if path.extension().is_some_and(|end| end == "rs") {
            if let Ok(source) = fs::read_to_string(&path) {
                into.push((path.display().to_string(), source));
            }
        }
    }
}

/// The name a manifest publishes the crate under.
///
/// Read from the package table only. A workspace manifest names members rather than a crate, and
/// taking a name from one would put a directory's name on a summary of something else.
fn named(manifest: &str) -> String {
    valued(manifest, "package", "name").unwrap_or_default()
}

/// What the manifest says the crate is built on.
fn depended(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            let key = key.trim().trim_matches('"');
            if !key.is_empty() {
                names.push(key.to_owned());
            }
        }
    }
    names
}

/// One key of one table of a manifest, where both are there.
fn valued(manifest: &str, table: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line.trim_matches(['[', ']']) == table;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some((found, value)) = line.split_once('=') {
            if found.trim() == key {
                return Some(value.trim().trim_matches('"').to_owned());
            }
        }
    }
    None
}
