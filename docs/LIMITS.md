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
evidence, so a very short passage is read under whatever convention set is cheapest and that may
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
