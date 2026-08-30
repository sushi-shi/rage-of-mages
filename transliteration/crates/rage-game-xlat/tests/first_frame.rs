//! R3 gate: the FIRST RENDERED FRAME of the strict transliteration.
//!
//! Boots the transliterated `Container/GameMIDlet` against the baseline jar
//! (`_originals/allods_176x220.jar`, read in-process through a jar-backed
//! `rage_game_xlat::resource::Resources`), drives the transliterated run loop on a
//! `VirtualClock`, and asserts the painted frame is REAL:
//!
//! - **FrameStats** (built from the framebuffer ourselves, never a tool's
//!   stdout): distinct-color count, dominant-color share, and non-background
//!   ink must clear thresholds justified from the actual frame — measured:
//!   `distinct: 9, dominant: 27599 (the white field), non_white: 9009, total:
//!   36608`. The 117×77 logo box is exactly 9009 pixels, every one non-white,
//!   drawn from the logo PNG's 8-color palette (+ the white field = 9).
//! - **A pixel oracle**: the logo region must equal the res0.pak entry-0 PNG
//!   (decoded via the INDEPENDENT `rage_formats::pak` walk — dev-dependency
//!   only) composited over white at the transliteration's own (R, S).
//!
//! Can-fail controls (banned shape #2 — a blank frame must FAIL): a fresh
//! (all-white) framebuffer and a uniformly-filled framebuffer both fail the
//! same `is_real_frame` predicate, and the frame at t≈0 — while the shipped
//! fade-in still covers the logo with an opaque white overlay — fails it too
//! (the SAME pipeline, earlier clock ⇒ the discriminator tracks program
//! state, not the harness).
//!
//! Missing `_originals` FAILS LOUDLY (a panic naming the fetch step) — never a
//! skip (docs/GATES.md rule 4).

use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;

use j2me_jvm::{Clock, VirtualClock};
use j2me_me::image::create_image_region;
use j2me_me::{source_over, Image};
use rage_game_xlat::canvas_m::{run_prologue, run_tick};
use rage_game_xlat::midlet::boot;
use rage_game_xlat::resource::{normalize_resource_name, Resources};

/// The baseline jar, fully indexed in memory. Panics loudly when `_originals`
/// has not been materialized.
struct JarResources {
    entries: HashMap<String, Vec<u8>>,
}

impl JarResources {
    fn open_baseline() -> JarResources {
        let jar_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../_originals/allods_176x220.jar"
        );
        let file = std::fs::File::open(jar_path).unwrap_or_else(|e| {
            panic!(
                "FIRST-FRAME GATE CANNOT RUN: baseline jar missing at {jar_path} ({e}). \
                 Materialize the corpus first: `just bootstrap <resources-dir>` \
                 (docs/GATES.md rule 4: corpus tests fail loudly, never skip)."
            )
        });
        let mut zip = zip::ZipArchive::new(file).expect("baseline jar: not a readable zip");
        let mut entries = HashMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).expect("baseline jar: entry unreadable");
            if f.is_dir() {
                continue;
            }
            let mut bytes = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut bytes).expect("baseline jar: entry read");
            entries.insert(f.name().to_string(), bytes);
        }
        JarResources { entries }
    }
}

impl Resources for JarResources {
    fn resource_as_stream(&self, name: &str) -> Option<Vec<u8>> {
        self.entries.get(normalize_resource_name(name)).cloned()
    }
}

/// Shareable deterministic clock: the test keeps one handle to advance time
/// while the `Game` owns the other.
#[derive(Clone)]
struct SharedClock(Rc<VirtualClock>);

impl Clock for SharedClock {
    fn current_time_millis(&self) -> i64 {
        self.0.current_time_millis()
    }
}

/// Coarse pixel statistics of a painted frame (mirrors the sibling port's
/// `gothic-linux/src/frame.rs` idea; computed in-process from the pixels).
#[derive(Debug, Clone, Copy)]
struct FrameStats {
    distinct: usize,
    dominant: usize,
    non_white: usize,
    total: usize,
}

/// `Image::create_mutable` fills opaque white; an unpainted frame keeps it,
/// and the logo screen's own background is the same white.
const WHITE: u32 = 0xFFFF_FFFF;

