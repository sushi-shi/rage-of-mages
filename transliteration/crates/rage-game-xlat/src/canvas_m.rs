//! Class `m` — `GameCanvas` (symbols.toml): the Nokia FullCanvas + Runnable
//! game loop. **This slice**: boot (`m.<init>`), the loop skeleton (`m.a()V`,
//! `m.b()V`, `m.run`), and the paint path to the FIRST RENDERED FRAME — the
//! Nival logo screen (`m.paint` → `m.r` → `m.a(Graphics)` → `m.b(Graphics)`
//! case 1 → `m.c(Graphics)`), plus the small state helpers those touch
//! (`m.c()V`/`m.d()V`/`m.f()V`/`m.h()V`/`m.i()V`/`m.j()V`, `m.a(I)I`,
//! `showNotify`/`hideNotify`, and the music no-op pair `m.h(I)V`/`m.i(I)V`).
//!
//! Implementation #1: strict transliteration. Provably the same program as the
//! recovered Java, NOT idiomatic Rust — do not refactor (docs/TRANSLITERATION.md).
//! Source: `_reference/decompile/176x220/{jadx,cfr}/m.java` (CFR pins the
//! `<init>` field-initializer order). Numeric shapes verified against
//! `_reference/numeric-shapes.json` (R8) — per method:
//! - `m.<init>()V` = [imul,iadd,ishl,i2s,isub,i2s,iinc,iinc,iinc,
//!   idiv,isub,idiv,isub,imul] (the f233b gradient bake; the players loop
//!   `iinc` sits in the stubbed MMAPI block; `R/S` centering; `T*U`).
//! - `m.run()V` = [lcmp,lcmp,ladd,ladd,lsub,lcmp] (split here into
//!   [`run_prologue`] + [`run_tick`]).
//! - `m.paint(G)V` = [isub]; `m.c()V` = [isub];
//! - `m.d()V` = [iinc×5, lsub,lcmp] (the iincs live in the 5/9/104 arms —
//!   `todo!` this slice; the case-1 gate carries the lsub/lcmp).
//! - `m.b(G)V` — a long shape whose leading `iadd` is the `Q++` ported here;
//!   the rest belongs to the 101/102/104/106 arms (`todo!` this slice).
//! - `m.c(G)V` = [lsub,lcmp,lsub,lmul,ldiv,l2i,i2s,lcmp,ishl,iadd,i2s,iinc]
//!   (the logo fade — ported in full).
//! - `m.r(G)V` = [isub,isub,isub]; `m.a(I)I` = [irem]; `m.h()V` = [];
//!   `m.a()V`/`m.b()V`/`m.f()V`/`m.i()V`/`m.j()V` = [].
//!
//! STOP LINE (anti-bog): every state past the logo screen — the 5000 ms
//! logo→menu transition (`m.d()V` case 1 body), the screen-stack paint, the
//! world/menu painters, sprite loading (`m.Q()`, `g.b()`), MMAPI, RMS — is a
//! `todo!` naming its slice. Reaching one is a loud seam, never a silent wrong
//! frame.

use crate::common::aj_init;
use crate::game::Game;
use crate::jrandom::JavaRandom;
use crate::loader::{close_pak, open_pak, read_entry, ResourceLoader};
use crate::screens::g_init;
use crate::text::{ae_init, ae_set_font, bind_graphics};
use crate::trig::o_init;
use j2me_jvm::{i32_div, i32_rem, i32_shl, i64_div};
use j2me_me::image::create_image_region;
use j2me_me::{Graphics, Image};
use j2me_nokia::get_direct_graphics;

