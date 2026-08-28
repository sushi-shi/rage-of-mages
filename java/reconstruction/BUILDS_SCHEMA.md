# `builds.toml` schema — the provenance authority (playbook R2)

`builds.toml` is the **single tracked owner** of build provenance. The binary
payloads themselves are never committed — they live in the private resources
location and are materialized into git-ignored `_originals/` by `fetch.py`.
`verify.py` reconciles every materialized payload against the `sha256` + `bytes`
table here.

`gen_builds.py` fills in the **mechanical** fields automatically; a human
reconciles the **judgment** fields in Phase 1 (marked *TODO* / *HINT* below).

## Top-level keys

| key | meaning |
|---|---|
| `title` | human title of the game |
| `slug` | repo slug |
| `fork` | `"2d"` (Graphics2D) or `"3d"` (M3G) — from the M3G probe |
| `baseline` | *TODO Phase 1* — id of the quality/completeness baseline build |
| `naming_reference` | *TODO Phase 1* — id of the build whose text justifies semantic names |

## `[[payload]]` — one row per UNIQUE payload (deduped by sha256)

A payload is a single JAR (`kind = "jar"`) or JAD (`kind = "jad"`). The **same**
payload appearing under several names / in several containers is **one row** with
multiple `collected_as` + `containers` entries (meaningful aliases, kept).

| field | kind | meaning |
|---|---|---|
| `id` | mechanical | human-readable, uniqueness-gated identifier — **never** a hash slug that can alias (R2) |
| `sha256` | mechanical | the payload's content hash — **identity is this, never the filename** |
| `bytes` | mechanical | exact byte length (verified) |
| `kind` | mechanical | `"jar"` or `"jad"` |
| `is_3d` | mechanical | M3G probe result for this payload |
| `class_count` | mechanical | number of `.class` entries |
| `midlet_name` / `midlet_version` / `vendor` / `cldc` | mechanical | from `MANIFEST.MF` (strings, may lie about language) |
| `resolution` / `device` | mechanical | derived from the filename (a hint; confirm from canvas dims) |
| `declared_language` | **HINT** | from the filename/manifest — **verify from decoded content, never trust it** (R10) |
| `official` | **TODO** | `true` = official recovery target; set `false` and move to `[[archived]]` if a fan repack |
| `repack_tag` | mechanical | a detected fan-repack signature (`by X`, a modder domain) — a hint to archive |
| `lineage` | **TODO Phase 1** | code family (device/vendor port, version family, language fork) from fingerprints |
| `collected_as` | mechanical | every original distribution name this payload was seen under |
| `containers` | mechanical | `_originals/<file>` (top-level) and/or `inside _originals/<zip>` (nested) |
| `companion_jad` / `companion_of` | mechanical | link a JAR to its JAD and vice-versa |
| `notes` | curatorial | aliases, evidence, anything a reviewer must know |

## `[[archived]]` — same schema, for fan repacks/mods

Preserved and verified (evidence is never discarded), but **not** recovery
targets. Move a `[[payload]]` here once Phase 1 confirms it is unofficial. `verify.py`
checks these too.

## `[[container]]` — immutable zip wrappers

| field | meaning |
|---|---|
| `id`, `sha256`, `bytes` | identity of the zip |
| `collected_as` | the zip's distribution name |
| `holds` | sha256 of every member (jars, jads, loose files) — the provenance link |

## Rules (R2)

- Never modify the contents of a surviving archive. Filenames are curatorial;
  contents are immutable. **Identity is the `sha256`**, never the id or filename.
- Delete only a cryptographically proven accidental duplicate; **keep meaningful
  aliases** (same payload under two names) and document them.
- Assign a semantic field (`official`, `lineage`, true language, `baseline`) only
  when bytecode / decoded content / cross-build evidence supports it.
