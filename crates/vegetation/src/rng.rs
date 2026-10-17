//! The plant's one random source, forked by path.
//!
//! # The one idea
//!
//! **A node's randomness is a function of the plant's seed and the node's hand path, and of nothing
//! else.** Inserting a bough therefore does not reshuffle a twig on the other side of the trunk, because
//! no draw depends on the order nodes were created in — which is what makes an edit to a *generated
//! default* local, and the reason a default is ordinary editable data the moment it exists.
//!
//! # The generator
//!
//! SplitMix64's mix function seeds a stream and advances it, so the whole generator is one `u64` of state
//! and two multiplies: no table to keep in step, the same numbers on every platform, and a period no
//! plant can reach. [`Rng::for_path`] folds the path into the seed one index at a time, so a node's stream
//! is derived from its parent's rather than looked up.

/// The multiplication constant SplitMix64's mix uses to advance its state.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// Salt for the root's stream, so that a seed of `0` is still a stream.
const ROOT_SALT: u64 = 0x2545_F491_4F6C_DD1D;

/// One deterministic stream of numbers, derived from a plant's seed and a node's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A stream from a seed alone: the plant's own source, which a node's stream is derived from.
    pub fn new(seed: u64) -> Self {
        Self {
            state: mix(seed ^ ROOT_SALT),
        }
    }

    /// The stream of the node at `path`, in the plant grown from `seed`.
    ///
    /// # Invariants
    ///
    /// **It depends on the seed and the path and on nothing else**, so two plants that differ anywhere
    /// else give the node at `path` the same stream. That is the property that makes an edit local, and
    /// `tree_probe` measures it by inserting a sibling and comparing the stream of an untouched node.
    pub fn for_path(seed: u64, path: &[u32]) -> Self {
        let mut state = mix(seed ^ ROOT_SALT);
        for index in path {
            state = mix(state ^ (u64::from(*index) + 1));
        }
        Self {
            state: mix(state ^ (path.len() as u64)),
        }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = mix(self.state.wrapping_add(GOLDEN));
        self.state
    }

    /// A number in `[0, 1)`, with 24 bits of it, which is exactly what an `f32` mantissa holds.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }

    /// A number in `[low, high)`, or `low` when the range is empty.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        if high <= low {
            return low;
        }
        low + (high - low) * self.unit()
    }

    /// A number in `[-1, 1)`.
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// SplitMix64's finaliser: the avalanche every seed and every advance goes through.
fn mix(state: u64) -> u64 {
    let mut state = state.wrapping_add(GOLDEN);
    state = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^ (state >> 31)
}
