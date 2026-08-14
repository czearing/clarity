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
