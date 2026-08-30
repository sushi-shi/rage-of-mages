//! Corpus + oracle gate: every parser against every real blob of its kind from
//! the RU baseline (`allods_176x220.jar`) and the EN build
//! (`Rage-of-Mages_J2ME_EN_v11.jar`), each guarded by an INDEPENDENT oracle —
//! never `decode(encode(x))`:
//!
//! - `.map`  — the engine's own invariant: every raw cell lies in `128..=173`
//!   (the `m.f251o` remap domain), plus byte-exact framing and cross-build
//!   identity of the campaign maps.
//! - `.utf`  — the cross-language identity (R11): RU and EN carry the same
//!   record count per file (pinned counts), and the language-keyed record 0.
//! - `.pak`  — the `png` crate (an unrelated second implementation) must fully
//!   decode every decrypted entry that carries the PNG signature.
//! - `.int`  — ideal trigonometry: `value[deg] == round(10000·sin/cos(deg))`,
//!   max error 0.
//! - `scenes.pak` — exact-EOF framing with the independently re-derived head
//!   (`tools/corpus/inspect_formats.py`: first ints `19, 4, …`).
//!
//! Rulebook R1/R2: no game bytes are committed; the jars are read from the
//! immutable `_originals/` at runtime, in-process via the `zip` crate. If a jar
//! is absent this gate FAILS LOUDLY — it never skips to green (a skip that
//! reads as a pass is a banned vacuous shape, R3). The can-fail proof lives in
//! each module's negative-control unit tests, which reject one-unit-perturbed
//! blobs.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rage_formats::int_table::IntTable;
use rage_formats::map::Map;
use rage_formats::pak::Pak;
use rage_formats::scenes::Scenes;
use rage_formats::utf::Utf;

const BASELINE: &str = "allods_176x220.jar"; // behavior authority (RU)
const EN: &str = "Rage-of-Mages_J2ME_EN_v11.jar"; // English text source

const PNG_SIGNATURE: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// `_originals/` at the repo root; FAIL LOUDLY when it is missing.
fn originals_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../_originals");
    assert!(
        dir.is_dir(),
        "missing `_originals/` at {} — materialize the corpus with \
         `just bootstrap <resources>`; this gate never skips (R3)",
        dir.display()
    );
    dir
}

/// Open one corpus jar in-process; FAIL LOUDLY when it is absent or unreadable.
fn open_jar(name: &str) -> zip::ZipArchive<File> {
    let path = originals_dir().join(name);
    assert!(
        path.is_file(),
        "missing corpus jar {} — materialize the corpus with \
         `just bootstrap <resources>`; this gate never skips (R3)",
        path.display()
    );
    let file = File::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    zip::ZipArchive::new(file).unwrap_or_else(|e| panic!("read zip {}: {e}", path.display()))
}

/// Read one member's bytes out of an open jar.
fn member(jar: &mut zip::ZipArchive<File>, name: &str) -> Vec<u8> {
    let mut f = jar
        .by_name(name)
        .unwrap_or_else(|e| panic!("missing jar member {name}: {e}"));
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read jar member {name}: {e}"));
    buf
}

// ---------------------------------------------------------------------------
// .map — engine cell-range oracle + cross-build identity
// ---------------------------------------------------------------------------

#[test]
fn maps_parse_with_cells_in_the_engine_remap_domain() {
    for jar_name in [BASELINE, EN] {
        let mut jar = open_jar(jar_name);
        let names: Vec<String> = jar
            .file_names()
            .filter(|n| n.ends_with(".map"))
            .map(str::to_owned)
            .collect();
        // 1..12 campaign + net0..4 + 101 multiplayer — both full builds.
        assert_eq!(names.len(), 18, "{jar_name}: expected 18 maps");

        for name in &names {
            let blob = member(&mut jar, name);
            let map = Map::parse(&blob)
                .unwrap_or_else(|e| panic!("{jar_name}!{name}: map rejected: {e}"));
            // Framing restated from the parsed header, against the raw blob.
            assert_eq!(
                blob.len(),
                4 + map.width() as usize * map.height() as usize,
                "{jar_name}!{name}: length != 4 + w*h"
            );
            assert!(
                map.width() > 0 && map.height() > 0,
                "{jar_name}!{name}: empty grid"
            );
            // The engine's oracle: the loader computes `rawByte - 128` and
            // indexes the 46-entry `m.f251o` table, so every stored cell must
            // lie in the closed range 128..=173 — a fact of the game's code,
            // not of this parser.
            assert!(
                map.cells().iter().all(|c| (128..=173).contains(c)),
                "{jar_name}!{name}: cell outside 128..=173"
            );
            // Getter agrees with the raw row-major layout.
            assert_eq!(map.cell(0, 0), Some(map.cells()[0]));
            assert_eq!(
                map.cell(map.width() - 1, map.height() - 1),
                map.cells().last().copied()
            );
            assert_eq!(map.cell(map.width(), 0), None);
        }
    }

    // Campaign terrain is build-independent (FORMATS: byte-identical across
    // all four builds) — the decoded grids must agree RU↔EN.
    let (mut ru, mut en) = (open_jar(BASELINE), open_jar(EN));
    for i in 1..=12 {
        let name = format!("res/{i}.map");
        let a = Map::parse(&member(&mut ru, &name)).unwrap();
        let b = Map::parse(&member(&mut en, &name)).unwrap();
        assert_eq!(a, b, "{name}: campaign map differs across builds");
    }
}

