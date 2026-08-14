# Rules

Every rule the engine enforces, and where it comes from. Nothing is enforced that is not here.

The grammar follows Huddleston and Pullum, *The Cambridge Grammar of the English Language*,
Cambridge University Press, 2002, chapters 3 to 5. Where that work describes a construction as
current, the engine accepts it, even when a style guide objects.

## Agreement

| Rule | Requires | Example it rejects |
| --- | --- | --- |
| `DeterminerNumber` | A determiner and its noun share number | "a dogs" |
| `SubjectVerb` | A tensed verb agrees with the head of its subject | "the dogs runs" |

Agreement is judged over the clause, not over neighbouring words. The head of the subject noun
phrase governs the verb, so a modifier in between is stepped past. This is what separates "the key
to the cabinets is missing", which is correct, from "the key to the cabinets are missing", which
is the attraction error it resembles.

## Complementation

| Rule | Requires |
| --- | --- |
| `ModalTakesBase` | A modal is followed by the plain form |
| `ToTakesBase` | Infinitival "to" is followed by the plain form |
| `PrepositionTarget` | A preposition takes a noun phrase, not a tensed verb |
| `DoubledTense` | Two tensed verbs are not adjacent |
| `StrandedParticiple` | A participle has an auxiliary before it |

Whether "to" is infinitival or a preposition is decided by the verb before it. Subcategorization
is a property of the individual verb and cannot be derived, so the verbs that license a
to-infinitive are listed.

## Structure

| Rule | Requires |
| --- | --- |
| `DeterminerTarget` | A determiner introduces a noun phrase |
| `PronounIsWhole` | A pronoun has no noun attached to it |
| `AttributiveSingular` | A noun modifying another noun is singular |
| `NoPredicate` | A sentence has a subject and a tensed verb |
| `DoubledTense` | A negated verb is followed by a plain form, not a second tensed one |
| `Unmarked` | A sentence opens with a capital and closes with a mark |
| `Unapostrophed` | A contraction keeps its apostrophe |

`NoPredicate` is not a check applied after the fact. It is a constraint on the search: if the
cheapest reading has no tensed verb, each word that could carry tense is forced in turn and the
cheapest sentence wins. Without it, "the child walk" reads as a noun phrase and no rule is broken.

## Structural and conventional

The rules above divide in two. Agreement, government, and predication are structural: breaking one
makes a sentence ungrammatical in any English at all. The capital, the closing mark, the
apostrophe, and the demand for a predicate are conventional: whether they apply depends on what is
being written.

`register` recovers which conventions a passage holds to, so a haiku is not told it needs a verb
and a text message is not told it needs a period. The list of conventions is:

| Convention | Requires |
| --- | --- |
| `Predicate` | Every unit is a sentence with a subject and a tensed verb |
| `Marks` | Every unit opens with a capital and closes with a mark |
| `Apostrophes` | Contractions are spelled with their apostrophe |
| `Fresh` | A content word is not echoed within eight words |
| `Plain` | Roundabout, buried, and filler wording is avoided |

Agreement is deliberately absent from that list. No register can excuse it, which is what keeps the
inference from becoming an excuse for anything the writer does.

The recovery is a second dynamic program over the passage, one unit at a time. Waiving a convention
costs half of what a fault costs, so a convention is dropped only when it explains more than half
the units it touches; changing register between units costs a full fault, so a single fragment
cannot turn a paragraph into verse.

No form of writing is named anywhere in the source. Adding a convention adds a dimension to the
search, and every combination of conventions is considered, so a form nobody anticipated is handled
by the same machinery as one that was.

## Clarity

Integration cost from dependency locality theory. Attaching a word to the head it depends on costs
one unit for every new discourse referent introduced in between, so distance is counted in
entities rather than words.

Gibson, "Linguistic complexity: locality of syntactic dependencies", *Cognition* 68, 1998, 1 to 76.
Gibson, "The dependency locality theory", in *Image, Language, Brain*, MIT Press, 2000, 95 to 126.

## Style

Each entry in `style` is a phrase with a shorter equivalent, or a repetition that carries nothing.
The replacement is always named, so a writer sees the trade rather than an instruction.
