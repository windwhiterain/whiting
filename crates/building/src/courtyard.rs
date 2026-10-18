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
//! - **the void loop**, whose faces look *into* the courtyard
//!   ([`MassSurface::VoidWall`](crate::mesh::MassSurface::VoidWall));
//! - **the same loop offset outward by `mass.wall`** (`expand::offset_loop` with
//!   `OffsetSide::Outward`), whose faces look out into the rooms
//!   ([`MassSurface::VoidInner`](crate::mesh::MassSurface::VoidInner));
//! - the strip of wall top between them ([`MassSurface::VoidTop`](crate::mesh::MassSurface::VoidTop))
//!   and the annulus at the loop's own elevations
//!   ([`MassSurface::VoidBase`](crate::mesh::MassSurface::VoidBase)).
//!
//! The result is a torus, so its Euler characteristic is `0` where a plain box is `2`, and it is a
//! **solid of its own** ([`SolidId::Void`](crate::mesh::SolidId::Void)) rather than a second ring
//! inside the mass's one.
//!
//! **The ring itself is the mass's own walk.** What this module owns is the part that is a
//! courtyard's and nobody else's — taking the offset **outward** rather than inward — and then it
//! hands the two loops to `expand::loops_of`, which is the one derivation of "a loop and the loop a
//! wall's thickness away from it", and to `expand::ring_walk`, which is the same walk that builds a
//! mass's own wall. A hole cut in a courtyard's wall therefore arrives by exactly the path a window in
//! a mass's wall does: one [`Opening`](crate::openings::Opening), one bay, one set of reveals, and no
//! second implementation to agree with the first about what a jamb is.
//!
//! **A part's frame is read off the same two loops, and that is why this module has no second ring
//! for it.** A part pinned in a courtyard bay needs the strip of wall the bay was measured on, which
//! is the void loop and its outward offset; [`expand::part_frame`] asks `expand::loops_of` for
//! exactly what `ring` asks for and reads one edge of the result, so "which way does a courtyard's
//! wall go" has one answer in the crate rather than one per reader of it.
//!
//! # What is refused, and why
//!
//! **A void loop with a reflex corner is walked, and the one shape that cannot be is refused by
//! number rather than by name.** The ring is one quad per loop edge between two loops of the same
//! length — the mass's own construction, which draws an L or a U outline without trouble — and the
//! outward walk was measured across a reflex corner rather than argued about: on an L, a T, a
//! rectangle with a bite and a courtyard open on one side it mitred every corner `wall` out along both
//! of its edges, produced a simple loop enclosing exactly `area + wall · perimeter +
//! wall² · Σ tan(turn / 2)`, and closed into a ring whose volume was `(outer − inner) × height` to
//! better than `2 × 10⁻⁵ m²` — the same `f32` noise the convex case already carries. What is left is
//! narrower and is arithmetic rather than taste: **a bite in the loop has to be wider than
//! `2 × wall`**, because the ring stands `wall` out of the loop and so `wall` into each side of the
//! bite. Every way of failing that is one of `expand::offset_loop`'s own faults and each names the
//! **loop** it is about, so a courtyard's fault and a mass's fault are never the same fault.
//!
//! [`Violation::VoidNotSimple`](crate::validate::Violation::VoidNotSimple) and
//! [`Violation::VoidReachesTheWall`](crate::validate::Violation::VoidReachesTheWall) are the other two
//! courtyard faults in the document half. The first is about the **loop's own shape** — a void loop that
//! meets itself at a point its author did not write down, which the walk already refuses by
//! [`Problem::OffsetSelfIntersecting`] or [`Problem::MiterTooLong`] and which `validate` now also says in
//! those terms, because a subtraction that happened to disagree with itself is not the sentence. The
//! second is the only one about the ring rather than the loop: a void loop inside the outline is not a
//! courtyard that *fits*, because its wall stands outside it.

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