/// `m.<init> ()V` — the constructor: declaration-order field initializers,
/// then the ctor body (CFR `m.java:273-704`). Resource side effects in bytecode
/// order: `/sincos/*.int` (via `new o()`), `/res/common.utf` (via `new aj`),
/// `/res/res0.pak` entries 0/1/2, then the two `.mid`s (stubbed MMAPI block).
#[allow(clippy::needless_range_loop)] // faithful Java index loops
pub fn m_init(g: &mut Game) {
    // --- field initializers, declaration order ---
    g.m.f135a = 0;
    // f136a (GameMIDlet back-ref) and f137a (Thread handle) collapse (R4).
    g.m.paused = false;
    g.m.quit_flag = false;
    g.m.multiplayer = false;
    // new Random(System.currentTimeMillis()) — clock sample #1.
    g.m.rng = JavaRandom::with_seed(g.clock.current_time_millis());
    g.m.text = ae_init(); // new ae()
    g.m.loader = ResourceLoader::default(); // new j()
    g.m.trig = o_init(g.resources.as_ref()); // new o(): reads /sincos/*.int
    g.m.f142a = vec![None; 15];
    g.m.g = 1;
    g.m.f154a = 0;
    g.m.f155b = 0;
    g.m.f156c = 0;
    g.m.f157d = 0;
    g.m.f158e = 0;
    g.m.f159e = false;
    g.m.f160f = false;
    g.m.f161g = false;
    g.m.f162h = false;
    g.m.f163i = false;
    g.m.f164j = false;
    g.m.f165k = false;
    g.m.f166l = false;
    g.m.f167m = false;
    g.m.f168n = false;
    g.m.f169o = false;
    g.m.f170p = false;
    g.m.f171h = 0;
    g.m.f172a = {
        let mut v = Vec::with_capacity(280);
        v.resize_with(280, || None);
        v
    };
    g.m.f174b = Some(
        Image::create_mutable(176, 208)
            .expect("createImage(176, 208): MIDP positive dims — unguarded in Java"),
    );
    // f175a = f174b.getGraphics() — collapsed (game.rs module doc).
    g.m.f176a = vec![0; 24];
    g.m.f177b = vec![0; 169];
    g.m.f178a = vec![0i64; 12];
    g.m.f179c = vec![0; 27];
    g.m.f180d = vec![0; 50];
    g.m.f181e = vec![0; 13];
    g.m.f182f = vec![0; 20];
    g.m.f189f = 0;
    // --- ctor body ---
    g.m.common = None; // this.f153a = null
                       // Dead stores preserved as record: the body allocates and discards
                       // `{0,0,0}`, the 5×3 `{5..1}` table, `{10,20,30,40,50}`,
                       // `{4,4,3,3,1,1,1,1,1,1}`, and one bare `new Vector()` — no effect.
    g.m.f193a = Vec::new();
    g.m.f194b = Vec::new();
    g.m.f195c = Vec::new();
    g.m.f196d = Vec::new();
    g.m.f197e = Vec::new();
    g.m.f198f = Vec::new();
    g.m.f199g = Vec::new();
    g.m.f200h = Vec::new();
    g.m.f201a = None;
    g.m.f209h = vec![
        vec![52, 52, 52, 54, 54, 54, 55, 55, 55, 55, 56],
        vec![55, 55, 55, 55, 56, 50, 56],
        vec![55, 55, 55, 55, 53, 53, 52, 52, 52, 53, 53, 53, 53, 53, 53],
        vec![55, 55, 55, 55, 53, 53, 52, 52, 52, 56, 56, 56],
        vec![51, 51, 51, 54, 54, 54, 55, 55, 55, 50, 50, 50, 51, 51],
        vec![52, 52, 52, 51, 51, 55, 55, 51],
        vec![
            51, 51, 51, 51, 51, 51, 52, 51, 51, 56, 56, 56, 51, 51, 55, 55, 55,
        ],
        vec![52, 52, 52, 51, 53, 53, 51, 51, 51, 50],
        vec![52, 54, 54, 54, 55, 55, 55, 51, 51],
        vec![51, 51, 51, 51, 51, 51, 55],
        vec![51, 51, 51, 52, 52, 52, 52, 52, 52, 56],
        vec![35, 54, 54, 54],
        vec![52, 55, 51, 51, 51],
        vec![55, 56, 50, 55, 54, 52],
        vec![51, 49, 51, 51, 55, 51, 49, 51, 51, 55, 51, 49, 51, 51, 55],
    ];
    g.m.f210g = vec![0; g.m.f209h.len()];
    g.m.f211k = vec![0; g.m.f209h.len()];
    g.m.f212r = 0;
    g.m.music_players = [None, None]; // new Player[2]
    g.m.f215b = vec![None; 5];
    g.m.f216r = false;
    g.m.f217s = 0;
    g.m.f218t = 0;
    g.m.f219i = Vec::new();
    g.m.f220c = None;
    g.m.f221u = 0;
    g.m.f222v = 0;
    g.m.w = 0;
    g.m.x = 0;
    g.m.y = 15;
    g.m.z = 15;
    g.m.f223A = 0;
    g.m.B = 0;
    g.m.C = 0;
    g.m.D = 0;
    g.m.G = 0;
    g.m.H = 0;
    g.m.f226b = 0;
    g.m.M = 0;
    g.m.P = 0;
    g.m.f227h = vec![0; 280];
    g.m.f228i = vec![0; 280];
    g.m.Q = 0;
    g.m.f229g = 0;
    g.m.f231a = 15;
    g.m.V = 183;
    g.m.f232s = false;
    g.m.f233b = vec![0i16; 320];
    g.m.W = 10;
    g.m.f234j = vec![
        vec![],
        vec![10, 26],
        vec![],
        vec![],
        vec![10, 56],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![8, 27],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    ];
    g.m.X = -1;
    g.m.f236d = None;
    g.m.aa = -1;
    g.m.f238u = false;
    g.m.ab = 0;
    g.m.f239j = Vec::new();
    g.m.f240k = Vec::new();
    g.m.f241l = Vec::new();
    g.m.f242m = Vec::new();
    g.m.f244l = None;
    g.m.f245j = None;
    g.m.f246b = vec![None; 5];
    g.m.f251o = vec![
        26, 44, 46, 33, 47, 48, 27, 28, 29, 30, 31, 36, 37, 38, 39, 40, 41, 42, 43, 36, 37, 38, 39,
        40, 41, 42, 43, 49, 50, 51, 60, 61, 52, 53, 55, 32, 56, 45, 36, 58, 54, 59, 26, 57, 35, 34,
    ];
    g.m.f252k = (0..13).map(|_| vec![3, 3, 3, 5]).collect();
    g.m.f253l = vec![
        vec![1, 1, 0, 1, 1, 0, 1, 2, 0],
        vec![1, 1, 1, 1, 0],
        vec![1, 1, 2, 1, 0],
        vec![1, 1, 3, 1, 0],
        vec![1, 1, 4, 1, 0],
        vec![1, 1, 5, 1, 0],
        vec![1, 1, 6, 1, 0],
        vec![1, 1, 7, 1, 0],
        vec![1, 1, 8, 1, 1, 0, 8, 2, 0],
        vec![1, 1, 9, 1, 1, 0, 9, 2, 0],
        vec![1, 1, 10, 1, 0],
        vec![1, 1, 11, 1, 0],
        vec![1, 1, 12, 1, 0],
    ];
    g.m.f254m = vec![
        vec![20, 12],
        vec![30, 12],
        vec![30, 12, 10, 13],
        vec![30, 12, 20, 13],
        vec![30, 12, 30, 9, 30, 6],
        vec![10, 12, 40, 13, 5, 10],
        vec![10, 12, 40, 13, 15, 10],
        vec![5, 12, 45, 13, 40, 10],
        vec![5, 12, 50, 13, 25, 10],
        vec![5, 12, 45, 13, 30, 10],
        vec![5, 12, 45, 13, 40, 10],
        vec![5, 12, 45, 13, 45, 10],
        vec![5, 12, 50, 13, 50, 10],
    ];
    g.m.active_music_track = -1;
    g.m.music_volume = 40;
    g.m.f255n = Vec::new();
    g.m.f258n = vec![0; 14];
    g.m.f260n = vec![None; 18];
    g.m.f264o = Vec::new();
    g.m.f265x = false;
    g.m.f266y = true;
    g.m.f267p = None;
    g.m.f268z = false;
    g.m.f271p = Vec::new();
    g.m.ao = 0;
    g.m.f273q = vec![0, -1, 0, 1, -1, 0, 1, 0];
    g.m.f276o = vec![
        vec![255, 0, 0],
        vec![124, 124, 124],
        vec![0, 255, 0],
        vec![0, 173, 239],
        vec![0, 0, 255],
        vec![75, 0, 73],
        vec![236, 0, 140],
        vec![242, 151, 34],
    ];
    g.m.f279e = vec![
        vec![vec![]],
        vec![vec![27, 1, 5, 2, 0, 22, 1, 8]],
        vec![vec![]],
        vec![vec![]],
        vec![vec![60, 3, 43, 1, 16, 2, 0, 162, 1, 8]],
        vec![vec![]],
        vec![vec![]],
        vec![vec![116, 0, 165, 1, 1, 13, 2, 8]],
        vec![vec![]],
        vec![vec![]],
        vec![vec![]],
        vec![vec![153, 11, 11, 1], vec![154, 3, 80, 3, 81, 8]],
    ];
    g.m.f280f = vec![
        vec![vec![0, 528, 64, -1, 21, 39, -200]],
        vec![vec![3, 32, 336, -1, -500]],
        vec![vec![1, 16, 16, 40, 33], vec![2, 32, 416, -1, 15]],
        vec![],
        vec![
            vec![6, 16, 816, 6, -1, 27],
            vec![7, 32, 32, -1, 4],
            vec![8, 624, 32, -1, -700],
        ],
        vec![
            vec![9, 288, 208, -1, 7, -4000],
            vec![10, 384, 288, -1, 10, -2000],
            vec![11, 560, 416, -1, -3000],
            vec![12, 304, 416, -1, 19, -1000],
        ],
        vec![],
        vec![],
        vec![],
        vec![
            vec![13, 288, 208, -1, 5, -10],
            vec![14, 384, 288, -1, -5000],
        ],
        vec![],
        vec![],
        vec![vec![15, 0, 0, -1, -300, 15]],
        vec![vec![16, 0, 0, -1, 6]],
        vec![],
        vec![],
        vec![vec![26, 0, 0, 60, 10, 7, 40]],
        vec![],
        vec![vec![20, 0, 0, -1, 1, -1000]],
        vec![
            vec![17, 0, 0, 116, 22],
            vec![17, 0, 0, 116, 16],
            vec![17, 0, 0, 116, 34],
        ],
        vec![vec![21, 0, 0, -1, 25, 28], vec![22, 0, 0, -1, 10]],
        vec![vec![23, 0, 0, -1, 14, 20], vec![24, 0, 0, -1, 5]],
        vec![vec![25, 0, 0, -1, 11]],
        vec![],
    ];
    g.m.f284o = vec![0; 100];
    g.m.f285p = vec![0; 100];
    g.m.f286q = vec![0; 100];
    g.m.f287r = vec![0; 100];
    g.m.f288a = vec![false; 100];
    g.m.f289s = vec![0; 100];
    g.m.f290t = vec![0; 100];
    g.m.f291u = vec![0; 100];
    g.m.aq = 0;
    g.m.ar = 20;
    g.m.r#as = 0;
    g.m.at = 2;
    g.m.au = 100;
    // The 20×16 alpha-gradient bake into f233b:
    // f233b[(i*16)+i2] = (short)(s << 12); s = (short)(s - 1);
    for i in 0..20i32 {
        let mut s: i16 = 15;
        for i2 in 0..16i32 {
            let idx = i.wrapping_mul(16).wrapping_add(i2); // imul, iadd
            g.m.f233b[idx as usize] = i32_shl(s as i32, 12) as i16; // ishl, i2s
            s = ((s as i32).wrapping_sub(1)) as i16; // isub, i2s
        }
    }
    g.m.common = Some(aj_init(g.resources.as_ref())); // new aj(this): /res/common.utf
                                                      // f138a.setSeed(System.currentTimeMillis()) — clock sample #2.
    let seed = g.clock.current_time_millis();
    g.m.rng.set_seed(seed);
    // The res0.pak walk: open with key 'S' (0x53), entries 0/1/2.
    open_pak(&mut g.m.loader, g.resources.as_ref(), "/res/res0.pak", 83);
    let b_arr = read_entry(&mut g.m.loader); // entry 0: the Nival logo PNG
    g.m.f173a = Some(create_image_region(&b_arr, 0, b_arr.len() as i32).expect(
        "IllegalArgumentException: m.<init> createImage(res0.pak entry 0) — unguarded in Java",
    ));
    let b_arr2 = read_entry(&mut g.m.loader); // entry 1: the font atlas PNG
                                              // Java argument order: the descriptor entry (arg 1) is read from the pak
                                              // BEFORE createImage(bArr2) (arg 2) runs — reads stay sequential 0,1,2.
    let descriptor = read_entry(&mut g.m.loader); // entry 2: the font descriptor
    let atlas = create_image_region(&b_arr2, 0, b_arr2.len() as i32).expect(
        "IllegalArgumentException: m.<init> createImage(res0.pak entry 1) — unguarded in Java",
    );
    ae_set_font(&mut g.m.text, &descriptor, atlas);
    // System.gc() — a JVM hint, no observable effect.
    // TODO(next-slice): the MMAPI try-block — `Manager.createPlayer` over
    // `j.a("res/bgsound.mid")` / `j.a("res/bgsound1.mid")`, then
    // realize/prefetch/setLoopCount(-1) per player (the players-loop `iinc`).
    // j2me-me `media` is a stub, so the players stay None — exactly the shipped
    // `catch (Exception) {}` arm (audio absent). First-frame pixels unchanged.
    g.m.text.f = -1; // this.f139a.f = -1
    close_pak(&mut g.m.loader); // this.f140a.a()
    g.m.screens = vec![
        g_init(0),  // f147b
        g_init(3),  // f146a
        g_init(1),  // f148c
        g_init(2),  // f149d
        g_init(4),  // f150e
        g_init(5),  // f151f
        g_init(6),  // f152g
        g_init(7),  // h
        g_init(8),  // i
        g_init(9),  // j
        g_init(10), // k
        g_init(11), // l
        g_init(12), // m
        g_init(13), // n
        g_init(14), // o
        g_init(15), // p
        g_init(16), // q
        g_init(17), // r
        g_init(18), // s
        g_init(19), // t
        g_init(21), // u
        g_init(22), // v
    ];
    let (t, u) = {
        let logo = g.m.f173a.as_ref().expect("set above");
        (logo.width(), logo.height()) // T = getWidth(), U = getHeight()
    };
    g.m.T = t;
    g.m.U = u;
    g.m.R = 88i32.wrapping_sub(i32_div(g.m.T, 2).expect("ArithmeticException")); // idiv, isub
    g.m.S = 104i32.wrapping_sub(i32_div(g.m.U, 2).expect("ArithmeticException")); // idiv, isub
    g.m.f230a = Some(vec![0i16; g.m.T.wrapping_mul(g.m.U) as usize]); // imul
}

