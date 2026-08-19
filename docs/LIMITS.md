# Limits

What this engine does not do, and why saying so is part of the design.

## It refuses words it does not know

The lexicon lists the closed classes exhaustively and derives open classes from inflection. A word
that neither settles is reported unknown and the sentence is not called clean. Coverage of ordinary
English prose is therefore partial, and the refusal rate is the honest measure of it.

Growing the lexicon raises coverage without touching a rule. That is the intended way to improve
the engine.

## It reads pairs, and one clause relation

Categories are decoded from neighbouring pairs. Subject to verb agreement is the only dependency
resolved over a longer span, because it is the only one that can be located exactly without a
parse. Coordination inside a subject, ellipsis, and inversion are out of reach.

## It repairs by inflection only

A repair swaps one form of a word for another. Nothing is added, removed, or reworded, so a repair
never changes what was meant. A sentence needing a rewrite gets no repair, which is reported rather
than approximated.

## It scores no readability formula

Flesch-Kincaid and its relatives count syllables and sentence length. Those correlate with
difficulty in the corpora the formulas were fitted to, but they do not cause it, and writing to
satisfy them can leave a text harder to read. Nothing here is scored by a measure that cannot say
why. Integration cost can.

## It infers conventions, and can be wrong about short passages

The register of a passage is recovered from the passage itself. Two or three units carry little
evidence, so a short passage is read under whatever convention set is cheapest and that may
not be what the writer intended. The engine is at its most confident where there is most to read.

A single sentence handed over on its own is checked under every convention except the closing mark,
because an isolated sentence says nothing about the punctuation habits of the text it came from.

## It judges sentences, not documents

There is no model of paragraph structure, topic flow, or argument. Style notes are local by
construction.

## It reads its own writing well, not perfectly

`tests/prose.rs` runs the engine over every doc comment and every documentation file in this
repository, six hundred units of prose written to explain the crate and not to exercise it, and
records what it still cannot answer for. The count is a bound the test holds the engine to in both
directions: a change that reads this prose worse fails, and a change that reads it better fails
until the new count is written down.

What is left is a long tail rather than a class. Each of these is a real gap, and each is here
because the reading behind it is wrong for a reason the engine has not yet been taught:

- A unit with no verb is charged as though it should have had one, which pushes the search to read
  some noun as a verb and produces a disagreement that was never in the writing. Whether a unit
  must be a sentence is a question for the register, but the register is recovered from readings
  and the reading has already been made. Reading each unit twice, once as a sentence and once as a
  phrase, is the fix, and it is not free: a phrase reading of "the dog run" is three nouns and
  clean, and a register that waived the predicate could then hide a disagreement behind it.
- A subject and its verb can be separated by more than one clause, and only one set-aside clause is
  tracked, so the third level of nesting answers to the wrong subject.
- Notional agreement is not modelled: "the rest attach", "every combination and every convention
  is", and their kind take the number the writer means rather than the number the head carries.
- A participial or absolute clause hanging off a sentence is read as though it were the sentence's
  own predicate.

Counting them here rather than suppressing them is the point. An engine that quietly stopped
reporting what it gets wrong would score better and be worth less.

## It does not hold "do" to agreement

`do`, `does` and `did` are listed with the modals, because what they share with a modal is what the
engine needed from them: they demand the plain form of a verb after them, so "the trains do not
moves" is caught by the demand rather than by a list of what may follow what.

What they do not share with a modal is inflection. A modal has one form and agrees with nothing,
which is why nothing asks it to, and treating "do" the same way means "the train do not move" goes
unreported. The reading is not wrong about anything else, and the demand is still enforced.

The fix is a category of its own for the auxiliary, which is a verb that inflects and also demands
a plain verb, rather than a modal that does neither. It is not in yet.

## One missing predicate is now one fault

A sentence that ends inside a clause it opened used to be charged twice for having no verb: once
for the sentence and once for the clause set aside. Both charges are the same rule, and a cost in
this framework names the rule it charges, so the two now come to one.

That made a verbless nested reading cheaper than it was, which moved the reading the search picks
in a handful of places and raised the crate's own count by one. A rise here means the model got
more honest, not that the reading got worse. The bound is recorded in both directions, so no
change like it can pass unnoticed.

## A register has to reach the search, not just its output

A register said which faults were held against a passage, and it said so after the passage had
been read. Waiving the demand for a predicate therefore suppressed the missing-predicate fault and
kept everything that fault had caused: the reading had already turned a noun into the verb the
convention was asking for, and the invented verb disagreed with something, so a heading that was
excused for having no verb was written up for the agreement error it did not have either.

