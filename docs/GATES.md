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