/// `m.a ()V` — start: `f137a = new Thread(this); A = false; f137a.start()`.
/// Single-threaded host: arming clears the quit flag; the host then calls
/// [`run_prologue`] once and [`run_tick`] at the loop cadence (the Thread
/// handle collapses, R4).
pub fn m_start(g: &mut Game) {
    g.m.quit_flag = false;
}

/// `m.b ()V` — stop: raise the quit flag, drop the thread handle (collapsed),
/// enter state 100.
pub fn m_stop(g: &mut Game) {
    g.m.quit_flag = true;
    g.m.g = 100;
}

/// `m.run ()V`, before the `while (!A)` loop: publish the static back-refs
/// (`r.a = this; n.f292a = this; g.a = this; q.f313a = this` — collapsed, R4),
/// sample the clock, enter the logo state, run `m.f()V`.
pub fn run_prologue(g: &mut Game) {
    g.m.f154a = g.clock.current_time_millis();
    g.m.g = 1;
    resume_boot(g);
}

/// One `while (!A)` iteration of `m.run ()V`. Returns `f156c`, the
/// `Thread.sleep` amount — the host advances the clock by it before the next
/// tick. `repaint(); serviceRepaints();` is the synchronous owed-paint service
/// (R9): the paint lands into `fb` before the tick returns.
pub fn run_tick(g: &mut Game, fb: &mut Image) -> i64 {
    g.m.f154a = g.clock.current_time_millis();
    if !g.m.paused {
        if g.m.f157d == 0 {
            // lcmp
            g.m.f158e = g.m.f154a;
        } else if g.m.f157d == 9 {
            // lcmp
            g.m.f157d = -1;
        }
        g.m.f157d = g.m.f157d.wrapping_add(1); // ladd
        g.m.f155b = g.m.f154a.wrapping_add(62); // ladd — the 62 ms frame deadline
        update(g); // c()
        clear_key_latches(g); // i()
        if !g.m.quit_flag {
            g.canvas.request_repaint(); // repaint()
            if g.canvas.service_repaints() {
                // serviceRepaints(): the owed paint runs now, synchronously.
                let mut gfx = Graphics::new(fb);
                m_paint(g, &mut gfx);
            }
        }
    }
    g.m.f154a = g.clock.current_time_millis();
    g.m.f156c = g.m.f155b.wrapping_sub(g.m.f154a); // lsub
    if g.m.f156c < 1 {
        // lcmp
        g.m.f156c = 1;
    }
    // Thread.sleep(f156c) — host-advanced virtual time.
    g.m.f156c
}

