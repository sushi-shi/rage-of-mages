# Device-runtime API surface — Rage of Mages (Allods, Nival, J2ME 2D)

Status: **Phase 1, reversed from the baseline bytecode.** This is the exact
`javax.microedition.*` / Nokia / device-relevant `java.*` surface the baseline
`allods_176x220.jar` (~37 obfuscated classes, launching `Container/GameMIDlet`)
actually calls — the closed contract the future `rage-me` device-runtime crate
must model and *otherwise reject* (rulebook R9/R10, playbook
`docs/ARITHMETIC_AND_RUNTIME.md` Part B).

Every count below is from `javap -c -p -constants` over the extracted `.class`
files (the authority for API use); every behavioural claim cites the class letter
and method as they appear in the git-ignored decompile
`_reference/decompile/176x220/{jadx,cfr}/` (regenerate with
`python3 tools/java/decompile.py`). Obfuscated class letters match the bytecode;
jadx field aliases (e.g. `f214a`) are quoted where they aid reading. Counts are
static call-site counts (constant-pool references), not runtime frequencies.

## Class map for this document

| letter | role (this doc) |
|--------|-----------------|
| `Container/GameMIDlet` | the `MIDlet` (lifecycle) |
| `m` | **the canvas + game loop** — `extends com.nokia.mid.ui.FullCanvas implements Runnable`. Owns paint, input, audio, RMS, the game clock. |
| `g` | screen/overlay objects (pushed on a stack `m.f193a`); credits `drawString` |
| `ae` | text/clip helper — caches the "current `Graphics`" statically |
| `r` | base drawable superclass of most screens; static back-ref `r.a = m` |
| `e`, `b` | `Runnable` worker threads (Bluetooth multiplayer I/O) |
| `t` | `implements javax.bluetooth.DiscoveryListener` (multiplayer) |
| `j` | resource loader (`getResourceAsStream` + `.pak`/`.mid` slurp) |
| `o` | sincos tables (`getResourceAsStream` of `.int`) |

---

## 1. `javax.microedition.lcdui` — Graphics / Image / Font

### 1.1 `Graphics` — methods used (all sites; caller mostly `m`, some `g`)

| method | signature | count | notes |
|--------|-----------|------:|-------|
| `drawImage` | `(Image;III)V` | **177** | `(img, x, y, anchor)`. The workhorse blit. |
| `setClip` | `(IIII)V` | **110** | clip is set constantly; **no `clipRect`** — clip is always *replaced*, never intersected. |
| `setColor` | `(III)V` | 49 | r,g,b form dominates. |
| `fillRect` | `(IIII)V` | 26 | |
| `drawRect` | `(IIII)V` | 13 | |
| `drawString` | `(String;III)V` | 7 | **default font only** (see 1.3); all 7 in class `g` (a credits scroll: same string at x = 9, −149, −307 … anchor 20). |
| `setColor` | `(I)V` | 5 | 0x00RRGGBB form. |
| `drawArc` | `(IIIIII)V` | 4 | |
| `getClipX/Y/Width/Height` | `()I` | 3 each | read-back of current clip. |
| `drawLine` | `(IIII)V` | 2 | |
| `translate` | `(II)V` | 1 | |
| `fillArc` | `(IIIIII)V` | 1 | |

**Not used:** `clipRect`, `drawRegion`, `copyArea`, `getColor`, `drawRoundRect`,
`fillRoundRect`, `drawChar(s)`, `setStrokeStyle`, `getDisplayColor`,
`drawRGB`. The runtime must **reject** these (R10).

**Anchor constants (integer literals passed to `drawImage`; no named-constant
field is ever referenced).** Distinct anchors observed: `0`, `20`, `24`, `36`,
`3` →
`0` (top-left, permissive), `20`=`TOP|LEFT`, `24`=`TOP|RIGHT`, `36`=`BOTTOM|LEFT`,
`3`=`HCENTER|VCENTER`. Closed anchor set = **{0, 3, 20, 24, 36}**.

### 1.2 `Image` — factories / accessors

| member | signature | count | notes |
|--------|-----------|------:|-------|
| `getHeight` | `()I` | 16 | |
| `getWidth` | `()I` | 13 | |
| `getGraphics` | `()Ljavax/…/Graphics;` | 4 | off-screen buffers. |
| `createImage` | `(II)Image;` | 4 | blank w×h mutable image. |
| `createImage` | `([BII)Image;` | 3 | **decode PNG from `byte[]`** (`m.f()` init: `res0.pak` entries; `g` for `res1..4`). |

