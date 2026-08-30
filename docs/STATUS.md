# Status — Rage of Mages

Living record of what is recovered and verified. Newest first.

## Migration onto the shared `j2me-*` crates (2026-08-29)

The reusable, game-neutral layers were promoted out of this repo into the public
[`j2me-preservation-kit`](https://github.com/sushi-shi/j2me-preservation-kit).
rage-of-mages now **depends on full commit
`7aeba277a270eb58658d211bd3621b30956f6212`** instead of carrying
gothic-derived copies or reaching into a sibling checkout:

- **Deleted** `transliteration/crates/rage-jvm`, `transliteration/crates/rage-me`,
  `crates/rage-canvas`. The workspace is now just the game delta:
  `crates/rage-formats` + `transliteration/crates/rage-game-xlat`.
- **Git dependencies**, all locked to that same reviewed commit:
  - `j2me-jvm` (arithmetic: `i32_div`/`i32_shl`/… — renamed from `java_div`/`ishl`),
    `j2me-canvas` (ARGB `Image`, now fallible constructors), `j2me-me` (device
    runtime: Graphics/Canvas/media/rms + the `image` PNG-decode factories),
    `j2me-nokia` (the Nokia `DirectGraphics` sprite path — 1:1 with the old
    `rage-me::direct`), `j2me-codec` (`rage-formats`' `Reader` is now a thin
    adapter over it; the five parsers + `corpus_oracle` unchanged).
- **Added to the public kit** (the two pieces the shared layer was missing for a
  Nokia game): `j2me-nokia` (opt-in vendor crate) + the `plot`/`read` primitive on
  `j2me-me::Graphics` it builds on. All five `j2me-*` crates are crates.io-ready
  (metadata, version-ized deps, READMEs, `PUBLISHING.md`; `cargo package` green).
- Game-specific bits kept in-repo: the RoM resource seam (`rage_game_xlat::resource`),
  the closed key-code handling, and the format parsers.

**Verified:** the first-frame pixel oracle stays **exact** across the swap (it
renders through the same `j2me-canvas::source_over` it checks against), and the
full suite is green — 61 tests, `just first-frame` + `just check` EXIT 0. Behavior
is unchanged; this was a dependency/API swap, not a redesign.

## Phase 3 — strict transliteration (started 2026-08-28)

**Device runtime `rage-me` — built.** The Java ME / Nokia layer the transliteration
sits on is modeled exactly to `docs/DEVICE_RUNTIME.md` (reject the rest, R10), over
`rage-canvas`:
- `image` (`createImage(w,h)` + PNG-decode `createImage(byte[])`), `graphics`
  (color, **clip = replace** not intersect, translate, fillRect/drawRect/drawLine,
  anchored `drawImage`, approximate arcs, a non-gameplay `drawString` placeholder —
  no `Font` type, matching the zero-`Font` inventory),
- `direct` — Nokia `DirectGraphics`, the central sprite path: `drawPixels` for the
  closed pixel-format set `{ARGB4444, ARGB8888}` (nibble-replication ↔ high-nibble
  conversions, exact round-trip), `getPixels`, `FLIP_HORIZONTAL`,
- `canvas` — the R9 serialized paint/input queue (coalesced repaint, owed `Paint`
  polled **before** any key, synchronous `serviceRepaints`), event-driven raw
  key codes + `getGameAction`, the closed device-code accept set, `setCurrent`
  attach-once, full-redraw contract.
- `media`/`rms` are **stubs** carrying the §4/§5 contracts (single-channel MIDI,
  offset-binary-LE save wire) as `TODO(phase3)` — the focused next follow-up.

41 `rage-me` unit tests, each with a can-fail control (pixel-format wrong-expansion,
anchor placement, disjoint-clip replace, paint-before-key, out-of-vocabulary key
rejection, flip mirror; plus `should_panic` gates).

**`rage-game-xlat` — first rendered frame reached (milestone 1 of 4).** The
transliteration crate exists (impl #1: one `Game` struct of ~115 Java statics with
ownership collapses documented; every ported method a free function citing its
`// class.method`). The boot path — `GameMIDlet.<init>` → `m.<init>` (gradient bake,
resource binds) → `m.run` prologue → `m.paint`/`m.c(G)` — renders the shipped **Nival
logo** frame (res0.pak entry 0 over white, with the long-typed ARGB4444 fade-in),
loaded by the transliteration's OWN bytecode-reversed loaders (class `j` XOR-pak,
`ae` readUTF, `o` sincos), not by `rage-formats`. Gate `just first-frame`: a
`FrameStats` content assertion PLUS a two-implementation **pixel oracle** — the logo
box must equal `rage_formats::pak` (the independent decoder, dev-dep) entry 0
composited over white, zero mismatches (R12) — with blank / uniform / t≈0-fade-cover
can-fail controls, and a boot-state cross-check (sin table bit-equal to the
`int_table` oracle, font descriptor bound, pak closed). Every arithmetic-bearing
method audited against `_reference/numeric-shapes.json` (R8) and cited.
Heavy-but-first-frame-irrelevant init is stubbed as loud `todo!` seams or faithful
catch-arms (MMAPI player creation = the shipped `catch(Exception){}`).