Both readings now exist before either is chosen. A unit is read once as a sentence and once as a
phrase, and the passage decides between them by what each costs it, paying for the phrase
convention once and only if enough of its units want it. On documentation the engine had never
seen, this took the faults from forty to twenty-nine, and the agreement faults alone from nineteen
to nine.

What this does not do is make a phrase reading earn its keep. "The dog run fast." read as a phrase
is a determiner, a noun, a participle and an adjective, with nothing wrong anywhere, so in a file
whose other lines are summary phrases the disagreement goes unreported. Charging the phrase reading
for what the sentence reading still finds was tried and measured worse: it costs eight false alarms
on that same documentation to catch this, because the sentence reading of a genuine heading invents
a verb and the invented verb disagrees, which is the same circularity one level up.

## A gerund heading a subject was closing the phrase it opened

A gerund is a head, so reading one ended the phrase it was in. That is right for "keeps listing
them" and wrong for "listing them keeps the search exhaustive", where what follows the gerund is
its object and belongs to the subject. The clause treated the object as a second phrase and, having
already read a subject, took it for the subject of a clause of its own.

Where the phrase already stands now decides. Inside a phrase whose head has not arrived, an -ing
form is that phrase's head or a modifier of it; anywhere else it governs what follows, exactly as a
preposition governs its object. The same distinction settles a participle after the noun it
modifies, which had been leaving the head where it was and letting the next determiner open a
clause.

## A repair could invent a word, and did

Repair derived other forms of a word by inflection and never asked whether the result was a word.
Pointed at documentation it proposed ".es" for a full stop, "2es" for a numeral, and "Thes" for
"The": eighty-eight edits across twenty-eight files, every one of them damage. Three things were
wrong and each is now closed. A word made only of letters is the only thing inflection is offered.
Every candidate is put back to the lexicon before it is offered. And a closed class word has the
forms it has, because "a" gives "as" and "it" gives "its", which are real words and the wrong ones.
Repair is also now asked under the register the passage settled on, so a summary line is no longer
conjugated to give it the verb its register never wanted.

That took eighty-eight proposals to three. All three are still wrong, as are all ten on this crate,
so the write path is not fit to run unattended and `fix` writes nothing unless asked twice.

## Only a stated rewrite is written

Three passes over documentation now exist and they are not equally safe, which is the whole
finding. Repair derives a word: it is right about spelling and wrong about whether the word was
wanted, and on twenty-eight files of fitkit its six surviving proposals are six false alarms.
Condense selects: every cut it now makes is grammatical, but choosing which sentence carries the
point is not something fault counting can do, and the sentence it drops is sometimes the one worth
keeping. Neither is fit to run over a repository.

What is fit is the pass that writes down what it means. A phrase like "due to the fact that" has a
shorter equivalent that a person wrote next to it once and that holds wherever the phrase appears.
Nothing is derived, so nothing can be invented. Pointed at fitkit and at this crate it made
fifteen edits and every one was right.

Getting there took four corrections, and each was the same mistake. "rather" is a qualifier that
empties whatever it sits on, but "rather than" is a comparison and cutting the "rather" out of it
leaves a sentence missing a word. Now a qualifier has to have something a qualifier can attach to
before it is one, which protects every fixed pairing built on a qualifier and not just that one.
"there is" delays a subject, and the words to put in its place depend on a subject the opening
never named, so it was proposing "no allocation here" as a sentence. Now the rewrite is an
`Option`, `None` says a finding has no rewrite behind it, and a pass that writes files cannot
reach one. A worn noun is the same case: "rich tapestry" is worth reporting and what belongs there
instead is the writer's to choose. And "in relation to" is usually "about", except after a verb
that selected the preposition, where "stands in relation to" becomes "stands about"; where the
right wording depends on the verb, the entry states none.

The rule underneath all four: what a rewriting pass may write is exactly what somebody wrote down
for it, and the type says so. Every swap is also carried out and read back with both readers that
judged the original, so a swap that leaves more wrong than it found is never offered.

## It could not place the words it uses to talk about words

A crate about English writes about letters, and writing about a letter means naming it. Escaping
the quotes around a name is how a doc comment does that, and the escape was being dropped along
with emphasis and headings, on the grounds that a backslash is how a medium spells itself. What
that left was the letter standing bare in the sentence, so "y" was read as an English word and
reported as one nobody could place. An escaped mark is one delimiter written in two characters and
is now read as one, starting at the backslash, so what comes back out is the source text.

Placing it was still not enough. A quoted term was being held against the lexicon, which is asking
whether English lists a thing the writer has just named. It never will. A named term is placed as
a name and no longer counted against the reading, and with that the crate can place every word in
its own prose for the first time: nought, down from one, held now by equality rather than a bound.

