//! Class `g` — `Screen` (symbols.toml): a screen/overlay object pushed on `m`'s
//! screen stack; 22 instances are built by `m.<init>` with distinct kinds.
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/g.java`. Numeric shape
//! verified against `_reference/numeric-shapes.json` (R8):
//! `g.<init>(Lm;I)V` = [iinc] (the kind-8 zero-fill loop).
//!
//! **This slice** ports only the constructor (all the first frame executes —
//! the logo screen never enters a `g` object). The statics `g.a` (the `m`
//! back-reference), `g.f53a` (= `m.f139a`) and `g.f54a` (= `m.f140a`) collapse
//! under R4 — ported `g` methods will reach the one owner on `Game` directly.
//! `g.f113a` (the `aj` alias assigned from `a.f153a`) collapses the same way.
//! TODO(next-slice): everything else — `g.a()V` enter, `g.b()V` load,
//! `g.c()V` close, `g.d()V` update, the paint family `g.a/b/c(Graphics)`,
//! and the sprite-pack walks — the menu/loading screens.

/// The static final int-array tables of class `g` (interface-constant style).
pub mod g_tables {
    /// `g.f58i (jadx): [I`.
    pub const F58I: [i32; 5] = [0, 6, 7, 8, 9];
    /// `g.f59j (jadx): [I`.
    pub const F59J: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// `g.f60k (jadx): [I`.
    pub const F60K: [i32; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// `g.f61l (jadx): [I`.
    pub const F61L: [i32; 9] = [0, 1, 2, 3, 5, 6, 7, 8, 9];
    /// `g.m: [I`.
    pub const M: [i32; 10] = [0, 1, 2, 3, 5, 6, 7, 8, 9, 10];
}

/// One `g` instance — the fields the constructor and the field initializers
/// touch. The remaining ~60 fields (sprite caches, list cursors, the minimap
/// state, …) land with the screens slice; they are JVM-zero until then.
#[derive(Debug, Clone)]
pub struct GScreen {
    /// `g.b: I` — the screen kind selector (ctor argument).
    pub b: i32,
    /// `g.f56h (jadx): I` — zeroed by the ctor.
    pub f56h: i32,
    /// `g.f57i (jadx): I` — zeroed by the ctor.
    pub f57i: i32,
    /// `g.j: I` — zeroed by the ctor.
    pub j: i32,
    /// `g.f67a (jadx): [B` — kind-8 (level select) 280-slot flag array.
    pub f67a: Option<Vec<i8>>,
    /// `g.x: I` / `g.y: I` / `g.z: I` / `g.A: I` / `g.B: I` / `g.C: I` —
    /// kind-8 cursor/bounds state.
    pub x: i32,
    /// See [`GScreen::x`].
    pub y: i32,
    /// See [`GScreen::x`].
    pub z: i32,
    /// See [`GScreen::x`].
    pub a_upper: i32,
    /// See [`GScreen::x`] (`g.B`).
    pub b_upper: i32,
    /// See [`GScreen::x`] (`g.C`).
    pub c_upper: i32,
    // --- field initializers (run before the ctor body) ---
    /// `g.f65h (jadx): [[I` — 13 map-node coordinates.
    pub f65h: Vec<Vec<i32>>,
    /// `g.f68k (jadx): Z` — init true.
    pub f68k: bool,
    /// `g.G: I` — init 0.
    pub g_upper: i32,
    /// `g.H: I` — init 0.
    pub h_upper: i32,
    /// `g.f72a (jadx): Ljava/util/Vector;` — empty at boot.
    pub f72a: Vec<crate::game::Unmodeled>,
    /// `g.f76i (jadx): [[I` — 16 sprite-run descriptors.
    pub f76i: Vec<Vec<i32>>,
    /// `g.f83q (jadx): [I` — `int[5]`.
    pub f83q: Vec<i32>,
    /// `g.N: I` / `g.V: I` / `g.W: I` / `g.X: I` — init 0.
    pub n_upper: i32,
    /// See [`GScreen::n_upper`].
    pub v_upper: i32,
    /// See [`GScreen::n_upper`].
    pub w_upper: i32,
    /// See [`GScreen::n_upper`].
    pub x_upper: i32,
    /// `g.f95a (jadx): [C` — 8 spaces (the name-entry buffer).
    pub f95a: Vec<u16>,
    /// `g.f99r (jadx): [I`.
    pub f99r: Vec<i32>,
    /// `g.f100s (jadx): [I`.
    pub f100s: Vec<i32>,
    /// `g.f101t (jadx): [I` — `int[4]`.
    pub f101t: Vec<i32>,
    /// `g.f102c (jadx): [B` — `byte[13]`.
    pub f102c: Vec<i8>,
    /// `g.f103e (jadx): B` — init 15.
    pub f103e: i8,
    /// `g.f104f (jadx): B` / `g.f105g (jadx): B` — init 0.
    pub f104f: i8,
    /// See [`GScreen::f104f`].
    pub f105g: i8,
    /// `g.f108c (jadx): Ln;` — init null.
    pub f108c: Option<crate::game::Unmodeled>,
    /// `g.ar: I` — init 0.
    pub ar: i32,
    /// `g.f114w (jadx): [I` — the campaign act boundaries.
    pub f114w: Vec<i32>,
}

/// `g.<init> (Lm;I)V` — new_screen: publish the statics (collapsed, see the
/// module doc), zero the scroll state, then the per-kind arm (kind 0 re-zeroes;
/// kind 8 allocates the 280-slot flag array and its cursor bounds).
#[allow(clippy::needless_range_loop)] // faithful to the Java kind-8 zero-fill loop
pub fn g_init(kind: i32) -> GScreen {
    let mut s = GScreen {
        b: kind,
        f56h: 0,
        f57i: 0,
        j: 0,
        f67a: None,
        x: 0,
        y: 0,
        z: 0,
        a_upper: 0,
        b_upper: 0,
        c_upper: 0,
        f65h: vec![
            vec![10, 42, 0],
            vec![60, 26, 0],
            vec![26, 55, 0],
            vec![34, 77, 0],
            vec![95, 75, 0],
            vec![139, 101, 0],
            vec![126, 113, 0],
            vec![85, 103, 0],
            vec![74, 138, 0],
            vec![102, 138, 0],
            vec![111, 39, 0],
            vec![49, 121, 0],
            vec![33, 101, 0],
        ],
        f68k: true,
        g_upper: 0,
        h_upper: 0,
        f72a: Vec::new(),
        f76i: vec![
            vec![0, 3],
            vec![4, 3],
            vec![8, 3],
            vec![12, 3],
            vec![16, 3],
            vec![20, 3],
            vec![0, 0],
            vec![24, 2],
            vec![27, 2],
            vec![0, 0],
            vec![30, 3],
            vec![34, 4],
            vec![0, 0],
            vec![0, 0],
            vec![39, 4],
            vec![0, 0],
        ],
        f83q: vec![0; 5],
        n_upper: 0,
        v_upper: 0,
        w_upper: 0,
        x_upper: 0,
        f95a: vec![' ' as u16; 8],
        f99r: vec![0, 2, 3, 4],
        f100s: vec![0, 1, 3, 4],
        f101t: vec![0; 4],
        f102c: vec![0; 13],
        f103e: 15,
        f104f: 0,
        f105g: 0,
        f108c: None,
        ar: 0,
        f114w: vec![53, 94, 147, 204, 279],
    };
    // g.<init>: `a = mVar; f53a = a.f139a; f54a = a.f140a;` — statics collapse
    // (R4); `this.f113a = a.f153a` (the aj alias) collapses the same way.
    match s.b {
        0 => {
            s.f56h = 0;
            s.f57i = 0;
            s.j = 0;
        }
        8 => {
            let mut f67a = vec![0i8; 280];
            for i2 in 0..280 {
                f67a[i2] = 0; // the shipped redundant zero-fill loop (iinc)
            }
            s.f67a = Some(f67a);
            s.z = 279;
            s.a_upper = 0;
            s.y = 0;
            s.x = 0;
            s.b_upper = 0;
            s.c_upper = 100;
        }
        _ => {}
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_configure_their_arms() {
        let k0 = g_init(0);
        assert_eq!(k0.b, 0);
        assert!(k0.f67a.is_none());
        assert!(k0.f68k);
        assert_eq!(k0.f103e, 15);

        let k8 = g_init(8);
        assert_eq!(k8.f67a.as_ref().map(|v| v.len()), Some(280));
        assert_eq!((k8.z, k8.c_upper), (279, 100));

        let k3 = g_init(3);
        assert_eq!(k3.b, 3);
        assert!(k3.f67a.is_none());
    }
}
