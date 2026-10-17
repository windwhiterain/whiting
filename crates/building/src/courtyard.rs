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
//! are not the same fault.

use crate::expand::{OffsetSide, Problem, loop_area, offset_loop};
use crate::mass::{AuthoredLoop, MassId, PlanNode};
use crate::math::{Vec2, Vec3, lift};
use crate::mesh::{BuildingFace, Element, MassSurface};

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
/// slope therefore gives a base strip with the same warp a mass's own base has on a slope, and
/// [`FaceNotPlanar`](crate::validate::Violation::FaceNotPlanar) is what names it in both cases.
pub(crate) fn ring(
    mass: MassId,
    void: usize,
    loop_nodes: &[PlanNode],
    wall: f32,
    top: f32,
) -> Result<(Vec<Vec3>, Vec<BuildingFace>), Vec<Problem>> {
    let mut problems = Vec::new();
    let mut walked: Vec<PlanNode> = loop_nodes.to_vec();
    if loop_area(&positions(&walked)) < 0.0 {
        walked.reverse();
    }
    // **Convexity first, because it is the one restriction the offset walk cannot state.** Every
    // other thing the ring needs from its loop is one of the walk's own refusals, which name
    // themselves; a reflex corner is a restriction this crate keeps on purpose, so it is checked
    // here where it can say why.
    if !crate::fill::convex(&positions(&walked)) {
        problems.push(Problem::VoidNotConvex { mass, void });
        return Err(problems);
    }
    let Some(outer) = offset_loop(
        &walked,
        AuthoredLoop::Void { mass, void },
        wall,
        loop_area(&positions(&walked)),
        OffsetSide::Outward,
        &mut problems,
    ) else {
        return Err(problems);
    };

    // **Four points per node, numbered together**, so the ring's four loops are one run and a face
    // names its corners by an index into it rather than by arithmetic over four lists.
    let count = walked.len();
    let mut points = Vec::with_capacity(count * 4);
    let mut base = Vec::with_capacity(count);
    let mut height = Vec::with_capacity(count);
    let mut rooms_base = Vec::with_capacity(count);
    let mut rooms_top = Vec::with_capacity(count);
    for node in 0..count {
        let ground = walked[node].elevation;
        let first = points.len() as u32;
        base.push(first);
        height.push(first + 1);
        rooms_base.push(first + 2);
        rooms_top.push(first + 3);
        points.push(lift(walked[node].position, ground));
        points.push(lift(walked[node].position, top));
        points.push(lift(outer[node], ground));
        points.push(lift(outer[node], top));
    }

    let mut faces = Vec::with_capacity(count * 8);
    for edge in 0..count {
        let next = (edge + 1) % count;
        // **The four windings are the mass's own, with the two loops swapped.** The face at the void
        // loop looks into the courtyard as the inner wall looks into the mass; the face at the offset
        // loop looks out into the rooms as the outer wall looks out of the mass; and the top and base
        // strips run outer-to-inner on the same side each of the mass's own runs. They are read off
        // the mass rather than re-derived per face, because a solid's orientation is one fact about
        // all of its faces and four independent windings are four chances to be wrong.
        quad(
            &mut faces,
            [height[next], height[edge], base[edge], base[next]],
            Element::Mass(mass, MassSurface::VoidWall { void, edge }),
        );
        quad(
            &mut faces,
            [
                rooms_top[edge],
                rooms_top[next],
                rooms_base[next],
                rooms_base[edge],
            ],
            Element::Mass(mass, MassSurface::VoidInner { void, edge }),
        );
        quad(
            &mut faces,
            [rooms_top[edge], height[edge], height[next], rooms_top[next]],
            Element::Mass(mass, MassSurface::VoidTop { void, edge }),
        );
        quad(
            &mut faces,
            [rooms_base[edge], rooms_base[next], base[next], base[edge]],
            Element::Mass(mass, MassSurface::VoidBase { void }),
        );
    }
    Ok((points, faces))
}

/// Returns a loop's plan positions, which is what both its winding and its convexity are read from.
fn positions(loop_nodes: &[PlanNode]) -> Vec<Vec2> {
    loop_nodes.iter().map(|node| node.position).collect()
}

/// Adds a quad as two triangles, wound as it is given.
fn quad(faces: &mut Vec<BuildingFace>, corners: [u32; 4], element: Element) {
    faces.push(BuildingFace {
        indices: [corners[0], corners[1], corners[2]],
        element,
    });
    faces.push(BuildingFace {
        indices: [corners[0], corners[2], corners[3]],
        element,
    });
}