## A determiner left with nothing to introduce

"Answers no to walk" was charged for a determiner with no noun after it, and "what follows the
gerund is its object" was charged for disagreement, because "what" was only ever a subordinator.
Both are the same thing: a word that usually introduces a noun phrase standing in for one. English
lets most determiners do this and the crate already had somewhere to say so, since "all", "some"
and "most" were listed as pronouns; the list was missing "no", "each", "either", "neither",
"this", "that", "these" and "those", and the plain "what" behind "whatever".

"which" and "who" are deliberately not there. They do the same job in a question, but a sentence
of prose that opens with "which" is nearly always a relative clause left standing on its own, and
reading it as a sentence stopped a padded comment from being cut down to its point. The reading
that costs more than it returns is not taken.

## It read a word it was talking about as a word it was using

A crate that writes about wording quotes the wording it is writing about, and the plain pass
proposed cutting "rather" out of the sentence explaining that "rather" is a qualifier. The name of
a word is not a use of it. A named term is now given a key no phrase and no qualifier can match,
so wording is judged on the words a sentence uses and not on the ones it names.

## What a cut returns is the text that was there

A condensation was rebuilt out of the tokens it kept, one space between each pair, which is not
how anything was written: `[`Cost`]` came back as "[ `Cost` ]". A cut is a narrower view of text
that already exists, so it now returns a slice of it and the spacing is whatever the writer typed.

Slicing exposed the other half. Trimmed at its opening bracket, "How a [`Cost`] is turned into"
leaves "`Cost`] is turned into", which reads as prose and has a bracket closing nothing. A cut may
not separate a delimiter from its partner, and counting the pairs in what would be kept says so
for every kind of pair at once, without a list of the constructions they appear in. Across both
repositories that is 315 condensations, and not one of them now opens something it does not close.

## Repair is measured, and it is still not right

Two more ways of inventing a word are closed. English does not put one inflection on top of
another, so "summed" is no longer offered "summeds"; and no English stem ends in a doubled
consonant, which is the tell that "summed" cannot be reduced to "summ" and that the answer must be
refused rather than guessed at. Repair also asks how the word was read before offering it another
form, because number is a feature of nouns, verbs and determiners and not of adjectives, adverbs
or prepositions: "pairwise", "finer" and "out" were being handed plurals they have no way to take.
The lexicon learned the comparative and the "-wise" adverb so those readings are available at all.

That is thirty-one proposals down to twenty-seven, and every one of the twenty-seven is a false
alarm: quoted grammatical examples, doc comment fragments, and noun phrases used as headings.
Precision on prose the engine has not been tuned against remains nought, so nothing was written.
The write path is exercised by tests and by the plain pass; repair reports and does not apply.

## What the passes do on prose that is actually bad

Every measurement above was taken on this repository and on fitkit, and both are written with
some care. That flatters two passes and hides what the third is for, so the same three passes
were run over a file of the padded, hedged doc comments a language model writes when it is asked
to document code.

Repair found four disagreements and three of them were real: "this function delete the user",
"the users is removed", "each of the check are performed". Applied, they went into the file and a
second pass had nothing left to say. The fourth was "developers who wish", and it is the reason
the relative pronoun below was fixed.

The plain pass found nine, and all nine were right: "due to the fact that" for "because", "at this
point in time" for "now", "prior to" for "before", "in order to" for "to", "leverage" for "use",
and "it is important to note that this" for "this". This is what the pass is for, and slop is
where it shows.

Condense proposed six. Two dropped exactly the sentence that carried nothing. The rest kept the
weaker of two sentences. Handed a summary and its restatement, it kept "each of the check are
performed in sequence" and dropped the sentence that named what was being validated. The limit
recorded above is unchanged and is not a matter of tuning. Fault cost measures whether a sentence
is well formed, and being well formed is not the same as carrying the point.

So the passes are not equally ready, and the difference is not one of polish. The plain pass
states its replacement, which is why it can be trusted to a file. Repair derives one, which is
safe when the prose is ordinary and unsafe when it is dense. Condense selects one, and selection
is the open problem.

## A relative pronoun has no number of its own

"Developers who wish to leverage it" was charged with disagreement and offered "wishes". The
lexicon had "who" as third person singular, which it is not: a relative pronoun takes the number
of the noun it stands for, and that noun is outside the clause the verb is in. Both readings are
now offered and the sentence settles which, exactly as existential "there" has always been
handled here.

This is the largest single correction measured so far. Across fitkit it took the repair pass from
twenty-seven proposals to three, because a relative clause is how most sentences in a doc comment
attach a description to a thing.

