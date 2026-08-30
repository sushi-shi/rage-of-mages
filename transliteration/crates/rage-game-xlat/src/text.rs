//! Class `ae` — `TextEngine` (symbols.toml): the `.utf` record reader, the
//! bitmap-font descriptor, and the per-paint Graphics bind.
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/ae.java`; FORMATS.md .utf +
//! font descriptor; DEVICE_RUNTIME.md §1.3/§3.2. Numeric shapes verified
//! against `_reference/numeric-shapes.json` (R8):
//! `ae.a([BLImage;)V` = []; `ae.a(Ljava/io/DataInputStream;)V` =
//! [imul,isub,iadd,iinc, imul,isub,iadd,iinc, imul,isub,iadd,iinc, iinc,
//! imul,isub,iadd, iinc, isub] (three decimal parses, the advance-list walk,
//! then `d = f11a.length() - 1`).
//!
//! **This slice** ports the boot surface the first frame executes: the
//! constructor defaults, `open_stream`/`read_record`/`close_stream`
//! (`ae.a(InputStream)`, `ae.m3a`, `ae.b`), the font-descriptor load
//! (`ae.a([B,Image)` → `ae.a(DataInputStream)`), and the static Graphics bind.
//! TODO(next-slice): the glyph renderer + width metrics
//! (`ae.a(String,III)V`, `ae.a([BIII)V`, `ae.a(C)I`, `ae.a(String)I`,
//! `ae.m0a`/`ae.m1a`, the wrap engine `ae.a(String[],I)V`/`ae.a(String,I)[S`,
//! the clip-constrain paint `ae.a(IIII)V`, and `ae.a([B)I`) — no text is drawn
//! before the logo screen ends.

use crate::jio::DataInput;
use j2me_me::{Graphics, Image};

/// The `ae` singleton's fields (owned by `m.f139a` on the [`crate::Game`]).
///
/// The one Java **static** of `ae` — `f13a: Ljavax/microedition/lcdui/Graphics;`,
/// the current-paint Graphics cache — collapses under R4: a stored `Graphics`
/// would be a second borrow of the frame target, so every ported drawing helper
/// takes the frame's `&mut Graphics` explicitly (the frame-local view) and
/// [`bind_graphics`] documents the call site.
#[derive(Debug, Default)]
pub struct TextEngine {
    /// `ae.a: Ljavax/microedition/lcdui/Image;` — the glyph atlas (res0.pak
    /// entry 1, decoded).
    pub a: Option<Image>,
    /// `ae.f10a (jadx): I` — glyph cell width (descriptor record 2).
    pub f10a: i32,
    /// `ae.b: I` — glyph cell height (descriptor record 3).
    pub b: i32,
    /// `ae.c: I` — atlas columns (descriptor record 4).
    pub c: i32,
    /// `ae.f11a (jadx): Ljava/lang/String;` — the glyph string (descriptor
    /// record 1), kept as UTF-16 code units because `charAt`/`indexOf` index
    /// units, exactly like a Java String.
    pub f11a: Option<Vec<u16>>,
    /// `ae.f12a (jadx): [I` — per-glyph advances (descriptor record 5).
    pub f12a: Option<Vec<i32>>,
    /// `ae.d: I` — the fallback glyph index (`f11a.length() - 1`).
    pub d: i32,
    /// `ae.e: I` — glyph count (`f11a.length()`).
    pub e: i32,
    /// `ae.f: I` — inter-char spacing addend (`m.<init>` sets `-1`).
    pub f: i32,
    /// `ae.g: I` — line-height addend (never set at boot: 0).
    pub g: i32,
    /// `ae.h: I` / `ae.i: I` — saved clip x/y (init 0).
    pub h: i32,
    /// See [`TextEngine::h`].
    pub i: i32,
    /// `ae.j: I` / `ae.k: I` — saved clip w/h (init 200000).
    pub j: i32,
    /// See [`TextEngine::j`].
    pub k: i32,
    /// `ae.f14a (jadx): Z` — clip-constrain mode for `ae.a(IIII)V` (init false).
    pub f14a: bool,
}

/// `ae.<init> ()V` — only the field initializers (`h=0, i=0, j=200000,
/// k=200000, f14a=false`; everything else JVM-default).
pub fn ae_init() -> TextEngine {
    TextEngine {
        j: 200000,
        k: 200000,
        ..TextEngine::default()
    }
}

/// `ae.a ([BLjavax/microedition/lcdui/Image;)V` — bind the atlas image and
/// parse the descriptor blob: wrap it (`open_stream`), read the five records
/// (`read_font_descriptor`), close.
pub fn ae_set_font(t: &mut TextEngine, b_arr: &[i8], image: Image) {
    t.a = Some(image);
    // ae.a(new ByteArrayInputStream(bArr)) — never fails to construct.
    let bytes: Vec<u8> = b_arr.iter().map(|&b| b as u8).collect();
    let mut dis = ae_open_stream(Some(bytes));
    ae_read_font_descriptor(t, &mut dis);
    ae_close_stream(&mut dis);
}

/// `ae.a (Ljava/io/DataInputStream;)V` — read_font_descriptor: five `readUTF`
/// records — glyph string, cell width, cell height, atlas columns, per-glyph
/// advances (space-separated decimals) — all inside one `try/catch(Exception)`.
/// After the catch, `d = f11a.length() - 1` runs UNGUARDED: if the glyph
/// string never loaded the Java NPEs and the MIDlet dies — a faithful panic.
#[allow(clippy::needless_range_loop)] // faithful Java index loops
pub fn ae_read_font_descriptor(t: &mut TextEngine, dis: &mut DataInput) {
    let mut body = || -> Result<(), j2me_jvm::JavaError> {
        let glyphs: Vec<u16> = dis.read_utf()?.encode_utf16().collect();
        t.e = glyphs.len() as i32;
        t.f12a = Some(vec![0i32; t.e as usize]);
        t.f11a = Some(glyphs);
        let utf = dis.read_utf()?;
        t.f10a = 0;
        for c in utf.encode_utf16() {
            // (10 * acc) + (charAt - '0'): imul, isub, iadd — no digit guard.
            t.f10a = (10i32.wrapping_mul(t.f10a)).wrapping_add((c as i32).wrapping_sub('0' as i32));
        }
        let utf2 = dis.read_utf()?;
        t.b = 0;
        for c in utf2.encode_utf16() {
            t.b = (10i32.wrapping_mul(t.b)).wrapping_add((c as i32).wrapping_sub('0' as i32));
        }
        let utf3 = dis.read_utf()?;
        t.c = 0;
        for c in utf3.encode_utf16() {
            t.c = (10i32.wrapping_mul(t.c)).wrapping_add((c as i32).wrapping_sub('0' as i32));
        }
        let utf4: Vec<u16> = dis.read_utf()?.encode_utf16().collect();
        let advances = t.f12a.as_mut().expect("just allocated");
        let mut i4 = 0usize;
        for i5 in 0..t.e as usize {
            advances[i5] = 0;
            while i4 < utf4.len() {
                let c = utf4[i4];
                i4 += 1; // iinc before the space test — the shipped order
                if c == ' ' as u16 {
                    break;
                }
                advances[i5] = (10i32.wrapping_mul(advances[i5]))
                    .wrapping_add((c as i32).wrapping_sub('0' as i32));
            }
        }
        Ok(())
    };
    let _ = body(); // ae.a(DataInputStream) catch arm: swallow
                    // Unguarded after the catch: NPE (panic) when the glyph string is absent.
    t.d = (t
        .f11a
        .as_ref()
        .expect("NullPointerException: ae.a(DataInputStream) with no glyph string")
        .len() as i32)
        .wrapping_sub(1); // isub
}

/// `ae.a ()I` — line height: `b + g`.
pub fn ae_line_height(t: &TextEngine) -> i32 {
    t.b.wrapping_add(t.g)
}

/// `static ae.a (Ljavax/microedition/lcdui/Graphics;)V` — bind_graphics: the
/// Java caches the current paint `Graphics` in the static `f13a` for every
/// text/render helper. Under one-owner (R4) that static collapses: ported
/// helpers take the frame's `&mut Graphics` parameter instead, and this
/// function marks the bind site (`m.paint` calls it first every frame).
pub fn bind_graphics(_graphics: &mut Graphics<'_>) {}

/// `static ae.a (Ljava/io/InputStream;)Ljava/io/DataInputStream;` —
/// open_stream: wrap bytes in a DataInputStream; the Java `try/catch` around
/// the constructor can never fire (the ctor does not throw), so this always
/// yields a stream (possibly wrapping null).
pub fn ae_open_stream(bytes: Option<Vec<u8>>) -> DataInput {
    DataInput::new(bytes)
}

/// `ae.m3a (Ljava/io/DataInputStream;)Ljava/lang/String;` (jadx) —
/// read_record: one `readUTF` record, `null` on any exception.
pub fn ae_read_record(dis: &mut DataInput) -> Option<String> {
    dis.read_utf().ok()
}

/// `static ae.b (Ljava/io/DataInputStream;)V` — close_stream, exceptions
/// swallowed.
pub fn ae_close_stream(dis: &mut DataInput) {
    let _ = dis.close();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Authored descriptor bytes (never game data): five modified-UTF-8
    /// records — "AB!", "7", "9", "16", "5 6 4 ".
    fn descriptor() -> Vec<i8> {
        let mut out = Vec::new();
        for rec in ["AB!", "7", "9", "16", "5 6 4 "] {
            out.extend_from_slice(&(rec.len() as u16).to_be_bytes());
            out.extend_from_slice(rec.as_bytes());
        }
        out.into_iter().map(|b| b as i8).collect()
    }

    #[test]
    fn descriptor_parses_the_five_records() {
        let mut t = ae_init();
        assert_eq!((t.j, t.k), (200000, 200000));
        let atlas = Image::create_mutable(8, 8).expect("createImage(8, 8)");
        ae_set_font(&mut t, &descriptor(), atlas);
        assert_eq!(t.e, 3);
        assert_eq!(t.f10a, 7); // cell width
        assert_eq!(t.b, 9); // cell height
        assert_eq!(t.c, 16); // atlas columns
        assert_eq!(t.f12a.as_deref(), Some(&[5, 6, 4][..]));
        assert_eq!(t.d, 2); // fallback = len - 1
        assert!(t.a.is_some());
    }

    #[test]
    #[should_panic(expected = "NullPointerException")]
    fn empty_descriptor_dies_after_the_catch_like_java() {
        // The try swallows the EOF, then `d = f11a.length() - 1` NPEs.
        let mut t = ae_init();
        let atlas = Image::create_mutable(8, 8).expect("createImage(8, 8)");
        ae_set_font(&mut t, &[], atlas);
    }
}
