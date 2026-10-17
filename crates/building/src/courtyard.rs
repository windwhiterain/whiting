//! The courtyard: **the wall ring a mass's void is built as**.
//!
//! # Why this is its own module
//!
//! A courtyard used to be the hard part of a building, and the reason was the **shell**: one closed
//! surface had to carry both the plate and a wall standing in the middle of it, so the wall's inner
//! base edge had to pair with the paving *and* with the courtyard-facing face — three faces on one
//! edge, which no winding can close. In the volume model the question is gone: **the plate is a
//! solid of its own and is not cut at all** (the courtyard's floor *is* the plate's own top), and the
//! courtyard is a second ring that stands on it and closes itself. Nothing is shared between the two
//! rings, so nothing can be shared between them wrongly.
//!
//! # What is built
//!
//! Two loops and four surfaces, exactly as the mass's own ring is closed:
//!
//! - **the void loop**, whose faces look *into* the courtyard ([`MassSurface::VoidWall`]);
//! - **the same loop offset outward by `mass.wall`** ([`expand::offset_loop`] with
//!   [`OffsetSide::Outward`]), whose faces look out into the rooms ([`MassSurface::VoidInner`]);
//! - the strip of wall top between them ([`MassSurface::VoidTop`]) and the annulus at the loop's own
//!   elevations ([`MassSurface::VoidBase`]).
//!
//! The result is a torus, so its Euler characteristic is `0` where a plain box is `2`, and it is a
//! **solid of its own** ([`SolidId::Void`]) rather than a second ring inside the mass's one.
//!
//! **The ring itself is the mass's own walk.** What this module owns is the part that is a
//! courtyard's and nobody else's — taking the offset **outward** rather than inward, and refusing a
//! reflex corner before the walk that has no measurement for one is asked — and then it hands the two
//! loops to [`expand::loops_of`], which is the one derivation of "a loop and the loop a wall's
//! thickness away from it", and to [`expand::ring_walk`], which is the same walk that builds a mass's
//! own wall. A hole cut in a courtyard's wall therefore arrives by exactly the path a window in a
//! mass's wall does: one [`Opening`], one bay, one set of reveals, and no second implementation to
//! agree with the first about what a jamb is.
//!
//! **A part's frame is read off the same two loops, and that is why this module has no second ring
//! for it.** A part pinned in a courtyard bay needs the strip of wall the bay was measured on, which
//! is the void loop and its outward offset; [`expand::part_frame`] asks [`expand::loops_of`] for
//! exactly what [`ring`] asks for and reads one edge of the result, so "which way does a courtyard's
//! wall go" has one answer in the crate rather than one per reader of it.
//!
//! # What is refused, and why
//!
//! **A void loop that is not convex is refused by name.** The ring is one quad per loop edge between
//! two loops of the same length — the mass's own construction, which draws an L or a U outline
//! without trouble. The one case this crate has no measurement for is a *void* with a reflex corner,
//! because a mass's outline is offset **inward** and a void is offset **outward**, and the corner
//! that fails in one is not the corner that fails in the other. This crate has shipped a ring fill
//! wrong by area twice and the inward offset has closed a courtyard's mouth, so the restriction is
//! kept rather than lifted on an argument. [`Problem::VoidNotConvex`] is where it says so, and
//! [`Violation::VoidNotConvex`](crate::validate::Violation::VoidNotConvex) reports it in the document
//! where an author meets it first.
//!
//! The offset walk itself refuses everything else by name — a degenerate edge, a corner with no
//! miter, a join past the join limit, an offset that stops enclosing the loop it came from — and
//! every one of those names the **loop** it is about, because a courtyard's fault and a mass's fault
//! are not the same fault. So does every fault about a hole cut into this ring, by the same rule:
//! "opening 0 is outside edge 1" is two different pieces of masonry unless it says which.

use std::collections::BTreeMap;

use crate::expand::{self, Problem, Solid, ring_walk};
use crate::grid::FaceRef;
use crate::mass::{AuthoredLoop, MassId};
use crate::math::Vec3;
use crate::mesh::BuildingFace;
use crate::skeleton::BuildingSkeleton;

/// Builds one courtyard: the ring of wall a mass's void is, in its own point numbering.
///
/// # Invariants
///
/// **It returns points and faces rather than writing to a mesh**, which is what keeps
/// [`expand_mass`](crate::expand)'s whole-list-or-nothing rule when a mass holds several: a ring
/// that cannot be built takes nothing to the mesh, and neither does the wall around it.
///
/// **The winding is normalised before anything is built from the loop**, exactly as
/// [`fill::ring_fill`](crate::fill::ring_fill) normalises the loop it bridges. Which way a void loop
/// is walked is what says which side its material is on, and nothing downstream of here reads the
/// winding as written — a courtyard authored the other way round is a courtyard, not a building that
/// is not drawn.
///
/// **The base is at the void loop's own elevations**, which is the mass's own rule: a mass's wall
/// stands on its outline nodes' elevations and a courtyard's wall stands on its loop's. A loop on a
/// slope therefore gives a base annulus with the same warp a mass's own base has on a slope, and
/// [`FaceNotPlanar`](crate::validate::Violation::FaceNotPlanar) is what names it in both cases.
///
/// `grouped` is the mass's own grouping of every opening in the document by the face it names, handed
/// in rather than rebuilt: a mass may hold several courtyards and each of them reads the one group
/// that is its own, which is what keeps a hole in the mass's outer wall from being cut into a
/// courtyard's ring as well.
pub(crate) fn ring(
    skeleton: &BuildingSkeleton,
    grouped: &BTreeMap<FaceRef, Vec<usize>>,
    mass: MassId,
    void: usize,
    wall: f32,
    top: f32,
    problems: &mut Vec<Problem>,
) -> Result<(Vec<Vec3>, Vec<BuildingFace>), Vec<Problem>> {
    let loop_of = AuthoredLoop::Void { mass, void };
    // **The two loops come from the one derivation every ring asks for**, and with them the side the
    // offset went and the convexity restriction, because that restriction is about the *outward*
    // walk and belongs beside it rather than beside this module's caller. A caller that re-derived
    // them would be a second answer to "which way does a courtyard's wall go".
    let Some(loops) = expand::loops_of(skeleton, loop_of, wall, problems) else {
        return Err(Vec::new());
    };
    let ring = loops.ring(loop_of, wall, top);
    let mut solid = Solid::default();
    ring_walk(&mut solid, &ring, skeleton, grouped, problems);
    if !problems.is_empty() {
        return Err(Vec::new());
    }
    Ok(solid.into_parts())
}
