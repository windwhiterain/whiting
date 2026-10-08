//! Recording what an emit had to leave out.

use super::error::UnsupportedField;
use std::fmt;

/// What a degrading emit dropped.
///
/// # Why the caller gets this rather than a log line
///
/// A dropped field is invisible in the output. The surface renders as though the field
/// were zero, and nothing in the image says otherwise. An author looking at a frame has no
/// way to tell a deliberate omission from a bug, which is the same failure this whole
/// module is arranged to avoid. Handing the caller a list makes the omission something a
/// program can act on — a log line, a warning on stderr, a field on a render report —
/// rather than something only the emitter knew.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LossyReport {
    dropped: Vec<UnsupportedField>,
}

impl LossyReport {
    /// Records one field that could not be carried.
    pub(crate) fn record(&mut self, field: UnsupportedField) {
        self.dropped.push(field);
    }

    /// The fields that were dropped, in the order they were met.
    pub fn dropped(&self) -> &[UnsupportedField] {
        &self.dropped
    }

    /// Whether anything was dropped.
    pub fn is_empty(&self) -> bool {
        self.dropped.is_empty()
    }

    /// How many fields were dropped.
    pub fn len(&self) -> usize {
        self.dropped.len()
    }
}

impl fmt::Display for LossyReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dropped.is_empty() {
            return formatter.write_str("nothing was dropped");
        }
        write!(
            formatter,
            "{} field(s) dropped, the images are approximate:",
            self.dropped.len()
        )?;
        for field in &self.dropped {
            write!(formatter, "\n  - {field}")?;
        }
        Ok(())
    }
}