/// `m.showNotify ()V` — focus gained: clear both key sets, unpause, and (only
/// once the options record exists) restore the music volume twice.
pub fn show_notify(g: &mut Game) {
    clear_key_latches(g); // i()
    clear_key_held(g); // j()
    g.m.paused = false;
    if g.m.f247k.is_some() {
        // i(f247k[2]); i(f247k[2]); — each re-reads the options byte.
        let level = g.m.f247k.as_ref().expect("checked")[2] as i32; // byte widened
        set_music_volume(g, level);
        let level = g.m.f247k.as_ref().expect("checked")[2] as i32;
        set_music_volume(g, level);
    }
}

/// `m.hideNotify ()V` — focus lost: clear both key sets, pause, mute.
pub fn hide_notify(g: &mut Game) {
    clear_key_latches(g); // i()
    clear_key_held(g); // j()
    g.m.paused = true;
    set_music_volume(g, 0); // i(0)
}

/// `m.paint (Ljavax/microedition/lcdui/Graphics;)V` — bind the Graphics for
/// the text helpers, apply the shake, then paint the top of the screen stack
/// (or the base state when the stack is empty).
pub fn m_paint(g: &mut Game, gfx: &mut Graphics<'_>) {
    bind_graphics(gfx); // ae.a(graphics) — static bind, collapsed (text.rs)
    paint_shake(g, gfx); // r(graphics)
    let size = (g.m.f193a.len() as i32).wrapping_sub(1); // isub
    if size >= 0 {
        todo!("next-slice: screen-stack paint — ((g) f193a.elementAt(size)).a(Graphics)");
    } else {
        paint_root(g, gfx); // a(graphics)
    }
}

