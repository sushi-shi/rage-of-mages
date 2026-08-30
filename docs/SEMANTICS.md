# Game-logic semantics — what the container formats *mean*

Scope: the baseline **`allods_176x220`** (obfuscated class names cited are stable
for this build and for `Rage-of-Mages_EN_v11`, per `builds.toml`). This document
records the game-side semantics that `docs/FORMATS.md` (container framing) and
`docs/DEVICE_RUNTIME.md` (device API + save-wire framing) deliberately deferred:

1. what a `.map` cell **draws** and whether it is **walkable** (§1),
2. the per-level content that is hardcoded in `m` rather than stored in `.map` (§2),
3. the meaning of the three `scenes.pak` groups, including the `n(...)` spawn
   record (§3),
4. the exact per-element field order of the RMS save wire — `ab`, `n`, `q`
   payloads and the record-2 state vector (§4).

Evidence convention: `class.method` with the JVM descriptor when the obfuscated
name collides; `[jadx mNNx]` gives the decompiler's disambiguated alias. Facts
marked **(javap)** were confirmed against `javap -c -p` disassembly of the
baseline jar — the tie-breaking authority (R2). Facts marked **(corpus)** were
verified against the actual resource bytes in `_originals/allods_176x220.jar`.
Facts marked **(visual)** come from looking at the decrypted PNGs and carry
identity only, never behaviour. Everything re-derivable is re-derived by
`tools/corpus/semantics_evidence.py` (run it in the nix shell; it exits non-zero
on any mismatch with this document).

Uncertainties are collected in §5 (R10: flagged, not guessed).

---

## 1. Tile semantics — `.map` cell → sprite slot + passability

### 1.1 The global 280-slot sprite table

All images live in one global table `m.f172a : Image[280]`. A slot index is
simultaneously the entry index into the concatenation of `res0..res4.pak`:

| pak | entries (corpus) | global slots |
|---|---|---|
| `res0.pak` | 54 | 0–53 |
| `res1.pak` | 41 | 54–94 |
| `res2.pak` | 53 | 95–147 |
| `res3.pak` | 57 | 148–204 |
| `res4.pak` | 75 | 205–279 |

The boundary table is hardcoded as `g.f114w = {53, 94, 147, 204, 279}` (the
inclusive last slot of each pak); the case-8 branch of `g.a(int)` walks the pak
directory with `j` (skip / read-XOR-decrypt / `Image.createImage`) loading only
slots whose *need flag* is set. Slot→pak-entry: `slot − (first slot of pak)`.

Loading is demand-driven through two `byte[280]` need bitmaps in `m`:
`m.f227h` (what the current level needs — filled by the map loader, see §1.3)
and `m.f228i` (what the next mode needs — filled before entering combat/dungeon,
`m.java` jadx ~1340). `m.h()` (mode-switch dispatcher) passes the bitmap to
`g.h()` (mark everything loaded as evictable) → `g.a(byte[])` (needed slots:
keep or queue for load) → `g.b()` (evict + load, `a.f172a[slot] = null` /
`createImage`).

### 1.2 From `.map` byte to tile id — exact load order (javap)

`m.a(II)V` [jadx `m38a`] per cell, in this order (confirmed against bytecode —
`readByte; isub; bastore`, then `invokevirtual b:(II)V`, then the remap):

1. `f207a[row][col] = (byte)(raw − 128)` — raw code 0..45;
2. `b(row, col)` — the passability stamper (its **only** call site, javap); it
   reads the raw code and switches on `f251o[rawCode]`, i.e. on the remapped
   tile id;
3. `f207a[row][col] = f251o[f207a[row][col]]` — the grid now holds tile ids;
4. `f227h[tileId] = 1` — the tile's sprite slot is flagged for loading.

`m.f251o` is the 46-entry remap already recorded in FORMATS.md (spot-verified
against the `bipush/bastore` constructor sequence, javap). Tile id **is** the
sprite slot: the campaign tileset occupies slots 26..63 of `res0`/`res1`.

Grids (all `[height][width]`, jadx names): `f207a` tile ids, `f213c`
passability, `f208b` splash-ground overlay (§1.6).

### 1.3 Post-load slot flagging (m.a(II)V)

After the grid loop the loader adds companion slots: if any tile 49 → flag 60
and 62; if any 50 → flag 61 and 62; if any 36 → flag all of 36..43 (water
animates through all 8 frames); always flags 3..11 and 16..25 (HUD/cursor
sprites), 63 (rain splash), and per spawned entity `s.e[type][0..3]` and
`s.e[type][8]` (§3.4). If ground pickups exist, 279. The dungeon branch
(`level < 0`) instead flags 64..82 (dungeon tileset) plus the same HUD ranges.

### 1.4 The three paint passes

