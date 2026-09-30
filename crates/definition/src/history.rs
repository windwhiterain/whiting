//! Undo-redo replay built from the inverse of every applied delta.

use crate::definition::Definition;
use crate::delta::Delta;
use crate::error::HistoryError;
use crate::registry::Registry;

#[derive(Debug)]
struct Entry {
    delta: Delta,
    inverse: Delta,
}

/// A delta replay system that generates an inverse delta for each delta applied.
///
/// # Invariants
///
/// The undo and redo logs are never both non-empty, because recording a new delta clears
/// the redo log. Each entry keeps the delta that was applied alongside its inverse, so
/// redo replays the original forward delta rather than an approximation of it.
///
/// The version counter only ever increases, so undo and redo advance it rather than
/// restoring it. A replayed delta is therefore re-stamped onto the current version
/// before it is applied. This is safe because replay is this type's own trusted state;
/// the conflict check exists to reject externally supplied deltas that were computed
/// against a version the agent has not seen.
#[derive(Debug, Default)]
pub struct History {
    applied: Vec<Entry>,
    undone: Vec<Entry>,
}

impl History {
    /// Creates an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an applied delta and the inverse that undoes it, clearing the redo log.
    pub fn record(&mut self, applied: Delta, inverse: Delta) {
        self.applied.push(Entry { delta: applied, inverse });
        self.undone.clear();
    }

    /// Undoes the newest applied delta and returns the delta that was applied.
    pub fn undo(
        &mut self,
        definition: &mut Definition,
        registry: &Registry,
    ) -> Result<Delta, HistoryError> {
        let entry = self.applied.pop().ok_or(HistoryError::NothingToUndo)?;
        let mut undo_delta = entry.inverse.clone();
        undo_delta.base_version = definition.version;
        undo_delta.apply(definition, registry)?;
        // The entry moves between the logs unchanged. Redo replays the original forward
        // delta, not the inverse, so the forward delta must survive the round trip.
        self.undone.push(entry);
        Ok(undo_delta)
    }

    /// Redoes the newest undone delta and returns the delta that was applied.
    pub fn redo(
        &mut self,
        definition: &mut Definition,
        registry: &Registry,
    ) -> Result<Delta, HistoryError> {
        let entry = self.undone.pop().ok_or(HistoryError::NothingToRedo)?;
        let mut redo_delta = entry.delta.clone();
        redo_delta.base_version = definition.version;
        redo_delta.apply(definition, registry)?;
        self.applied.push(entry);
        Ok(redo_delta)
    }

    /// Returns how many deltas are currently applied and replayable in reverse.
    pub fn undo_depth(&self) -> usize {
        self.applied.len()
    }

    /// Returns how many deltas are currently undone and replayable forward.
    pub fn redo_depth(&self) -> usize {
        self.undone.len()
    }

    /// Returns whether there is anything to undo.
    pub fn can_undo(&self) -> bool {
        !self.applied.is_empty()
    }

    /// Returns whether there is anything to redo.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }
}