**Not used:** `createImage(Image, x,y,w,h, transform, ...)` (region/rotate),
`createImage(InputStream)`, `createRGBImage`, `createImage(String)`. Reject.

Sprite pipeline: PNG `byte[]` → `Image.createImage([B,0,len)` → `DirectGraphics.
getPixels(…, 4444)` into a `short[]` → cached and blitted with `drawPixels`
(§2). So most in-game pixels never go through `Graphics.drawImage`; they go
through Nokia `drawPixels`.

### 1.3 `Font` — **NOT USED AT ALL**

`javap` finds **zero** references to `javax.microedition.lcdui.Font`,
`getFont`, `setFont`, `stringWidth`, `charWidth`, `getHeight`,
`getBaselinePosition`, or any face/style/size constant. Consequences:

- All *measured* in-game text uses the game's **own bitmap font** (the `res0.pak`
  glyph atlas + descriptor decoded in `docs/FORMATS.md`), rendered via
  `drawPixels`/`drawImage`. `stringWidth` etc. are computed from the descriptor's
  per-glyph advance table in game code, never from `Font`.
- The only `Font` dependency is the **default device font** implicitly bound to
  the 7 `Graphics.drawString` calls in class `g` (credits/help text). The game
  **never queries its metrics**, so R11's "model `Font` metrics so `stringWidth`
  = Σ`charWidth`" is effectively **moot here** — there is no metric feedback loop
  to honour. `rage-me` needs only a substitute default face that `drawString`
  can render; its advances are unobservable to the program.

---

## 2. Nokia `com.nokia.mid.ui.*` — **used heavily** (this is the real blitter)

The base class is **Nokia `FullCanvas`** (`m extends
com.nokia.mid.ui.FullCanvas`), i.e. the game is a Nokia full-screen canvas, and
pixel work goes through `DirectGraphics`, not standard `Graphics`.

| member | signature | count | notes |
|--------|-----------|------:|-------|
| `DirectUtils.getDirectGraphics` | `(Graphics;)DirectGraphics;` | 24 | wraps the paint `Graphics` per draw. |
| `DirectGraphics.drawPixels` | `([SZIIIIIIII)V` | **25** | **`short[]` ARGB4444** sprites (format `4444`). |
| `DirectGraphics.drawPixels` | `([IZIIIIIIII)V` | 3 | **`int[]` ARGB8888** (format `8888`) — full-screen fade/darkness overlays. |
| `DirectGraphics.getPixels` | `([SIIIIIII)V` | 3 | read Image → `short[]` `4444` (sprite bake). |
| `DirectGraphics.getPixels` | `([IIIIIIII)V` | 1 | read into `int[]` `8888`. |
| `DirectGraphics.drawImage` | `(Image;IIII)V` | 2 | Nokia anchored image draw. |
| `FullCanvas.<init>` | `()V` | 1 | `m`'s super-ctor. |

**Pixel formats actually passed (closed set):**
- `4444` = `TYPE_USHORT_4444_ARGB` (16-bit, the sprite/atlas format; almost always
  with `transparency = true`).
- `8888` = `TYPE_INT_8888_ARGB` (32-bit, overlays).

**Manipulation (transform) arg (closed set):** `0` (identity) everywhere except
one site using **`8192` = `FLIP_HORIZONTAL`** (`m.d(Graphics)` region, sprite
mirroring). No rotate/vertical-flip constants appear.

`rage-me` must model `DirectGraphics` `drawPixels`/`getPixels` for exactly these
two formats and these two manipulations, and reject other formats
(`444`, `565`, `1555`, `888`, `8888` variants beyond ARGB) and other transforms.

---

## 3. `Canvas` / `Displayable` / `Display` — input & paint model

### 3.1 Input is **event-driven, not polled**

`getKeyStates()` is **never called** (zero refs). Input arrives only through the
overridden `m.keyPressed(int)` and `m.keyReleased(int)`. `getGameAction(int)` is
called 3× (in `keyPressed`, `keyReleased`, and the menu-combo matcher `m.j(int)`).

