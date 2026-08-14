//! Cut each passage on standard input down to its point.

fn main() {
    let raw = std::fs::read_to_string(std::env::args().nth(1).expect("a file")).expect("readable");
    for (at, passage) in raw.split("===").enumerate() {
        let passage = passage.trim();
        if passage.is_empty() {
            continue;
        }
        let core = clarity::condense::condense(passage);
        println!("--- {} (dropped {})", at + 1, core.dropped);
        println!("about: {:?}", core.about);
        println!("{}", core.text());
    }
}
