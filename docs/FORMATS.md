# Original resource formats — Rage of Mages (Allods, Nival, J2ME 2D)

Status: **Phase 1 reversed from the consumer bytecode.** Every custom format
below was decoded by reading the class/method that *reads* it in the baseline
`allods_176x220.jar` (obfuscated ~37-class program launching
`Container/GameMIDlet`), never by guessing (rulebook R10/R11). Each finding is
re-derived straight from the jar bytes by `tools/corpus/inspect_formats.py`
(maps, `.utf` record counts, `.pak` framing + XOR, `.int` curves) — that tool
**passes** on the current corpus. The decompile that this spec cites lives,
git-ignored, in `_reference/decompile/176x220/{jadx,cfr}/` (regenerate with
`python3 tools/java/decompile.py`); `javap -c -p` on the extracted `.class`
files is the tie-breaking authority for any ambiguous rename.

Endianness is **mixed** and stated per field: Java `DataInputStream`
(`readInt/readShort/readUTF`) is **big-endian**; the `.map` header is decoded by
a hand-rolled **little-endian** `u16` helper (`m.a(byte[],int)`). Obfuscated
names below are the real single-letter bytecode names (from `javap -p`); where
jadx invents a disambiguating alias (e.g. `m38a`) it is noted in brackets.

## Language classification (declared vs. actual) — R10/R11

The authoritative language key is the **decoded `.utf` content**, not the
filename or manifest. The `.utf` container has **no in-file encoding flag** — text
is always Java modified-UTF-8 (`readUTF`), so the flag column records that fixed
fact, not a per-file byte.

| build (jar)                     | declared (filename / `MIDlet-Name`)   | actual decoded text | text container / encoding |
|---------------------------------|---------------------------------------|---------------------|---------------------------|
| `allods_176x220` **(baseline)** | RU — "Allods", vendor Nival           | **Russian** (Cyrillic UTF-8, verified: `1.utf[0]="Аллоды"`) | `.utf` = Java `readUTF` stream, modified-UTF-8; no encoding byte |
| `Rage-of-Mages_J2ME_EN_v11`     | EN — "Rage of Mages by SCOURER"       | **English** (`1.utf[0]="Allods"`, `1.utf[2]="[c] 2005 Nival Interactive"`) | same container; **same record count per file** as RU (see below) |

**Cross-language identity (R11), machine-verified.** Every `.utf` file holds the
*identical* `readUTF` record count in the RU baseline and the EN build, and each
tiles to exact EOF, even though the byte sizes differ (RU Cyrillic is ~2×):
`1`=28, `2`=4, `common`=506, `dialogs0`=322, `dialogs1`=258, `messages`=40,
`quests`=124, `rumours`=77, `skills`=102. So record *index i* is the same concept
in both languages — the English strings overlay the baseline 1:1.

## Build axis — which blobs are shared vs. resolution/region-specific

CRC-32 compared across all four surviving builds (128×160 Etty repack, 176×220
baseline, 240×320 reduced, EN v11):

- **`res/1.map` .. `res/12.map` — byte-identical across ALL four builds.** The
  campaign level terrain is build-independent: **parse once.** (`net0..4.map`,
  `101.map` = multiplayer, only in the two full builds.)
- **`res0.pak`, `res3.pak`, `res4.pak` — distinct in every build** (per-device
  sprite atlases; `res0` also carries the resolution-specific bitmap font, and
  `res3/res4` carry text-baked images that also differ RU↔EN).
- **`res1.pak`** shared by {128, 176, EN}, differs only in 240.
  **`res2.pak`** shared by {176, 240, EN}, differs only in 128.
  **`scenes.pak`** shared by {128, 176, EN}, differs only in 240.
- **`sincos/sin.int`, `sincos/cos.int`** shared by {176, 240, EN}; the 128 Etty
  repack regenerated them (values differ — likely rescaled; the baseline table is
  the authority, scale 10000).

---

## `res/N.map`, `res/netN.map`, `res/101.map` — level terrain grid

- **Role:** the tile grid for a level. **Terrain only** — the file carries *no*
  object/NPC/trigger/exit placement (see below).
- **Consumer:** class **`m`**, method **`a(int i, int i2)`** [jadx `m38a`]. `i` =
  level index; when `m.d` (multiplayer) the name is `/res/net<i>.map`, else
  `/res/<i+1>.map`. Read with a raw `DataInputStream`.
- **Endianness:** header `u16` is **little-endian** (`m.a(byte[],int)` returns
  `b[i] + (b[i+1]<<8)`); body is raw bytes.

```text
offset  size            field
0       u16 LE          width   (m.f203n)     low byte first; high byte 0 in corpus
2       u16 LE          height  (m.f204o)
4       width*height    cells   1 byte/cell, row-major: height rows × width cols
```

File length is **exactly** `4 + width*height` for all 18 maps (verified) — there
is nothing after the grid.

