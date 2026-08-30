//! Class `aj` — `CommonTable` (symbols.toml): the `common.utf` parser — 28
//! count-prefixed string groups, skipping `//` comment records.
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/aj.java`; FORMATS.md
//! common.utf. Numeric shapes verified against `_reference/numeric-shapes.json`
//! (R8): `aj.<init>(Lm;)V` = [] (Integer.parseInt and readUTF are calls);
//! `aj.a([Ljava/lang/String;)V` = [iinc]; `aj.a()Ljava/lang/String;` = [].
//!
//! CFR pins the real `<init>` order (the jadx field-initializer display is
//! reordered): `f26a = m` FIRST, then the stream open, then the 28 groups in
//! declaration order `a..z, A, B`, then `ae.b(stream)` closes.

use crate::jio::{parse_int_radix10, DataInput};
use crate::resource::Resources;
use crate::text::{ae_close_stream, ae_open_stream, ae_read_record};

/// The `aj` singleton's fields (owned by `m.f153a` on the [`crate::Game`]).
/// Group names are the Java field letters verbatim (`a..z, A, B`) — semantics
/// per group are not yet established (R10: no guessed names). `aj.f26a` (the
/// `m` back-reference) and `aj.f27a` (the parse-time stream, closed before the
/// ctor returns) collapse under R4.
#[allow(non_snake_case)] // the Java fields `A`/`B` are distinct from `a`/`b`
#[derive(Debug, Clone, Default)]
pub struct CommonTable {
    pub a: Vec<String>,
    pub b: Vec<String>,
    pub c: Vec<String>,
    pub d: Vec<String>,
    pub e: Vec<String>,
    pub f: Vec<String>,
    pub g: Vec<String>,
    pub h: Vec<String>,
    pub i: Vec<String>,
    pub j: Vec<String>,
    pub k: Vec<String>,
    pub l: Vec<String>,
    pub m: Vec<String>,
    pub n: Vec<String>,
    pub o: Vec<String>,
    pub p: Vec<String>,
    /// `aj.q` — carries the strings `m.b(Graphics)` draws on the load screens
    /// (`q[5]` = the press-a-key prompt, `q[6]` = the level label).
    pub q: Vec<String>,
    pub r: Vec<String>,
    pub s: Vec<String>,
    pub t: Vec<String>,
    pub u: Vec<String>,
    pub v: Vec<String>,
    pub w: Vec<String>,
    pub x: Vec<String>,
    pub y: Vec<String>,
    pub z: Vec<String>,
    pub A: Vec<String>,
    pub B: Vec<String>,
}

/// `aj.<init> (Lm;)V` — parse_common_utf: open `/res/common.utf`, read the 28
/// groups (each `new String[Integer.parseInt(next_record(), 10)]` then filled),
/// close. Nothing here is guarded: a malformed count
/// (`NumberFormatException`), a missing file (NPE through the null stream on
/// the first `readUTF` → `aj.a()` returns null → NPE on `startsWith`) — each
/// would kill the MIDlet, so each panics faithfully.
pub fn aj_init(resources: &dyn Resources) -> CommonTable {
    let mut t = CommonTable::default();
    let mut stream = ae_open_stream(resources.resource_as_stream("/res/common.utf"));
    {
        let group = |s: &mut DataInput| -> Vec<String> {
            let count = parse_int_radix10(&aj_next_record(s))
                .expect("NumberFormatException: aj.<init> group count");
            // `new String[count]` — negative would be NegativeArraySizeException.
            assert!(count >= 0, "NegativeArraySizeException: aj.<init>");
            let mut arr = vec![String::new(); count as usize];
            aj_read_group(s, &mut arr);
            arr
        };
        t.a = group(&mut stream);
        t.b = group(&mut stream);
        t.c = group(&mut stream);
        t.d = group(&mut stream);
        t.e = group(&mut stream);
        t.f = group(&mut stream);
        t.g = group(&mut stream);
        t.h = group(&mut stream);
        t.i = group(&mut stream);
        t.j = group(&mut stream);
        t.k = group(&mut stream);
        t.l = group(&mut stream);
        t.m = group(&mut stream);
        t.n = group(&mut stream);
        t.o = group(&mut stream);
        t.p = group(&mut stream);
        t.q = group(&mut stream);
        t.r = group(&mut stream);
        t.s = group(&mut stream);
        t.t = group(&mut stream);
        t.u = group(&mut stream);
        t.v = group(&mut stream);
        t.w = group(&mut stream);
        t.x = group(&mut stream);
        t.y = group(&mut stream);
        t.z = group(&mut stream);
        t.A = group(&mut stream);
        t.B = group(&mut stream);
    }
    ae_close_stream(&mut stream);
    t
}

/// `aj.a ([Ljava/lang/String;)V` — read_group: fill each slot with the next
/// non-comment record.
pub fn aj_read_group(stream: &mut DataInput, arr: &mut [String]) {
    for slot in arr.iter_mut() {
        *slot = aj_next_record(stream);
    }
}

/// `aj.a ()Ljava/lang/String;` — next_record: read records, skipping any that
/// begin with `//`. A read failure returns null from `ae.m3a` and the
/// `startsWith` then NPEs — faithful panic.
pub fn aj_next_record(stream: &mut DataInput) -> String {
    loop {
        let s = ae_read_record(stream)
            .expect("NullPointerException: aj.a() readUTF failed (startsWith on null)");
        if !s.starts_with("//") {
            return s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf_record(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(s.len() as u16).to_be_bytes());
        out.extend_from_slice(s.as_bytes());
        out
    }

    #[test]
    fn groups_parse_counts_and_skip_comments() {
        // Authored stream: a 2-record group with an interleaved comment, then
        // a 1-record group.
        let mut bytes = Vec::new();
        for rec in ["2", "// header", "first", "second", "1", "only"] {
            bytes.extend(utf_record(rec));
        }
        let mut s = ae_open_stream(Some(bytes));
        let count = parse_int_radix10(&aj_next_record(&mut s)).unwrap();
        let mut g1 = vec![String::new(); count as usize];
        aj_read_group(&mut s, &mut g1);
        assert_eq!(g1, vec!["first".to_string(), "second".to_string()]);
        let count2 = parse_int_radix10(&aj_next_record(&mut s)).unwrap();
        let mut g2 = vec![String::new(); count2 as usize];
        aj_read_group(&mut s, &mut g2);
        assert_eq!(g2, vec!["only".to_string()]);
    }

    #[test]
    #[should_panic(expected = "NullPointerException")]
    fn truncated_stream_dies_like_java() {
        let mut s = ae_open_stream(Some(utf_record("3")));
        let _ = aj_next_record(&mut s); // "3"
        let _ = aj_next_record(&mut s); // EOF → m3a null → NPE
    }
}