/// `m.c ()V` — the per-tick update dispatch: top-of-stack screen update, or
/// the base state update when the stack is empty.
pub fn update(g: &mut Game) {
    let size = (g.m.f193a.len() as i32).wrapping_sub(1); // isub
    if size >= 0 {
        todo!("next-slice: screen-stack update — ((g) f193a.elementAt(size)).d()");
    } else {
        update_state(g); // d()
    }
}

/// `m.d ()V` — the base state machine. **This slice**: case 1 (the logo
/// screen) up to its 5000 ms / any-key exit — the transition body (`Q()`,
/// `g()`, the class-`g` sprite loads, the `f215b` bakes) is the next slice's
/// entry point. The other arms are later slices.
pub fn update_state(g: &mut Game) {
    match g.m.g {
        1 => {
            // lsub, lcmp — the 5 s logo dwell, or any key.
            if g.m.f154a.wrapping_sub(g.m.f229g) >= 5000 || g.m.f170p {
                todo!(
                    "next-slice: logo→menu transition (m.d()V case 1 — Q(); g(); \
                     i.b/i.a sprite loads; f215b bakes; f147b.b())"
                );
            }
        }
        5 => todo!("next-slice: map-screen update (m.d()V case 5)"),
        9 => todo!("next-slice: battle update (m.d()V case 9)"),
        101 => todo!("next-slice: save-prompt update (m.d()V case 101)"),
        102 => todo!("next-slice: continue-prompt update (m.d()V case 102)"),
        104 | 106 => todo!("next-slice: in-level update (m.d()V cases 104/106)"),
        _ => {} // no default arm in the Java switch
    }
}