**`m.keyPressed(int i)`** (evidence: `javap` tableswitch + jadx `m.java:6053`):
1. sets `f170p = true` **unconditionally** ("any key pressed" latch — so the
   canvas accepts *any* int without throwing);
2. `if (i == -6) f168n = true;` (left softkey latch);
3. `if (i == -7) { f169o = true; return; }` (right softkey latch);
4. else store raw code `f171h = i` (field `h:I`), then `switch(getGameAction(i))`:

   | game action | value | fields set on press |
   |-------------|------:|---------------------|
   | UP    | 1 | `f160f`=true (held), `f164j`=true (latch) |
   | LEFT  | 2 | `f159e`=true (held), `f163i`=true (latch) |
   | RIGHT | 5 | `f162h`=true (held), `f166l`=true (latch) |
   | DOWN  | 6 | `f161g`=true (held), `f165k`=true (latch) |
   | FIRE  | 8 | `f167m`=true (latch) |
   | 3,4,7, GAME_A..D (9–12), default | — | no-op |

**`m.keyReleased(int i)`**: `-6`/`-7` return immediately (softkeys have no
release action). Else `switch(getGameAction(i))` clears only the **held**
directional booleans: UP→`f160f`, LEFT→`f159e`, RIGHT→`f162h`, DOWN→`f161g`.
FIRE and the "latch" set (`f163i..f166l`, `f164j`, `f167m`, `f168n`, `f169o`) are
**edge-triggered** — set on press, consumed and reset by the run loop, never
cleared on release.

**Raw numeric keys** feed a separate consumer, the combo/gesture matcher
`m.j(int)` (`javap` `m.j`, jadx `m.java:6360+`): it matches the retained raw code
`f171h` (and/or `getGameAction(f171h)`) against a hard-coded table
`f209h` (`h:[[I`) of ASCII key-code sequences. Distinct table codes:
**`35`(`#`), `49`–`56`(`1`–`8`)**. Separately, `f171h == 42` (`*`) is tested in
`m.java:853` and `g.java:3710` (a toggle). No `0`(48), `9`(57) in the table.

**Closed key-code set (R10) the runtime must accept, and otherwise reject:**
- `-6` (left softkey), `-7` (right softkey) — handled by raw code;
- any device code whose `getGameAction()` ∈ **{UP=1, DOWN=6, LEFT=2, RIGHT=5,
  FIRE=8}**;
- raw ASCII **`49`–`56` (`1`–`8`)** and **`35` (`#`)** — combo/gesture table;
- raw **`42` (`*`)** — map/menu toggle.

Any other keycode only flips the generic `f170p` "any-key" latch (used by
"press any key" screens) and produces no game-state change. `rage-me` should
model `keyPressed`/`keyReleased` delivery of the raw code, treat the above as the
behaviour-bearing closed set, and reject codes outside the device's own valid
range. **No named `Canvas.KEY_*`/`UP`/`DOWN` constant field is ever referenced** —
the game hard-codes integer literals and relies on `getGameAction` for mapping.

Not used: `keyRepeated`, `pointerPressed/Released/Dragged` (no touch),
`setFullScreenMode` (`FullCanvas` is already full-screen).

### 3.2 Paint model (R9 — MIDP serialized paint/input)

- **`m.paint(Graphics g)`** (`m.java:773`) does a **full redraw every time**:
  `ae.a(g)` (bind current `Graphics` into the static text helper) → `r(g)` (base /
  map) → the **top of the screen stack** `f193a` (`((g)f193a.lastElement()).a(g)`)
  or, if the stack is empty, `a(g)` (main game view). It never assumes retained
  pixels — satisfying "every Paint must fully redraw" (the runtime must supply a
  fresh clip == full screen each paint; there is no partial-repaint contract).
- **Game loop drives paint synchronously**: `m.run()` does, once per frame,
  `repaint(); serviceRepaints();` (`javap` offsets 128/132; only when not quitting).
  Counts: `Canvas.repaint` = 5, `Canvas.serviceRepaints` = 5. This is the exact
  `repaint(); serviceRepaints();` idiom the R9 seam test must reproduce: the owed
  paint is serviced **before** the next key is routed, repaints coalesce, and
  `serviceRepaints` consumes the owed paint before the loop sleeps.

### 3.3 `Display` / focus / lifecycle-adjacent

