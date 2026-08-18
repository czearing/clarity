//! Print the reading and the faults for each sentence given on the command line.

use clarity::check::check;
use clarity::grammar::Sentence;

fn main() {
    for text in std::env::args().skip(1) {
        let report = check(&Sentence::read(&text));
        println!("{text}");
        for (token, tag) in Sentence::read(&text).tokens.iter().zip(&report.tags) {
            println!("  {:12} {tag:?}", token.word);
        }
        println!("  faults {:?}", report.faults);
    }
}
