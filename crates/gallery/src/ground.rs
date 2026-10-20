//! The ground a building scene draws, with every building's own floor cut out of it.
//!
//! # Why the ground is cut
//!
//! A building's plate is a solid whose **top is the ground line**, and the ground is a surface at the
//! ground line too: every pixel of a plate that is visible at all is a **depth tie**, and a raster
//! settles a tie by the order it drew the two surfaces in. That is why a floor has been invisible,
//! or a few pixels, or a count of 1,656 that `--locate` inside answered as a wall. A hole under the
//! plate is what gives the question an answer: inside the hole the plate is the only surface, so a
//! pixel there names the plate or names nothing.
//!
//! # What is cut, and why the plate rather than the footprint
//!
//! The hole is **the ground plate's own plan outline**, not the mass's whole footprint, and both are
//! inside the mass. `derive::starter` seats its plate on the mass's **inner loop**, so cutting the
//! outline would only take away ground that the wall ring's base annulus already stands over and
//! that no camera above the ground can see; `terrace` and `wing` author their plates over the whole
//! footprint instead, and there the hole reaches the wall's outer face exactly. One rule, read off
//! the document, is the right hole in both cases and leaves no moat in either — which a hole quantised
//! to the ground grid could not do, since a ground cell is metres across and a building is metres
//! across too.
//!
//! # Invariants
//!
//! - **The ground stays an element-less mesh.** It carries no `authored_id` and writes the reserved
//!   zero from a constant, exactly as it did. What changed is which triangles it holds.
//! - **Only a plate standing on its host mass's ground level is cut.** A mezzanine is a `Slab` too,
//!   and cutting the ground under one would drop a hole in the site under a floor two metres up. The
//!   test is the plate's own two elevations against the mass's lowest node: a ground plate spans that
//!   node, a mezzanine's underside is above it, and a flat roof's plate is above it as well.
//! - **The hole is the plate, so there is never a floor-less hole.** The loop cut is read from the
//!   same `slabs` the scenes pack their `floors` meshes from, which is what makes "the plate is the
//!   only surface in its own hole" a fact about the document rather than a hope about the picture.
//! - A cell no plate reaches is written **byte for byte** as [`document::geometry::heightfield`] wrote
//!   it: only a cell a cut reached is cut.

use building::Building;
use definition::PackedList;

use document::geometry::{self, Station, between, plan_of};

/// Builds the ground a scene draws, with every building's ground plate cut out of it.
///
/// # Invariants
///
/// The grid, the heights and the divisions are [`document::geometry::heightfield`]'s own, and the plates
/// are read from the **document the scene draws** rather than from a second derivation of it, so a
/// plate the scene forgot to draw is still a plate the ground gets out of the way of.
pub fn mesh(
    building: &Building,
    divisions: usize,
    extent: f64,
    height: impl Fn(f64, f64) -> f64,
) -> PackedList {
    let plates = ground_plates(building);
    geometry::heightfield_over(divisions, extent, height, |cell| outside(cell, &plates))
}

/// Returns the plan loops the ground must not be drawn under: one a building's own floor.
///
/// # Invariants
///
/// A plate with no host, or one whose host the document does not hold, is **not** a building's floor
/// — it is a step or a canopy standing on its own — and cutting the ground under one would be cutting
/// the site out from under something that is not a room.
fn ground_plates(building: &Building) -> Vec<Plate> {
    let skeleton = &building.skeleton;
    skeleton
        .slabs
        .iter()
        .filter(|slab| stands_on_its_mass(slab, skeleton))
        .map(|slab| {
            Plate::new(
                slab.outline
                    .iter()
                    .map(|at| [f64::from(at[0]), f64::from(at[1])])
                    .collect(),
            )
        })
        .collect()
}