**Seam / next slice:** the tick after ~5000 ms logo dwell (or any key) hits `m.d()`
case 1 — the logo→menu transition (`m.Q()`, `m.g()`, class-`g` sprite-pack loads,
`f215b` bakes, `f147b.b()` → main menu), then the screen-stack paint/update and the
`ae` glyph renderer. Milestones remaining: first in-game frame → the player moves →
native window plays it. `docs/SEMANTICS.md` supplies the tile/scene/save semantics
those slices need.

## Phase 2 — decompile + naming + device-runtime inventory (2026-08-28)

**Reviewed-naming + R8 gates — built.** The baseline bytecode is now guarded by
two proven-can-fail gates (`docs/GATES.md`):
- `just numeric-shape` — the R8 arithmetic authority: 37 classes / 437 methods /
  4427 numeric+conversion opcodes captured to git-ignored
  `_reference/numeric-shapes.json` (byte-identical regen; `--self-test` catches a
  one-opcode drift). Every method's Rust transliteration is checked against this.
- `just symbols` — the R10 naming ledger `java/reconstruction/symbols.toml`: 78
  members across 10 classes, all resolving to real `(obf, descriptor)` pairs in
  the bytecode, roles evidence-cited (`m`=loop/paint/save/map+scene load, `o`=trig,
  `ae`=text/font, `j`=pak/mid loader, `g`=screen dispatch, `aj`=common.utf,
  `q`=abilities, `n`=entity, `ab`=stat record, `Container/GameMIDlet`=MIDlet);
  inferred overload picks flagged `medium`, uncertain members omitted not guessed.

**Device-runtime inventory.**
The baseline `allods_176x220` is decompiled (JADX + CFR, git-ignored under
`_reference/decompile/176x220/`; regenerate with `tools/java/decompile.py`). The
**device-runtime API surface** the baseline actually uses is inventoried in
`docs/DEVICE_RUNTIME.md` (re-derived + checked by `tools/corpus/api_surface.py`,
11 invariants green) — the spec the future `rage-me` crate must model exactly and
otherwise reject (R9/R10). Standout findings that shape the port:

- **`lcdui.Font` is NEVER used** (0 refs). All measured text is the game's own
  bitmap font packed in `res0.pak`, drawn via Nokia `drawPixels` — so the R11
  substitute-font-metrics problem does not arise for gameplay text.
- **Nokia `DirectGraphics` is central** (base class = Nokia `FullCanvas`): the
  sprite path is PNG→`Image`→`getPixels(4444)`→`short[]`→`drawPixels`. Pixel
  formats are a closed `{ARGB4444, ARGB8888}`; blit flips `{0, FLIP_HORIZONTAL}`.
  `drawImage(…,anchor)` ×177, `setClip` ×110 (always replace); no `drawRegion`.
- **Input is event-driven** (`keyPressed`/`keyReleased`; `getKeyStates` unused).
  Closed key-code set: softkeys `-6/-7`; game actions UP=1/DOWN=6/LEFT=2/RIGHT=5/
  FIRE=8; ASCII `1`–`8` (49–56), `#` (35), `*` (42). Reject anything else (R10).
- **Audio** = two MIDI players, one active, `setLoopCount(-1)`; **`setMediaTime`
  is never called** (do NOT model a rewind); loser is stopped, never mixed (R9).
- **RMS save wire (R5)** = three stores (`system`/slot/`net`), hand-packed
  **offset-binary little-endian** (`write int = v − Integer.MIN_VALUE`, LE 4B;
  long LE 8B). Record framing fully documented; a few per-element layouts
  (`ab.*`, `n.*`) flagged as follow-up before the wire is byte-complete.
- **Game loop** = single thread, **62 ms/frame (~16 fps)**, `repaint();
  serviceRepaints();`, `paint()` fully redraws each frame (no retained
  framebuffer); `setCurrent` once. Bluetooth (JSR-82) multiplayer is a real but
  separable surface — a single-player baseline can stub/reject it.