/// `m.f ()V` — the loop's state (re)entry hook: resume-load for states 5/106,
/// then `h()`.
pub fn resume_boot(g: &mut Game) {
    match g.m.g {
        5 | 106 => todo!(
            "next-slice: resume path (m.f()V cases 5/106 — close all screens, g(), \
             m38a(ag, ah), f243v = true)"
        ),
        _ => {}
    }
    enter_state(g); // h()
}

/// `m.h ()V` — mark the state's backdrop ready (`b = true`) and do its entry
/// work; case 1 latches the logo timestamp.
pub fn enter_state(g: &mut Game) {
    g.m.b = true;
    match g.m.g {
        1 => {
            g.m.f229g = g.m.f154a;
        }
        5 | 8 | 9 => {
            todo!("next-slice: map HUD bake (m.h()V cases 5/8/9 — i.h(); i.a(f227h); i.b(); h(1))")
        }
        104 => todo!("next-slice: in-level HUD bake (m.h()V case 104 — i.a(f228i))"),
        106 => todo!("next-slice: in-level resume bake (m.h()V case 106 — + t())"),
        _ => {}
    }
}

/// `m.a (I)I` — `Math.abs(f138a.nextInt() % i)`. An `i` of 0 is an unguarded
/// `ArithmeticException` (the MIDlet dies) — a faithful panic.
pub fn rand_below(g: &mut Game, i: i32) -> i32 {
    i32_rem(g.m.rng.next_int(), i) // irem
        .expect("ArithmeticException: m.a(I)I with i == 0")
        .wrapping_abs()
}

