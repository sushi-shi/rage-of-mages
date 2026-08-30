//! The `Game` ownership backbone (rulebook R4).
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md,
//! ARITHMETIC_AND_RUNTIME.md Part A).
//!
//! The baseline's singletons collapse into ONE struct: `Container/GameMIDlet`
//! constructs exactly one `m` (the Nokia FullCanvas + game loop + world), which
//! owns one `ae`/`j`/`o`/`aj` and 22 `g` screen objects. Every Java field of
//! those singletons has exactly one persistent owner here; ported methods are
//! FREE FUNCTIONS `fn f(g: &mut Game, ...)`, never `impl` methods.
//!
//! Naming: fields carry the reviewed semantic name where
//! `java/reconstruction/symbols.toml` establishes one (with the obfuscated
//! `(obf: descriptor)` cited), and otherwise the stable jadx identifier
//! verbatim (R10: no guessed semantics). Types follow the primitive mapping
//! (`byte→i8`, `byte[]→Vec<i8>`, `short→i16`, `int→i32`, `long→i64`,
//! reference→`Option<..>`).
//!
//! R4 object-graph notes (cross-links that COLLAPSE under one-owner):
//! - `m.f136a` (the `GameMIDlet` back-ref), `GameMIDlet.f0a` (the `m` ref) and
//!   `GameMIDlet.a` (the Display) — the MIDlet pair is [`Game`] itself plus
//!   [`Game::display`]/[`Game::canvas`].
//! - the statics published by `m.run` (`r.a`, `n.f292a`, `g.a`, `q.f313a` — all
//!   `= this`) and `g.<init>` (`g.f53a = m.f139a`, `g.f54a = m.f140a`),
//!   plus `aj.f26a` and `g.f113a` — every one is a second name for a struct
//!   already on `Game`, so no field exists for them.
//! - `m.f137a` (the `Thread` handle) — single-threaded host: `m.a()V` arms the
//!   loop and the host drives `run_tick` (see `canvas_m`).
//! - `m.f175a` (the persistent `Graphics` over the `f174b` buffer) — a stored
//!   borrow is unrepresentable; the buffer-paint slice derives a frame-local
//!   `Graphics` and must then also model its PERSISTENT translate/clip/color
//!   state (recorded seam, TODO(next-slice)).
//!
//! Scope: this backbone carries every field `m.<init>` initializes plus the
//! fields the ported boot/first-frame slice reads. The ~100 remaining `m`
//! fields (world grids, entity vectors' element types, fog/save scratch, the
//! interface `s` stat tables) join with the slices that exercise them — their
//! JVM-default homes are documented here so later increments do not invent new
//! owners.

use crate::common::CommonTable;
use crate::jrandom::JavaRandom;
use crate::loader::ResourceLoader;
use crate::resource::Resources;
use crate::screens::GScreen;
use crate::text::TextEngine;
use crate::trig::TrigTables;
use j2me_jvm::Clock;
use j2me_me::{Canvas, Display, Image};

/// A Java object reference whose behavior this slice does not model (entities
/// `n`, projectiles, the `ah` cursor object, MMAPI `Player`s, …). It gives the
/// owning field a home (R4); it is typed concretely when the methods that use
/// it are ported. Ported code never dereferences it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unmodeled;

/// Indices into [`M::screens`] for the 22 `g` objects `m.<init>` builds, named
/// by their jadx field on `m` (creation order = kind): `f147b`(0), `f146a`(3),
/// `f148c`(1), `f149d`(2), `f150e`(4), `f151f`(5), `f152g`(6), `h`(7), `i`(8),
/// `j`(9), `k`(10), `l`(11), `m`(12), `n`(13), `o`(14), `p`(15), `q`(16),
/// `r`(17), `s`(18), `t`(19), `u`(21), `v`(22). The Java screen-stack Vector
/// (`m.f193a`) holds ALIASES of these objects; here the stack holds these
/// indices (one owner, R4).
pub mod screen {
    pub const F147B: usize = 0;
    pub const F146A: usize = 1;
    pub const F148C: usize = 2;
    pub const F149D: usize = 3;
    pub const F150E: usize = 4;
    pub const F151F: usize = 5;
    pub const F152G: usize = 6;
    pub const H: usize = 7;
    pub const I: usize = 8;
    pub const J: usize = 9;
    pub const K: usize = 10;
    pub const L: usize = 11;
    pub const M: usize = 12;
    pub const N: usize = 13;
    pub const O: usize = 14;
    pub const P: usize = 15;
    pub const Q: usize = 16;
    pub const R: usize = 17;
    pub const S: usize = 18;
    pub const T: usize = 19;
    pub const U: usize = 20;
    pub const V: usize = 21;
}