/// Returns whether a slab is its host mass's plate on the ground, rather than a floor above it.
///
/// # Invariants
///
/// **It reads the slab's own [`SlabRole`](building::slab::SlabRole), and never its elevations.** The
/// rule this replaced — the slab's two levels span its host's lowest node — is the same guess the
/// building crate's own coverage check used to make, and it had the same millimetre window: a floor
/// whose underside reaches the ground line is a floor and not a plate, yet it satisfied the test and
/// would have had the ground cut away from under the storey above it. A mezzanine 2.2 m up never
/// reached the test at all, which is why no scene saw the window.
///
/// The two remaining stages are the same two the crate's own check keeps: a plate with no host, and a
/// plate whose host the document does not hold, are both slabs standing on their own, and cutting the
/// ground under either is cutting the site out from under something that is not a room. `O(1)`.
fn stands_on_its_mass(slab: &building::slab::Slab, skeleton: &building::BuildingSkeleton) -> bool {
    if slab.role != building::slab::SlabRole::Plate {
        return false;
    }
    let Some(host) = slab.host else {
        return false;
    };
    skeleton.masses.get(host.0).is_some()
}

/// One building's ground plate, as the plan loop the ground is cut away from.
struct Plate {
    /// The plate's own plan outline, as the document holds it and in whatever winding it wound it.
    plan: Vec<[f64; 2]>,
    /// The box that loop fills in plan, which is the one test a cell nowhere near it can fail.
    low: [f64; 2],
    high: [f64; 2],
}

impl Plate {
    /// Builds a plate from its plan outline.
    fn new(plan: Vec<[f64; 2]>) -> Self {
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for at in &plan {
            for axis in 0..2 {
                low[axis] = low[axis].min(at[axis]);
                high[axis] = high[axis].max(at[axis]);
            }
        }
        Self { plan, low, high }
    }

    /// Returns whether a cell's own box shares this plate's.
    ///
    /// It is a bound and not a test of membership: a cell that passes and is nowhere near the plate
    /// is refused later by every split that finds no crossing. A ground of forty thousand cells
    /// spends a box test per plate per cell and nothing else.
    fn reaches(&self, cell: &[Station]) -> bool {
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for station in cell {
            let at = plan_of(*station);
            for axis in 0..2 {
                low[axis] = low[axis].min(at[axis]);
                high[axis] = high[axis].max(at[axis]);
            }
        }
        (0..2).all(|axis| low[axis] <= self.high[axis] && self.low[axis] <= high[axis])
    }
}

/// Returns one ground cell's pieces with every plate's plan region taken out of them.
fn outside(cell: &[Station], plates: &[Plate]) -> Vec<Vec<Station>> {
    let mut left = vec![cell.to_vec()];
    for plate in plates {
        if !plate.reaches(cell) {
            continue;
        }
        let mut cut = Vec::new();
        for piece in left {
            cut.extend(against(piece, plate));
        }
        left = cut;
        if left.is_empty() {
            break;
        }
    }
    left
}

/// Returns the pieces of `piece` that stand **outside** one plate's plan region.
///
/// # Invariants
///
/// - **Every edge of the loop splits the piece, whether or not the piece reaches that edge.** A
///   piece that has been split cannot cross that edge's segment any more, so once every edge has
///   split it, no piece straddles the loop's boundary — and that is what lets the crossing count
///   below *decide* which side of the plate a piece is on rather than infer it. It is also why the
///   loop need not be convex for the cut to be right, and why the pieces can multiply by two per
///   edge and no further: a convex piece split by a line is two convex pieces.
/// - **The winding of the loop is never read.** The crossing count is the same whichever way round
///   the plan polygon was wound, so a plate written the other way round is the same hole.
fn against(piece: Vec<Station>, plate: &Plate) -> Vec<Vec<Station>> {
    let count = plate.plan.len();
    let mut pieces = vec![piece];
    for index in 0..count {
        let from = plate.plan[index];
        let to = plate.plan[(index + 1) % count];
        let mut split = Vec::new();
        for piece in pieces {
            match sides_of(&piece, from, to) {
                (None, None) => split.push(piece),
                (Some(left), None) => split.push(left),
                (None, Some(right)) => split.push(right),
                (Some(left), Some(right)) => {
                    split.push(left);
                    split.push(right);
                }
            }
        }
        pieces = split;
    }
    pieces
        .into_iter()
        .filter(|piece| !holds(plan_middle(piece), &plate.plan))
        .collect()
}