Play-area viewport: `setClip(9, 9, 158, 190)`; a cell paints at
`(col*16 − camX + 9, row*16 − camY + 9)`; offsets below are relative to that
cell anchor. Camera `camX/camY = m.f205p/f206q`. Order inside the frame paint
`m.m(Graphics)`: `g()` (ground) → `h()` (corpses `n.b(G)` = `s.e[type][8]`
image plus optional blood pool, unit shadows `n.c(G)`, ground pickups
`f172a[279]`) → `i()` (under-effects) → `j()` (objects + live entities) →
`k()` (overhead) → `l()` (over-effects).

- **`m.g(Graphics)` — ground pass.** First tiles `f172a[26]` (100×116 grass
  base) across the viewport in a 3×3 loop keyed on `camX % 100`, `camY % 116`;
  then for every visible cell with tile id **27..43** draws `f172a[id]` at the
  cell anchor. Two specials: id 35 draws at offset (−45, −35) (centred big
  sprite); ids 49/50 additionally draw `f172a[62]` at (−15, −22). **Water
  animation happens here**: if the cell's id is ≥ 36 it is incremented in the
  grid before drawing, wrapping 44 → 36 — an 8-frame cycle advanced *per
  repaint of that cell* (paint mutates `f207a`; a port must keep the frame
  state per cell, not per timer).
- **`m.j(Graphics)` — object pass, depth-interleaved.** Iterates cells in row
  order; draws tile ids **44..59** with per-id anchors (table below), and after
  each cell draws any entity whose footprint cell (`n.h`,`n.i`) equals this
  cell (queue `af.b`) and any effect `r` at this cell — the painter's
  algorithm that lets units walk "behind" tall objects.
- **`m.k(Graphics)` — overhead pass.** Draws tree canopies over everything:
  id 49 → `f172a[60]` at (−35, −79); 50 → `f172a[61]` at (−37, −89);
  60 → `f172a[60]` at (−35, −47); 61 → `f172a[61]` at (−37, −57).

### 1.5 The tile table