/// The mutable statics of class `g` (the per-instance state lives in
/// [`GScreen`]).
#[derive(Debug, Default)]
pub struct GStatics {
    /// `g.n: [I` — a shared scratch table (`None` = null until a screen builds
    /// it; next slice).
    pub n: Option<Vec<i32>>,
}

/// The whole program's state: one owner per Java datum (R4).
pub struct Game {
    /// Host injection: `Class.getResourceAsStream` (the game's [`Resources`]).
    pub resources: Box<dyn Resources>,
    /// Host injection: `System.currentTimeMillis` (j2me-jvm [`Clock`]).
    pub clock: Box<dyn Clock>,
    /// `GameMIDlet.a: Ljavax/microedition/lcdui/Display;` — display.
    pub display: Display,
    /// The Nokia `FullCanvas` base of `m` (the 176×220 baseline device; the
    /// program paints its fixed 176×208 extent).
    pub canvas: Canvas,
    /// Class `m` — the canvas + world + game loop singleton.
    pub m: M,
    /// Class `g`'s mutable statics.
    pub g_statics: GStatics,
}

impl Game {
    /// A `Game` in the JVM's pre-`<init>` zeroed state, wired to its host
    /// providers. `crate::midlet::boot` then runs the recovered constructors.
    pub fn new(resources: Box<dyn Resources>, clock: Box<dyn Clock>) -> Game {
        Game {
            resources,
            clock,
            display: Display::default(), // Display.getDisplay(MIDlet)
            canvas: Canvas::new(176, 220),
            m: M::default(),
            g_statics: GStatics::default(),
        }
    }
}

