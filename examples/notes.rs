//! Print the style notes for each sentence given on the command line.
fn main() {
    for text in std::env::args().skip(1) {
        println!("{text}: {:?}", clarity::assess::assess(&text).notes);
    }
}
