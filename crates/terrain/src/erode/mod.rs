//! Iterative hydraulic erosion over a height field: multiple flow direction routing
//! weighted by slope, flow accumulation in descending height order, and one Eulerian
//! sediment step per round.
//!
//! # Invariants
//!
//! - **The round is: read the field, accumulate, then exchange.** Shares and slopes are
//!   read from the heights at the start of the round; the accumulation cascades water and
//!   suspended load together down the *previous* round's routing; and only then does the
//!   fresh routing take over. Routing by the current field instead would let a cell hand
//!   its water to a neighbour whose height it had not yet seen, and the accumulation would
//!   depend on the order the update happened to write in.
//! - **The routing fans out.** A cell hands its water and its load to every neighbour
//!   below it, in shares of the slope each descends by, rather than to the steepest one
//!   alone. A single receiver concentrates every cell's flow onto grid-aligned paths, and
//!   the channels that leaves read as rows of dots along the lattice diagonals rather than
//!   as drainage. The shares sum to one, so what leaves a cell arrives across its
//!   neighbours and the books close on the fan-out exactly as they did on the hop.
//! - **A cell with nothing below it keeps everything it holds.** Its shares are all zeros,
//!   so the fan-out moves nothing out of it: a pit is a lake, not a drain, and it is the
//!   only kind of cell the exchange reads as a sink at all.
//! - **The visit that hands water down a chain hands standing load down it too**, at
//!   [`ErodeParams::carry`] of what the cell holds. Load therefore cascades exactly as far
//!   as the routes reach and is never deleted on the way; the only place material leaves
//!   the massif is an outlet, and the books close on that.
//! - **The books close.** `eroded − deposited = suspended + spilled`, and `initial − final =
//!   suspended + spilled`, up to the order the rounds are summed in. An interior pit keeps
//!   what gathers in it, because the deposit is capped at the load that arrived.
//! - **Cells with no rock under them are never touched.** Not visited, not routed to, not
//!   updated. The massif is a disc and the lattice a rectangle, and the rectangle's corners
//!   are not ground the solver is entitled to move.
//!
//! # What this port does not carry from the reference solver
//!
//! The scheme is `planet_x`'s `px_field_alg::erode`, transcribed. Four things in it are not
//! here, and each is a consequence of this workspace rather than a change of scheme:
//!
//! - **The cube atlas.** A `Domain::Cube` field has no boundary, so the reference gives it
//!   an ocean outlet or a degenerate drain. A massif's boundary is its own footprint, which
//!   is already in [`grid`], and there is nothing left for an ocean to stand in for.
//! - **The band threading.** The reference splits its two stencil passes across threads by
//!   row band. There is no thread model here to split across, and the accumulation was
//!   serial there too — so the whole round is one pass, and the only thing lost is wall
//!   clock on the two passes that were already the cheap half.
//! - **The field width limit.** The reference carries receivers in an `f32` band buffer and
//!   refuses a field wider than `2^24` cells because the index stops being exact. Here the
//!   route is a share per neighbour slot over `usize` indices, so there is no such limit and
//!   no error for it.
//! - **The scratch routing's initial state.** The reference's scratch buffer starts with
//!   receiver zero for every cell, so on the first round every cell's water is dumped into
//!   cell zero and its capacity is inflated for that round. There is no load to route on the
//!   first round — it is all zero — so this starts with no route anywhere instead, and the
//!   first round is a round of rainfall rather than a round of one very wet corner.

pub mod grid;
pub mod order;
pub mod transport;

use crate::heightfield::{FieldError, HeightField};

pub use grid::{D8, INV_STEP};
pub use order::by_height_descending;
pub use transport::{Hydraulic, Transport, delta_of};

/// The water a cell can hold.
///
/// A depression a few cells deep is full, and a solver that accumulates without a bound runs
/// away in the pits rather than in the channels: a pit gathers from everything above it and
/// its capacity then grows with what it gathered, so an unbounded solver digs a hole whose
/// own slope keeps it digging.
const WATER_MAX: f32 = 64.0;