## Two joined gerunds may be one activity or two

"Cutting and rejoining a log is all the model needs" was charged for a singular verb after a
coordinated subject, on the rule that two subjects joined are plural. That rule is right for
nouns and wrong for gerund phrases: "cutting and rejoining are two operations" counts them and
"cutting and rejoining a log is" names one act, and both are ordinary English. Nothing inside the
sentence says which was meant.

A gerund subject is now held apart from a singular noun, because it is a clause doing a noun's
job, and joining two of them gives a subject that agrees either way and is charged for neither.
Joined nouns are untouched: "the dog and the cat is here" is still caught, and so is "running
tests are easy", where there is only one gerund and nothing to be ambiguous about.

The engine reads its own prose one fault better for this, and fitkit's remaining proposals fell
from three to two.

## What is still wrong, and what it means for running this on a repository

Two false alarms are left on fitkit and about twenty-five on this repository, which is the denser
of the two. They are no longer one root but several small ones. A gerund and its object are not
held together as one subject phrase, so "reading prose out of a file and reading it back in"
still loses its subject to "prose". Neither "let" nor "letting" is known to take a bare
infinitive, so "let go" is charged. A noun phrase used as a heading is asked to be a sentence.
A reduced relative with its "that" left out is not recognised.

An attempt was made to close the first of these by treating a second gerund before the verb as
joined to the first. It changed neither repository and it would have hidden a real fault in
"reading files using a parser is easy", so it was reverted. It is recorded here so it is not
tried again without the missing piece, which is a gerund phrase that keeps its object.

In practice: point the plain pass at a repository and allow it to write. Point repair at a
repository, read the diff, and expect it to be right on ordinary prose and wrong on prose
that is mostly quoted code, headings and grammatical examples. Do not let condense write at all.

## A word that ends in an inflection's letters without carrying the inflection

An unseen repository asked the engine to read "the value must exceed the limit". It charged the
modal for having no plain form after it, and the repair sent to mend it offered "exce".

The lexicon read any word ending in "ed" as a preterite and, having placed it, withheld the plain
form. "need", "exceed", "speed" and "proceed" end in those letters without carrying the
inflection, so each was denied the reading it actually had.

English adds "d" to a stem that already ends in "e" and "ed" to every other stem. So a word whose
"ed" comes off to leave an "e" was never spelled by that rule: "ne" would give "ned", not "need".
The letters belong to the stem, and the word now keeps its plain form as well. The preterite is
still offered, because "freed" really is one, and the sentence chooses. This is the rule the
lexicon already used for "chorus" against a plural.

The engine reads the prose in this repository exactly as it did before, so the fix cost nothing
it was getting right.

## What licenses taking an ending off

Removing an inflection invents a stem, where adding one derives a form from a word already in
hand. Offered on spelling alone over an unseen repository, the strip was wrong nineteen times out
of nineteen: "defin", "hydrat", "dissolv", "measur", "stat", "ag" and "ne". Its only two real
words were correct participles it would have broken.

Two things now have to hold. A rule that asks for the plain form must have been broken there, so
the sentence is short of a form rather than merely holding a word that ends in those letters. And
the word must have been read as a participle or a preterite. Both are answered by what the engine
already settled rather than by the spelling. "He must walked" is repaired to "he must walk", and
the nineteen inventions are gone.

## Speed

The engine took about eight tenths of a second on every sentence, whatever the sentence said, and
about twenty-five minutes on a repository of a hundred thousand words. Nearly every second went
into the decode, which laid out a successor list for every state in the grid before it started and
then swept them again at every word. A sentence reaches very few of them.

The decode now asks for a successor list one state at a time, prices a state only when something
still in play can reach it, and carries the live states forward. Backpointers are kept for the
live states of each step rather than for the whole grid. This is fixed in fitkit, so any model
built on it gains the same.

A sentence now costs about five thousandths of a second. That repository takes eleven seconds,
this one takes three, and the self-audit that ran for five to thirteen minutes now takes four
seconds. Every reading is byte for byte what it was, which is how the change was checked.

## The head of a noun phrase is its last noun

An unseen repository wrote "with inversion where the volume fractions are equal", and the engine
charged the plural verb and sent a repair offering "is".

The clause fixed its subject on the first noun it read and never revised it, so "the volume" was
what "are" had to answer to. English puts the head of a noun phrase last: "the volume fractions"
is about fractions, which is the same reading the rule against a plural modifier already takes,
since it charges "dog books" for the modifier and leaves the head alone. A noun standing straight
after the noun the phrase is headed by now takes the head over. A determiner, a numeral or a
pronoun still begins a phrase of its own, so "the conventions a passage holds to" is unchanged.

