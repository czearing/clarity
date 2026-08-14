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

## Register

A haiku has no verb. A text message has no closing period and often no apostrophe. A chorus repeats
itself on purpose. None of that is a mistake, and none of it is a kind of writing the engine has
been told about.

There is no list of genres anywhere in the source. There is a list of *conventions* a passage may or
may not hold to, and a second dynamic program over the whole passage that recovers which ones it
kept. A convention is dropped only when dropping it explains more than half the units, and the
choice is sticky, so one fragment cannot turn a paragraph into verse.

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

let found = read("hey\nim running late\nthe train dont move");
assert!(!found.last().unwrap().1.faults.is_empty());
```

Adding a convention adds a dimension to the search and nothing else. Nothing names a form of
writing, so a form the author never considered is handled the same way as one he did.

## Refusal

A word the lexicon cannot place is reported, and the sentence is not called clean:

```rust
use clarity::assess::assess;

let found = assess("the frobnicator runs");
assert_eq!(found.report.unknown, [1]);
assert!(!found.is_clean());
```

This is the point of the design. An engine that guesses is right most of the time and cannot tell
you when it is not. This one is right about everything it accepts, and says so when it declines.

## Measured

Every sentence in `tests/corpus.rs` and every passage in `tests/registers.rs` is labelled.

| | |
| --- | --- |
| Grammatical sentences accepted | 55 of 55, no false alarms |
| Faulty sentences caught and named | 40 of 40, correct rule every time |
| Faulty sentences repaired to clean | 40 of 40, never more than two swaps |
| Wasteful sentences named | 12 of 12 |
| Passages whose conventions were recovered | 5 of 5, and the planted fault caught in each |
| Check a short sentence | 10.5 us |
| Check a fifteen word sentence | 36 us |
| Find a repair | 73 us |

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
