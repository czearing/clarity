//! Print every fault the engine finds in one file's prose, with the rule that names it.
use std::path::Path;

use clarity::prose::{from_markdown, from_source};
use clarity::register::read;
use clarity::text::Text;

fn main() {
    let path = std::env::args().nth(1).expect("a path to read");
    let source = std::fs::read_to_string(&path).expect("a readable file");
    let prose = if Path::new(&path)
        .extension()
        .is_some_and(|kind| kind == "rs")
    {
        from_source(&source)
    } else {
        from_markdown(&source)
    };
    for (unit, (_, report)) in Text::read(&prose).units.iter().zip(read(&prose)) {
        for fault in &report.faults {
            println!("{} <- {}", unit.text(), fault.rule.says());
        }
    }
}
