//! The neighbour relation the solver routes through: which cells touch, and which of them
//! water leaves by.
//!
//! # Invariants
//!
//! - **The eight offsets are one cell or `√2` of one, and the drop is normalised by that
//!   distance before it is compared.** An unnormalised drop reads a diagonal `√2` times
//!   steeper than it is for the same height difference, which makes the diagonal preference
//!   systematic rather than incidental — every diagonal run on a slope of one would beat
//!   every axis run on a slope of 1.4, whatever the terrain is.
//! - **A cell with no rock under it has no neighbours at all**, so it can neither send water
//!   nor receive it. The massif is a disc and the lattice a rectangle, and the difference is
//!   carried by the mask rather than by a border rule: a cell past the rim is not a cell at
//!   the edge of a plateau, it is not part of the massif.
//! - **An outlet is a cell on the rim, and only a sink there loses its load.** The rim is
//!   where a massif's valleys reach the foot of it, so it is where its rivers leave; an
//!   interior pit is a lake, and a lake keeps what gathers in it. This is the reference
//!   solver's ocean outlet read onto a footprint, where an ocean stood in for "the boundary
//!   the water actually reaches".

use core::f32::consts::FRAC_1_SQRT_2;

use crate::heightfield::HeightField;

/// The eight neighbours, in the fixed order ties are broken by.
pub const D8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// The reciprocal of each [`D8`] slot's step distance: four edge neighbours are one cell
/// away and four diagonal ones `√2`.
pub const INV_STEP: [f32; 8] = [
    FRAC_1_SQRT_2,
    1.0,
    FRAC_1_SQRT_2,
    1.0,
    1.0,
    FRAC_1_SQRT_2,
    1.0,
    FRAC_1_SQRT_2,
];

/// The steepest distance-normalised drop from one cell to a lower neighbour, or nothing.
///
/// The first strictly greater drop wins, which makes the tie-break the fixed [`D8`] order
/// and nothing else: a cell with two neighbours at exactly the same slope routes to the
/// same one on every run, on every machine, and after every rebuild of the lattice.
pub fn receiver(field: &HeightField, heights: &[f32], at: usize) -> Option<usize> {
    let here = heights[at];
    let mut best = 0.0_f32;
    let mut to = None;
    for (slot, step) in INV_STEP.iter().enumerate() {
        let Some(next) = neighbor(field, at, slot) else {
            continue;
        };
        let drop = (here - heights[next]) * step;
        if drop > best {
            best = drop;
            to = Some(next);
        }
    }
    to
}

/// The steepest distance-normalised drop from one cell to any neighbour, down or not.
///
/// This is the slope the transport law is asked about, and it is the *largest* drop rather
/// than the receiver's: a cell that happens to route sideways still sits on a face whose
/// steepness is what decides how much it can carry.
pub fn slope(field: &HeightField, heights: &[f32], at: usize) -> f32 {
    let here = heights[at];
    let mut best = 0.0_f32;
    for (slot, step) in INV_STEP.iter().enumerate() {
        if let Some(next) = neighbor(field, at, slot) {
            best = best.max((here - heights[next]) * step);
        }
    }
    best
}

/// Returns the cell one [`D8`] step away, where the massif is.
pub fn neighbor(field: &HeightField, at: usize, slot: usize) -> Option<usize> {
    let column = at % field.width();
    let row = at / field.width();
    let (dx, dy) = D8[slot];
    let next = (column as i32 + dx, row as i32 + dy);
    if next.0 < 0 || next.1 < 0 {
        return None;
    }
    field
        .index(next.0 as usize, next.1 as usize)
        .filter(|at| field.is_inside(*at))
}

/// Returns whether a cell sits on the massif's rim, which is where its water leaves.
pub fn is_outlet(field: &HeightField, at: usize) -> bool {
    (0..D8.len()).any(|slot| neighbor(field, at, slot).is_none())
}
