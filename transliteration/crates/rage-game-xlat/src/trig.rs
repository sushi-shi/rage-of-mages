//! Class `o` — `TrigTables` (symbols.toml): the fixed-point sine/cosine lookup.
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/o.java`; FORMATS.md sincos.
//! Numeric shapes verified against `_reference/numeric-shapes.json` (R8):
//! `o.<init>` = [iinc]; `o.a(S)I` = [iadd,i2s,isub,i2s,isub,isub,ineg,isub,ineg];
//! `o.b(S)I` = [iadd,i2s,isub,i2s,isub,ineg,isub,ineg,isub].
//!
//! The tables are DATA loaded from `/sincos/sin.int` + `/sincos/cos.int`
//! (90 × big-endian `i16`, scale 10000) — never a closed-form substitute
//! (ARITHMETIC_AND_RUNTIME.md Part A: duplicate quantized tables bit-for-bit).

use crate::jio::DataInput;
use crate::resource::Resources;

/// The `o` singleton's fields (owned by `m.f141a` on the [`crate::Game`]).
#[derive(Debug, Clone)]
pub struct TrigTables {
    /// `o.a: [S` — sin_table: `short[90]` from `/sincos/sin.int`.
    pub a: Vec<i16>,
    /// `o.b: [S` — cos_table: `short[90]` from `/sincos/cos.int`.
    pub b: Vec<i16>,
    /// `o.f312a (jadx): Z` — tables available (false when a read failed).
    pub f312a: bool,
}

/// `o.<init> ()V` — read both quarter tables; any exception (a missing
/// resource NPEs through the null-wrapping DataInputStream) lands in the catch:
/// available = false plus the shipped console message.
pub fn o_init(resources: &dyn Resources) -> TrigTables {
    let mut t = TrigTables {
        a: vec![0i16; 90],
        b: vec![0i16; 90],
        f312a: true,
    };
    let mut body = || -> Result<(), j2me_jvm::JavaError> {
        let mut sin = DataInput::new(resources.resource_as_stream("/sincos/sin.int"));
        let mut cos = DataInput::new(resources.resource_as_stream("/sincos/cos.int"));
        for i in 0..90 {
            t.a[i] = sin.read_short()?;
            t.b[i] = cos.read_short()?;
        }
        Ok(())
    };
    if body().is_err() {
        // o.<init> catch arm.
        t.f312a = false;
        println!("Sincos is not av!!!");
    }
    t
}

/// `o.a (S)I` — sin(deg) in fixed-point 1e4: normalize into [0,360) with the
/// short-narrowing loops, then fold the quarter table with the Java sign
/// pattern (including the redundant range re-checks, preserved verbatim).
#[allow(clippy::manual_range_contains)] // faithful to the Java `s >= x && s < y` chain
pub fn o_sin(t: &TrigTables, mut s: i16) -> i32 {
    if !t.f312a {
        return 0;
    }
    while s < 0 {
        s = ((s as i32).wrapping_add(360)) as i16; // iadd, i2s
    }
    while s >= 360 {
        s = ((s as i32).wrapping_sub(360)) as i16; // isub, i2s
    }
    if s < 90 {
        return t.a[s as usize] as i32;
    }
    if s >= 90 && s < 180 {
        return t.b[((s as i32).wrapping_sub(90)) as usize] as i32; // isub
    }
    if s >= 180 && s < 270 {
        return -(t.a[((s as i32).wrapping_sub(180)) as usize] as i32); // isub, ineg
    }
    if s >= 270 {
        return -(t.b[((s as i32).wrapping_sub(270)) as usize] as i32); // isub, ineg
    }
    0
}

/// `o.b (S)I` — cos(deg), the parallel quadrant fold.
#[allow(clippy::manual_range_contains)] // faithful to the Java `s >= x && s < y` chain
pub fn o_cos(t: &TrigTables, mut s: i16) -> i32 {
    if !t.f312a {
        return 0;
    }
    while s < 0 {
        s = ((s as i32).wrapping_add(360)) as i16; // iadd, i2s
    }
    while s >= 360 {
        s = ((s as i32).wrapping_sub(360)) as i16; // isub, i2s
    }
    if s < 90 {
        return t.b[s as usize] as i32;
    }
    if s >= 90 && s < 180 {
        return -(t.a[((s as i32).wrapping_sub(90)) as usize] as i32); // isub, ineg
    }
    if s >= 180 && s < 270 {
        return -(t.b[((s as i32).wrapping_sub(180)) as usize] as i32); // isub, ineg
    }
    if s >= 270 {
        return t.a[((s as i32).wrapping_sub(270)) as usize] as i32; // isub
    }
    0
}

/// `o.a ()S` — the constant fixed-point scale.
pub fn o_scale() -> i16 {
    10000
}

/// `o.a (I)I` — abs.
pub fn o_abs(i: i32) -> i32 {
    if i < 0 {
        i.wrapping_neg()
    } else {
        i
    }
}

/// `o.b (I)I` — index of the cos-table entry nearest to `i` (linear scan over
/// all 90 entries, first-minimum wins; the duplicated distance computation is
/// the shipped shape).
pub fn o_nearest_cos_index(t: &TrigTables, i: i32) -> i32 {
    let mut best = 100000;
    let mut idx = 0;
    for i3 in 0..t.b.len() {
        if o_abs(i.wrapping_sub(t.b[i3] as i32)) < best {
            best = o_abs(i.wrapping_sub(t.b[i3] as i32));
            idx = i3 as i32;
        }
    }
    idx
}

impl Default for TrigTables {
    /// The pre-`<init>` JVM default is never observable (the field initializer
    /// runs the ctor); this exists so `Game` can be assembled before `m.<init>`
    /// executes, mirroring the JVM's zeroed object.
    fn default() -> Self {
        TrigTables {
            a: Vec::new(),
            b: Vec::new(),
            f312a: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoResources;
    impl Resources for NoResources {
        fn resource_as_stream(&self, _name: &str) -> Option<Vec<u8>> {
            None
        }
    }

    struct FakeSincos;
    impl Resources for FakeSincos {
        fn resource_as_stream(&self, name: &str) -> Option<Vec<u8>> {
            // 90 BE shorts: sin[i] = i, cos[i] = 1000 + i (authored test data).
            let mut out = Vec::new();
            for i in 0..90i16 {
                let v = if name.ends_with("sin.int") {
                    i
                } else {
                    1000 + i
                };
                out.extend_from_slice(&v.to_be_bytes());
            }
            Some(out)
        }
    }

    #[test]
    fn missing_resource_takes_the_catch_arm() {
        let t = o_init(&NoResources);
        assert!(!t.f312a);
        assert_eq!(o_sin(&t, 45), 0); // unavailable → 0, not a panic
    }

    #[test]
    fn quadrant_folding_matches_the_java_sign_pattern() {
        let t = o_init(&FakeSincos);
        assert!(t.f312a);
        assert_eq!(o_sin(&t, 10), 10); // Q1: a[s]
        assert_eq!(o_sin(&t, 100), 1010); // Q2: b[s-90]
        assert_eq!(o_sin(&t, 190), -10); // Q3: -a[s-180]
        assert_eq!(o_sin(&t, 280), -1010); // Q4: -b[s-270]
        assert_eq!(o_cos(&t, 10), 1010); // Q1: b[s]
        assert_eq!(o_cos(&t, 100), -10); // Q2: -a[s-90]
        assert_eq!(o_cos(&t, 190), -1010); // Q3: -b[s-180]
        assert_eq!(o_cos(&t, 280), 10); // Q4: a[s-270]
                                        // Normalization wraps negatives and overshoots through the short casts.
        assert_eq!(o_sin(&t, -350), o_sin(&t, 10));
        assert_eq!(o_sin(&t, 730), o_sin(&t, 10));
        assert_eq!(o_scale(), 10000);
    }
}
