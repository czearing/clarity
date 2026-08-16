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
