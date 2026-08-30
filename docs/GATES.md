# Gates and the can-fail discipline (R3) — Rage of Mages

> No gate is trusted until it has been shown to go red on an injected defect.

The full discipline (the can-fail rule, the four vacuous-gate shapes, the
independent two-implementation oracle pattern, and the anti-bog protocol) lives
in the j2me home's `docs/GATES.md`. This file is this game's live gate ledger:
every gate the project has today, with its command and its can-fail proof. Add a
row when you add a gate.

## Current gates

| Gate | Command | Can-fail proof |
| --- | --- | --- |
| Originals provenance | `just originals-verify` | `just originals-verify-canfail` (proven RED on a one-byte payload corruption) |
| Formats corpus + oracles | `just formats-corpus` | in-test negative controls reject one-unit-perturbed blobs (a `.map` one byte short/long, a `.utf` record over-running EOF, a `.pak` length that breaks the total invariant, an odd-length `.int`, a `scenes` blob cut mid-scene) |
| R8 numeric-shape authority | `just numeric-shape` | `just numeric-shape-canfail` (proven RED: flips one recorded opcode — idiv→fdiv — in memory and the byte-identical check names the drifted method) |
| R10 symbols ledger | `just symbols` | `just symbols-canfail` (proven RED: injects a non-existent `(obf, descriptor)` member into a real class and the resolution check rejects it) |
| Transliteration first frame | `just first-frame` | in-test controls fail the same `is_real_frame` predicate: a blank (all-white) framebuffer, a uniform non-white fill, and the t≈0 fade-cover frame from the SAME pipeline (state-sensitivity); the logo box is additionally pinned pixel-exact against the independent `rage_formats::pak` + PNG oracle; a missing `_originals` corpus panics loudly (rule 4), never skips |

## Rules (restated)

1. Every gate ships with a can-fail proof (`--self-test` / an in-test negative
   control), proven RED by a one-unit perturbation you then reverse (never
   `git checkout`).
2. Ban the four vacuous shapes: comparing against a quantity the tool never
   returns; an assertion whose subject can vanish while it holds; a skip that
   reads as a pass; a ratio of a set against itself.
3. Build the quantity you assert on yourself (pixel masks, sample counts) — never
   parse an image/audio tool's stdout numerically.
4. Corpus-dependent tests fail loudly when `_originals` is absent — never skip to
   green.
5. Retired/ignored tests carry an honest header and run by a named target.