**Game-logic semantics.** `docs/SEMANTICS.md` decodes what the container formats
*mean* (needed for the in-game/sim/save transliteration slices), gated by
`tools/corpus/semantics_evidence.py` (green; `--self-test` red-proven on a
corrupted `f251o`): the `.map` cell → 280-slot sprite table (`m.f172a`, pak
boundaries `{53,94,147,204,279}`) + `m.b` passability + three paint passes; the
`scenes.pak` model (group 1 = a 23-opcode script interpreter, group 2 = trigger
rects, group 3 = `n(...)` spawns — with per-level tallies); per-level entry
points/specials in `m.T()..ae()` (a correction: these are NOT spawn lists); and
the **byte-complete RMS save wire** (`ab`=47 B, `q`=228 B/57 ints, `n`=46+name+228
B, record 2 = 412 B). ~40 of 57 `q` ability columns + a few edge fields remain
honestly flagged uncertain.

## Phase 1 — classification + format triage (COMPLETE 2026-08-28)

**Format parsers + independent oracles — DONE.** Every custom format has a
bounded, panic-free Rust parser in `crates/rage-formats` (`map`, `utf`, `pak`,
`int_table`, `scenes`), each reversed from its consumer bytecode (see
`docs/FORMATS.md`) and guarded by a **genuinely independent** oracle over real
`_originals/` blobs (gate `just formats-corpus`, in `docs/GATES.md`):
- `int_table` (sincos) vs pure `round(10000·sin/cos(deg))` math — exact, error 0;
- `.utf` RU↔EN record-count identity (28/4/506/322/258/40/124/77/102) + language-
  keyed record 0 ("Аллоды"/"Allods") — the R11 cross-language identity;
- `.pak` XOR-`0x53` framing validated by the independent `png` crate decoding
  every PNG-signed entry (279/build) + the `4+4n+Σlen` length invariant;
- `.map` engine-remap domain (every raw cell ∈ 128..=173) + campaign byte-identity.
Each parser ships a malformed-input negative control as its can-fail proof (R3).
Test tally: 45 Rust tests green (3 canvas + 32 formats-unit + 6 corpus-oracle + 4
jvm), `cargo fmt`/`clippy -D warnings` clean, and `just check` green end to end.

**Build-role reconciliation — DONE (2026-08-28).** The four builds were compared
from the bytes (CRC-32 resource identity + obfuscated class-NAME-set diff):

- The 12 campaign levels `res/1.map`..`12.map` are **byte-identical across all
  four builds** — level data is build-independent (parse once).
- `allods_176x220` and `Rage-of-Mages_EN_v11` share an **identical 37-class name
  set** — the same program, differing only in language (RU vs EN) + recompile.
- `allods_240x320` is a **reduced 31-class strict subset** (missing classes
  `ae–aj` + the multiplayer maps `net0..4`/`101.map`) on MIDP-1.0 — an official
  but cut-down low-end port, **not** the completeness baseline. This corrects the
  scaffold's tentative `baseline = 240x320`.
- `allods_128x160` ("by Etty") is a 32-class reduced fan repack.

**Decision (user):** ship the port in **English**, anchored to **official**
behavior. `builds.toml` now records `baseline = allods_176x220` (behavior /
levels / sprites) and `naming_reference = Rage-of-Mages_EN_v11` (English `.utf`
strings, overlaid 1:1 since the class set is identical). The two out-of-scope
builds (`128x160` Etty repack, `240x320` reduced variant) were moved to
`[[archived]]` — preserved and still hash-verified, but not recovery targets
(nothing deleted). `just originals-verify` re-reconciles green (**2 payloads + 2
archived**), and its can-fail proof now targets the new baseline.

**Rust workspace — bootstrapped.** Root `Cargo.toml` workspace
(`[profile.dev] overflow-checks = true` for R8; deps pinned as in gothic), with
the game-agnostic foundation crates in place: `rage-jvm` (faithful Java
arithmetic, copied verbatim from the proven gothic port), `rage-canvas` (ARGB
`Image` + `source_over`), and `rage-formats` (bounded `Reader` + the format
parsers above).

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

1. ~~Baseline vs naming-reference build~~ **RESOLVED (2026-08-28):**
   `baseline = allods_176x220`, `naming_reference = Rage-of-Mages_EN_v11`; ship
   English text over official behavior (see Phase 1 above).
2. True per-build shipped language — **in progress:** decoding the `.utf` string
   tables to confirm RU (176x220) vs EN (EN_v11); filenames lie (R10/R11).
3. Custom-format semantics — **in progress:** reversing `.map`/`.utf`/`.pak`/
   `.int` from their consumer bytecode in the baseline decompile.
4. Reference oracle source for Phase 3 (FreeJ2ME-Plus capture vs a headless-JVM
   MIDP host vs the decompiled Java) — **deferred** until the transliteration
   runs a game loop (revisit with the user then; the expensive-if-wrong choice).
