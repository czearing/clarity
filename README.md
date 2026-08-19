# clarity

A grammar and clarity engine that can explain every judgement it makes.

English marks a plural noun and a present tense verb with the same letter. Only context tells
`runs` in "the dog runs" from `runs` in "the morning runs". So the categories are not guessed word
by word; they are decoded, by a dynamic program over every reading the lexicon allows, with the
rules of grammar as the cost of moving between them.

The cheapest reading of a sentence is the one that breaks the fewest rules. Whatever it still pays
for is exactly what is wrong, and every fault comes back with the rule that produced it.

```rust
use clarity::assess::assess;

let found = assess("the key to the cabinets are missing");

assert_eq!(found.report.faults[0].rule.says(), "a tensed verb must agree with its subject");
assert_eq!(found.repaired().text(), "the key to the cabinets is missing");
```

Note which noun it agreed with. `cabinets` is next to the verb; `key` is the subject. Agreement is
judged over the clause, stepping past the modifier, so the engine does not make the mistake the
sentence was designed to provoke.

## What it does

| | |
| --- | --- |
| Tags | Decodes the category of every word from context, not from a guess |
| Judges | Names the rule behind every fault, never asserts one |
| Repairs | Finds the fewest single-word swaps that leave nothing to report |
| Scores | Integration cost, which is what makes a sentence hard to hold in mind |
| Trims | Redundancy, roundabout connectives, buried verbs, filler, worn phrases, echoes |
| Infers | Works out what a passage holds itself to before holding it to anything |
| Refuses | Says a word is unknown rather than judging a sentence it cannot read |
| Documents | Writes a doc comment from what a signature and a body prove, and reads it back before writing it |

## Register

A haiku has no verb. A text message has no closing period and often no apostrophe. A chorus repeats
itself on purpose. None of that is a mistake, and none of it is a kind of writing the engine has
been told about.

There is no list of genres anywhere in the source. There is a list of *conventions* a passage may or
may not hold to, and a second dynamic program over the whole passage that recovers which ones it
kept. A passage pays for each convention it drops, once, and any unit may then be read
under it. Dropping one has to explain more than it costs, so a convention broken once is a mistake
and a convention broken twice is how the passage is written, and one fragment cannot turn a
paragraph into verse.

```rust
use clarity::register::{of, Convention};

assert!(of("an old pond\na frog jumps in\nthe sound of water").waives(Convention::Predicate));
assert!(of("hey\nim running late\ncant help it").waives(Convention::Apostrophes));
assert_eq!(of("The parser reads the file. It returns a tree."), clarity::register::Register::STRICT);
```

Conventions cover the predicate, the opening capital and closing mark, the apostrophe, freshness,
and plainness. Agreement is deliberately not among them, so no register can excuse it:

```rust
use clarity::register::read;

let found = read("hey\nim running late\nthe train were late");
assert!(!found.last().unwrap().1.faults.is_empty());
```

Adding a convention adds a dimension to the search and nothing else. Nothing names a form of
writing, so a form the author never considered is handled the same way as one he did.

## Refusal

Closed classes are listed outright, because English coins no new determiners and no new
prepositions. Open classes cannot be listed and are not: a word the lexicon has never seen is read
by its shape, and the categories that shape allows are offered to the search like any others. So a
word nobody has written before is read, and what is claimed about it is claimed by the rules around
it rather than by an entry.

```rust
use clarity::assess::assess;

let found = assess("the frobnicator runs");
assert!(found.report.unknown.is_empty(), "it is shaped like a noun, so it is read as one");
assert!(found.is_clean());
```

A string that is not shaped like an English word at all is a different matter. Nothing is guessed
about it, it is reported, and the sentence resting on it is not called clean:

```rust
use clarity::assess::assess;

let found = assess("the qwrtz runs");
assert_eq!(found.report.unknown, [1]);
assert!(!found.is_clean());
```

This is the point of the design. An engine that guesses is right most of the time and cannot tell
you when it is not. This one says which of the two it is doing.

## Measured

Every sentence in `tests/corpus.rs` and every passage in `tests/registers.rs` is labelled, and
every number in the table below is asserted against that corpus by a test, so it cannot go stale
without the suite going red.

Labelled sentences measure an engine against writing chosen to measure it. `tests/prose.rs`
measures it against writing that was not: every doc comment and documentation file in this
repository, written to explain the crate rather than to exercise it. What it still cannot read
there is counted, held to that count in both directions, and set out in `docs/LIMITS.md`.