/// Returns a piece's two sides of the line through two plan points, each `None` where that side of
/// the line holds no piece at all.
///
/// # Invariants
///
/// **The line and not the segment**, and the reason it is right is in [`against`]: a piece cut by
/// every edge's line cannot cross any edge's segment, so it cannot straddle the loop. Splitting by
/// the segment instead would be the same cut for fewer pieces and a case this function would have to
/// get right twice.
fn sides_of(
    piece: &[Station],
    from: [f64; 2],
    to: [f64; 2],
) -> (Option<Vec<Station>>, Option<Vec<Station>>) {
    let side = |at: [f64; 2]| {
        (to[0] - from[0]) * (at[1] - from[1]) - (to[1] - from[1]) * (at[0] - from[0])
    };
    (half(piece, &side, true), half(piece, &side, false))
}

/// Returns the part of a convex piece on one side of a line, or `None` when that side holds nothing.
///
/// # Invariants
///
/// **A corner standing on the line is a corner of both sides**, so the crossing point is written only
/// where the two ends are strictly opposite: writing it where one of them is already on the line
/// would put the same corner in the piece twice, and a piece with a doubled corner has a middle that
/// is no longer inside it.
fn half(piece: &[Station], side: &impl Fn([f64; 2]) -> f64, above: bool) -> Option<Vec<Station>> {
    let count = piece.len();
    let mut out = Vec::with_capacity(count + 1);
    for index in 0..count {
        let here = piece[index];
        let there = piece[(index + 1) % count];
        let (at_here, at_there) = (side(plan_of(here)), side(plan_of(there)));
        if if above {
            at_here >= 0.0
        } else {
            at_here <= 0.0
        } {
            out.push(here);
        }
        if (at_here < 0.0 && at_there > 0.0) || (at_here > 0.0 && at_there < 0.0) {
            out.push(between(here, there, at_here / (at_here - at_there)));
        }
    }
    (out.len() >= 3).then_some(out)
}

/// Returns whether a plan point stands inside a closed loop, by the crossing count.
///
/// # Invariants
///
/// An edge counts when its two ends straddle the point **in `z`**, which is the half-open rule: it
/// makes a point exactly as high as a vertex count once rather than twice, so the count is right on
/// the loop's own boundary as well as away from it. It is read on a piece's **middle** and never on
/// a corner, because a corner of a cut piece stands exactly on the plate's own edge by construction.
fn holds(point: [f64; 2], plan: &[[f64; 2]]) -> bool {
    let mut crossings = 0;
    for index in 0..plan.len() {
        let from = plan[index];
        let to = plan[(index + 1) % plan.len()];
        if (from[1] > point[1]) == (to[1] > point[1]) {
            continue;
        }
        let at = from[0] + (point[1] - from[1]) * (to[0] - from[0]) / (to[1] - from[1]);
        if point[0] < at {
            crossings += 1;
        }
    }
    crossings % 2 == 1
}

/// Returns a point strictly inside a convex piece: the mean of its corners.
///
/// # Invariants
///
/// Every corner carries a positive weight in that mean, so for a piece with any area at all the mean
/// is interior — which is the property [`holds`] needs, because a point on the boundary is counted by
/// one side of the rule or the other and either answer would drop or keep the wrong piece.
fn plan_middle(piece: &[Station]) -> [f64; 2] {
    let mut middle = [0.0f64; 2];
    for station in piece {
        let at = plan_of(*station);
        middle[0] += at[0];
        middle[1] += at[1];
    }
    let count = piece.len() as f64;
    [middle[0] / count, middle[1] / count]
}
