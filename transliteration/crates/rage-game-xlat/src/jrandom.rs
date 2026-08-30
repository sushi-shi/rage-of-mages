//! `java.util.Random` — the exact 48-bit linear-congruential generator behind
//! `m.f138a` (the gameplay RNG, symbols.toml `m.a: Ljava/util/Random;` = rng).
//! To be *provably the same program* the sequence must be bit-for-bit Java's,
//! so this reproduces the algorithm from the JDK spec.
//!
//! Implementation #1 support: a JVM primitive the transliteration relies on (it
//! belongs to the `java.util` runtime, not to the game). Production
//! `new Random(System.currentTimeMillis())` routes the seed through the
//! injectable `j2me_jvm::Clock`, so a test run is deterministic (R3: seed
//! nondeterminism).

/// `java.util.Random`. The state is the 48-bit `seed`; `nextInt()` is `next(32)`.
#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: i64,
}

const MULTIPLIER: i64 = 0x5DEECE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

impl JavaRandom {
    /// `new Random(seed)` — scrambles the seed exactly as the JDK does.
    pub fn with_seed(seed: i64) -> Self {
        Self {
            seed: (seed ^ MULTIPLIER) & MASK,
        }
    }

    /// `setSeed(seed)` — `m.<init>` re-seeds `f138a` from the clock right after
    /// the `aj` parse.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
    }

    /// `protected int next(int bits)` — advance the LCG and take the top `bits`.
    fn next(&mut self, bits: i32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        // Java: `(int)(seed >>> (48 - bits))`. `seed` is a masked non-negative
        // 48-bit value; unsigned shift then narrowing matches `(u64 >> n) as i32`.
        ((self.seed as u64) >> (48 - bits)) as i32
    }

    /// `nextInt()` — a uniformly distributed 32-bit `int` (the only draw the
    /// baseline uses: `m.a(I)I` computes `Math.abs(nextInt() % i)`).
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
}

impl Default for JavaRandom {
    /// A deterministic default (seed 0); `m.<init>` overrides from the clock.
    fn default() -> Self {
        JavaRandom::with_seed(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The JDK documents these exact outputs for `new Random(0)`. If the LCG
    // were wrong by one constant the very first value would differ — a real
    // can-fail control (R3).
    #[test]
    fn matches_reference_java_sequence_seed_0() {
        let mut r = JavaRandom::with_seed(0);
        assert_eq!(r.next_int(), -1155484576);
        assert_eq!(r.next_int(), -723955400);
        assert_eq!(r.next_int(), 1033096058);
    }

    #[test]
    fn set_seed_restarts_the_sequence() {
        let mut r = JavaRandom::with_seed(0);
        let _ = r.next_int();
        r.set_seed(0);
        assert_eq!(r.next_int(), -1155484576);
    }
}