| | |
| --- | --- |
| Grammatical sentences accepted | 74 of 74, no false alarms |
| Faulty sentences caught and named | 20 of 20, correct rule every time |
| Faulty sentences repaired to clean | 20 of 20, never more than two swaps |
| Wasteful sentences named | 12 of 12 |
| Passages whose conventions were recovered | 5 of 5, and the planted fault caught in each |
| Faults left in seven hundred units of the crate's own prose | 88, bounded in both directions |

Every example above is compiled and run as a doctest. Timings are what `cargo bench` reports for
`benches/read.rs`; on the machine that wrote this, checking a short sentence takes about two
milliseconds, a fifteen word sentence about two, and finding a repair about four. The length
barely matters, because the first step has to consider every state and the rest of the sentence
only visits the few that stay in play. A repository of a hundred thousand words takes eleven
seconds. Those are the only numbers here that no test asserts, because a time is a fact about a
machine.

## Design

Built on [fitkit](https://github.com/czearing/fitkit), which supplies the search, the cited laws,
and the refusals. Grammar contributes only the vocabulary and the costs:

- `tag` is the category a word can hold, kept small because the search costs its square.
- `lexicon` is a law. Closed classes are listed exhaustively, since English coins no new
  determiners. Open classes come from inflection. Anything else is unknown.
- `grammar` prices every pair of neighbouring categories. A rule broken costs a great deal; an
  unusual but legal pair costs a little.
- `check` reads the sentence, insists on a predicate, and reports what the reading still pays for.
- `register` is a second search, over the passage rather than the sentence, that recovers the
  conventions the writing holds to before any of them is enforced.
- `repair`, `clarity`, and `style` work on the reading, not the raw text.

Read `docs/` for the rules and their sources, and for what the engine deliberately does not do.

## Licence

MIT or Apache-2.0.

## Writing about an input

```text
describe src/                   # write about a tree of source
describe article.txt            # write about a page of prose
describe book.txt               # write about a book
```

One binary, and it is not told which of those it was given. A directory is read as source and a
file is read as prose, and after that both are the same two values: a corpus of the words the
input used, and a set of claims about the parts worth writing about. Everything particular to a
kind of input lives in `read`, and nothing particular to a kind of input lives anywhere else.

Two searches settle what appears. A subset search over the claims decides which parts are worth
stating, weighing what each part holds against how much of it the input actually describes, and
against how much vocabulary two parts share. A path search over the input's own words decides the
wording of each clause: a clause is a subsequence of the text it is about, so a word can be left
out and one passage can be spliced to another, but a place already used cannot be returned to.
Saying the same thing twice is not expensive, it is unrepresentable.

Nothing here holds a phrasing. There is no sentence written in this repository for an output to
be filled into, and no list of verbs: a word can only reach a reader if the input put it there,
which the test suite checks by taking every word written and finding the place it was read from.
The words that name a property are hashed on the way in, so what a caller calls something cannot
turn up in a sentence. Where a sentence ends is learned too, from which mark this text attaches to
its words, is followed by a capital more often than chance would explain, and finishes a passage
more often than chance would put it there.

A part the input says nothing about is refused rather than written about, and an input that never
finishes a sentence is refused rather than guessed at. That is why the engine can be handed a
framework, a wiki page and a novel without being changed for any of them: it was never taught what
any of them are.

Two more things are settled by measurement rather than by rule. A word that says something about a
part is given one place in a clause and not every place, so saying it twice is unavailable rather
than expensive, and where the input repeats itself the writing does not. And every ratio is weighed
by how many observations it rests on, because a word written once in one place looks infinitely
characteristic of that place, and a measure that believes it writes sentences out of whatever the
input happened to say once.

The whole of fitkit, seven crates and ten thousand words of doc comments, is read and written about
in 0.15 seconds. A TypeScript application takes 0.07, an encyclopedia entry 0.05, and a novel of
seven hundred thousand characters 0.19.

Of the four hundred and thirty words it wrote about fitkit, every one is written in fitkit's
source, and fifty six percent of the neighbouring pairs are pairs fitkit itself wrote; the rest are
joins the search made. What it produces is a compression of what each part says about itself, and
it reads like one: several lines are not sentences a person would sign.