/// `m.i ()V` — clear the edge-latched key flags + the last raw code.
pub fn clear_key_latches(g: &mut Game) {
    g.m.f163i = false;
    g.m.f164j = false;
    g.m.f165k = false;
    g.m.f166l = false;
    g.m.f167m = false;
    g.m.f168n = false;
    g.m.f169o = false;
    g.m.f170p = false;
    g.m.f171h = 0;
}

/// `m.j ()V` — clear the held directional flags + the last raw code.
pub fn clear_key_held(g: &mut Game) {
    g.m.f159e = false;
    g.m.f160f = false;
    g.m.f161g = false;
    g.m.f162h = false;
    g.m.f171h = 0;
}

/// `m.a (Ljavax/microedition/lcdui/Graphics;)V` — paint the state's frame when
/// its backdrop is ready, else run the state entry (`h()`).
pub fn paint_root(g: &mut Game, gfx: &mut Graphics<'_>) {
    if g.m.b {
        paint_state(g, gfx); // b(graphics)
    } else {
        enter_state(g); // h()
    }
}

/// `m.b (Ljavax/microedition/lcdui/Graphics;)V` — the per-state painter.
/// **This slice**: the blink counter and case 1 (the logo). The other arms are
/// later slices (their arithmetic is the tail of this method's recorded shape).
pub fn paint_state(g: &mut Game, gfx: &mut Graphics<'_>) {
    g.m.Q = g.m.Q.wrapping_add(1); // iadd (field increment)
    if g.m.Q == 6 {
        g.m.Q = 0;
    }
    match g.m.g {
        1 => paint_logo(g, gfx), // c(graphics)
        5 | 9 => todo!("next-slice: world paint (m.b(Graphics) cases 5/9 — m/e/d/f painters)"),
        101 => todo!("next-slice: save-prompt paint (m.b(Graphics) case 101)"),
        102 => todo!("next-slice: continue-prompt paint (m.b(Graphics) case 102)"),
        104 | 106 => todo!("next-slice: in-level paint (m.b(Graphics) cases 104/106)"),
        _ => {} // no default arm in the Java switch
    }
}