/// Class `m`'s fields (jadx identifiers, semantic names where symbols.toml
/// establishes them). Grouped by declaration order; `Default` is the JVM
/// zeroed object — `crate::canvas_m::m_init` applies the real `<init>` values.
#[allow(non_snake_case)] // Java fields (V, W, X, Q, T, U, R, S, …) kept verbatim
#[derive(Default)]
pub struct M {
    /// `m.f135a (jadx): B` — init 0.
    pub f135a: i8,
    /// `m.b: Z` — screen-baked flag: true once `m.h()V` has prepared the
    /// current state's backdrop; false while loading (`m.g()V` clears it).
    pub b: bool,
    /// `m.c: Z` — `paused` (symbols.toml): hideNotify sets, showNotify clears.
    pub paused: bool,
    /// `m.A: Z` (private) — `quit_flag` (symbols.toml): the run loop exits on it.
    pub quit_flag: bool,
    /// `m.d: Z` — `multiplayer` (symbols.toml).
    pub multiplayer: bool,
    /// `m.f138a (jadx): Ljava/util/Random;` — `rng` (symbols.toml), seeded from
    /// the clock in `m.<init>`.
    pub rng: JavaRandom,
    /// `m.f139a (jadx): Lae;` — `text_helper` (symbols.toml).
    pub text: TextEngine,
    /// `m.f140a (jadx): Lj;` — `resource_loader` (symbols.toml).
    pub loader: ResourceLoader,
    /// `m.f141a (jadx): Lo;` — `trig` (symbols.toml).
    pub trig: TrigTables,
    /// `m.f142a (jadx): [[S` — `short[15][]` sprite-pixel cache rows.
    pub f142a: Vec<Option<Vec<i16>>>,
    /// `m.g: I` — the game-state selector (init 1 = the logo state; 5 map,
    /// 9 battle, 100 quitting, 101/102 load prompts, 104/106 in-level).
    pub g: i32,
    /// `m.f154a (jadx): J` — the loop's `currentTimeMillis` sample.
    pub f154a: i64,
    /// `m.f155b (jadx): J` — the frame deadline (`f154a + 62`).
    pub f155b: i64,
    /// `m.f156c (jadx): J` — the sleep remainder.
    pub f156c: i64,
    /// `m.f157d (jadx): J` — the frame counter mod 10 (`9 → -1 → 0` wrap).
    pub f157d: i64,
    /// `m.f158e (jadx): J` — the timestamp latched each counter wrap.
    pub f158e: i64,
    /// `m.f159e..f162h (jadx): Z` — held directional keys (cleared by `m.j()V`).
    pub f159e: bool,
    /// See [`M::f159e`].
    pub f160f: bool,
    /// See [`M::f159e`].
    pub f161g: bool,
    /// See [`M::f159e`].
    pub f162h: bool,
    /// `m.f163i..f170p (jadx): Z` — edge-latched keys (cleared by `m.i()V`;
    /// `f170p` is the any-key latch the logo/load screens poll).
    pub f163i: bool,
    /// See [`M::f163i`].
    pub f164j: bool,
    /// See [`M::f163i`].
    pub f165k: bool,
    /// See [`M::f163i`].
    pub f166l: bool,
    /// See [`M::f163i`].
    pub f167m: bool,
    /// See [`M::f163i`].
    pub f168n: bool,
    /// See [`M::f163i`].
    pub f169o: bool,
    /// See [`M::f163i`].
    pub f170p: bool,
    /// `m.f171h (jadx): I` — the last raw key code.
    pub f171h: i32,
    /// `m.f172a (jadx): [Ljavax/microedition/lcdui/Image;` — the 280-slot
    /// sprite image cache.
    pub f172a: Vec<Option<Image>>,
    /// `m.f174b (jadx): Ljavax/microedition/lcdui/Image;` — the 176×208
    /// off-screen backdrop buffer (`createImage(176,208)` in the initializers).
    pub f174b: Option<Image>,
    // `m.f175a: Ljavax/microedition/lcdui/Graphics;` — f174b's persistent
    // Graphics: COLLAPSED (see the module doc; TODO(next-slice) models its
    // persistent translate/clip/color when the buffer paint is ported).
    /// `m.f176a (jadx): [B` — `byte[24]`.
    pub f176a: Vec<i8>,
    /// `m.f177b (jadx): [B` — `byte[169]` (the 13×13 fog-of-war grid).
    pub f177b: Vec<i8>,
    /// `m.f178a (jadx): [J` — `long[12]`.
    pub f178a: Vec<i64>,
    /// `m.f179c (jadx): [B` — `byte[27]`.
    pub f179c: Vec<i8>,
    /// `m.f180d (jadx): [B` — `byte[50]`.
    pub f180d: Vec<i8>,
    /// `m.f181e (jadx): [B` — `byte[13]`.
    pub f181e: Vec<i8>,
    /// `m.f182f (jadx): [B` — `byte[20]` (the one-shot event flags `m16a` tests).
    pub f182f: Vec<i8>,
    /// `m.f189f (jadx): J` — init 0.
    pub f189f: i64,
    // --- ctor body ---
    /// `m.f193a (jadx): Ljava/util/Vector;` — THE SCREEN STACK: aliases of the
    /// 22 screen objects in Java; here indices into [`M::screens`] (R4).
    pub f193a: Vec<usize>,
    /// `m.f194b (jadx): Ljava/util/Vector;` — world entity list (elements next
    /// slice).
    pub f194b: Vec<Unmodeled>,
    /// `m.f195c (jadx): Ljava/util/Vector;`.
    pub f195c: Vec<Unmodeled>,
    /// `m.f196d (jadx): Ljava/util/Vector;`.
    pub f196d: Vec<Unmodeled>,
    /// `m.f197e (jadx): Ljava/util/Vector;`.
    pub f197e: Vec<Unmodeled>,
    /// `m.f198f (jadx): Ljava/util/Vector;`.
    pub f198f: Vec<Unmodeled>,
    /// `m.f199g (jadx): Ljava/util/Vector;`.
    pub f199g: Vec<Unmodeled>,
    /// `m.f200h (jadx): Ljava/util/Vector;`.
    pub f200h: Vec<Unmodeled>,
    /// `m.f201a (jadx): Lah;` — the cursor/selection object (init null).
    pub f201a: Option<Unmodeled>,
    /// `m.f209h (jadx): [[I` (final) — the 15 ambient-sound pattern rows.
    pub f209h: Vec<Vec<i32>>,
    /// `m.f210g (jadx): [B` — `byte[f209h.len]`.
    pub f210g: Vec<i8>,
    /// `m.f211k (jadx): [I` — `int[f209h.len]`.
    pub f211k: Vec<i32>,
    /// `m.f212r (jadx): I` — the screen-shake countdown `m.r(Graphics)` decays.
    pub f212r: i32,
    /// `m.f214a (jadx): [Ljavax/microedition/media/Player;` — `music_players`
    /// (symbols.toml): the two looping MIDI players.
    /// TODO(next-slice): MMAPI realize/prefetch (j2me-me `media` is a stub);
    /// `None` = the ctor's catch arm (no pixel effect).
    pub music_players: [Option<Unmodeled>; 2],
    /// `m.f215b (jadx): [[S` — `short[5][]` baked overlay pixel runs.
    pub f215b: Vec<Option<Vec<i16>>>,
    /// `m.f216r (jadx): Z`.
    pub f216r: bool,
    /// `m.f217s (jadx): I` / `m.f218t (jadx): I` — the tile cursor.
    pub f217s: i32,
    /// See [`M::f217s`].
    pub f218t: i32,
    /// `m.f219i (jadx): Ljava/util/Vector;`.
    pub f219i: Vec<Unmodeled>,
    /// `m.f220c (jadx): Ln;` — the selected unit (init null).
    pub f220c: Option<Unmodeled>,
    /// `m.f221u (jadx): I`.
    pub f221u: i32,
    /// `m.f222v (jadx): I`.
    pub f222v: i32,
    /// `m.w: I`.
    pub w: i32,
    /// `m.x: I`.
    pub x: i32,
    /// `m.y: I` — init 15.
    pub y: i32,
    /// `m.z: I` — init 15.
    pub z: i32,
    /// `m.f223A (jadx): I`.
    pub f223A: i32,
    /// `m.B: I`.
    pub B: i32,
    /// `m.C: I`.
    pub C: i32,
    /// `m.D: I`.
    pub D: i32,
    /// `m.G: I`.
    pub G: i32,
    /// `m.H: I`.
    pub H: i32,
    /// `m.f226b (jadx): B`.
    pub f226b: i8,
    /// `m.M: I`.
    pub M: i32,
    /// `m.P: I`.
    pub P: i32,
    /// `m.f227h (jadx): [B` — `byte[280]` (a sprite-pack residency map).
    pub f227h: Vec<i8>,
    /// `m.f228i (jadx): [B` — `byte[280]` (its in-level sibling).
    pub f228i: Vec<i8>,
    /// `m.Q: I` — the paint blink counter (`b(Graphics)` wraps it at 6).
    pub Q: i32,
    /// `m.f229g (jadx): J` — the logo-state entry timestamp (`m.h()V` case 1).
    pub f229g: i64,
    /// `m.f231a (jadx): S` — the logo fade alpha nibble (init 15 = opaque).
    pub f231a: i16,
    /// `m.V: I` — init 183 (the cursor sprite index).
    pub V: i32,
    /// `m.f232s (jadx): Z`.
    pub f232s: bool,
    /// `m.f233b (jadx): [S` — `short[320]`: the 20×16 baked alpha gradient
    /// (`(15 - i2) << 12` per row) the ctor fills.
    pub f233b: Vec<i16>,
    /// `m.W: I` — init 10.
    pub W: i32,
    /// `m.f234j (jadx): [[I` (final) — 27 rows.
    pub f234j: Vec<Vec<i32>>,
    /// `m.X: I` — init -1.
    pub X: i32,
    /// `m.f236d (jadx): Ln;` — init null.
    pub f236d: Option<Unmodeled>,
    /// `m.aa: I` — init -1.
    pub aa: i32,
    /// `m.f238u (jadx): Z`.
    pub f238u: bool,
    /// `m.ab: I`.
    pub ab: i32,
    /// `m.f239j (jadx): Ljava/util/Vector;` — the party list.
    pub f239j: Vec<Unmodeled>,
    /// `m.f240k (jadx): Ljava/util/Vector;`.
    pub f240k: Vec<Unmodeled>,
    /// `m.f241l (jadx): Ljava/util/Vector;`.
    pub f241l: Vec<Unmodeled>,
    /// `m.f242m (jadx): Ljava/util/Vector;`.
    pub f242m: Vec<Unmodeled>,
    /// `m.f244l (jadx): [I` — init null.
    pub f244l: Option<Vec<i32>>,
    /// `m.f245j (jadx): [B` — init null.
    pub f245j: Option<Vec<i8>>,
    /// `m.f246b (jadx): [Ljava/lang/String;` — `String[5]` (all null).
    pub f246b: Vec<Option<String>>,
    /// `m.f247k (jadx): [B` — the options record (`showNotify` reads `[2]` as
    /// the music volume; null until options load).
    pub f247k: Option<Vec<i8>>,
    /// `m.f251o (jadx): [B` (final) — the 46-entry glyph remap row.
    pub f251o: Vec<i8>,
    /// `m.f252k (jadx): [[I` (final) — 13×4.
    pub f252k: Vec<Vec<i32>>,
    /// `m.f253l (jadx): [[I` (final).
    pub f253l: Vec<Vec<i32>>,
    /// `m.f254m (jadx): [[I` (final).
    pub f254m: Vec<Vec<i32>>,
    /// `m.ak: I` — `active_music_track` (symbols.toml): init -1 = none.
    pub active_music_track: i32,
    /// `m.al: I` — `music_volume` (symbols.toml): init 40.
    pub music_volume: i32,
    /// `m.f255n (jadx): Ljava/util/Vector;`.
    pub f255n: Vec<Unmodeled>,
    /// `m.f258n (jadx): [I` — `int[14]`.
    pub f258n: Vec<i32>,
    /// `m.f260n (jadx): [[I` — `int[18][]` (the rewards.txt row cache
    /// `m.m43a` fills lazily).
    pub f260n: Vec<Option<Vec<i32>>>,
    /// `m.f264o (jadx): Ljava/util/Vector;`.
    pub f264o: Vec<Unmodeled>,
    /// `m.f265x (jadx): Z`.
    pub f265x: bool,
    /// `m.f266y (jadx): Z` — init true.
    pub f266y: bool,
    /// `m.f267p (jadx): [B` — init null.
    pub f267p: Option<Vec<i8>>,
    /// `m.f268z (jadx): Z`.
    pub f268z: bool,
    /// `m.f271p (jadx): Ljava/util/Vector;`.
    pub f271p: Vec<Unmodeled>,
    /// `m.ao: I`.
    pub ao: i32,
    /// `m.f273q (jadx): [B` (final) — the 4-neighbour step table
    /// `{0,-1,0,1,-1,0,1,0}`.
    pub f273q: Vec<i8>,
    /// `m.f276o (jadx): [[I` — the 8 player-color RGB rows.
    pub f276o: Vec<Vec<i32>>,
    /// `m.f279e (jadx): [[[I` (final) — per-level scripted-event tables.
    pub f279e: Vec<Vec<Vec<i32>>>,
    /// `m.f280f (jadx): [[[I` (final) — per-level placement tables.
    pub f280f: Vec<Vec<Vec<i32>>>,
    /// `m.f284o (jadx): [I` — `int[100]` (particle pool x).
    pub f284o: Vec<i32>,
    /// `m.f285p (jadx): [I` — `int[100]`.
    pub f285p: Vec<i32>,
    /// `m.f286q (jadx): [I` — `int[100]`.
    pub f286q: Vec<i32>,
    /// `m.f287r (jadx): [I` — `int[100]`.
    pub f287r: Vec<i32>,
    /// `m.f288a (jadx): [Z` — `boolean[100]` (particle alive flags).
    pub f288a: Vec<bool>,
    /// `m.f289s (jadx): [I` — `int[100]`.
    pub f289s: Vec<i32>,
    /// `m.f290t (jadx): [I` — `int[100]`.
    pub f290t: Vec<i32>,
    /// `m.f291u (jadx): [I` — `int[100]`.
    pub f291u: Vec<i32>,
    /// `m.aq: I`.
    pub aq: i32,
    /// `m.ar: I` — init 20.
    pub ar: i32,
    /// `m.as: I`.
    pub r#as: i32,
    /// `m.at: I` — init 2.
    pub at: i32,
    /// `m.au: I` — init 100.
    pub au: i32,
    /// `m.f153a (jadx): Laj;` — `common_table` (symbols.toml); null until the
    /// ctor parses `common.utf`.
    pub common: Option<CommonTable>,
    /// `m.f173a (jadx): Ljavax/microedition/lcdui/Image;` — the Nival logo
    /// (res0.pak entry 0); `m.g()V` nulls it when the logo state ends.
    pub f173a: Option<Image>,
    /// The 22 `g` screen objects (see [`screen`] for the index names).
    pub screens: Vec<GScreen>,
    /// `m.T: I` (final) — the logo width (`f173a.getWidth()`).
    pub T: i32,
    /// `m.U: I` (final) — the logo height.
    pub U: i32,
    /// `m.R: I` (final) — the logo left: `88 - T/2`.
    pub R: i32,
    /// `m.S: I` (final) — the logo top: `104 - U/2`.
    pub S: i32,
    /// `m.f230a (jadx): [S` — `short[T*U]`: the logo fade overlay buffer
    /// (`m.g()V` nulls it with the logo).
    pub f230a: Option<Vec<i16>>,
}