Eight false alarms went from the unseen repository and the engine reads its own prose exactly as
it did.

## Respelling a word the lexicon can already read

A repair offered "citru" for "citrus" in "all ten citrus fruits". Putting the ending back spells
the word again, so the round trip that catches "categorys" says nothing here.

What settles it is that the lexicon already reads "citrus" as a singular, because English does not
spell a plural onto a stem that already ends in a vowel letter. A rule asking for a singular and a
word that already offers one disagree about the reading, not about the spelling, so there is
nothing to respell and the repair is declined. The word "dogs" offers no singular reading of
its own, so it still reaches "dog".

## What the passes are worth on an unseen repository

Measured over a hundred and forty prose bearing files, about a hundred thousand words.

The plain pass proposed twenty cuts and every one of them was right: "actually", "simply", "very"
and "a majority of", each removed from a sentence that says the same thing without it. That pass
is safe to let write.

Repair proposed thirty eight swaps and one of them was right, which is a precision of about one in
forty. Nine were doc comment summaries, which are noun phrases by convention and were asked to be
sentences. Four were words read as nouns that are not nouns, so an invented plural passed a test
that only asks what a word looks like. The remaining twenty four were agreement, where the engine
had settled on a subject the writer did not mean. None of them invented a word, which the two
earlier sections are what fixed, but a swap that is wrong is still wrong.

The plain pass can be run and allowed to write. Repair is worth running for its diff alone, and
it should not be allowed to write unattended on prose this dense in code, headings and quoted
identifiers. Condense should not be allowed to write at all.

## Speed on a book

Ninety thousand words of ordinary prose, six thousand sentences: twenty six seconds to read, thirty
nine to read and find repairs, forty five to read and find cuts. The unseen repository takes eleven
seconds for the same work, because a source file is mostly code.

## Writing doc comments from the code

This pass no longer exists. What is recorded below was measured while it did, and it is kept
because the limits it ran into are the reason the engine that replaced it is built the way it is.
It wrote a doc comment by choosing among sentence shapes held in its own source, so what it could
say was bounded by what somebody had thought to write down there. `describe` holds no shapes at
all: it is given a corpus and a set of claims, and every word it writes is one the input used.
The section after this one records what that cost and what it bought.

The `document` pass read a Rust file with `syn`, gathers what the code proves about each public
item, and writes a doc comment for the few items whose code proves something the declaration does
not already show. It never reads the code's meaning.
It reads the signature and the shape of the body. Every finding carries a price that says how sure
it is. A signature states its facts outright and they are cheap. A fact that holds on every path
through a body costs more. A fact that holds on only some paths costs most. A line is written when
its price plus its length comes in under what saying nothing costs.

Nothing about the code is written into the engine. There is no table of phrasings and no list of
verbs. A name is split into its words, the signature says whether the call does something or
names something, and English decides the rest.

Two things bound what it can say wrong.

The grammar is the first of them. Every sentence this pass writes goes back through the same engine
that reads the repository, and the engine throws away any sentence it faults. Adding a rule to
the grammar therefore tightens what may be generated without a line changing here. Read back over
the unseen repository, what it writes carries 0 faults and 0 unknown words.

The edit is the second of them. This pass can express one edit and no other, which is to insert a run
of `///` lines at the start of a line. Touching code is not something it declines to do. Touching
code is something it has no way to say.
Run against book-cook-ai's 841 files it wrote 1767 lines across 274 files, and `git diff` reported
1767 added and 0 deleted, every added line a doc comment. The repository still built.

## What the pass decides from the signature rather than the word

English lets almost any short word be a plain verb, so the word cannot say whether a name
describes an act or a thing. `stability(&self) -> f64` is read as a thing because the signature
gives an answer back and takes nothing, which is why it reads "The stability." and not
"Stabilities."

Which word carries the act is a separate question, and the ending cannot answer it either. Every
word ending in "s" is offered a third person reading by its shape, which is how a name beginning
"aqueous" once wrote "Aqueous the sucrose dielectric loss factor". A third person reading is
taken only when the word left after the ending is itself a verb, which "hold" is and "aqueou" is
not.

## What a name's number can and cannot be judged against

The `--names` pass reports a name whose number disagrees with its type: `assay: &[&str; 7]` is
written for one thing and holds seven, and `remained_liquid_at_lowest_temperatures: bool` is
written for many and holds one.

Two kinds of name were dropped from this after their findings were read by hand.

A number type is not asked about at all. A number can count many things or measure many units, so
`water_determinations: u8` and `duration_minutes` in a float are both good English over one
number.