/// The power the fan-out weights each descending direction's share by.
///
/// One is the share of the slope itself, so a cell's flow goes where its ground actually
/// falls and every lower neighbour takes some of it. It is a constant rather than a knob in
/// [`ErodeParams`] because it is a property of the routing scheme, not of a solve: a caller
/// who wants the steepest-descent scheme back reads [`grid::receiver`] and routes by that,
/// and a caller who wants a different dial gets it by asking [`grid::fractions`] for one.
const FLOW_EXPONENT: f32 = 1.0;

/// The knobs one solve reads.
///
/// Every field is a plain scalar under a stable name, so a caller can write a repair row
/// that names it. The output shape follows the input field, so there is no shape knob here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErodeParams {
    /// How much water every cell starts each round with.
    pub rainfall: f32,
    /// The suspended load scale: a cell's capacity is this times its slope times its water.
    pub capacity: f32,
    /// How much of a cell's excess capacity it cuts away per round.
    pub erode_rate: f32,
    /// How much of a cell's unmet capacity it fills per round.
    pub deposit_rate: f32,
    /// The share of a cell's standing load that travels with its water each round.
    pub carry: f32,
    /// How many rounds to run.
    pub iterations: u32,
}

impl Default for ErodeParams {
    /// Returns the parameters a solve runs with when a caller says nothing.
    ///
    /// They are the reference solver's, unchanged, and they are calibrated for a field
    /// measured in the hundreds over tens of thousands of cells rather than for any
    /// particular massif. What they produce is a first answer to measure and not a
    /// landscape, which is why the gallery's `landscape` scene runs them and prints the
    /// books before it draws anything.
    fn default() -> Self {
        Self {
            rainfall: 1.0,
            capacity: 0.02,
            erode_rate: 0.5,
            deposit_rate: 0.25,
            carry: 0.5,
            iterations: 40,
        }
    }
}

/// What a solve left besides the field.
///
/// Every quantity is in the field's own height units. Transport is conservative: the only
/// place material leaves the massif is an outlet zeroing a sink's standing load, so the
/// height book closes as `initial − final = suspended + spilled`.
#[derive(Debug, Clone, PartialEq)]
pub struct Books {
    /// Per-cell summed thickness this solve's deposition added.
    ///
    /// Non-negative, and an increment: a round deposits or does nothing, never removes by
    /// depositing.
    pub thickness: Vec<f32>,
    /// Total height removed by erosion across the solve.
    pub eroded: f64,
    /// Total height added by deposition across the solve; equals `Σ thickness`.
    pub deposited: f64,
    /// Load still held at the last round's end.
    pub suspended: f64,
    /// Load that left the massif at its outlets.
    pub spilled: f64,
}

/// The per-solve buffers, kept across the rounds so that a solve allocates once.
struct Scratch {
    /// The cells with rock under them, ascending, which is the order ties are broken by.
    inside: Vec<u32>,
    /// The visiting order, rebuilt from `inside` every round.
    order: Vec<u32>,
    /// The other half of the radix sort's ping-pong.
    aux: Vec<u32>,
    /// How much water each cell holds.
    water: Vec<f32>,
    /// The round's steepest distance-normalised drop per cell.
    slope: Vec<f32>,
    /// The shares the accumulation follows: the previous round's routing, one slot per
    /// [`D8`] neighbour and all zero for a cell with nothing below it.
    routing: Vec<[f32; 8]>,
    /// The shares just computed, waiting to become next round's routing.
    computed: Vec<[f32; 8]>,
}

impl Scratch {
    /// Lays out every buffer a solve over this field will need.
    fn of(field: &HeightField) -> Scratch {
        let cells = field.cells();
        let inside = field
            .inside()
            .iter()
            .enumerate()
            .filter(|(_, inside)| **inside)
            .map(|(at, _)| at as u32)
            .collect::<Vec<_>>();
        Scratch {
            order: vec![0; inside.len()],
            aux: vec![0; inside.len()],
            inside,
            water: vec![0.0; cells],
            slope: vec![0.0; cells],
            routing: vec![[0.0; 8]; cells],
            computed: vec![[0.0; 8]; cells],
        }
    }
}