Passability stamps from `m.b(II)V`: `f213c[..] = 1` at the listed cells
(relative to the tile's cell `[r][c]`); everything unlisted stays 0 = walkable.
`f213c` values: `0` free, `1` terrain-blocked, `−1` entity-occupied
(`n.k()` stamps the entity footprint, `n.j()` clears it); `m.m21b` temporarily
stamps 2×2 under ground pickups during dungeon spawn placement. Sprite sizes
are the decrypted PNG dimensions (corpus). Identities are (visual).

| id | png w×h | draw pass, offset | blocked cells | identity (visual) |
|----|---------|-------------------|---------------|-------------------|
| 26 | 100×116 | backdrop only (tiled) | — | grass base; a 26-cell draws nothing itself |
| 27 | 48×48 | ground, (0,0) | — | dirt patch (3×3 cells); stamps `f208b` 3×3 (§1.6) |
| 28, 29 | 16×48 | ground, (0,0) | — | vertical dirt strips |
| 30, 31 | 48×16 | ground, (0,0) | — | horizontal path strips |
| 32 | 64×16 | ground, (0,0) | `[r][c−1]`, `[r][c+4]` | plank bridge; span walkable, both banks beyond it blocked |
| 33, 34 | 32×16 | ground, (0,0) | `[r][c]`, `[r][c+1]`¹ | low hedge/edge rows |
| 35 | 90×68 | ground, (−45,−35) | `[r−1..r][c−2..c+1]` (try/caught at borders) | pit/crater, centred on its cell |
| 36–43 | 32×32 | ground, (0,0) | (id 36 only) `[r][c]`,`[r][c+1]`¹,`[r+1][c]`¹,`[r+1][c+1]`¹ | water, 8 anim frames; maps only ever store frame 36 (corpus: raw codes 12..26 unused); stamps `f208b` 2×2 |
| 44 | 47×32 | object, (+8−11,−8)=(−3,−8) | `[r][c..c+2]` | bush row |
| 45 | 59×60 | object, (0,0) | — | rocky outcrop (walkable — original behaviour) |
| 46 | 42×35 | object, (−9,−15) | `[r][c]` | tree stump |
| 47 | 48×80 | object, (0,−64) | `[r][c..c+2]`¹ | rock pillar |
| 48 | 48×80 | object, (0,−64) | if r>0: `[r][c]`,`[r][c+3]`; if r−1>0: `[r−1][c..c+3]` | cliff face |
| 49 | 37×41 | object, (−12,−15); +62 in ground; +60 overhead | `[r][c]` | tree A trunk (canopy 60 above units) |
| 50 | 43×47 | object, (−12,−15); +62 in ground; +61 overhead | `[r][c]` | tree B trunk (canopy 61 above units) |
| 51 | 12×25 | object, (0,−5) | `[r][c]` | signpost |
| 52, 53 | 16×16 | object, (0,0) | `[r][c]` | small props |
| 54 | 73×83 | object, (−19,−23) | `[r..r+2][c−1..c+2]` | tent |
| 55 | 90×82 | object, (−42,−50) | `[r−2][c..c+1]`, `[r−1][c−2..c+2]`, `[r][c−3..c+2]`, `[r+1][c−2..c]` | stone house |
| 56 | 57×69 | object, (−12,−25) | `[r][c..c+2]`, `[r+1][c−1..c+2]` | boulder |
| 57 | 76×94 | object, (−24,−46) | `[r−1][c−1..c+1]`, `[r][c−1..c+2]`, `[r+1][c−1..c+2]` | peaked-roof house |
| 58 | 74×130 | object, (−29,−110) | `[r−1][c−1..c+1]`, `[r][c−1..c+1]` | tall stone tower |
| 59 | 124×109 | object, (−54,−52) | `[r−1..r+1][c]`, `[r−2..r][c−1]`, `[r−2..r][c−2]`, `[r−2..r][c−3]`, `[r−1..r+1][c+1]`, `[r−1..r+1][c+2]`, `[r][c+3]` | burnt ruin |
| 60, 61 | 92×64, 92×74 | overhead only | — | canopy-only tiles (walkable shade over paths) |
| 62 | 61×65 | companion (never a map tile) | — | small canopy drawn under units for 49/50 |
| 63 | 6×5 | effect (class `a`) | — | rain-splash dot |

¹ bounds-guarded (`c+1 < width` etc.).

Notes:

- **Raw code 42 is an invisible blocker**: after the switch, `m.b(II)V` checks
  the *pre-remap* code — `if (f207a[r][c] == 42) f213c[r][c] = 1` — and raw 42
  remaps to tile 26 (plain grass). Remapped id 42 (from raw 17/25) is normal
  walkable water-frame 7 — the two 42s are unrelated.
- Which ids can appear in shipped maps (corpus, all 18 `.map` incl. `net*`):
  raw codes 0..11 and 27..45 → tile ids {26..36, 44..61}. Ids 37..43 arise only
  from the paint-time water cycle.
- Sprite-slot 2 of `res0` is the bitmap-font descriptor blob (FORMATS.md), not
  a PNG image; it is never flagged in `f227h`.
- **Dead code kept for fidelity**: `m.a(Ln;Ln;II)Z` compares the `byte`
  `f213c` cell against `sipush 1000` (javap) — always false; the whole method
  constantly returns false. Preserve, don't "fix".
- `m.f207a` can hold negative values at one reader: the `f228i` flag pass
  (jadx `m.java:1368`) decodes `b < 0` as slot `(−b)−1`. No writer of negative
  tile values was found in the jar; treat as vestigial (§5).

### 1.6 Rain and the `f208b` splash grid

`m.a(int x, int y, byte v, int n)` stamps an n×n square of value `v` into
`f208b`. The map loader stamps 27 as 3×3 under every dirt patch (tile 27) and
36 as 2×2 under every water tile — `f208b[cell] != 0` means "bare ground/water
that shows a splash". Weather: `ad(int seconds)` (subclass of effect `r`) is
rain — each frame it spawns 5 raindrops (class `a`: a white streak advancing
(+4, +8) px/frame; on landing, if `f208b` is set at the landing cell, draws
`f172a[63]`). It expires when the game clock passes `start + seconds*1000`;
the per-level calls use `ad(180000)` — effectively rain for the whole visit.

### 1.7 The dungeon tileset (levels with `m.ag < 0`)

Negative level indices are procedurally generated caves (§2.4). Their grid
holds direct codes (no `−128`, no `f251o`); base fill is `f172a[64]` (48×48)
tiled at 48 px, and the ground/overhead passes' `else`-branches map:

| code | slot (offset from cell, in cells) | code | slot |
|---|---|---|---|
| 1 | 66 `(c+1,r−1)` anchor RIGHT | 15 | 75 `(c−1,r−1)` |
| 2 | 67 `(c,r−1)` | 16 | 80 `(c−5,r−5)+8px x` |
| 3 | 69 `(c−1,r−1)` | 17 | 78 `(c−3,r−4)+4px x` |
| 4 | 70 `(c−1,r−1)` | 18 | 81 `(c−1,r−1)` |
| 5↔6 | 71/72 `(c−1,r−1)` — torch, flips each repaint | 19 | 79 `(c−1,r−3)` |
| 7 | 73 `(c−1,r+1)` anchor BOTTOM | 20 | 82 `(c−2,r−3)+12px y` |
| 8 | 68 `(c−1,r)` | 21 | 76 `(c−3,r−3)+8px x,y` |
| 9 | 74 `(c−1,r−1)` | 22 | 77 `(c−2,r−2)+10px x,+4px y` |
| 10 | 65 `(c−1,r−1)` | 23 | 35 `(c−5,r−4)+8px y` (the surface pit reused) |

Dungeon passability is stamped by `m.a([[B[[B)V` from the generated grid (not
by `m.b(II)V`); not further decoded here (§5).

---

## 2. Per-level content hardcoded in `m` (not in `.map`)

### 2.1 Correction to the FORMATS.md sketch

`m.T()..m.ae()` are **not** spawn lists. Unit/trigger/script content lives in
`scenes.pak` (§3); the twelve per-level methods only set the **party entry
point** — `m.ai`, `m.aj` (spawn cell) and `m.f135a` (initial facing 0..3) —
switched on `m.ap` (the entry variant passed as the second argument of
`m.a(II)V`, i.e. *which exit the party arrived through*), plus a few
level-specific extras. The party is placed at `(ai*16, aj*16)` facing `f135a`
(m38a, jadx ~4797) unless save-record 3 restores positions.

### 2.2 The twelve methods (level = campaign index)

| lvl | method | ap=0 | ap=1 | ap=2 | extras |
|---|---|---|---|---|---|
| 0 | `T` | (34,24) f0 | (22,5) f3 | (3,24) f1 | — |
| 1 | `U` | (28,37) f2 | (32,7) f0 | (33,28) f0 | — |
| 2 | `V` | (5,35) f1 | (40,28) f0 | — | rain ×2 |
| 3 | `W` | (10,47) f2 | (40,40) f1 | — | rain ×2 |
| 4 | `X` | (4,29) f1 | (43,31) f0 | — | — |
| 5 | `Y` | (3,40) f1 | (2,15) f0 | — | — |
| 6 | `Z` | (2,19) f1 | (25,20) f0 | — | rain ×1; puzzle randomisation (below) |
| 7 | `aa` | (25,10) f1 | (4,18) f0 | — | rain ×2 |
| 8 | `ab` | (55,15) f1 | (8,5) f0 | (36,29) f0 | rain ×1; dragon-relocation is in m38a (below) |
| 9 | `ac` | (52,6) f1 | (4,6) f0 | (48,52) f0 | — |
| 10 | `ad` | (52,52) f1 | (40,13) f0 | — | — |
| 11 | `ae` | (2,57) f1 | (53,56) f0 | — | rain ×1 |

"rain ×N" = N× `a(new ad(180000))` (stacked emitters = denser rain, §1.6).

**Level 6 (`Z`) — randomised puzzle.** If `m.f250n` (save record 7) is empty it
becomes 5 fresh random bytes `{a(3), a(2), a(4), a(3), a(4)}` (`m.a(int n)` =
`abs(rng.nextInt() % n)`). Four hardcoded groups of candidate trigger rects
(script ids 68, 69, 70, 71 — 3/2/4/3 candidates) are then appended to the
trigger list `f196d`, each group **omitting** the rect whose index equals
`f250n[k]` — i.e. per save, one randomly-chosen rect per group is *absent*.
Record 7 persists the choice. `f250n[4]` is written and saved but no reader was
found (§5).

**Level 8 — dragon relocation (in `m.a(II)V` itself, jadx ~4776).** One random
cell `(a(30)+5, a(55)+2)` is drawn; every scene entity with script-group
`g == 10` is re-placed at the nearest free cell to it (`m.a(Ln;II)V` spiral
search `m39a`), and a trigger `{119, (x−10)*16, (y−12)*16, (x+10)*16,
(y+12)*16}` around the **last** element of `f195c` is appended — once inside
the loop and once after it (a duplicate; original quirk, preserve).

### 2.3 Other hardcoded per-level tables in `m`'s constructor

- **`m.f280f : int[24][][]`** — ground pickups (index 0..11 campaign level,
  12..23 dungeon). Row = `{flagIdx, x_px, y_px, scriptId|−1, contents...}`
  where contents ≥ 0 are item ids (§4.1) and < 0 is `−gold`. At map load every
  row whose `f179c[flagIdx] == 0` (not yet taken) enters the active list
  `f200h` and is drawn as `f172a[279]`. Walking onto the 32×32 px pickup area
  (`m.m30f()`) sets `f179c[flagIdx] = 1`, adds the items via `m.c(int)`
  (id < 36 → equipment vector `f240k`, < 41 → gem vector `f242m`, else →
  potion/misc vector `f241l`), adds the gold to `m.ae`, and if `scriptId ≥ 0`
  queues it in `m.X`. Campaign contents: L0 `{0,528,64,−1,21,39,−200}`; L1
  `{3,32,336,−1,−500}`; L2 `{1,16,16,40,33}`, `{2,32,416,−1,15}`; L4
  `{6,16,816,6,−1,27}`, `{7,32,32,−1,4}`, `{8,624,32,−1,−700}`; L5
  `{9,288,208,−1,7,−4000}`, `{10,384,288,−1,10,−2000}`, `{11,560,416,−1,−3000}`,
  `{12,304,416,−1,19,−1000}`; L9 `{13,288,208,−1,5,−10}`,
  `{14,384,288,−1,−5000}`; others empty. (Note L4's `−1` inside the contents
  positions adds 1 gold — the code has no "skip" value; as-is.) Dungeon rows
  (12..23) carry `x=y=0`; the generator positions them in cleared rooms.
- **`m.f234j : int[27][]`** — "on group wiped" scripts, pairs
  `(scriptGroup, scriptId)` per level (index `ag`, dungeons at `13+d`): when no
  live entity with that `n.g` remains, the script fires (`m.p()`, gated by
  `f177b[scriptId] == 1`). Non-empty: level 1 `{10,26}`, level 4 `{10,56}`,
  dungeon 1 `{8,27}`.
- **Dungeon config**: `f252k` (per-dungeon room-grid dims), `f254m` (per-dungeon
  monster tables, pairs `(percent, typeId)` rolled per room cell), `f279e`
  (per-dungeon script rows appended to `f197e`), `f281g` (room bookkeeping).
  Decoding the generator `m.c(II)V` fully is future work (§5).

### 2.4 New game & dungeons

`m.P()` (new game): playtime 0; party = one `n(0,0, type 0 "swordsman", g 0,
rec 1, false, 10000,10000,10000)`; quest states `f176a[*]=0` except
`f176a[0]=1`; trigger states `f177b[*]=1` except a fixed dormant set (= 2:
ids 3,4,7,9..12,15,18,19,21..23,37,39,43,57,59,73..99,137,160..168); dungeon
seeds `f178a[*]=0`; pickups `f179c[*]=0`; scene-records `f180d[*]=1` except a
fixed active set (= 0: 0,1,3..5,7,9,10,17,19..25,27..34,48,49); chapter flags
`f181e = {1,1,1,0...}`; tutorial-shown `f182f[*]=0`; level 0 entry 0.

Dungeon levels use negative indices: `m.a(II)V` with `i < 0` calls the
generator `m.c(II)V` for dungeon `d = −i−1`; its layout PRNG is seeded from
`f178a[d]` (set to `currentTimeMillis()` on first entry, then persisted in save
record 2 — a dungeon keeps its layout across save/load).

---

## 3. `scenes.pak` — scripts, triggers, spawns

Framing per FORMATS.md (12 scenes × 3 nested big-endian int-array groups;
corpus: parses EOF-exact). Semantics:

### 3.1 Group 1 → `m.f197e` — script programs

Row = `{scriptId, instructions...}`. Script ids are globally unique across
levels (corpus: 0..158). An instruction is `opcode, args...` with total length
`s.f318b[opcode]`:

```
op:  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22
len: 3  3  8  2  2  2  4  2  1  1  1  3  3  3  3  2  3  1  3  4  3  2  1
```

`m.d(int id)` looks the program up, indexes its instructions (`f184i`), and
enters script mode (`m.g = 9`); `m.M()` executes one instruction per tick:

| op | args | effect (evidence: `m.M()`) |
|---|---|---|
| 0 | flag, v | `f177b[flag] = v` (arm/disarm triggers) |
| 1 | quest, v | `f176a[quest] = v` unless already 2/3; pings the quest HUD |
| 2 | grp, relGrp, axis, dx, dy, ?, flags | scripted walk of entity group `grp` toward `(dx,dy)` cells relative to `relGrp` (axis 1/2 = lock y/x; flags bit0/bit1 = variants) — arg 6 not fully decoded (§5) |
| 3 | dlgId | open dialogue screen (`f152g.t = dlgId`) |
| 4 | t | wait `t*100` ms with screen shake (`f187k/f188l` = ±8 px random), then a short `ad(180)` rain burst |
| 5 | grp | **recruit**: move entity `n.g == grp` from `f195c` into party `f239j` (a `g==3` mage inserts at index 2), team `s=0`, its equipment joins `f240k`, `f180d[n.f307b] = 2` (never respawns) |
| 6 | which, idx, v | **if**: `which==1 ? f176a : f177b`; if `[idx] != v` skip forward to the next op 17 (*else*) or op 9 (*endif*) — CFR M() is the clean rendering of this loop |
| 7 | grp | un-recruit (inverse of 5, `f180d[..] = 0`) |
| 8 | — | disarm the currently running script's own trigger (`f177b[cur] = 0`) |
| 9 | — | no-op / **endif** landing pad |
| 10 | — | level-complete screen (level `ag+1` splash) |
| 11 | lvl, entry | change level: `ag = lvl`, `ah = entry`, full reload |
| 12 | rec, v | `f180d[rec] = v` unless already 2; `v==2` also despawns live entities with `f307b == rec` |
| 13 | pick, v | `f179c[pick] = v` |
| 14 | ch, v | `f181e[ch] = v` (chapter/act markers) |
| 15 | page | show tutorial page once (`m.m16a`; `f182f[page]` latch, gated by option `f247k[3]`) |
| 16 | grp, dir | set facing of entity group `grp` |
| 17 | — | **else**: when *executed* (i.e. the true-branch ran into it) skip forward to the next op 9 |
| 18 | x, y | teleport the whole party to cell (x, y) |
| 19 | grp, show, force | show (`n.k()`) / hide (`n.j()`) entity group |
| 20 | grp, force | revive/reset entity group |
| 21 | grp | make group hostile (`f299f = true`) and mark it (boss intro) |
| 22 | — | set mission-complete flag `f191q` |

### 3.2 Group 2 → `m.f196d` — trigger zones

Row = `{scriptId, x1, y1, x2, y2}` (px, inclusive rect; corpus: every row is
length 5). Each tick `m.m30f()` fires the first rect containing the hero's
`(H, I)` **if** `scriptId ≥ 300 || f177b[scriptId] == 1`, calling `m.d(id)`.
(No shipped id is ≥ 300 — the clause exists to bypass the 169-entry flag
array.) Trigger-state byte `f177b[id]`: 0 = spent/disabled, 1 = armed,
2 = dormant until a script arms it. Scripts add rects at runtime too (the
level-6 and level-8 content of §2.2 and dungeon exits).

### 3.3 Group 3 → spawns (`new n(...)`)

Row (9 or 10 ints), read only when no save-record 4 exists; skipped when
`f180d[row[0]] == 2`:

| idx | → | meaning |
|---|---|---|
| 0 | `n.f307b` (byte) | **scene-record id** — index into the persistent state array `f180d[50]` (0 = active/visible, 1 = spawned hidden until a script reveals it, 2 = permanently gone: dead or recruited) |
| 1 | `n.H` | x position, px (entity centre; footprint cell `t = (H − j*8)/16`) |
| 2 | `n.I` | y position, px |
| 3 | `n.f295c` | creature type 0..17 — index into every `s.*` table; names `s.f319a`: swordsman, archer lady, healer, wizard, troll, hermit, orc, dragon, (8 unused), Goblin, mage, skeleton, bat, skeleton, evil squirrel, turtle, guard, poisoned troll |
| 4 | `n.g` | **script group id** — the handle ops 5/7/16/19/20/21 and `f234j` address; not unique per entity (groups act together) |
| 5 | `n.f299f` (≠0) | hostile/aggressive flag |
| 6 | `n.x` | stored & saved, **never read** (javap: no getfield outside `n`) — vestigial |
| 7 | `n.y` | **on-death script id**: when the entity dies hostile and `f177b[y] == 1`, the script queues (`m.p()`, jadx ~1512); 10000 = none |
| 8 | `n.z` | stored & saved, never read — vestigial |
| 9 (opt) | `n.f296d` | initial facing 0..7 (else type-dependent default, usually random `a(4)`) |

Constructor side effects (`n.<init>(IIIIBZIII)` → private `a(...)`): builds the
base ability vector `f303a` from `abilities.txt` row(s) and — for non-hero
types in campaign — scales stats 9..13 and 40..56 up by the current level
index (the difficulty ramp, jadx `n.java:131`); derives the effective vector
`f304b = f303a.a(this, armor, weapon)`; sets footprint `j×k` cells (heroes/
hermit/guard 1×1; most monsters 2×2; the two troll types 4 and 17 3×3),
collision half-extents `f293a/f294b` px, team `s` (0 = party — only type 0 at
new game; 1 = everyone else), display name from `common.utf` group `aj.c`,
and for hero types 0..3 fresh equipment `ab` pairs (weapon 0/3/6/9, armor
12/18/24/30). Type is also the key into the sprite-pack table `s.e[type][13]`:
`[0..7]` = 8-direction pack ids (the map loader flags `[0..3]` + `[8]`; the
full-preload path flags `[0..9]`), `[8]` = corpse image (`n.b(G)`), `[11]` =
blood-pool colour code (1 red / 2 green, gated by option `f210g[8]`), `[12]` =
starting animation frame; `s.c`/`s.f323c`/`s.f324d` = frame periods/sequences,
`s.f321a`/`s.f322b` = per-direction draw anchors, `s.f320c`/`s.d` = shadow
shape index and dimensions (−1 = no shadow, `n.c(G)`).

### 3.4 Corpus summary (from `tools/corpus/semantics_evidence.py`)

| lvl | scripts | triggers | spawns | spawn types |
|---|---|---|---|---|
| 0 | 19 | 21 | 6 | poisoned troll, hermit, archer lady, bat, 2×squirrel |
| 1 | 13 | 13 | 10 | 3×goblin, orc, healer, 4×guard, squirrel |
| 2 | 9 | 7 | 13 | 4×skeleton(13), 4×bat, 3×goblin, wizard, 1×type-8 |
| 3 | 15 | 8 | 11 | 5×turtle, 3×mage, hermit, skeleton(13), troll |
| 4 | 4 | 4 | 19 | 11×orc, 4×goblin, 2×turtle, 2×troll |
| 5 | 7 | 6 | 20 | 6×squirrel, 5×goblin, 3×turtle, 2×orc, 3×troll, skeleton |
| 6 | 40 | 37 | 66 | 16×goblin, 16×troll, 12×turtle, 12×mage, 4×skeleton(13), 3×squirrel, 2×bat, skeleton |
| 7 | 6 | 6 | 2 | 2×skeleton(13) |
| 8 | 7 | 6 | 14 | 6×goblin, 4×squirrel, 3×troll, skeleton |
| 9 | 14 | 10 | 23 | 8×squirrel, 6×goblin, 4×guard, 3×troll, mage, skeleton |
| 10 | 6 | 6 | 18 | 15×orc, hermit, 2×type-8 |
| 11 | 7 | 4 | 17 | 4×skeleton, 3×orc, 3×troll, 2×goblin, 2×turtle, 2×skeleton(13), mage |

---

## 4. RMS save-element layouts (completes DEVICE_RUNTIME §5.2)

All multi-byte integers below use the offset-binary little-endian codec
(`m.b(I[BI)V` / `m.c([BI)I`, longs `m.a(J[BI)V` / `m.a([BI)J`) — see
DEVICE_RUNTIME §5.1. Byte fields are written with a plain cast and read with a
plain (sign-extending) `baload` (javap). Readers mirror writers exactly.

### 4.1 `ab` — item record, **47 bytes** (`ab.c()`, javap `bipush 47`)

`ab` is an inventory item. `f3a` = item id: 0..11 weapons (4 hero classes × 3
tiers), 12..35 armor (4 × 6), 36..40 gems, 41+ misc/potions. `f4a` = kind,
*derived* from the id (`<12`→0 weapon, `<36`→1 armor, `<41`→2 gem, else 3);
`b` = socket count (`id%3 + 1` for weapons/armor, 0 otherwise); `a[10]` =
socketed gems (`ab` refs); `f6a` = "is socketed into something". Item prices
are `s.f327f[id]`.

Wire (`ab.a([BILm;)V` write / `ab.b` read):

| off | size | field |
|---|---|---|
| 0 | 1 | `f4a` kind |
| 1 | 4 | `f3a` item id (int) |
| 5 | 1 | `b` socket count |
| 6 | 1 | `f6a` (0/1) |
| 7 | 40 | 10 × int: index of `a[k]` in the gem vector `m.f242m`, −1 if empty |

Record 1 (`m.m34a()`): `ae` (party gold, int) · then the three item vectors
each as `count (int) + count × 47 bytes`: `f242m` (gems — **first**, so the
socket indices above resolve), `f240k` (equipment), `f241l` (misc/potions).

### 4.2 `q` — ability vector, **228 bytes** (`q.a()`, javap `sipush 228`)

57 × int (offset-LE), the entity's *base* stat row `q.i[57]` (from
`res/abilities.txt`, 57 columns — FORMATS.md). Written by `q.a([BI)V`, read by
`q.b([BI)V`. Known indices (evidence cited; the rest un-named, §5):
`i[0]` current HP (`n.c(int)` clamps to `[0, i[14]]`), `i[14]` max HP,
`i[7]` XP, `i[8]` character level (level-up when XP ≥ 50·Σ1.5^k: `n.m53a`),
`i[2]` current action points (refilled from `i[16]` in `n.o()`; recompute
clamps 0..40), `i[16]` AP-per-turn (clamp 1..20), `i[18]` clamp 0..40,
`i[9..13]` core stats (the ctor's difficulty ramp), `i[40..56]` learned
abilities (+level when > 0). The *effective* vector `f304b` is recomputed from
this + equipment on load, never stored.

### 4.3 `n` — entity record, **46 + (len+1) + 228 bytes** (`n.m45a()`, javap `bipush 46`)

Wire (`n.a([BI)V` write / `n.b([BI)V` read, byte-for-byte, javap):

| off | size | field | meaning (§3.3 unless noted) |
|---|---|---|---|
| 0 | 1 | `f295c` | creature type |
| 1 | 1 | `g` | script group id |
| 2 | 4 | `w` | unspent stat points (+5 per level-up) |
| 6 | 1 | `v` | unspent skill points (+1 per level-up) |
| 7 | 4 | index of `f305a` (armor) in `m.f240k`, −1 none |
| 11 | 4 | index of `f306b` (weapon) in `m.f240k`, −1 none |
| 15 | 1 | `d` | dead flag (0/1) |
| 16 | 1 | `f296d` | facing |
| 17 | 4 | `H` | x px |
| 21 | 4 | `I` | y px |
| 25 | 1 | `f299f` | hostile flag |
| 26 | 1 | `f307b` | scene-record id |
| 27 | 4 | `s` | team (0 party / 1 enemy) |
| 31 | 4 | `x` | vestigial |
| 35 | 4 | `y` | on-death script id |
| 39 | 4 | `z` | vestigial |
| 43 | 1 | `j` | footprint width, cells |
| 44 | 1 | `k` | footprint height, cells |
| 45 | 1 | L = name-blob length |
| 46 | L | name blob (below) |
| 46+L | 228 | `f303a` base ability vector (§4.2) |

**Name blob codec** (`ae.a(Ljava/lang/String;)[B` / inverse
`ae.a([B)Ljava/lang/String;`): `strlen` bytes of *glyph indices* (each char →
its index in the bitmap-font glyph string) **plus one trailing byte** =
`(total pixel width of the string) − 127`; so L = strlen + 1 and the trailing
byte is ignored on decode.

Records 3/4 (`m.m36c()` / `m.m37d()`): `count (int)` + that many `n` records —
rec 3 = the party `f239j` (element 0 becomes the hero `f202b` on load), rec 4 =
the level's live entities `f195c`. On load these blobs are held raw
(`f248l`/`f249m`) and parsed after the map loads (`m.c([B)V`/`m.d([B)V`);
a non-null `f249m` also suppresses the scenes.pak group-3 spawn pass.

### 4.4 Record 2 (`m.m35b()` / `m.e([B)V`) — the 412-byte world state

Fixed concatenation (sizes from the field initialisers, jadx `m.java:530-548`):

| off | size | field | meaning |
|---|---|---|---|
| 0 | 24 | `f176a[24]` bytes | per-quest state (0 none, 1 active, 2/3 terminal; quest→level map `s.f325d`) |
| 24 | 169 | `f177b[169]` bytes | per-script trigger state (0 spent, 1 armed, 2 dormant) |
| 193 | 96 | `f178a[12]` longs | per-dungeon layout PRNG seeds (0 = not yet generated) |
| 289 | 27 | `f179c[27]` bytes | ground-pickup taken flags (indices = `f280f` rows) |
| 316 | 50 | `f180d[50]` bytes | scene-record entity state (0 active / 1 hidden / 2 gone) |
| 366 | 13 | `f181e[13]` bytes | chapter/act markers (op 14; highest set index = current stage) |
| 379 | 20 | `f182f[20]` bytes | tutorial-page-shown latches |
| 399 | 4 | `ag` int | current level (negative = dungeon `−d−1`) |
| 403 | 1 | `f243v` (0/1) | quest-log filter mode (g.java:3838) |
| 404 | 8 | `f277i` long | accumulated playtime ms |

Records 5–7 framing is already in DEVICE_RUNTIME §5.2; semantics: rec 5
`f245j` = `3328 × 4` bytes of per-level fog/visited bits, rec 7 `f250n` =
the level-6 puzzle bytes (§2.2; length 0 until level 6 is first entered —
readers must accept both).

### 4.5 The multiplayer roster element (`"net"` store)

`n.c([BI)V` / `n.d([BI)V`, size `n.b()` = **105 + (len+1) + 228**: type (1) ·
`w` (4) · `v` (1) · `g` (1) · `s` (1) · armor as *inline* `{id (4), sockets (4),
10 × gem id (4)}` with −1 fill when absent · weapon likewise (2 × 48) · name
blob (1 + L) · 228-byte ability vector. Unlike §4.3 it stores item/gem **ids**,
not pool indices (there is no shared pool on the wire).

---

## 5. Explicitly uncertain / not decoded (R10)

- **`q.i` columns**: only the indices listed in §4.2 have evidence-backed
  meanings; the remaining ~40 of 57 (and the full meaning of `i[18]`, the
  `s.a` ability-parameter rows, `i[1]`) are undecoded. The *wire* is complete
  regardless (57 opaque ints).
- **Script op 2** (scripted walk): the roles of arg 6 and the two flag bits
  are only partially traced; the movement math (`m27a` pathfinding hand-off)
  is not decoded.
- **The dungeon generator `m.c(II)V`** (~400 lines): room layout, corridor
  carving, and the `f252k`/`f254m`/`f281g` field-by-field semantics are not
  decoded — only its inputs (seed `f178a[d]`, §2.4), its tile-code output
  space (§1.7), and its spawn/treasure hooks (§2.3) are pinned.
- **`n.f293a`/`f294b`**: used as ± pixel extents in one neighbourhood scan
  (n.java jadx ~890); called "collision half-extents" here on that single use —
  medium confidence.
- **`s.e[type][9]` and `[10]`** sprite-pack slots: flagged only by the
  full-preload path; which animations they hold is unverified. Same for the
  exact roles of `s.f321a` vs `s.f322b` (two per-direction anchor tables) and
  of `s.e[type][11]`'s value 5 (only 1/2 are handled by the blood-pool switch).
- **Negative tile ids** (§1.5): one reader exists, no writer found; possibly
  dead code from a richer build.
- **`f250n[4]`** is generated and persisted but no reader was located.
- **`m.f176a` values 2 vs 3** (quest terminal states): which is
  completed vs failed is not pinned (both block op 1 overwrites).
- Decompiler line numbers cited as `jadx ~NNNN` are conveniences into the
  git-ignored `_reference/decompile/176x220/jadx` tree, not stable identifiers;
  the class+method+descriptor citations are the durable ones.