A word the lexicon offers in both numbers is not asked about either, because English spells it
the same either way and `species: Vec<_>` is right as it stands.

Of a sample of twenty findings read by hand after both changes, nineteen were real. On
book-cook-ai the pass reports 748 across 4541 items.

## Speed of the doc pass

5010 items across 841 files in 5.2 seconds, including generating and reading back every comment
considered before all but four were left.
The grammar is the slow part of this repository and the doc pass only reads the short sentences
it writes, so it costs far less than a pass over prose.

## Nothing is written that the declaration already says

A comment a reader could have written from the line under it is worse than no comment. It costs a
reading and returns what was already there, and it cannot be told apart from a comment that was
checked. So a sentence has to earn its place by saying what the declaration does not, and most
cannot.

A summary is written from the name and from nothing else: the words are the name's words, put in
an order English can read. So it can never carry anything new, and it is offered to the search
having earned nothing. That is where "The acid system." above a thing called `AcidSystem` went.

A finding earns its place by where it was found, which the price already records. A signature is
read straight off the declaration the comment would sit above, so a sentence carrying it tells a
reader what they are looking at. A body has to be opened and followed, so a sentence carrying it
saves the reading it would have taken. Only the second kind is written.

The effect is severe and it is the point. On the unseen repository this fell from 1948 comments
over 5010 items to 3. Every one of the three says the same thing, that the item can stop the
program, and every one of them is a thing a caller has to know and cannot see. The rest were the
name spelled out again, or the return type spelled out again, above a declaration that already
said both.

What this measures is how little of what the pass knows is worth writing down, not how little it
knows. Widening it means finding more in bodies, because that is the only place a reader is not
already looking.

## A stop is only worth reporting where nothing else can be

A call answering with a failure or with nothing has somewhere to put what went wrong, and a reader
of the declaration already expects it. A stop written inside such a call is the author saying that
a case cannot arise, guarded somewhere they could see. A call answering with a plain value has no
such channel, so a stop in it is the whole of how it fails.

Only the second is written. Read by hand, the four first reported were two of each. Both of the
suppressed ones proved to be assertions that cannot fire: one converts a slice of exactly eight
bytes into an array of eight, and the other takes the last of a list a validator has already
refused to leave empty. Both return a `Result` that reports every failure that can happen. Both
comments would have been false.

The debug forms are deliberately not counted, since a release build compiles them away and a
comment saying otherwise would be wrong where it matters. Nothing is said about arithmetic that
overflows, an index out of range, or a division by zero, because those stop without being written
down anywhere the pass can see.

Whether a stop can be reached is never asked. A stop guarded by a check the pass cannot follow
still reads as a stop, which is the safe direction to be wrong in for a warning and the reason the
signature test above matters: it removes the cases where a reader had a better warning already.

## Carrying a comment is not the same as having said this

The pass used to skip any item that already had a doc comment, which sounds safe and hid the thing
it was built to find. Counted with a separate reader over one repository, of six hundred and
seventy-one public functions, fifteen can stop. Two were undocumented and both stop on a bare
unwrap, so there was nothing to report. Three had been given a section by this pass. The other ten
already had a comment, and not one of those comments mentioned that the function can stop at all.
Skipping them left the reader with prose that reads as though nothing can go wrong.

So a section the author never opened is now added below what they wrote, and every word they wrote
is left alone. A section they did open is theirs. A sentence with no heading has nowhere to go but
the summary, which they have already written, so it is left. Recall on that repository went from
three to seven.

What `expect` is handed is turned around before it is reported. A message given to `panic!`
describes the failure, so it is quoted as the failure. A message given to `expect` describes what
the caller was relying on, because the language asks for the reason the value is expected to be
there, so quoting it as the failure states the condition backwards. It is reported as the thing
that has to hold. That turn costs something: the message is then prose rather than a quoted span,
so it is graded, and one true warning was dropped for reading badly. The rule is the same as
everywhere else here: a sentence is written right, or it is not written.

## A stop is reported with the cause, or not at all

Saying that a call can stop tells a caller nothing they can act on. They cannot see which argument
to check, and the sentence takes a reading to arrive nowhere. What a caller needs is the cause, and
in the two places a stop is written the cause is already there in the source: an assertion carries
its check, and a stop written by hand usually carries a message. Both are literal tokens, so
repeating them invents nothing.

So the cause is read out and quoted:

```text
assert_eq!(values.len(), n * n)
    -> Panics unless `values.len()` and `n * n` are equal.

panic!("unsupported proteolysis state: {}", error.0)
    -> Panics with `unsupported proteolysis state`.
```

A message is cut where the first value is put into it, because what comes before that is the part
the author wrote. It goes under a `# Panics` heading, which is where a reader of Rust looks and the
only place `clippy::missing_panics_doc` accepts.

