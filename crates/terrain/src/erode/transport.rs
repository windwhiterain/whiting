//! The formula a solve hangs its exchange on, and the one expression every height change in
//! the solver goes through.
//!
//! # Invariants
//!
//! - **The coefficients ride the parameters and only the formula's shape varies per law.**
//!   That is what makes a law a law: two instances of the same shape with two sets of
//!   coefficients are the same operator run twice, and the solver's order — the descending
//!   sweep, the round structure, the books — is written once and is not a parameter of it.
//! - **Height and suspended load exchange one for one.** Everything eroded enters the
//!   suspension and everything deposited leaves it, which is what makes the books close; the
//!   clamp at the end of [`suspend`](Transport::suspend) is where that guarantee lands.

use crate::erode::ErodeParams;

/// The transport law: how much load a cell can hold, and what it holds after it moves.
pub trait Transport {
    /// The suspended load a cell can carry under this much slope and this much water.
    fn capacity(&self, params: &ErodeParams, slope: f32, water: f32) -> f32;

    /// The suspended load left in a cell after its height changed by `delta`.
    fn suspend(&self, params: &ErodeParams, delta: f32, suspended: f32) -> f32;
}

/// The shipped law: carrying capacity proportional to slope and to water.
///
/// # Invariants
///
/// Capacity is **clamped at zero and not at infinity**: a cell that cannot hold load reports
/// none rather than a negative amount, because the exchange below reads the gap between
/// capacity and what is suspended, and a negative capacity would read as a gap worth
/// filling before the cell has been asked whether it is standing under any water at all.
pub struct Hydraulic;

impl Transport for Hydraulic {
    fn capacity(&self, params: &ErodeParams, slope: f32, water: f32) -> f32 {
        (params.capacity * slope * water).max(0.0)
    }

    fn suspend(&self, _params: &ErodeParams, delta: f32, suspended: f32) -> f32 {
        (suspended - delta).max(0.0)
    }
}

/// The net height change of one cell: erosion when the cell cannot carry its load, and
/// deposition when it carries more than it holds, capped at the load actually standing there.
///
/// # Invariants
///
/// - **One expression, so the two places a round would otherwise compute it cannot drift
///   apart.** The height update and the suspension update are two passes over the same
///   numbers, and a solver that wrote the exchange twice wrote it twice.
/// - **The cap is what makes the exchange conservative on the deposit side**: a cell's height
///   cannot rise by more than its own suspension falls. It is part of the solver rather than
///   of the law, so a law cannot price the mass line out of existence by its own formula —
///   a deposition rate of ten is still bounded by what the cell was holding.
pub fn delta_of<L: Transport>(
    params: &ErodeParams,
    law: &L,
    slope: f32,
    water: f32,
    sediment: f32,
) -> f32 {
    let gap = law.capacity(params, slope, water) - sediment;
    if gap > 0.0 {
        -params.erode_rate * gap
    } else {
        (-params.deposit_rate * gap).min(sediment)
    }
}