fn analyze(img: &Image) -> FrameStats {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for &p in img.pixels() {
        *counts.entry(p).or_insert(0) += 1;
    }
    let total = img.pixels().len();
    let dominant = counts.values().copied().max().unwrap_or(0);
    let non_white = total - counts.get(&WHITE).copied().unwrap_or(0);
    FrameStats {
        distinct: counts.len(),
        dominant,
        non_white,
        total,
    }
}

impl FrameStats {
    /// A real, non-blank, non-uniform frame. Thresholds justified from the
    /// actual first frame (see [`first_frame_shows_the_nival_logo`], which
    /// prints the measured stats — `distinct: 9, dominant: 27599, non_white:
    /// 9009, total: 36608`): `distinct >= 8` requires the logo PNG's full
    /// 8-color palette to have landed (the palette + white = 9); `non_white >=
    /// 2000` sits 4.5× below the logo box's 9009 all-non-white pixels; and a
    /// uniform frame (`distinct == 1`, `dominant == total`, `non_white ∈
    /// {0, total}`) fails every clause.
    fn is_real_frame(&self) -> bool {
        self.distinct >= 8 && self.dominant < self.total && self.non_white >= 2000
    }
}

/// Boot the game and paint frames until the fade-in has fully revealed the
/// logo (elapsed >= 1000 ms at the last paint; the state still sits well below
/// the 5000 ms logo→menu transition, which is the next slice's `todo!` seam).
fn boot_and_render() -> (rage_game_xlat::Game, Image, SharedClock) {
    let clock = SharedClock(Rc::new(VirtualClock::new(0x5_0000)));
    let mut game = boot(
        Box::new(JarResources::open_baseline()),
        Box::new(clock.clone()),
    );
    run_prologue(&mut game);
    let mut fb = Image::create_mutable(176, 208).expect("createImage(176, 208)"); // the program's painted extent
    for _ in 0..20 {
        let sleep = run_tick(&mut game, &mut fb);
        clock.0.advance(sleep); // Thread.sleep(f156c)
    }
    (game, fb, clock)
}