Where the code gives no cause, nothing is written. A bare unwrap is the case: it can stop, and it
says nothing about when. Under the old rule it earned a comment, and that comment was the worst
one the pass produced.

Two things follow from reading the cause rather than the name. Bodies are now walked as trees
instead of read as text, so a stop is found wherever it is written: in the tail of a chain, or in
the one line of a closure handed to another call. Both of the stops that survive on the food
repository are of that second kind. Walking also settles by construction the case a text search
had to be patched for, of a name written inside a longer one.

One number moved with it. A word used to be charged outright, which read as a comparison and acted
as a length limit: a true sentence naming its cause runs eight words, and eight words plus the
price of a body fact came to exactly the value of saying something, so the search left it and wrote
nothing. A word is now charged against the shortest rival saying the same thing, which is what the
charge was always described as doing. Length now separates two ways of saying one thing and can no
longer silence the only sentence that says it.

## What is quoted from the code is not graded as prose

Every sentence the pass writes is read back by the engine and thrown away if the engine faults it.
A sentence naming a cause quotes the code, and code is not English. A bound written against the
largest number a machine word holds is four words to a tokeniser, and a fault to every rule that
expects them to agree:

```text
Panics unless `n < std::usize::MAX` holds.
```

Graded whole, that sentence was dropped, and it was true and the only warning its caller had.

So a span between backticks is stood in for by one word while the sentence is read, and written out
untouched. The frame around the span is still held to the whole of English, which is the part that
can be wrong. This narrows the guarantee and is worth saying plainly: the engine no longer vouches
for what is inside a span. What vouches for that is Rust. What is written back is handed to the
parser again in a test, and seventeen checks covering calls, indexing, casts, references, macros,
unary signs and precedence all come back as the expression they went in as.

Three faults were found by looking for them rather than by a gate. `expect` called with nothing
handed to it parses, and reaching for its first argument stopped the pass outright. A message was
cut at an empty pair of braces, so a value put in by name or with a format of its own was left in
the comment. And a sign written after another operator was spaced as though it joined two things:

```text
a == -1    ->    a == - 1
```

Each is now a test. The gates were green through all three,
because a gate compares the pass against itself and none of these was a disagreement with itself.

## Writing about an input with nothing written down to say

`describe` replaced both passes above. It is handed a corpus and a set of claims and nothing else,
so it cannot know whether it was given a repository, an encyclopedia entry or a novel, and the
same binary was run over all three without being changed.

Two searches, both from fitkit. A subset search over the claims decides which parts of the input
are worth stating; a path search over the input's own words decides the wording of each clause.
A clause is a subsequence of the passage it is about, so it can leave words out and splice one
passage to another but can never return to a place it has used, which is what makes repetition
unrepresentable rather than merely expensive.

Three things had to be learned rather than declared, and each was found by reading output that was
wrong. Where a sentence ends: the mark is attached to the word before it, is followed by a capital
more often than this text's own rate of capitals, and finishes a passage more often than chance
would put it there. All three are needed. Two of them admit the backtick that wraps a type name in
Rust documentation, and the engine then wrote passages made entirely of backticks. Whether a given
occurrence of that mark ends a sentence is decided per occurrence by what follows it, because the
dot in `0.5` is the same character as the one ending this sentence, and counting decimals as
sentence ends put the typical sentence at six words.

Length is not optimised, it is terminated. A model that scores a sentence by its probability always
prefers the empty one, and a model that pays per word always prefers the longest. Silence here is
reachable only through the mark this text ends sentences with, and the last step of the search
admits nothing else, so where a clause stops is a decision the search made.

The engine was rebuilt once more, and the reason is worth recording because it was a mistake about
what the job is. A clause used to be assembled: a path through the places of the input where words
bearing on a part had been read, free to leave words out and to splice one passage to another, as
long as it never went backwards. Every word cited the span it came from and the result was about
the right subject, so every test passed. It also could not be read. A sentence is not a bag of
characteristic words in a likely order, and the tests could not see the difference because none of
them asked whether the output was a sentence.

Shortening a real sentence was the next attempt, and it is the more instructive failure. A deletion
was allowed only where the input itself had written the two words that would then meet, which
sounds like the text vouching for the join. It vouches for the join and not for the sentence: what
came out was *Collins, my cousin Lydia's sad business has been concerned that their marriage took
place should be known*, every join attested and the whole thing nonsense. An author's words rear-
ranged are the author's words in name only.