**Cell decode (engine interpretation, R10).** Each stored byte lies in the closed
range **128..173** (verified across every map). The loader computes
`code = (byte)(rawByte - 128)` → `code ∈ 0..45`, then looks up the real tile id
`tile = m.f251o[code]`, where `m.f251o` is a fixed 46-entry table hardcoded in
`m`'s constructor: `{26,44,46,33,47,48,27,28,29,30,31,36,37,38,39,40,41,42,43,36,
37,38,39,40,41,42,43,49,50,51,60,61,52,53,55,32,56,45,36,58,54,59,26,57,35,34}`.
`m.b(i,j)` then uses the tile id to stamp passability flags (`m.f213c`). A Rust
parser only needs `width,height,cells[]`; the `-128` shift, `f251o` remap and
passability stamping are game-side.

**Placement is in code, not in the file.** After loading the grid, `m.a(int,int)`
dispatches per campaign level to hardcoded methods `T()`/`U()`/`V()`/…/`ae()`
(one per level `i=0..11`) that spawn units, items and triggers. Multiplayer maps
(`m.d`) skip this. So `.map` recovery = terrain only; entity placement must be
lifted from the `m` bytecode separately.

## `res/*.utf` — localization string tables  *(HIGH priority: EN overlay)*

- **Files:** `1, 2, common, dialogs0, dialogs1, messages, quests, rumours,
  skills`.
- **Role:** all in-game text (credits, help, menus, dialogue, quests, rumours,
  skill/creature descriptions, system messages).
- **Container:** a **flat concatenation of Java `readUTF()` records**, tiling the
  whole file to EOF with **no** count header, offset table, or encoding byte.
  Each record:

```text
u16 BE  byte_length          (big-endian; 0 = an empty record, used as a delimiter)
byte_length bytes            Java *modified* UTF-8 (== standard UTF-8 for all
                             corpus text: BMP, no NUL) — Cyrillic is 2 bytes/char
```

- **Encoding evidence:** every file opens via `ae.a(InputStream)` → a plain
  `DataInputStream`, and every consumer reads it with `readUTF()`
  (`ae.m3a(DataInputStream)` = one record). RU records decode as valid UTF-8
  Cyrillic; the record stream tiles to exact EOF. There is **no** custom charset
  and **no** glyph-index scheme in this container (glyph mapping is the separate
  bitmap-font descriptor, below).
- **Record grammar is per-file (decided by the consumer), layered on the flat
  stream:**
  - `1.utf`, `2.utf` — credits / intro help. `g.a(int)` [jadx cases 3/4] calls
    `ae.a(DataInputStream, 0)`: read records, joining with `'\n'`, until an empty
    record; then word-wrapped for display.
  - `messages.utf` — `g.a(int)` [case 7] calls `ae.a(DataInputStream, t)`: skip
    `t` empty-delimited sections, then read one section. `t` = message id.
  - `dialogs0.utf` / `dialogs1.utf` — `g.a(int)` [case 6] opens
    `/res/dialogs<t/55>.utf` and calls `ae.m4a(DataInputStream, t%55)`: blocks are
    separated by **double** empty records; skip to block `t%55`, read records to
    the next empty. So 55 dialogs per file, dialogue id `t`.
  - `common.utf` — class **`aj`**, constructor. **28 count-prefixed groups**
    (`aj.a … aj.B`): each group starts with one record that is a *decimal count
    string* (`Integer.parseInt`), followed by that many string records. Records
    beginning with `"//"` are **comments** and are skipped (`aj.a()` loop).
  - `quests.utf`, `rumours.utf`, `skills.utf` — read by `g` (methods around the
    quest/rumour/skill screens) as the same empty-record-delimited sections.
- **Cross-check:** record counts match RU↔EN for all nine files (table above) —
  the R11 language-axis identity. Reading record *i* against the EN build pins the
  meaning of RU record *i*.

## `res/res0.pak` .. `res/res4.pak` — XOR-packed sprite/data packs

- **Role:** packed atlases of complete PNG images (plus, in `res0`, the bitmap
  font atlas image + its descriptor blob).
- **Consumer:** class **`j`** — `j.a(String, byte key, Class)` opens & reads the
  directory; `j.a()` (no-arg `byte[]`) [jadx `m14a`] returns the *current* entry
  (decrypting); `j.a(int n)` skips `n` entries; `j.a()` (void) closes. Callers
  (`g.a(int)` case 8 for `res1..4`; `m.a(String,byte,Class)` in the ctor for
  `res0`) pass **key `(byte)83` = `0x53` = ASCII `'S'`**.
- **Endianness:** big-endian (`readInt`).

```text
i32 BE  count
count × i32 BE  length[i]           entry byte-lengths (the directory; addressed
                                    by sequential index — no names, no offsets)
then concatenated, back to back:
  entry i : length[i] bytes, each byte XOR 0x53   -> a complete PNG
```

File length is **exactly** `4 + 4*count + Σ length[i]` (verified, all five packs),
and `entry0 ^ 0x53` begins with the PNG signature `89 50 4E 47`. To reach entry
*N* the game `skipBytes(length[k])` for `k<N` (`j.a(int)`), so a parser must walk
the directory cumulatively. `g` builds `Image.createImage` from each decrypted
byte[]; entry order = sprite-slot order for that pack.

**Font descriptor (special entries of `res0.pak`).** In `m`'s ctor: entry 0 = a
PNG (`m.f173a`); entry 1 = the font **atlas** PNG; entry 2 = the font
**descriptor** byte[], parsed by `ae.a(DataInputStream)` as five `readUTF`
records: `[0]` the glyph string (all characters in atlas order, e.g. the Cyrillic
alphabet — verified `"АБВГДЕЖЗИЙКЛ…"`), `[1]` glyph cell width (decimal string,
`"10"`), `[2]` cell height (`"13"`), `[3]` atlas columns (`"12"`), `[4]`
space-separated per-glyph advance widths. This descriptor is *not* a standalone
resource file; it rides inside `res0.pak`.

## `res/scenes.pak` — per-level scripting data (NOT an XOR pak)

Despite the `.pak` extension this is a **different format** — big-endian nested
`i32` arrays, **not** the class-`j` XOR container.

- **Consumer:** class **`m`**, method **`g(int i)`** (`i` = level index). Raw
  `DataInputStream`, `readInt()` throughout (**big-endian**).
- **Structure:** a flat sequence of per-level scenes; to reach scene `i` the
  loader skips `i` scenes. Each scene = **three groups**, each group a ragged
  int-array table:

```text
per scene, 3 groups; each group:
  i32 BE  rows
  rows × ( i32 BE len ; len × i32 BE )
```

The three groups populate `m.f197e`, `m.f196d`, `m.f195c`. Group 3's rows are
spawn records: `row[0]`=type, `row[1..]` fed to `new n(...)` (an entity —
x,y,w,h,flags,…, 9–10 ints). The third group is read only when `m.f249m == null`,
but is always present on disk (each scene = exactly 3 groups). No top-level count;
scene index = level index. First ints of scene 0 in the baseline: `19, 4, …`.

## `sincos/sin.int`, `sincos/cos.int` — trig lookup tables

- **Role:** fixed-point sine/cosine for isometric/rotation math.
- **Consumer:** class **`o`**, constructor `o()` reads both via
  `getResourceAsStream("/sincos/sin.int" | "/sincos/cos.int")` into
  `short[90] o.a` / `short[90] o.b` with `readShort()` (**big-endian i16**).
- **Layout:** `90 × i16 BE`, one entry per **degree 0..89** (a quarter turn), 1°
  step. **Scale = 10000** (`o.a()` returns the constant `10000`): stored value =
  `round(10000 · sin(deg))` / `round(10000 · cos(deg))` — verified with **max
  error 0** against the ideal curve. `sin[0]=0, sin[45]=7071, sin[89]=9998`;
  `cos[0]=10000, cos[45]=7071, cos[89]=175`.
- **Full circle** is reconstructed by quadrant folding in `o.a(short deg)` (sine)
  and `o.b(short deg)` (cosine), which normalise `deg` into `[0,360)` and mirror
  the quarter table with the right sign — e.g. `sin(90..179)=cos[deg-90]`,
  `sin(180..269)=-sin[deg-180]`, etc.

## Plain-text tables — `res/abilities.txt`, `res/rewards.txt`

Human-readable ASCII **TSV**, not binary. Consumer: `m.a(int[][] cache, int row,
String name, int ncols)` (`rewards` via `m.m43a`, `ncols=44`; `abilities` via
`q.a(int)`, `ncols=57`). Format: lines end `\r\n`; a line starting `//` is a
comment (the header row is `//name\t…` / `//\t…`); each data row is a name column
then `ncols` **tab-separated signed integers** (parser reads to first `\t`, then
accumulates digits, `-` flips sign, advancing one column per non-digit). Columns
are stat vectors — `abilities.txt` = per-creature stat rows (cur hp, mp, str,
dex, resistances, …); `rewards.txt` = per-reward payout rows (exp, gold, armour,
weapon counts, …).

## Standard formats (no custom wrapper)

- **`.png`** (`res/icon.png`, and the decrypted `.pak` entries) — standard PNG,
  signature `89 50 4E 47 0D 0A 1A 0A` (verified). Consumed by
  `Image.createImage`.
- **`.mid`** (`res/bgsound.mid`, `res/bgsound1.mid`) — standard SMF, `MThd`
  header (verified). Loaded raw into memory by the static
  `j.a(String, Class)` (a byte-count-then-`readFully` slurp, **no** XOR) and
  played via MMAPI `Manager.createPlayer(..., "audio/midi")` (R9). No parser
  needed.

## Reproduce / regenerate

```sh
# inside the nix dev shell (see CLAUDE.md):
python3 tools/java/decompile.py            # -> _reference/decompile/176x220/{jadx,cfr}/
python3 tools/corpus/inspect_formats.py    # re-derives + checks every framing fact above
```
