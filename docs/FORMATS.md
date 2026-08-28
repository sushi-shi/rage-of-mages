# Original resource formats — Rage of Mages

Status: **Phase 1 pending.** Every custom format below gets a Rust parser in the
`*-formats` crate that **rejects malformed/truncated input with a typed error
without panicking** and is exercised against **every applicable unique blob**
(playbook R1/R2/R3/R10). Decode each format from its **consumer** (the bytecode
that reads it), not by guessing; state each multi-byte field's endianness.

## Language classification (declared vs. actual) — R10/R11

Fill in during Phase 1. The filename and the internal language code both lie in
the sibling ports; the **authoritative** key is the decoded string-table content.

| build | declared (filename/manifest) | actual (decoded) | text container flag |
|---|---|---|---|
| _TODO_ | | | |

## Formats present in the corpus

Populate from `catalog_resources.py` (Phase 1). For each: role, endianness,
layout, the consumer that reads it, and the unique blobs it covers. Model any
text container from **evidence** (encoded-string table vs glyph indices — a
per-format finding, not an assumption — R11).

- `.png` — standard sprites/tiles/UI; slot numbering maps to a role.
- `.mid` / `.wav` — audio; playback via the host's MMAPI model (R9).
- _custom packs_ — _TODO: list the game's own extensions and reverse each._
