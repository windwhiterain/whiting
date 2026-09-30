//! Monotonic version counter of a [`Definition`](crate::Definition).

use std::fmt;

use facet::Facet;

/// Monotonically increasing counter, incremented once per applied [`Delta`](crate::Delta).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Facet)]
#[facet(transparent)]
pub struct Version(u64);

impl Version {
    /// The version a freshly created definition starts at.
    pub const INITIAL: Self = Self(0);

    /// Creates a version from a raw counter.
    pub const fn new(counter: u64) -> Self {
        Self(counter)
    }

    /// Returns the raw counter.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the version that follows this one.
    ///
    /// # Panics
    ///
    /// Panics at `u64::MAX`, because a delta can never be applied past the last version.
    pub const fn incremented(self) -> Self {
        assert!(self.0 < u64::MAX, "version counter is exhausted");
        Self(self.0 + 1)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<u64> for Version {
    fn from(counter: u64) -> Self {
        Self::new(counter)
    }
}