What is left is the only honest thing available to something that may not write a word of its own:
find the sentence the input already wrote about the part, and report it whole. Everything the
engine decides is now a decision about selection, which is a decision it can support with evidence,
and nothing it decides is a decision about language, which it has no standing to make.

Three smaller things had to be learned to make that work, all of them by reading output that was
wrong. A mark a word is written with on every occasion it appears is part of the spelling, so
a title written before a name does not end a sentence — with the run required to be longer than
chance would produce, because a word seen twice which ends both times has shown nothing. A paragraph break in a doc
comment is a break the author made, so a heading no longer runs into the sentence beneath it. And a
sentence written in more marks and lone characters than half this text's sentences use is a table
row, a line of mathematics or a bibliography entry rather than prose. The comparison is against the
middle sentence and not the average token, because a text full of mathematics has an average its
own prose never reaches.

What this costs is worth stating plainly. The engine cannot condense. It cannot write a sentence
that says what two sentences said, it cannot report a count, and where an author wrote nothing
usable about a part it says nothing about that part. What it buys is that no line it has ever
emitted was written by it.

Measured: fitkit, 0.12 seconds. A TypeScript application, 0.04. An encyclopedia article, 0.02. A
novel of seven hundred and thirty thousand characters, 0.15. Across all four, every line appears
verbatim in the input, and no two words appear beside each other that the input did not write
beside each other. The condition was
thirty seconds. One number in the old pass was wrong by a factor a reader would notice: asking for
the exact best subset of fifty four claims is two to the fifty four combinations, and the search
sat there. It is asked for the exact answer up to twenty claims and a beam beyond, and it reports
which of the two it used.

## Reporting comments that say nothing

The same question, asked of comments someone already wrote. This pass no longer exists either; the
paragraph below is what was measured while it did. `document --noise` reported a doc
comment whose every word carrying a point is a word of the name, of an argument, or of the type
answered with. Endings that only mark number or person come off both sides first, so a comment
that calls a thing what it is called says it again however the sentence had to inflect it.

Nothing is deleted. The pass can be sure it found no word of the author's in a comment, which is a
reason to look and not a verdict, and a comment belongs to whoever wrote it.

The test is deliberately generous, since one word of the author's own is enough to keep a comment.
That is the safe direction to be wrong in: it will miss padding, and it will not tell anyone to
delete something that says something. Over 4271 doc comment lines on the unseen repository it
reports none, and over a file written to be redundant it reports every line.

## Names the doc pass still reads as acts when they are not

Whether a name describes an act is settled by the signature, but which word carries the act is
settled by the lexicon, and the lexicon allows a plain verb reading to almost any word by shape.
That is right in a sentence, where the words around it settle the question, and wrong in a name,
where there are none.

Two of the three ways this went wrong are closed. A word already carrying a third person ending is
only read as a verb when the word left after that ending is one the lexicon lists, which stops
"Grams the sample" and "Aqueous the sucrose dielectric loss factor" without stopping "holds". A
closed-class word is never the act of a name, which stops "Withs".

The third is open. A name beginning with an unlisted noun or adjective is still allowed a verb
reading by shape, so "new", "finite", "sodium" and "eugenol" are inflected as though they were
verbs. Measured over 1948 comments on an unseen repository, 479 are led by a verb and 46 of those
are this mistake, which is 2.4% of the comments written.

It is left open because no evidence separates the two cases. The lexicon ranks "parse" and
"sodium" alike, both are absent from the listing, and both read clean as a noun phrase and as an
act, so the grammar cannot choose between them either. Requiring the listing would lose
"Evaluates", "Records" and "Parses" along with the mistakes, and requiring a verb-forming ending
would keep "Speciates" and "Hydrolyzes" but still lose the rest. A rule that costs more than it
saves is not an improvement.

## Three defects a green suite could not see

Running the whole tool over an unseen repository to write a report found three faults that every
test had passed over.

Output piped to `head` ended the run in a crash. When the reader closes the pipe the next write
fails, and the print macros treat a failed write as a fatal error. A reader who has stopped
reading is the ordinary
end of a report, so writing now goes through one place that exits quietly instead.

A singular noun ending in "s" was read as a plural. Number is derived from spelling, and stripping
the "s" from "gas" gives "ga", whose plural spells "gas" again, so the round trip licensed it. It
made `The gas is hot.` carry a subject-verb fault and a field named `needs_gas` a naming finding.
These words are a closed class in English, so they are listed.

An adverb was given a plural: "correctly" became "correctlies" and "everywhere" became
"everywheres". English does not inflect an adverb for number, and the rule is no longer asked for
a word the lexicon reads only as one.

None of the three was reachable from the crate's own tests, because a test asks the engine whether
it agrees with itself. Reading real output is what found all three.