/// `m.c (Ljavax/microedition/lcdui/Graphics;)V` — THE FIRST FRAME: the Nival
/// logo on a white 176×208 field, faded in through an ARGB4444 white overlay
/// whose alpha nibble decays 15→0 across the first second
/// (`(short)(((1000 - j) * 15) / 1000)` — all-long arithmetic, then l2i+i2s),
/// holds 0 from 1000 ms, and is left untouched (its last value) past 4000 ms.
#[allow(clippy::needless_range_loop)] // faithful Java index loop
pub fn paint_logo(g: &mut Game, gfx: &mut Graphics<'_>) {
    let j = g.m.f154a.wrapping_sub(g.m.f229g); // lsub
    gfx.set_color(16777215);
    gfx.fill_rect(0, 0, 176, 208);
    gfx.draw_image(
        g.m.f173a
            .as_ref()
            .expect("NullPointerException: m.c(Graphics) with f173a null"),
        g.m.R,
        g.m.S,
        20,
    )
    .expect("drawImage: baseline anchor 20 (TOP|LEFT) cannot be rejected");
    if j < 1000 {
        // lcmp; then (1000 - j) * 15 / 1000 in longs: lsub, lmul, ldiv, l2i, i2s.
        g.m.f231a = (i64_div(1000i64.wrapping_sub(j).wrapping_mul(15), 1000)
            .expect("ArithmeticException") as i32) as i16;
    } else if j < 4000 {
        // lcmp
        g.m.f231a = 0;
    }
    // (f231a << 12) + 4095 — ishl, iadd, i2s per pixel; the field value is
    // loop-invariant (nothing in the loop writes it), so one read is identical.
    let alpha = g.m.f231a;
    let buf =
        g.m.f230a
            .as_mut()
            .expect("NullPointerException: m.c(Graphics) with f230a null");
    for i in 0..buf.len() {
        buf[i] = i32_shl(alpha as i32, 12).wrapping_add(4095) as i16;
    }
    // DirectUtils.getDirectGraphics(g).drawPixels(f230a, true, 0, T, R, S, T, U, 0, 4444)
    get_direct_graphics(gfx)
        .draw_pixels_4444(buf, true, 0, g.m.T, g.m.R, g.m.S, g.m.T, g.m.U, 0, 4444)
        .expect("drawPixels: baseline-closed arguments cannot be rejected");
}

/// `m.r (Ljavax/microedition/lcdui/Graphics;)V` — the screen shake: while the
/// countdown runs, translate by two fresh `a(5) - 2` jitters (dx drawn before
/// dy, order preserved — the RNG stream is shared state).
pub fn paint_shake(g: &mut Game, gfx: &mut Graphics<'_>) {
    if g.m.f212r > 0 {
        g.m.f212r = g.m.f212r.wrapping_sub(1); // isub (field decrement)
        let dx = rand_below(g, 5).wrapping_sub(2); // isub
        let dy = rand_below(g, 5).wrapping_sub(2); // isub
        gfx.translate(dx, dy);
    }
}

/// `m.h (I)V` — select_music_track: guarded by `ak != i` (re-selecting the
/// active track is a no-op); on a change mute the old track, switch, restore
/// the saved volume. The `try/catch{}` around each `i(...)` call swallows
/// errors the stub cannot produce.
pub fn select_music_track(g: &mut Game, i: i32) {
    let i2 = g.m.music_volume;
    if g.m.active_music_track != i {
        if g.m.active_music_track >= 0 {
            set_music_volume(g, 0);
        }
        g.m.active_music_track = i;
        if g.m.active_music_track >= 0 {
            set_music_volume(g, i2);
        }
    }
}

/// `m.i (I)V` — set_music_volume. The whole Java body sits in one
/// `try/catch (Exception) {}`: with `ak == -1` the very first `f214a[ak]`
/// indexes out of bounds into the catch; with a null Player (the stubbed MMAPI
/// ctor arm — see [`m_init`]) `getControl` NPEs into the catch. Either way the
/// observable state (`al`) is untouched. Only a live realized player reaches
/// the volume writes — TODO(next-slice): the real MMAPI path on j2me-me
/// `media`; until it exists that branch is a loud seam.
pub fn set_music_volume(g: &mut Game, _i: i32) {
    let ak = g.m.active_music_track;
    if ak < 0 || ak as usize >= g.m.music_players.len() {
        return; // ArrayIndexOutOfBounds → the catch arm
    }
    if g.m.music_players[ak as usize].is_none() {
        return; // NullPointerException on getControl → the catch arm
    }
    todo!("next-slice: MMAPI VolumeControl start/stop (m.i(I)V) — no realized players exist yet");
}
