//! `rage-game-xlat` — the STRICT transliteration of Rage of Mages / Allods
//! (Nival, J2ME), implementation #1.
//!
//! This crate exists to be **provably the same program** as the recovered
//! `allods_176x220` bytecode — the executable spec a later idiomatic engine is
//! validated against — NOT good Rust. Do not refactor it. The rules live in
//! `/home/sheep/Projects/gothic-mobile/docs/TRANSLITERATION.md` (the proven
//! sibling contract) and the j2me home's `docs/ARITHMETIC_AND_RUNTIME.md`:
//!
//! - **R4 (one owner):** every Java datum lives on the single [`Game`] struct
//!   (`game`); ported methods are FREE FUNCTIONS `fn f(g: &mut Game, ...)`.
//!   The cross-linked singletons (`m` ↔ `GameMIDlet`, the `g`-class statics,
//!   the run-loop back-refs) collapse into `Game`.
//! - **R8 (Java arithmetic):** widen `byte`/`short` to `i32` before arithmetic,
//!   narrow last; `wrapping_*`, `i32_div`/`i64_div`/`i32_rem`, masked
//!   shifts from `j2me-jvm`; each ported method's opcode order is checked
//!   against `_reference/numeric-shapes.json` (cited in its doc comment).
//! - **Exceptions / defects:** a Java `try/catch` becomes a `Result` handled
//!   the way the catch did; an unguarded throw is a faithful panic; shipped
//!   defects are preserved with a source comment.
//! - **Resources:** bytes come through the game's [`resource::Resources`]
//!   trait (`Class.getResourceAsStream`); the game's OWN reversed loaders
//!   (`loader` = class `j`, `text` = `ae`, `common` = `aj`, `trig` = `o`)
//!   decode them. The independent `rage-formats` parsers are a TEST-ONLY
//!   oracle (dev-dependency), never a runtime decoder.
//!
//! **Ported through the first-frame slice** (boot → the first rendered frame):
//! the `Container/GameMIDlet` boot (`midlet`), `m.<init>` and the run-loop
//! skeleton + the logo-screen paint path (`canvas_m`), classes `o` (`trig`),
//! `j` (`loader`), `aj` (`common`), the `ae` boot surface (`text`), and the
//! `g` constructor (`screens`). The first frame is the **Nival logo** fading
//! in over white (`m.c(Graphics)`); `tests/first_frame.rs` is the R3 gate.
//!
//! **The seam to the next slice** is marked by `todo!` at every state edge:
//! the 5000 ms logo→menu transition (`m.d()V` case 1: `m.Q()`, `m.g()`, the
//! class-`g` sprite-pack loads), the screen-stack paint/update, the world and
//! menu painters, MMAPI audio, and RMS saves.

pub mod canvas_m;
pub mod common;
pub mod game;
pub mod jio;
pub mod jrandom;
pub mod loader;
pub mod midlet;
pub mod resource;
pub mod screens;
pub mod text;
pub mod trig;

pub use game::Game;
