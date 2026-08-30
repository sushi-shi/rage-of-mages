//! `Container/GameMIDlet` — the MIDlet application entry (lifecycle only).
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/Container/GameMIDlet.java`.
//! Numeric shapes (R8): `<init>()V` = [], `quit()V` = [] — no arithmetic.

use crate::canvas_m::{m_init, m_start, m_stop, show_notify};
use crate::game::Game;
use crate::resource::Resources;
use j2me_jvm::Clock;

/// `GameMIDlet.<init> ()V` — boot: `a = Display.getDisplay(this);
/// f0a = new m(); f0a.f136a = this; a.setCurrent(f0a); f0a.a();`.
///
/// The `Display` lives on [`Game::display`] and the `m` singleton is
/// [`Game::m`] (the back-reference collapses, R4). `setCurrent` makes the
/// canvas visible (the runtime delivers `showNotify`, routed to
/// `m.showNotify`); `f0a.a()` arms the game thread — the host then drives
/// `run_prologue` + `run_tick` (see `canvas_m`).
///
/// `startApp`/`pauseApp`/`destroyApp` are empty in the baseline (the real
/// pause path is `hideNotify`) — nothing to port.
pub fn boot(resources: Box<dyn Resources>, clock: Box<dyn Clock>) -> Game {
    let mut g = Game::new(resources, clock);
    m_init(&mut g); // this.f0a = new m()
                    // this.f0a.f136a = this — collapses (R4).
                    // this.a.setCurrent(this.f0a): attach-once; fires the canvas showNotify …
                    // (j2me-me setCurrent takes the previous displayable; boot attaches
                    // the single FullCanvas with no predecessor).
    g.display.set_current(None, &mut g.canvas);
    // … which the device routes to the game's override m.showNotify().
    show_notify(&mut g);
    m_start(&mut g); // this.f0a.a()
    g
}

/// `GameMIDlet.quit ()V` — `f0a.b(); notifyDestroyed();`. The AMS
/// notification is a host concern; the observable game effect is `m.b()V`.
pub fn quit(g: &mut Game) {
    m_stop(g);
}
