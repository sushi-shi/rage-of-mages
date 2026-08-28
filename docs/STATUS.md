# Status — Rage of Mages

Living record of what is recovered and verified. Newest first.

## Phase 0 — resource-free foundation (scaffolded)

- Repo initialized resource-free (R1): CC0 `LICENSE`, `.gitignore` written first
  (matches `_originals`/`_reference`/`target` by name, no trailing-slash globs).
- `java/reconstruction/builds.toml` — the provenance authority — auto-generated
  from the resources dir: every unique JAR payload deduped by sha256 (top-level +
  nested-in-zip), with byte length, aliases, containers, MIDlet/device metadata,
  and a 2D/3D probe. Judgment fields (official/archived, true language, baseline)
  flagged for Phase 1.
- `tools/originals/{verify,fetch,gen_builds}.py`: the `originals-verify` gate is
  **proven able to fail** (`just originals-verify-canfail`).
- Fork: **2d** (see the j2me home's `docs/FORK_2D_3D.md`).

## Difficulty tier & recommended first phases

**Tier 1 (smallest surface).** 2D, ~31–37 classes, vendor **Nival Interactive**.
This is ONE game across four builds — the RU "Allods" release (128×160, 176×220,
240×320) and the EN "Rage of Mages" repack — confirmed the same codebase by a
bytecode/resource fingerprint (identical obfuscated class-name set + byte-identical
`res/*.map` level packs; see the merge note in `builds.toml`). The axis is
resolution × region/title. Custom packs: `.map` (level data, shared across builds),
`.utf` (localization — the RU/EN difference), `.pak`, `.int`.

Recommended next: **Phase 1** — classify (class/method
fingerprints across the unique payloads, cross-build deltas, lineage/language
reconciliation, pick baseline + naming reference) and format triage (a
malformed-input-rejecting parser per custom format, tested on every blob).

## Open decisions (from the playbook)

1. Baseline vs naming-reference build (decide from Phase 1 fingerprints).
2. True per-build shipped language (decode the string tables; filenames lie).
3. Custom-format semantics (reverse from consumers in Phase 1).
4. Reference oracle source for Phase 3 (FreeJ2ME-Plus capture vs a headless-JVM
   MIDP host vs the decompiled Java).
