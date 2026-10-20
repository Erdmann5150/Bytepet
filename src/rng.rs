//! A small PCG32 generator. Its state is saved with the game, so a catch-up after the app was
//! closed replays exactly the same way, and linked games share a seed.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Default for Pcg32 {
    fn default() -> Self {
        Pcg32::new(0x853c_49e6_748f_ea9b)
    }
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut r = Pcg32 { state: 0, inc: (seed.rotate_left(17) << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    /// A seed from the clock, for new games.
    pub fn from_time() -> Self {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Pcg32::new(n ^ (std::process::id() as u64).rotate_left(32))
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform float in [0, 1).
    pub fn f64(&mut self) -> f64 {
        (self.next_u32() as f64) / 4_294_967_296.0
    }

    /// Integer in [lo, hi] inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    /// Integer in [0, n).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len())])
        }
    }

    /// Index chosen by weight.
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        if total == 0 {
            return 0;
        }
        let mut r = self.next_u64() % total;
        for (i, &w) in weights.iter().enumerate() {
            if r < w as u64 {
                return i;
            }
            r -= w as u64;
        }
        weights.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Pcg32::new(42);
        let mut b = Pcg32::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
            let r = a.range(-3, 7);
            assert!((-3..=7).contains(&r));
            b.range(-3, 7);
        }
        let s = serde_json::to_string(&a).unwrap();
        let mut c: Pcg32 = serde_json::from_str(&s).unwrap();
        assert_eq!(a.next_u32(), c.next_u32());
    }
}