// ---------------------------------------------------------------------------
// .utf — the cross-language record-count identity (R11)
// ---------------------------------------------------------------------------

#[test]
fn utf_record_counts_are_the_cross_language_identity() {
    // The R11 identity, machine-verified in Phase 1: identical record count
    // per file in RU and EN, even though byte sizes differ (~2× for Cyrillic).
    const EXPECTED: [(&str, usize); 9] = [
        ("1", 28),
        ("2", 4),
        ("common", 506),
        ("dialogs0", 322),
        ("dialogs1", 258),
        ("messages", 40),
        ("quests", 124),
        ("rumours", 77),
        ("skills", 102),
    ];

    let (mut ru, mut en) = (open_jar(BASELINE), open_jar(EN));
    for (stem, want) in EXPECTED {
        let name = format!("res/{stem}.utf");
        // Parse success already proves exact-EOF tiling (the parser rejects
        // ragged streams), and every record is a Rust `String`, hence valid
        // UTF-8 by construction.
        let r = Utf::parse(&member(&mut ru, &name))
            .unwrap_or_else(|e| panic!("RU {name}: rejected: {e}"));
        let e = Utf::parse(&member(&mut en, &name))
            .unwrap_or_else(|e| panic!("EN {name}: rejected: {e}"));
        assert_eq!(r.len(), want, "RU {name}: record count");
        assert_eq!(e.len(), want, "EN {name}: record count");
    }

    // The language key is the decoded content, never the filename (R10):
    // record 0 of 1.utf names the game in each build's language.
    let ru_1 = Utf::parse(&member(&mut ru, "res/1.utf")).unwrap();
    let en_1 = Utf::parse(&member(&mut en, "res/1.utf")).unwrap();
    assert_eq!(ru_1.get(0), Some("Аллоды"));
    assert_eq!(en_1.get(0), Some("Allods"));
}

// ---------------------------------------------------------------------------
// .pak — the `png` crate as an independent second implementation
// ---------------------------------------------------------------------------

#[test]
fn pak_entries_decrypt_to_pngs_a_second_implementation_decodes() {
    for jar_name in [BASELINE, EN] {
        let mut jar = open_jar(jar_name);
        let mut png_entries = 0usize;
        for i in 0..5 {
            let name = format!("res/res{i}.pak");
            let blob = member(&mut jar, &name);
            let pak = Pak::parse(&blob)
                .unwrap_or_else(|e| panic!("{jar_name}!{name}: pak rejected: {e}"));

            // Length invariant restated against the raw blob.
            let payload: usize = pak.entries().iter().map(Vec::len).sum();
            assert_eq!(
                blob.len(),
                4 + 4 * pak.len() + payload,
                "{jar_name}!{name}: length != 4 + 4*count + Σ length[i]"
            );

            // Entry 0 is a PNG in every pack (verified corpus fact).
            assert!(
                pak.entry(0).is_some_and(|e| e.starts_with(&PNG_SIGNATURE)),
                "{jar_name}!{name}: entry 0 is not a PNG"
            );

            // The oracle: every PNG-signed decrypted entry must FULLY decode
            // through the `png` crate — an implementation that shares nothing
            // with our XOR+framing code. A wrong key or a mis-framed directory
            // cannot produce hundreds of valid PNG streams by accident.
            for (k, entry) in pak.entries().iter().enumerate() {
                if entry.starts_with(&PNG_SIGNATURE) {
                    let mut reader =
                        png::Decoder::new(&entry[..])
                            .read_info()
                            .unwrap_or_else(|e| {
                                panic!("{jar_name}!{name}[{k}]: png header rejected: {e}")
                            });
                    let mut pixels = vec![0u8; reader.output_buffer_size()];
                    let info = reader.next_frame(&mut pixels).unwrap_or_else(|e| {
                        panic!("{jar_name}!{name}[{k}]: png body rejected: {e}")
                    });
                    assert!(
                        info.width > 0 && info.height > 0,
                        "{jar_name}!{name}[{k}]: degenerate png"
                    );
                    png_entries += 1;
                }
            }
        }
        // Pinned tally so the subject cannot silently vanish (R3): 280 entries
        // per build, all PNGs except res0's font-descriptor entry.
        assert_eq!(png_entries, 279, "{jar_name}: png entry tally");
    }
}