- `Display.getDisplay(MIDlet)` = 1 (in `GameMIDlet.<init>`), `Display.setCurrent(
  Displayable)` = 1 — the canvas `m` is shown once. **Attach the canvas / listeners
  once** (R9): there is a single `setCurrent`.
- **`m.showNotify()`** (focus gained): unpause (`c=false`) and **restore audio
  volume** (`i(f247k[2])`).
- **`m.hideNotify()`** (focus lost / interrupted): **pause** (`c=true`, which the
  run loop checks: it keeps spinning but skips update+`repaint` while `c`) and
  **mute** (`i(0)`). This — not `pauseApp` — is the real app-switch pause.

---

## 4. `javax.microedition.media` (MMAPI) — single background-music channel

Two MIDI players pre-created in `m.f()` (init, `m.java:666`):
`f214a[0]` = `res/bgsound.mid`, `f214a[1]` = `res/bgsound1.mid`, each fed a
`ByteArrayInputStream` over the raw `.mid` slurp (`j.a(name, class)`), content
type **`"audio/midi"`**.

| member | signature | count | notes |
|--------|-----------|------:|-------|
| `Manager.createPlayer` | `(InputStream;String;)Player;` | 2 | the two tracks. |
| `Player.realize` | `()V` | 1 | both, at init (loop of 2). |
| `Player.prefetch` | `()V` | 1 | both, at init. |
| `Player.setLoopCount` | `(I)V` | 1 | `-1` = **infinite loop** on both. |
| `Player.start` | `()V` | 1 | in `m.i(int)`. |
| `Player.stop` | `()V` | 1 | in `m.i(int)`. |
| `Controllable.getControl` | `(String;)Control;` | 1 | `"VolumeControl"`. |
| `VolumeControl.setLevel` | `(I)I` | 2 | |
| `VolumeControl.setMute` | `(Z)V` | 2 | |

**`setMediaTime` is NOT called** (zero refs). Do not model a `setMediaTime(0)`
rewind for this game — the playbook's generic "rewind via stop+setMediaTime(0)"
does not apply here.

**Single-channel-of-state model (matches R9 audio contract):**
- `m.ak` = index (0/1) of the *currently active* track, `-1` = none. `m.al` =
  current volume level. Both tracks are always realized+prefetched; **only one is
  ever `start()`-ed at a time.**