/// Erodes a field, and returns the field it left behind.
///
/// The lattice, the mask and [`HeightField::base`] are the input's: erosion moves rock, and
/// the ground beyond the rim is not rock the solver was asked to move.
pub fn erode<L: Transport>(
    params: &ErodeParams,
    field: &HeightField,
    law: &L,
) -> Result<HeightField, FieldError> {
    erode_traced(params, field, law).map(|(field, _)| field)
}

/// [`erode`] plus the transport books.
pub fn erode_traced<L: Transport>(
    params: &ErodeParams,
    field: &HeightField,
    law: &L,
) -> Result<(HeightField, Books), FieldError> {
    let mut heights = field.heights().to_vec();
    let mut sediment = vec![0.0; field.cells()];
    let mut books = Books {
        thickness: vec![0.0; field.cells()],
        eroded: 0.0,
        deposited: 0.0,
        suspended: 0.0,
        spilled: 0.0,
    };
    let mut scratch = Scratch::of(field);
    for _ in 0..params.iterations {
        round(
            params,
            law,
            field,
            &mut heights,
            &mut sediment,
            &mut books,
            &mut scratch,
        );
    }
    books.suspended = sediment.iter().map(|value| *value as f64).sum();
    Ok((field.with_heights(heights)?, books))
}

/// One erosion round over the whole field.
fn round<L: Transport>(
    params: &ErodeParams,
    law: &L,
    field: &HeightField,
    heights: &mut [f32],
    sediment: &mut [f32],
    books: &mut Books,
    scratch: &mut Scratch,
) {
    for index in scratch.inside.iter() {
        let at = *index as usize;
        scratch.slope[at] = grid::slope(field, heights, at);
        scratch.computed[at] = grid::fractions(field, heights, at, FLOW_EXPONENT);
    }

    by_height_descending(
        heights,
        &scratch.inside,
        &mut scratch.order,
        &mut scratch.aux,
    );
    scratch.water.fill(params.rainfall);
    for index in scratch.order.iter() {
        let at = *index as usize;
        // Water and load ride the same fan-out, so one descending visit cascades both
        // downstream exactly as far as the routes reach. The shares are read in the fixed
        // `D8` order and sum to one, so a cell that has somewhere lower to go hands over
        // everything it carries and a pit hands over nothing and keeps it all. The clamp on
        // the load is the conservativity of the exchange: for any `carry` a round moves part
        // of what the cell stands in, and what leaves a cell is subtracted from that same
        // cell, so nothing is created and nothing is deleted on the way.
        let carried = (sediment[at] * params.carry).clamp(0.0, sediment[at]);
        for (slot, share) in scratch.routing[at].iter().enumerate() {
            if *share == 0.0 {
                continue;
            }
            let Some(to) = grid::neighbor(field, at, slot) else {
                continue;
            };
            scratch.water[to] = (scratch.water[to] + scratch.water[at] * share).min(WATER_MAX);
            let moving = carried * share;
            sediment[at] -= moving;
            sediment[to] += moving;
        }
    }
    std::mem::swap(&mut scratch.routing, &mut scratch.computed);

    for index in scratch.inside.iter() {
        let at = *index as usize;
        let delta = delta_of(
            params,
            law,
            scratch.slope[at],
            scratch.water[at],
            sediment[at],
        );
        let mut after = law.suspend(params, delta, sediment[at]);
        // An outlet that is also a sink is water leaving the massif, so its load leaves
        // with it; a sink out on the plain is a lake and keeps what it gathered.
        if grid::is_outlet(field, at) && grid::is_sink(&scratch.routing[at]) {
            books.spilled += after as f64;
            after = 0.0;
        }
        if delta > 0.0 {
            books.thickness[at] += delta;
            books.deposited += delta as f64;
        } else {
            books.eroded += -(delta as f64);
        }
        heights[at] += delta;
        sediment[at] = after;
    }
}