#[test]
fn first_frame_shows_the_nival_logo() {
    let (game, fb, _clock) = boot_and_render();

    // 20 ticks × 62 ms: the last paint saw ~1178 ms elapsed — the fade is done
    // (alpha nibble 0) and the logo is fully revealed.
    let stats = analyze(&fb);
    println!("first-frame stats: {stats:?}");
    assert!(
        stats.is_real_frame(),
        "the rendered first frame is blank/uniform: {stats:?}"
    );

    // Pixel oracle: the logo region equals res0.pak entry 0 (decoded through
    // the INDEPENDENT rage-formats pak walk) composited over the white field
    // at the transliteration's own centering (R, S).
    let jar = JarResources::open_baseline();
    let pak_bytes = jar
        .resource_as_stream("/res/res0.pak")
        .expect("res0.pak in the baseline jar");
    let pak = rage_formats::pak::Pak::parse(&pak_bytes).expect("oracle pak parse");
    let logo_png = pak.entry(0).expect("res0.pak entry 0");
    // Decode the oracle PNG through the same MIDP factory the game uses
    // (`Image.createImage(byte[], off, len)` = j2me-me create_image_region),
    // which takes the Java `byte[]` (signed octets).
    let logo_bytes: Vec<i8> = logo_png.iter().map(|&b| b as i8).collect();
    let logo =
        create_image_region(&logo_bytes, 0, logo_bytes.len() as i32).expect("oracle logo decode");

    // The transliteration derived T/U/R/S from ITS OWN pak walk + PNG decode;
    // they must agree with the oracle's dimensions and the 88/104 centering.
    assert_eq!((game.m.T, game.m.U), (logo.width(), logo.height()));
    assert_eq!(game.m.R, 88 - logo.width() / 2);
    assert_eq!(game.m.S, 104 - logo.height() / 2);

    let mut mismatches = 0usize;
    for y in 0..logo.height() {
        for x in 0..logo.width() {
            let expected = source_over(logo.get(x, y).unwrap(), WHITE);
            let actual = fb.get(game.m.R + x, game.m.S + y).unwrap();
            if expected != actual {
                mismatches += 1;
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "logo region diverges from the res0.pak entry-0 oracle"
    );

    // The frame outside the logo box is the state-1 white fill.
    assert_eq!(fb.get(0, 0), Some(WHITE));
    assert_eq!(fb.get(175, 207), Some(WHITE));
}

#[test]
fn fade_in_start_is_uniform_and_fails_the_gate_frame_check() {
    // The SHIPPED fade defect-free behavior: at t≈0 the overlay alpha nibble
    // is 15 (opaque white), so the very first painted frame is uniformly white
    // — and must FAIL the same predicate the revealed frame passes. This is
    // the state-sensitivity control: one pipeline, two clock positions, two
    // verdicts.
    let clock = SharedClock(Rc::new(VirtualClock::new(0x5_0000)));
    let mut game = boot(
        Box::new(JarResources::open_baseline()),
        Box::new(clock.clone()),
    );
    run_prologue(&mut game);
    let mut fb = Image::create_mutable(176, 208).expect("createImage(176, 208)");
    let _ = run_tick(&mut game, &mut fb); // one paint at elapsed 0
    let stats = analyze(&fb);
    println!("t=0 frame stats: {stats:?}");
    assert_eq!(stats.distinct, 1, "t=0 must be the opaque white fade cover");
    assert!(
        !stats.is_real_frame(),
        "the t=0 fade cover must FAIL the real-frame predicate (can-fail control)"
    );
    // The paint DID run — its side effects are visible in game state.
    assert_eq!(game.m.Q, 1, "m.b(Graphics) ran once (the blink counter)");
    assert_eq!(game.m.f157d, 1, "one loop iteration elapsed");
    assert_eq!(game.m.f231a, 15, "fade alpha starts opaque (m.c(Graphics))");
}

#[test]
fn blank_and_uniform_frames_fail_the_gate() {
    // Banned shape #2 controls: the exact assertion the gate uses must go RED
    // on a blank frame and on a uniform non-background fill.
    let blank = Image::create_mutable(176, 208).expect("createImage(176, 208)"); // all white — unpainted
    assert!(!analyze(&blank).is_real_frame(), "a blank frame passed");

    let mut filled = Image::create_mutable(176, 208).expect("createImage(176, 208)");
    {
        let mut g = j2me_me::Graphics::new(&mut filled);
        g.set_color(0x0012_3456);
        g.fill_rect(0, 0, 176, 208);
    }
    let stats = analyze(&filled);
    assert!(
        !stats.is_real_frame(),
        "a uniform non-white fill passed: {stats:?}"
    );
}

#[test]
fn boot_state_matches_the_recovered_constructor() {
    let (game, _fb, _clock) = boot_and_render();

    // Trig tables loaded through the transliteration's own reader; the values
    // cross-check the independent rage-formats int_table oracle bit-for-bit.
    assert!(game.m.trig.f312a, "sincos tables must load from the jar");
    let jar = JarResources::open_baseline();
    let sin = rage_formats::int_table::IntTable::parse(
        &jar.resource_as_stream("/sincos/sin.int").unwrap(),
    )
    .expect("oracle sin.int parse");
    assert_eq!(
        game.m.trig.a,
        sin.values(),
        "sin table diverges from oracle"
    );
    assert_eq!(game.m.trig.a[45], 7071, "sin(45°) fixed-point (FORMATS.md)");

    // common.utf parsed: the load screens read q[5]/q[6] — the group exists.
    let common = game.m.common.as_ref().expect("aj parsed at boot");
    assert!(
        common.q.len() >= 7,
        "common.utf group q too short: {}",
        common.q.len()
    );

    // The font descriptor bound the atlas and the advances.
    assert!(game.m.text.a.is_some(), "font atlas bound");
    assert_eq!(
        game.m.text.f12a.as_ref().map(|v| v.len() as i32),
        Some(game.m.text.e),
        "one advance per glyph"
    );
    assert_eq!(game.m.text.f, -1, "m.<init> sets ae.f = -1");

    // Loop/state after the rendered ticks: still in the logo state, backdrop
    // ready, the pak closed, no screens pushed.
    assert_eq!(game.m.g, 1);
    assert!(game.m.b);
    assert!(game.m.f193a.is_empty());
    assert!(game.m.loader.a.is_none() && game.m.loader.f119a.is_none());
    assert_eq!(game.m.f231a, 0, "fade fully revealed after >= 1000 ms");
}