- **`m.h(int i)`** switches the active track: **guarded by `if (ak != i)`** — so
  re-selecting the same track is a **no-op** (this is the "buffer replacement, not
  process kill" property: menu music does not restart on re-entry). On a real
  change it stops the old track (`i(0)`), sets `ak=i`, then `i(al)`.
- **`m.i(int level)`** is the play/stop primitive on the active track: `level>0` →
  `start()`, `setLevel(level)`, `setMute(false)`; `level<=0` → `setLevel(0)`,
  `setMute(true)`, `stop()`. Every call is wrapped in `try/catch(Exception)`
  swallowing failures (faithful: audio is best-effort).

So contention is **priority arbitration** — the inactive track is fully stopped,
never mixed. `rage-me` models one logical MIDI channel of state; the loser is
ignored; audio decodes in-process; a track that decodes to silence must not count
as "playing".

---

## 5. `javax.microedition.rms` (RecordStore) — the save system (R5-critical)

There **is** a full save system. `javap` counts: `openRecordStore(String;Z)` = 8,
`setRecord` = 13, `getRecord` = 11, `addRecord` = 11, `closeRecordStore` = 8,
`deleteRecordStore(String)` = 5. Three stores:

| store name | records | contents | writers / readers |
|------------|--------:|----------|-------------------|
| `"system"` | 2 | rec1 = `f247k` (settings byte[]; `f247k[0]`=current slot id, `f247k[2]`=volume), rec2 = `f210g` (options/flags byte[]) | `m` (`m.java:4574` read, `4589`/`4618` write) |
| `"" + f247k[0]` (slot id as decimal string) | 7 | the save game — see field order below | `m.N()` write (`m.java:4375`), `m.O()` read (`m.java:4411`), `m.getRecord(6)` summary read (`4544`) |
| `"net"` | 2 | rec1 = `m13a()` blob, rec2 = `m34a()` (multiplayer roster) | `g` (`g.java:4529` write, `4564` read) |

Stores are always created by `openRecordStore(name, true)` + N× `addRecord(null,
0,0)` (reserve N empty records) + `setRecord(i, …)` (fill by 1-based index). This
fixed-slot layout (never `addRecord` of real data) is the save wire's outer shape.

### 5.1 The save wire — **there is no `DataOutputStream`**

`java.io.DataOutputStream` is **never used** (0 refs). Records are packed **by
hand** with four helpers on `m` (`m.java:4101–4144`):

- **`b(int v, byte[] a, off)`** — write int: `v2 = v - Integer.MIN_VALUE`, then 4
  bytes **little-endian** (`a[off]=v2&255; v2>>=8; …`).
- **`c(byte[] a, off)`** — read int: 4 bytes LE (unsigned-extended), return
  `acc - Integer.MIN_VALUE`.
- **`a(long v, byte[] a, off)`** — write long: `v2 = v - Long.MIN_VALUE`, 8 bytes
  **little-endian**.
- **`m33a(byte[] a, off)`** — read long: 8 bytes LE, `- Long.MIN_VALUE`.

So every int/long on the save wire is **offset-binary (excess-2³¹ / excess-2⁶³),
little-endian**: stored = `value − MIN_VALUE` (≡ flipping the sign bit) in LE byte
order. `byte[]` and boolean fields are stored raw, 1 byte each. This bias is
non-obvious and must be reproduced bit-for-bit (R5/R8): a plain LE int32 would
diverge by 0x80000000.

### 5.2 Field order per save record (from `m.N()` / the `m##`-serializers)

**Slot store, records 1–7** (`m.N()`, `m.java:4375`):

- **rec 1 = `m34a()`** (units/objects, `m.java:4175`): `b(ae)` (int) · `b(f242m.
  size())` (int) · `f242m.size()` × `ab` record (`ab.c()` bytes each, packed by
  `ab.a(byte[],off,m)`) · `b(f240k.size())` (int) · that many `ab` records ·
  `b(f241l.size())` (int) · that many `ab` records.
- **rec 2 = `m35b()`** (hero/roster stats, `m.java:4200`): raw bytes of `f176a[]`,
  then `f177b[]`, then `f178a[]` (each element an 8-byte long via `a(long,…)`),
  then `f179c[]`, `f180d[]`, `f181e[]`, `f182f[]` (each raw 1-byte), then `b(ag)`
  (int), then 1 byte `f243v` (boolean), then `a(f277i)` (long) at the end.
- **rec 3 = `m36c()`** (entity vector `f239j`, `m.java:4272`): `b(f239j.size())`
  (int) · each `n` entity packed by `n.a(byte[],off)` for `n.m45a()` bytes.
- **rec 4 = `m37d()`** (entity vector `f195c`, `m.java:4306`): same shape as rec 3
  over `f195c`.
- **rec 5 = `f245j`** — raw byte[] (fog/visited-tile bitmap; built in `m.a(int[])`).
- **rec 6 = 25-byte slot summary** (built inline at `m.java:4396`):
  off0 `b(f243v ? ag : 200)` (int) · off4 `b(ae)` (int) · off8 byte
  `(f202b.f303a.i[8] + 1)` (hero level) · off9 `a(f277i)` (long, playtime) · off17
  `a(System.currentTimeMillis())` (long, save timestamp).
- **rec 7 = `f250n`** — raw byte[].

Readers mirror the order exactly: `m.b(byte[])` (rec1), `m.e(byte[])` (rec2, using
`c`/`m33a`), rec3/rec4 kept raw as `f248l`/`f249m` then parsed lazily by
`m.c/d(byte[])`, rec5 via `m.a(byte[])`, rec7 into `f250n`.

**`"system"` store**: rec1 = `f247k` raw byte[], rec2 = `f210g` raw byte[]
(no int packing — plain arrays).

`rage-me` reproduces this byte layout exactly for compatibility validation even
though the modern engine will not keep RMS on-disk compatibility.

---

## 6. `javax.microedition.midlet` — MIDlet lifecycle

`Container/GameMIDlet extends MIDlet`. Members:
`<init>` = 1 (constructs, calls `Display.getDisplay`), `startApp()`,
`pauseApp()` (**`final`, empty** — pause is really `m.hideNotify`),
`destroyApp(boolean)`, `quit()` → `MIDlet.notifyDestroyed()` (= 1). The run loop
exits on the `m.A` flag; `notifyDestroyed` tells the AMS it may reclaim.

**`getAppProperty` is NOT used** (0 refs). The game reads no JAD/manifest
properties at runtime — build/region/resolution differences are baked into the
resources and code, not queried. Reject `getAppProperty`.

---

## 7. Device-relevant `java.*`

| member | count | device relevance |
|--------|------:|------------------|
| `System.currentTimeMillis` | 14 | **the game clock** — frame timing (`m.run`), `Random` seeding, save timestamp. |
| `Thread.start` | 3 | three threads: `m` (game loop, `m.f137a`, started in `m.f()`), `e` and `b` (`new Thread(this).start()` — Bluetooth I/O). |
| `Thread.sleep` | 3 | frame pacing (`m.run`) + worker back-off. |
| `Runnable` | — | implemented by `m`, `e`, `b`. |
| `System.gc` | 10 | forced after each big resource load (device memory pressure); observable pauses only. |
| `Random.nextInt` | 23 | gameplay RNG. |
| `Random.setSeed` | 1 | `f138a.setSeed(currentTimeMillis())` in `m.f()` — **seed the LCG from the clock**; faithful `java.util.Random` (48-bit LCG) required (R8). |
| `DataInputStream.readInt` | 14 | big-endian resource parse (`.pak`, `scenes.pak`, `.map` body). |
| `DataInputStream.readByte` | 13 | |
| `DataInputStream.readUTF` | 10 | `.utf` text records. |
| `DataInputStream.readShort` | 2 | sincos `.int`. |
| `DataInputStream.skipBytes` / `readFully` / `close` | 1 / 1 / 7 | pack directory walk / `.mid` slurp. |
| `Class.getResourceAsStream` | 17 | loads `/res/*.utf`, `/res/*.pak`, `/sincos/*.int`, `/res/*.mid`, `/res/*.map`. |
| `Math.abs/min/max` | 50 / 39 / 39 | pure (not device); noted for completeness. |

Reading direction of resources is **big-endian** `DataInputStream` (readInt/
readShort/readUTF), **except** the hand-rolled little-endian helpers for `.map`
headers and the RMS save wire (§5.1) — consistent with `docs/FORMATS.md`.

---

## 8. The game-loop / threading model (summary)

`m.run()` (`javap` line ~38623) is the single game thread:

```
publish static back-refs (r.a = n.a = g.a = q.a = this)
a = currentTimeMillis(); state = 1; f()            // init
while (!A) {                                        // A = quit flag
    a = currentTimeMillis()
    if (!c) {                                       // c = paused (hideNotify)
        <advance frame counter d, wrap 9→-1>
        b = a + 62                                  // 62 ms frame deadline (~16 FPS)
        c()                                         // update
        i()                                         // update
        if (!A) { repaint(); serviceRepaints(); }   // R9 serialized paint
    }
    a = currentTimeMillis()
    c = b - a                                        // remaining budget
    if (c < 1) c = 1
    try { Thread.sleep(c) } catch (Exception) {}     // pace to deadline
}
return
```

- **Fixed 62 ms frame period** (target ≈16 fps); update runs, then a synchronous
  `repaint(); serviceRepaints();`, then sleep the remainder (min 1 ms). The whole
  loop is wrapped so any exception per frame is swallowed and the loop continues.
- **Pause** is data (`c`), toggled by `hideNotify`/`showNotify`; the thread never
  dies on pause, it just idles.
- Two extra `Runnable` threads (`e`, `b`) exist **only for Bluetooth multiplayer**.

## 9. Bluetooth multiplayer (JSR-82 + `javax.microedition.io`) — present, scoped

Not part of the single-player device contract, but real and device-relevant:
`Connector.open(String)` (2), `L2CAPConnection` `send`/`receive`/`ready`/
`getReceiveMTU` (7/4/4/4), `DiscoveryAgent.startInquiry`/`searchServices`/
`cancel*`, `LocalDevice`, `ServiceRecord` (SDP attribute build), `DataElement`,
`UUID`, `Connection.close` (4). Driver classes: `t` (`DiscoveryListener`), `e`,
`b`. Only reached on the multiplayer path (`m.d`); the campaign baseline does not
enter it. A single-player `rage-me` may stub/reject JSR-82 entirely but must not
mistake it for JSR-82-over-serial or GCF sockets — it is **L2CAP Bluetooth**.

---

## 10. What `rage-me` must model / must reject

**Must model:**
- **`Graphics`**: `drawImage(img,x,y,anchor)`, `setClip` (replace, never
  intersect), `setColor(r,g,b)`/`setColor(rgb)`, `fillRect`, `drawRect`,
  `drawLine`, `drawArc`/`fillArc`, `translate`, `getClip{X,Y,Width,Height}`,
  `drawString` (default font). Anchors {0,3,20,24,36}.
- **`Image`**: `createImage([B,off,len)` (PNG decode), `createImage(w,h)`,
  `getGraphics`, `getWidth`/`getHeight`.
- **Nokia `DirectGraphics`/`DirectUtils`**: `getDirectGraphics`, `drawPixels`
  and `getPixels` for **ARGB4444 (`short[]`)** and **ARGB8888 (`int[]`)**,
  manipulations **{0, FLIP_HORIZONTAL=8192}**, `DirectGraphics.drawImage`.
- **`FullCanvas`** as the full-screen base; **event-driven** `keyPressed`/
  `keyReleased`; `getGameAction`; `repaint`/`serviceRepaints`; `showNotify`/
  `hideNotify` pause+mute; `Display.getDisplay`/`setCurrent` (once).
- **Serialized paint (R9)**: owed paint before next key; `repaint();
  serviceRepaints();` per frame; full redraw each paint (no retained framebuffer
  reliance).
- **Key-code closed set (R10)**: `{-6, -7}` ∪ {codes → game actions
  UP/DOWN/LEFT/RIGHT/FIRE} ∪ {`35`, `42`, `49`–`56`}. Any other code only sets the
  generic any-key latch.
- **MMAPI single channel**: two prefetched looping (`setLoopCount(-1)`) MIDI
  players, exactly one active (`ak`), `start`/`stop` + `VolumeControl`
  level/mute; re-selecting the active track is a no-op (no restart); **no
  `setMediaTime`**.
- **RMS**: three stores (`"system"`, slot-id-named, `"net"`), reserve-then-`setRecord`
  layout, and the **offset-binary little-endian** save wire (int = `v −
  Integer.MIN_VALUE` LE; long = `v − Long.MIN_VALUE` LE; bytes/booleans raw) in
  the exact field order of §5.2.
- **Timing/RNG**: `System.currentTimeMillis` clock, 62 ms frame budget, faithful
  `java.util.Random`, big-endian `DataInputStream` for resources.

**Must reject (unused — accepting them would be a broader-than-baseline API,
violating R10):** `Font` and all its metrics; `Graphics.clipRect`, `drawRegion`,
`copyArea`, `getColor`, `drawRGB`, `draw/fillRoundRect`, `drawChar`,
`setStrokeStyle`; `Image.createImage` region/rotate/`InputStream`/`createRGBImage`;
`Canvas.getKeyStates` (polling), `keyRepeated`, pointer events,
`setFullScreenMode`; `Player.setMediaTime`; `DataOutputStream`;
`MIDlet.getAppProperty`; DirectGraphics pixel formats other than 4444/8888 and
manipulations other than 0/FLIP_HORIZONTAL; key codes outside the §3.1 closed set.

**Uncertain / to pin later:**
- Exact `ab.c()` byte width and `ab.a/b` field order (save rec 1) and
  `n.m45a()`/`n.a`/`n.b` field order (recs 3/4) are **not decoded here** — they
  live in classes `ab` and `n` and must be reversed before the save wire is
  byte-complete. This doc fixes the *record framing and int/long encoding*; the
  per-element layouts are follow-up work.
- The precise semantics of `res/bgsound*.mid` re-trigger vs. resume across
  `stop()`/`start()` without `setMediaTime` depend on MMAPI player-state rules; the
  faithful model is "stopped track resumes on `start()`", but with
  `setLoopCount(-1)` and the `ak != i` guard the observable effect is continuous
  looping with instant track switches — validate against a frame/audio oracle if
  music timing ever needs to be exact.
- `drawString` default-font pixels are a documented substitute (R11); the program
  never measures them, so any monospace-ish face at a fixed advance is faithful
  to the *observable* behaviour.

---

### Reproduce

```sh
# inside the nix dev shell (see /home/sheep/Projects/j2me/CLAUDE.md):
python3 tools/java/decompile.py                 # -> _reference/decompile/176x220/{jadx,cfr}/
python3 tools/corpus/api_surface.py             # re-derives every count/table above from the jar
```