#[test]
fn res0_font_descriptor_rides_in_the_pak_as_a_five_record_utf_stream() {
    // FORMATS: res0.pak entry 2 is the bitmap-font descriptor — five readUTF
    // records: glyph string, cell width, cell height, atlas columns, advances.
    for (jar_name, glyphs_head) in [(BASELINE, "АБВГ"), (EN, "ABCD")] {
        let mut jar = open_jar(jar_name);
        let pak = Pak::parse(&member(&mut jar, "res/res0.pak")).unwrap();
        let descriptor = pak.entry(2).expect("res0.pak entry 2");
        assert!(
            !descriptor.starts_with(&PNG_SIGNATURE),
            "{jar_name}: descriptor entry must not be a PNG"
        );
        let utf = Utf::parse(descriptor)
            .unwrap_or_else(|e| panic!("{jar_name}: descriptor rejected: {e}"));
        assert_eq!(utf.len(), 5, "{jar_name}: descriptor record count");
        assert!(
            utf.get(0).is_some_and(|g| g.starts_with(glyphs_head)),
            "{jar_name}: glyph string does not open with the alphabet"
        );
        // Cell width/height/columns are decimal strings (Integer.parseInt-ed).
        for k in 1..=3 {
            let rec = utf.get(k).unwrap();
            assert!(
                rec.parse::<u32>().is_ok_and(|v| v > 0),
                "{jar_name}: descriptor record {k} ({rec:?}) is not a positive decimal"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// .int — ideal trigonometry as the fully independent oracle
// ---------------------------------------------------------------------------

#[test]
fn int_tables_match_ideal_trigonometry_with_zero_error() {
    // sin/cos tables are shared by {176, 240, EN}; check both jars we load.
    for jar_name in [BASELINE, EN] {
        let mut jar = open_jar(jar_name);
        let sin = IntTable::parse(&member(&mut jar, "sincos/sin.int"))
            .unwrap_or_else(|e| panic!("{jar_name}!sin.int rejected: {e}"));
        let cos = IntTable::parse(&member(&mut jar, "sincos/cos.int"))
            .unwrap_or_else(|e| panic!("{jar_name}!cos.int rejected: {e}"));
        assert_eq!(sin.len(), 90, "{jar_name}: sin entry count");
        assert_eq!(cos.len(), 90, "{jar_name}: cos entry count");

        // The oracle is the mathematics itself: stored value = round(10000·f).
        // Max error must be 0 (verified property of the corpus tables).
        for deg in 0..90usize {
            let rad = (deg as f64).to_radians();
            assert_eq!(
                sin.get(deg),
                Some((10000.0 * rad.sin()).round() as i16),
                "{jar_name}: sin[{deg}]"
            );
            assert_eq!(
                cos.get(deg),
                Some((10000.0 * rad.cos()).round() as i16),
                "{jar_name}: cos[{deg}]"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// scenes.pak — exact-EOF framing with the independently re-derived head
// ---------------------------------------------------------------------------

#[test]
fn scenes_frame_to_exact_eof_with_the_rederived_head() {
    for jar_name in [BASELINE, EN] {
        let mut jar = open_jar(jar_name);
        // Parse success proves the 3-groups-per-scene grammar tiles the blob
        // to exact EOF.
        let scenes = Scenes::parse(&member(&mut jar, "res/scenes.pak"))
            .unwrap_or_else(|e| panic!("{jar_name}!scenes.pak rejected: {e}"));
        // One scene per campaign level (m.g(i), i = level index 0..11).
        assert_eq!(scenes.len(), 12, "{jar_name}: scene count");
        // Head pinned by tools/corpus/inspect_formats.py, which re-derives the
        // first ints straight from the bytes: 19, 4, 0, 3, 0, 8 — i.e. group 0
        // has 19 rows and its row 0 is the 4 ints [0, 3, 0, 8].
        let scene0 = scenes.get(0).unwrap();
        assert_eq!(scene0[0].len(), 19, "{jar_name}: scene0 group0 rows");
        assert_eq!(scene0[0][0], vec![0, 3, 0, 8], "{jar_name}: scene0 row0");
        // Every scene is exactly three groups by construction, and in this
        // corpus every group of every scene carries rows — group 3 is present
        // on disk even when unused at runtime (verified in both builds).
        for (i, scene) in scenes.scenes().iter().enumerate() {
            assert!(
                scene.iter().all(|g| !g.is_empty()),
                "{jar_name}: scene {i} has an empty group"
            );
        }
    }
}
