//! Reports what a built building is and what an author has got wrong, measured rather than
//! asserted.
//!
//! This is the reporting vehicle for the `building` crate's `validate` module, and the pattern is
//! `mountain`'s census probe: it prints facts, never asserts intent. Two things make it worth
//! running rather than a list of pass messages:
//!
//! - The **base case** is a hand-built reference volume — a wall ring and the plate under it — because
//!   a check that is only ever shown on a derived building shows itself working only where `derive`
//!   happens to be right. The reference is what a correct expansion of a mass looks like, so zero
//!   violations on it is the reference every other number is read against.
//! - Every check has a **positive control** beside it: the same cube damaged one way, or the same
//!   starter broken one way, so a check that cannot be made to fire is visible as such rather than
//!   as silence.
//!
//! ```text
//! cargo run -p whiting-building --example validate_probe
//! ```

use whiting_building::BuildingSkeleton as Skeleton;
use whiting_building::derive::{RoofStyle, Starter, floor_slab, starter};
use whiting_building::grid::{FaceRef, GRID_TOLERANCE, Rhythm};
use whiting_building::mass::{Attachment, Mass, MassId, PlanNode};
use whiting_building::math::Vec2;
use whiting_building::mesh::{
    BuildingFace, BuildingMesh, Element, MassSurface, RoofSurface, SolidId,
};
use whiting_building::openings::{Inset, Opening, Openings};
use whiting_building::ops::{clear_face, split_face};
use whiting_building::roof::{CrestPoint, EaveStation, Roof, RoofPlane, RoofRow};
use whiting_building::skeleton::{Building, BuildingSkeleton};
use whiting_building::slab::{SlabId, SlabSurface};
use whiting_building::validate::{Violation, document, mesh};

/// The reference volume's shape: a square `PLAN` wide, a wall `WALL` thick standing `TOP` up, and a
/// plate `SLAB` thick under it.
///
/// A point is `[x, y, z]` with **`y` up**, so a plan point is the `x, z` pair — the crate's
/// [`lift`](whiting_building::math::lift) is the one place the two are joined. The inner loop is
/// `WALL` in from the outline, so the ring's section is an annulus `PLAN - 2 x WALL` across.
const PLAN: f32 = 2.0;
const WALL: f32 = 0.4;
const SLAB: f32 = 0.4;
const TOP: f32 = 2.0;

/// The ring's sixteen welded points: the outline's four corners at its base and at the wall top, and
/// the four corners `WALL` metres inside them at the same two elevations.
const RING_POINTS: [[f32; 3]; 16] = [
    // the outline at the base, in the loop's own order
    [0.0, 0.0, 0.0],
    [PLAN, 0.0, 0.0],
    [PLAN, 0.0, PLAN],
    [0.0, 0.0, PLAN],
    // the outline at the wall top
    [0.0, TOP, 0.0],
    [PLAN, TOP, 0.0],
    [PLAN, TOP, PLAN],
    [0.0, TOP, PLAN],
    // the inner loop at the base
    [WALL, 0.0, WALL],
    [PLAN - WALL, 0.0, WALL],
    [PLAN - WALL, 0.0, PLAN - WALL],
    [WALL, 0.0, PLAN - WALL],
    // the inner loop at the wall top
    [WALL, TOP, WALL],
    [PLAN - WALL, TOP, WALL],
    [PLAN - WALL, TOP, PLAN - WALL],
    [WALL, TOP, PLAN - WALL],
];

/// The ring's thirty-two facets: four edges and four surfaces each, two triangles per quad, with
/// the base annulus last because that is the order `expand`'s own ring walk emits them in.
///
/// Every quad is wound out of its own material, and the windings are the four the walk uses: the
/// authored face across its top and base, the top annulus inward, the offset face back, and the
/// base annulus from the outer loop across to the inner one.
const RING_FACES: [([u32; 3], MassSurface); 32] = [
    // edge 0
    ([4, 5, 1], MassSurface::OuterWall { edge: 0 }),
    ([4, 1, 0], MassSurface::OuterWall { edge: 0 }),
    ([4, 12, 13], MassSurface::WallTop { edge: 0 }),
    ([4, 13, 5], MassSurface::WallTop { edge: 0 }),
    ([13, 12, 8], MassSurface::InnerWall { edge: 0 }),
    ([13, 8, 9], MassSurface::InnerWall { edge: 0 }),
    // edge 1
    ([5, 6, 2], MassSurface::OuterWall { edge: 1 }),
    ([5, 2, 1], MassSurface::OuterWall { edge: 1 }),
    ([5, 13, 14], MassSurface::WallTop { edge: 1 }),
    ([5, 14, 6], MassSurface::WallTop { edge: 1 }),
    ([14, 13, 9], MassSurface::InnerWall { edge: 1 }),
    ([14, 9, 10], MassSurface::InnerWall { edge: 1 }),
    // edge 2
    ([6, 7, 3], MassSurface::OuterWall { edge: 2 }),
    ([6, 3, 2], MassSurface::OuterWall { edge: 2 }),
    ([6, 14, 15], MassSurface::WallTop { edge: 2 }),
    ([6, 15, 7], MassSurface::WallTop { edge: 2 }),
    ([15, 14, 10], MassSurface::InnerWall { edge: 2 }),
    ([15, 10, 11], MassSurface::InnerWall { edge: 2 }),
    // edge 3
    ([7, 4, 0], MassSurface::OuterWall { edge: 3 }),
    ([7, 0, 3], MassSurface::OuterWall { edge: 3 }),
    ([7, 15, 12], MassSurface::WallTop { edge: 3 }),
    ([7, 12, 4], MassSurface::WallTop { edge: 3 }),
    ([12, 15, 11], MassSurface::InnerWall { edge: 3 }),
    ([12, 11, 8], MassSurface::InnerWall { edge: 3 }),
    // the base annulus, one strip per edge
    ([0, 1, 9], MassSurface::WallBase),
    ([0, 9, 8], MassSurface::WallBase),
    ([1, 2, 10], MassSurface::WallBase),
    ([1, 10, 9], MassSurface::WallBase),
    ([2, 3, 11], MassSurface::WallBase),
    ([2, 11, 10], MassSurface::WallBase),
    ([3, 0, 8], MassSurface::WallBase),
    ([3, 8, 11], MassSurface::WallBase),
];

/// The plate's eight welded points: the **whole** outline at the ring's base elevation and one
/// `SLAB` below it.
///
/// The plate is written twice over — four points at `0` and four at `-SLAB` — rather than sharing the
/// ring's own base corners, because the expander builds the two solids separately and splices them
/// separately: they stand at one elevation over the same ground and never name the same vertex.
const PLATE_POINTS: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [PLAN, 0.0, 0.0],
    [PLAN, 0.0, PLAN],
    [0.0, 0.0, PLAN],
    [0.0, -SLAB, 0.0],
    [PLAN, -SLAB, 0.0],
    [PLAN, -SLAB, PLAN],
    [0.0, -SLAB, PLAN],
];

/// The plate's twelve facets: two caps over the whole outline and two triangles per edge.
const PLATE_FACES: [([u32; 3], SlabSurface); 12] = [
    ([3, 1, 0], SlabSurface::Top),
    ([1, 3, 2], SlabSurface::Top),
    ([7, 4, 5], SlabSurface::Bottom),
    ([5, 6, 7], SlabSurface::Bottom),
    ([4, 0, 1], SlabSurface::Edge { index: 0 }),
    ([4, 1, 5], SlabSurface::Edge { index: 0 }),
    ([5, 1, 2], SlabSurface::Edge { index: 1 }),
    ([5, 2, 6], SlabSurface::Edge { index: 1 }),
    ([6, 2, 3], SlabSurface::Edge { index: 2 }),
    ([6, 3, 7], SlabSurface::Edge { index: 2 }),
    ([7, 3, 0], SlabSurface::Edge { index: 3 }),
    ([7, 0, 4], SlabSurface::Edge { index: 3 }),
];

/// Builds the reference volume: the wall ring and the plate under it, twenty-four points and
/// forty-four facets, both wound outward and both closed.
///
/// # The numbers every other reading is against
///
/// | solid | faces | points | volume | euler | loops | manifold |
/// | --- | --- | --- | --- | --- | --- | --- |
/// | the ring, [`SolidId::Mass`] | 32 | 16 | `5.1200` | **`0`** | 0 | true |
/// | the plate, [`SolidId::Slab`] | 12 | 8 | `1.6000` | **`2`** | 0 | true |
///
/// **The ring's `0` and the plate's `2` are the two halves of `check_solid`'s expectation, and they
/// are not interchangeable.** A ring is a tube: its outer and inner surfaces meet at the wall top
/// and again along the base annulus, so the surface is a torus and `2 - 2g = 0` at one genus. A
/// plate is a prism, so it is a sphere and reads `2`. `check_solid` derives the same two numbers from
/// the document rather than from the mesh — `2 - 2 x (one ring + holes)` for anything that is a ring
/// and `2` for anything that is not — so **a reference that read `2` for the mass would report
/// `SolidNotASphere` on a mass that is exactly right**, which is what this fixture did while it was
/// still the shell's single closed box.
///
/// The ring's volume is `(PLAN^2 - (PLAN - 2 x WALL)^2) x TOP = 2.56 x 2` and the plate's is
/// `PLAN^2 x SLAB`: both are on paper beside the measurement, so the two readings can differ for a
/// reason rather than because the sum of the triangles is what volume means.
fn cube() -> BuildingMesh {
    let mut built = ring();
    let below = plate();
    let offset = built.points.len() as u32;
    built.points.extend(below.points);
    built.faces.extend(below.faces.into_iter().map(|mut face| {
        face.indices = face.indices.map(|corner| corner + offset);
        face
    }));
    built
}

/// Builds the wall ring alone: a closed tube, and the only one of the two solids that is a mass.
fn ring() -> BuildingMesh {
    BuildingMesh {
        points: RING_POINTS.to_vec(),
        faces: RING_FACES
            .iter()
            .map(|(indices, surface)| BuildingFace {
                indices: *indices,
                element: Element::Mass(MassId(0), *surface),
            })
            .collect(),
    }
}

/// Builds the plate alone: the prism over the whole outline, which is a slab solid and nothing else.
fn plate() -> BuildingMesh {
    BuildingMesh {
        points: PLATE_POINTS.to_vec(),
        faces: PLATE_FACES
            .iter()
            .map(|(indices, surface)| BuildingFace {
                indices: *indices,
                element: Element::Slab(SlabId(0), *surface),
            })
            .collect(),
    }
}

/// Returns the reference with one face's winding reversed: an inconsistent surface, closed and
/// edge-manifold, which is the only way to open a surface without breaking manifoldness.
///
/// Face `0` is the ring's first outer wall, so this damages the mass and leaves the plate green.
fn one_face_turned_inside_out() -> BuildingMesh {
    let mut built = cube();
    built.faces[0].indices.swap(1, 2);
    built
}

/// Returns the reference with one face taken away: a hole with no area, so its plan area cannot see it.
fn one_face_missing() -> BuildingMesh {
    let mut built = cube();
    built.faces.remove(6);
    built
}

/// Returns the reference with one face written twice: an edge three faces use, which no orientation
/// of a closed surface can fix.
fn one_face_duplicated() -> BuildingMesh {
    let mut built = cube();
    built.faces.insert(6, built.faces[0]);
    built
}

/// Returns the reference with one corner lifted a quarter of a metre off the planes it belongs to.
///
/// Point `6` is the ring's outline corner at `z = PLAN` under the wall top, so three of the ring's
/// surfaces leave their planes and the plate is untouched.
fn one_corner_lifted() -> BuildingMesh {
    let mut built = cube();
    built.points[6][1] += 0.25;
    built
}

/// Returns two plates welded into one solid: closed, manifold, outward, and two spheres.
///
/// **The plates are the pair, and not the rings, because the rings would have gone quiet.** Two rings
/// in one [`SolidId::Mass`] handle read euler `0 + 0 = 0`, which is exactly what `check_solid` expects
/// of one ring — the failure [`SolidId::Void`] exists to name, reached by the fixture rather than by a
/// document. Two plates in one [`SolidId::Slab`] handle read `2 + 2 = 4` against an expected `2`, so
/// the control still says what it was built to say: one handle holding two bodies is not a solid.
fn two_plates_one_solid() -> BuildingMesh {
    let first = plate();
    let offset = first.points.len() as u32;
    let mut built = plate();
    built.points.extend(
        first
            .points
            .iter()
            .map(|point| [point[0] + 4.0, point[1], point[2]]),
    );
    built
        .faces
        .extend(first.faces.iter().map(|face| BuildingFace {
            indices: face.indices.map(|corner| corner + offset),
            element: face.element,
        }));
    built
}

/// Returns the outline of a square mass standing from `from` to `to` in plan.
fn square(from: (f32, f32), to: (f32, f32), elevation: f32) -> Vec<PlanNode> {
    vec![
        PlanNode::new([from.0, from.1], elevation),
        PlanNode::new([to.0, from.1], elevation),
        PlanNode::new([to.0, to.1], elevation),
        PlanNode::new([from.0, to.1], elevation),
    ]
}

/// Returns a house with one mass and a two plane roof over it.
fn house() -> BuildingSkeleton {
    let mass = Mass::new(square((0.0, 0.0), (6.0, 6.0), 0.0), 3.2);
    let roof = Roof::new(MassId(0), square((0.0, 0.0), (6.0, 6.0), 3.1), 0.25)
        .with_crest(vec![
            CrestPoint::new([3.0, 0.0], 4.6),
            CrestPoint::new([3.0, 6.0], 4.6),
        ])
        .with_planes(vec![
            RoofPlane::new(vec![
                RoofRow::new(0, EaveStation::new(0.0)),
                RoofRow::new(0, EaveStation::new(0.5)),
            ]),
            RoofPlane::new(vec![
                RoofRow::new(0, EaveStation::new(0.5)),
                RoofRow::new(1, EaveStation::new(0.5)),
            ]),
        ]);
    BuildingSkeleton::new(vec![mass], vec![roof])
}

/// Returns the starter hip's 7x11 document, whose eave stands 0.5 m outside a 7x11 mass.
///
/// It is the building the coverage check is read against: four planes over a 96 square metre eave,
/// which is the shape whose planes tile the loop exactly when every one of them reaches it.
fn hip_starter() -> BuildingSkeleton {
    starter(&Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    })
    .skeleton
}

/// Returns the starter hip with its last plane taken away: an eave corner the roof no longer
/// reaches, authored by deleting one strip of rows rather than by moving a station.
///
/// The expander closes whatever skin it is handed — the skin, its own reverse one thickness below,
/// and a fascia band around the **skin's own boundary** — so this is the control the closure
/// predicates cannot make: every one of them stays green and the roof has a hole in it.
fn hip_starter_missing_an_end_plane() -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    skeleton.roofs[0].planes.pop();
    skeleton
}

/// Returns the starter hip with one triangle of the wall ring's base annulus deleted: a ring that no
/// longer closes on its own inner loop, which is the mass-side reading of the same fault.
///
/// It is the **base strip** that is cut and not the plate's underside, because the base is the mass's
/// own face; the plate is a solid of its own and has faults of its own to report.
fn hip_starter_base_short() -> (BuildingSkeleton, BuildingMesh) {
    let skeleton = hip_starter();
    let mut built = Building::new(skeleton.clone())
        .build()
        .expect("the starter expands");
    let base: Vec<usize> = built
        .faces_of(SolidId::Mass(MassId(0)))
        .into_iter()
        .filter(|index| {
            matches!(
                built.faces[*index].element,
                Element::Mass(_, MassSurface::WallBase)
            )
        })
        .collect();
    built.faces.remove(base[0]);
    (skeleton, built)
}

/// Returns the hip starter with its plate pulled `inset` metres in on every side, so it no longer
/// reaches the outline of the mass it stands under.
///
/// **The plate is the only thing that moves.** A mass has one plate and one outline, and an author
/// arrives at this fault by carrying a footprint across from a smaller building rather than by
/// editing the mass, so the mass is left exactly as `derive::starter` wrote it and every number the
/// check reports is the plate's alone against the outline it has to meet.
fn hip_starter_plate_too_small(inset: f32) -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    shrink_loop(&mut skeleton.slabs[0].outline, inset);
    skeleton
}

/// Walks a plan loop in by `inset` metres on every side, which is the one change that makes a slab
/// measurably short of the floor it stands over.
///
/// **It is the centre of the loop's own bounding box**, which is the middle of a rectangle's four
/// corners, so the loop goes in by `inset` on all four sides whatever size the starter is, and the
/// area it loses is `inset` times the loop's perimeter — the same conversion
/// [`COVERAGE_TOLERANCE`] makes, so a reader can see the shortfall without expanding it. `O(nodes)`.
fn shrink_loop(outline: &mut [Vec2], inset: f32) {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for point in outline.iter() {
        for axis in 0..2 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    let middle = [(low[0] + high[0]) * 0.5, (low[1] + high[1]) * 0.5];
    for point in outline.iter_mut() {
        for axis in 0..2 {
            point[axis] += inset * (middle[axis] - point[axis]).signum();
        }
    }
}

/// Returns the hip starter with an intermediate floor on its own inner loop: a second hosted slab on
/// the same mass, and deliberately smaller than the outline.
///
/// **This is the control the plate check's staging needs, and it is the reason the staging exists.**
/// [`floor_slab`] seats the floor on the mass's inner loop so the floor *meets* its walls rather than
/// running under them, and it names the same host a plate does. Nothing but the slab's own elevations
/// tells a floor from a plate, and a floor that covered the whole outline would be a deck running
/// through the masonry. On this starter the floor measures `6.4000 x 10.4000 = 66.5600` against the
/// outline's `77.0000`, so an unstaged check would fire on it loudly.
fn hip_starter_with_a_floor() -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    let mass = skeleton.masses[0].clone();
    let floor = floor_slab(&mass, MassId(0), 2.2, 0.22).expect("2.2 m is inside a 3.2 m wall");
    skeleton.slabs.push(floor);
    skeleton
}

/// Returns the hip starter with a floor **half a millimetre above its mass's ground line**: the one
/// placement a rule that picks the plate out of the elevations cannot tell.
///
/// [`floor_slab`] refuses it — its guard is `elevation > lowest`, so a floor is never *at* the ground
/// — so the floor is derived at `2.2` and walked down onto `lowest + 0.0005`, which is inside the
/// millimetre of margin a rule reading elevations has to admit. Its loop is then pulled in half a
/// metre on every side, because a floor seated on the mass's own inner loop covers that loop exactly
/// and a coverage check would then have nothing to report: **the pair with
/// [`hip_starter_plate_too_small`] is what makes the reading mean something**, since the two slabs
/// differ in nothing but what the document calls them and how high they stand.
fn hip_starter_with_a_floor_at_the_ground_line() -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    let mass = skeleton.masses[0].clone();
    let mut floor = floor_slab(&mass, MassId(0), 2.2, 0.22).expect("2.2 m is inside a 3.2 m wall");
    let thickness = floor.thickness();
    let lowest = mass
        .outline
        .iter()
        .map(|node| node.elevation)
        .fold(f32::INFINITY, f32::min);
    floor.top = lowest + 0.0005;
    floor.bottom = floor.top - thickness;
    shrink_loop(&mut floor.outline, 0.5);
    skeleton.slabs.push(floor);
    skeleton
}

/// Returns the hip starter carrying **one hosted slab of every role**, with each of them seated on
/// `host`: its own plate, an intermediate floor on the same inner loop, and the plate a flat roof is.
///
/// # Why one document carries all three
///
/// The three are either one fault or they are not, and a fixture that shows them one at a time cannot
/// answer that: three documents that each happen to be silent is also what a check reading only the
/// plate would produce. One document with the seating moved on all three at once is the pair that makes
/// the answer mean something, and it is the only control here where the roles are the *only* difference
/// between the two reports — the plan, the elevations and the host index are one value read twice.
///
/// **The roof is the flat arm's own slab**, carried across from a [`Starter`] built with
/// [`RoofStyle::Flat`], because that is the only place the crate builds a
/// [`SlabRole::Roof`](whiting_building::slab::SlabRole::Roof). It is
/// seated over the **eave** — the outline offset *outward* by the overhang and sunk — which is the half
/// of the question the plate and the floor do not raise: a roof is seated on something a plate never
/// was, and the check still says the same thing about it. The hip starter's own pitched roof stays where
/// it is, because a validation control has no business changing what is drawn.
///
/// `flat.slabs[0]` is that roof rather than the ground plate the same starter also carries: the flat arm
/// puts what stands on the wall top first and the plate the mass stands on second, and this fixture
/// needs the one that hangs over the eave.
fn three_hosted_roles(host: MassId) -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    let mass = skeleton.masses[0].clone();
    let flat = starter(&Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Flat,
        ..Starter::default()
    })
    .skeleton;
    let floor = floor_slab(&mass, MassId(0), 2.2, 0.22).expect("2.2 m is inside a 3.2 m wall");
    skeleton.slabs.push(floor);
    skeleton.slabs.push(flat.slabs[0].clone());
    for slab in &mut skeleton.slabs {
        slab.host = Some(host);
    }
    skeleton
}

/// Returns the hip starter with its plate taken away: a mass standing on nothing.
///
/// **This is the case the report has to answer in one line, and the answer is that it is not a
/// fault.** A part template's skeleton is one mass and no slabs — the shutter is exactly that — and
/// the `yard` scene's store carries no plate because its plate was never carried across with its
/// mass. A check that reported this would report the crate's own documents, so the check starts from
/// the slabs a document holds rather than from the masses it wants to see them under.
fn hip_starter_with_no_plate() -> BuildingSkeleton {
    let mut skeleton = hip_starter();
    skeleton.slabs.clear();
    skeleton
}

/// Returns the reference with one corner walked onto the line through its own face's other two
/// corners: that face's first facet has no area, and nothing about it has left its plane.
///
/// This is the fault a planarity check cannot see either — the other corners of the wall that corner
/// belonged to are still in their planes — and it is the shape a node written one step along the edge
/// behind it takes. Closure is untouched by it on purpose: the edges are welded by index, so a corner
/// standing on a line is still a corner every neighbour shares.
///
/// **Point `5` is walked to `1.5 x` along the edge from the ring's first outer wall's corner `4`
/// through `2`**, which is past both ends and therefore distinct from every other point: the facet
/// `[4, 5, 2]` becomes exactly collinear and the walk is visible as a shape rather than as a
/// coincidence. [`two_corners_at_one_point`] walks the same corner to the one place this one is not.
fn one_corner_on_the_line() -> BuildingMesh {
    let mut built = cube();
    built.points[5] = [3.0, -1.0, 0.0];
    built
}

/// Returns the reference with one corner walked onto **the position of its own neighbour on the face
/// beside it**: two corners at one point, so the facets between them have no area.
///
/// It walks the **same corner onto the same neighbour** that [`one_corner_on_the_line`] walks away
/// from, so the two controls read the same facet `[4, 5, 1]` and the only difference between them is
/// the one under test: a corner a step further along its line, or a corner on top of another.
///
/// **This is the class `FaceDegenerate` reports**, and it is the other class from
/// [`one_corner_on_the_line`]: the phase's own square hip roof stood two crest points at one spot —
/// points are welded by authored identity, so they are two indices — and the triangle between them
/// measured exactly zero while closure, Euler, manifoldness and orientation were all green. A
/// collinear triple is not reported, because a split host needs it to close; a coincidence is always
/// a fault, because no authored element has two corners in one place on purpose.
fn two_corners_at_one_point() -> BuildingMesh {
    let mut built = cube();
    built.points[5] = built.points[4];
    built
}

/// Returns a quad of four corners lying on the sloped plane `z = x + y`, whose normal
/// `(−1, −1, 1)` has three non-zero components, with one corner `lift` off it along that normal.
///
/// **The sloped plane is the whole point of this control.** A fitter that answers with a coordinate
/// axis reads a group of this shape perfectly well when it lies flat, because the flat plane's
/// normal happens to be an axis, and reads it as wildly out of plane when it does not — which is
/// why a control made of walls and floors cannot catch it and this one can. Measured on this tip
/// before the fitter was fixed: the hip starter's own trapezoid, exactly coplanar, read a departure
/// of `0.8`, which is the height the crest stands above the eave.
fn sloped_quad(lift: f32) -> BuildingMesh {
    let corner = [-1.0, -1.0, 1.0];
    let normal = corner.map(|one| one / 3.0f32.sqrt());
    let mut built = BuildingMesh::new();
    built.points = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 2.0],
        [0.0, 2.0, 2.0],
        [
            1.0 + normal[0] * lift,
            3.0 + normal[1] * lift,
            4.0 + normal[2] * lift,
        ],
    ];
    let element = Element::Mass(MassId(0), MassSurface::WallTop { edge: 0 });
    built.faces = vec![
        BuildingFace {
            indices: [0, 1, 2],
            element,
        },
        BuildingFace {
            indices: [0, 2, 3],
            element,
        },
    ];
    built
}

/// Returns the area one face covers, printed so that the control's threshold is read against the
/// number the check is comparing and not against the word "degenerate".
fn triangle_area(built: &BuildingMesh, index: usize) -> f32 {
    let [a, b, c] = built.triangle(&built.faces[index]);
    let normal = whiting_building::math::cross3(
        whiting_building::math::subtract(b, a),
        whiting_building::math::subtract(c, a),
    );
    0.5 * (whiting_building::math::dot3(normal, normal)).sqrt()
}

/// Returns the distinct solids a mesh holds, in the order its faces first name them.
///
/// **This is [`validate::mesh`](whiting_building::validate::mesh)'s own gathering rule** — a
/// `BuildingMesh` carries no count of what it was built from, so the solids are read out of the
/// faces' own provenance — and a mesh of one solid reports one solid rather than an empty reading for
/// every kind the document could have held.
fn solids(built: &BuildingMesh) -> Vec<SolidId> {
    let mut found: Vec<SolidId> = Vec::new();
    for face in &built.faces {
        let solid = face.element.solid();
        if !found.contains(&solid) {
            found.push(solid);
        }
    }
    found
}

/// Prints one mesh's size, every solid it holds with its four measurements and its coverage reading,
/// and every violation the mesh check found for the whole mesh.
///
/// The violations are printed once per mesh and not once per solid, because [`mesh`] answers about
/// the whole mesh: printing them under each solid would report the same list twice over a fixture
/// that is two solids, and say the fault belonged to whichever solid happened to be read last.
fn report_mesh(name: &str, skeleton: &BuildingSkeleton, built: &BuildingMesh) {
    println!(
        "{name}: {} faces, {} points",
        built.faces.len(),
        built.points.len()
    );
    for solid in solids(built) {
        report_solid(skeleton, built, solid);
    }
    report(&mesh(skeleton, built));
}

/// Prints one solid's four measurements and the two areas the coverage check compares.
fn report_solid(skeleton: &BuildingSkeleton, built: &BuildingMesh, solid: SolidId) {
    println!(
        "  {solid:?}: {} faces, volume {:.4}, euler {}, boundary loops {}, manifold {}",
        built.faces_of(solid).len(),
        built.solid_signed_volume(solid),
        built.solid_euler_characteristic(solid),
        built.solid_boundary_loops(solid),
        built.solid_is_manifold(solid),
    );
    report_areas(skeleton, built, solid);
}

/// Prints the coverage reading: the plan area of the surface the check measures and the plan area
/// of the loop it has to reach, which are the two numbers the violation carries.
///
/// **The two surfaces measured are the two that cover an authored loop.** A roof's skin has to tile
/// its eave loop and a plate's top has to stand over its host mass's outline; a mass's ring base is
/// the annulus between the outline and the offset loop, which is not a loop an author drew and
/// cannot be compared with the outline's area — the reading the shell model made possible, and the
/// one [`Violation::RoofDoesNotCover`] is still the last word on.
fn report_areas(skeleton: &BuildingSkeleton, built: &BuildingMesh, solid: SolidId) {
    let faces = built.faces_of(solid);
    let measured: Vec<usize> = faces
        .iter()
        .copied()
        .filter(|index| {
            matches!(
                built.faces[*index].element,
                Element::Roof(_, RoofSurface::Skin { .. }) | Element::Slab(_, SlabSurface::Top)
            )
        })
        .collect();
    let covered = built.plan_area_of(&measured);
    let authored = authored_loop_area(skeleton, solid);
    println!(
        "  coverage: measured {:.4} against authored loop {}, over {} faces",
        covered.abs(),
        authored
            .map(|area| format!("{area:.4}"))
            .unwrap_or_default(),
        measured.len(),
    );
}

/// Returns the plan area a solid's measured surface has to reach, or `None` when the solid names no
/// loop at all.
///
/// **A plate's loop is its host mass's outline and every other slab's is its own**, and the two are
/// told apart by the slab's elevations — a plate's top stands at its host's ground line, while an
/// intermediate floor and a flat roof stand above it and cover less than the outline on purpose. That
/// rule is **re-derived here rather than asked of the validator**, so the number printed and the
/// number reported are two readings rather than one answer printed twice, the way [`report_strips`]
/// re-derives the ground line under a hole.
fn authored_loop_area(skeleton: &BuildingSkeleton, solid: SolidId) -> Option<f32> {
    match solid {
        SolidId::Roof(id) => skeleton.roof(id).map(|roof| roof.plan_area()),
        SolidId::Slab(id) => {
            let slab = skeleton.slab(id)?;
            match slab.host.and_then(|host| skeleton.mass(host)) {
                Some(mass) if slab.top <= mass.lowest() => Some(mass.plan_area()),
                _ => Some(slab.plan_area()),
            }
        }
        // A mass's ring is an annulus, a part has no loop of its own in this document — its geometry
        // lives in its template — a wall is a box rather than a loop, and a courtyard's ring is the
        // annulus between two loops of its own.
        SolidId::Mass(_) | SolidId::Part(_) | SolidId::Wall(_) | SolidId::Void { .. } => None,
    }
}

/// Prints every violation the document check found for a skeleton.
fn report_document(name: &str, skeleton: &BuildingSkeleton) {
    println!(
        "{name}: {} masses, {} roofs",
        skeleton.masses.len(),
        skeleton.roofs.len(),
    );
    report(&document(skeleton));
}

/// Prints a violation list, and says which variants it holds.
fn report(violations: &[Violation]) {
    println!("  violations: {}", violations.len());
    for violation in violations {
        println!("    {violation:?}");
    }
    let named: Vec<&str> = violations.iter().map(variant_name).collect();
    println!("  variants fired: {}", named.join(", "));
}

/// Returns the name of a violation's variant, which is what a positive control is read by.
fn variant_name(violation: &Violation) -> &'static str {
    match violation {
        Violation::OutlineReversed { .. } => "OutlineReversed",
        Violation::OutlineTooShort { .. } => "OutlineTooShort",
        Violation::OutlineNotSimple { .. } => "OutlineNotSimple",
        Violation::ZeroHeight { .. } => "ZeroHeight",
        Violation::UnattachedOverlap { .. } => "UnattachedOverlap",
        Violation::MassHostMissing { .. } => "MassHostMissing",
        Violation::RoofHostMissing { .. } => "RoofHostMissing",
        Violation::RoofEaveTooShort { .. } => "RoofEaveTooShort",
        Violation::RoofPlaneTooShort { .. } => "RoofPlaneTooShort",
        Violation::RoofCrestMissing { .. } => "RoofCrestMissing",
        Violation::DidNotExpand { .. } => "DidNotExpand",
        Violation::SolidOpen { .. } => "SolidOpen",
        Violation::SolidNonManifold { .. } => "SolidNonManifold",
        Violation::SolidInsideOut { .. } => "SolidInsideOut",
        Violation::SolidNotASphere { .. } => "SolidNotASphere",
        Violation::RoofDoesNotCover { .. } => "RoofDoesNotCover",
        Violation::RoofSkinNotADisc { .. } => "RoofSkinNotADisc",
        Violation::RoofSkinLeavesTheEave { .. } => "RoofSkinLeavesTheEave",
        Violation::GridFaceMissing { .. } => "GridFaceMissing",
        Violation::GridTooShort { .. } => "GridTooShort",
        Violation::GridNotOrdered { .. } => "GridNotOrdered",
        Violation::GridOutsideFace { .. } => "GridOutsideFace",
        Violation::PlacementTemplateMissing { .. } => "PlacementTemplateMissing",
        Violation::PlacementBayMissing { .. } => "PlacementBayMissing",
        Violation::PlacementOutsideBay { .. } => "PlacementOutsideBay",
        Violation::PlacementWallMissing { .. } => "PlacementWallMissing",
        Violation::PlacementOffItsWall { .. } => "PlacementOffItsWall",
        Violation::PlacementMountOnAWall { .. } => "PlacementMountOnAWall",
        Violation::PlacementSocketMissing { .. } => "PlacementSocketMissing",
        Violation::PlacementAnchorMissing { .. } => "PlacementAnchorMissing",
        Violation::WallTooShort { .. } => "WallTooShort",
        Violation::WallNotThick { .. } => "WallNotThick",
        Violation::WallTooLow { .. } => "WallTooLow",
        Violation::SlabNotThick { .. } => "SlabNotThick",
        Violation::SlabNotAPolygon { .. } => "SlabNotAPolygon",
        Violation::SlabHostMissing { .. } => "SlabHostMissing",
        Violation::PlateCoverage { .. } => "PlateCoverage",
        Violation::OpeningBayMissing { .. } => "OpeningBayMissing",
        Violation::OpeningExceedsBay { .. } => "OpeningExceedsBay",
        Violation::OpeningOutsideWall { .. } => "OpeningOutsideWall",
        Violation::OpeningOverlaps { .. } => "OpeningOverlaps",
        Violation::OpeningOnAFarLoop { .. } => "OpeningOnAFarLoop",
        Violation::FaceNotPlanar { .. } => "FaceNotPlanar",
        Violation::VoidNotInside { .. } => "VoidNotInside",
        Violation::VoidNotSimple { .. } => "VoidNotSimple",
        Violation::VoidOverlaps { .. } => "VoidOverlaps",
        Violation::VoidReachesTheWall { .. } => "VoidReachesTheWall",
        Violation::FaceDegenerate { .. } => "FaceDegenerate",
    }
}

/// Returns a house with a rhythm on all four faces: three bays across and two storeys up, so every
/// bay is `2.0` m wide and `1.6` m tall on a 6 m wall standing `3.2` m high over a `0.4` m slab.
fn gridded_house() -> BuildingSkeleton {
    let mut skeleton = house();
    for face in skeleton.faces_of(MassId(0)) {
        split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 2 },
        )
        .expect("a 6 by 6 wall carries three bays across and two storeys up");
    }
    skeleton
}

/// Returns the gridded house with a window and a door, both inset on every side: the opening half's
/// reference, the way the welded cube is the mesh half's.
fn windowed_house() -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    let window = Opening::new(
        FaceRef::new(MassId(0), 0),
        0,
        1,
        Inset::new(0.25, 0.25, 0.5, 0.5),
    );
    // A door is a hole in the bottom row whose sill is at the elevation the mass is authored at: the
    // wall face stands on the slab's underside, one slab below it, so the wall under that sill is the
    // slab's own thickness and the hole is interior like any other.
    let door = Opening::new(
        FaceRef::new(MassId(0), 1),
        0,
        1,
        Inset::new(0.25, 0.25, 0.0, 0.5),
    );
    skeleton.openings.push(window);
    skeleton.openings.push(door);
    skeleton
}

/// Returns the level house with one hole in the bottom row, its sill `sill` metres above the bay's own
/// sill.
///
/// **Two door shapes, and both are legal.** An inset only ever moves a hole inward, so the sill can
/// be raised but never lowered: `0.0` is a door, sitting on the elevation the mass is authored at with
/// one slab of wall under it, and `0.4` — the mass's own slab thickness — stands a second slab higher
/// and clears the ground line by twice the slab. Nothing about the bottom row can reach the ground
/// line on level ground; that takes a wall on a slope, which is what [`sloped_house`] carries.
fn bottom_row_hole(sill: f32) -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    skeleton.openings.push(Opening::new(
        FaceRef::new(MassId(0), 0),
        0,
        1,
        Inset::new(0.25, 0.25, sill, 0.5),
    ));
    skeleton
}

/// Returns the gridded house with one face's rhythm taken away and an opening still naming a bay on
/// it, and an opening naming a row and a column past the stops of a face that still has them.
///
/// **Both ways of missing a bay in one document**, because they are different mistakes: a face that
/// was never divided is one, and a slot from a family that has since been replaced by a narrower one
/// is the other.
fn openings_off_the_grid() -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    clear_face(&mut skeleton, FaceRef::new(MassId(0), 1))
        .expect("edge 1 carried a rhythm a moment ago");
    skeleton.openings.push(Opening::new(
        FaceRef::new(MassId(0), 1),
        0,
        1,
        Inset::new(0.25, 0.25, 0.5, 0.5),
    ));
    skeleton.openings.push(Opening::new(
        FaceRef::new(MassId(0), 0),
        5,
        9,
        Inset::new(0.25, 0.25, 0.5, 0.5),
    ));
    skeleton
}

/// Returns the gridded house with three openings that cannot be cut, one per way an inset fails: one
/// that is not a distance inward, one whose two insets leave no width, one whose two leave no
/// height.
///
/// They share a bay on purpose. A hole with no rectangle has no area to share, so these three never
/// reach the overlap comparison — which is the staging, visible.
fn openings_eating_their_bays() -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    let face = FaceRef::new(MassId(0), 0);
    for inset in [
        Inset::new(-0.1, 0.25, 0.5, 0.5),
        Inset::new(1.0, 1.0, 0.5, 0.5),
        Inset::new(0.25, 0.25, 1.0, 0.8),
    ] {
        skeleton.openings.push(Opening::new(face, 0, 0, inset));
    }
    skeleton
}

/// Returns the gridded house with three holes that each reach one edge of their wall face, on three
/// different walls so that no two of them are ever compared for overlap: the edge's own start, the
/// edge's own end, and the wall top.
///
/// **The fourth boundary — the ground line — is a separate fixture**, because on level ground no hole
/// can reach it: a grid's bottom row already starts one slab above the wall's foot, so the two
/// boundaries are genuinely different shapes and one fixture cannot carry both.
fn openings_reaching_the_wall() -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    let cases = [
        (0usize, 0usize, 0usize, Inset::new(0.0, 0.25, 0.5, 0.5)),
        (1, 0, 2, Inset::new(0.25, 0.0, 0.5, 0.5)),
        (2, 1, 1, Inset::new(0.25, 0.25, 0.5, 0.0)),
    ];
    for (edge, row, column, inset) in cases {
        skeleton.openings.push(Opening::new(
            FaceRef::new(MassId(0), edge),
            row,
            column,
            inset,
        ));
    }
    skeleton
}

/// Returns a house whose second wall climbs six metres over its own length, with one hole in it cut
/// by `inset`.
///
/// **The slope is the whole point.** The wall face is then a trapezoid standing on a straight ground
/// line, and it is the only geometry here in which a hole can reach that line without an inset that
/// is not a distance inward: on level ground a bay's sill already stands a slab above the wall's
/// foot. The mass is `6 x 6` with the wall top at `9.2`, so the grid is `u 0..6` by `y 0..4.6` and the
/// second bay of the bottom storey spans `u 2..4`.
fn sloped_house(inset: Inset) -> BuildingSkeleton {
    let outline = vec![
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([6.0, 0.0], 0.0),
        PlanNode::new([6.0, 6.0], 6.0),
        PlanNode::new([0.0, 6.0], 0.0),
    ];
    let mut skeleton = BuildingSkeleton::new(vec![Mass::new(outline, 9.2)], Vec::new());
    split_face(
        &mut skeleton,
        FaceRef::new(MassId(0), 1),
        &Rhythm::Even { count: 3 },
        &Rhythm::Even { count: 2 },
    )
    .expect("a wall that climbs six metres carries three bays across and two storeys up");
    skeleton
        .openings
        .push(Opening::new(FaceRef::new(MassId(0), 1), 0, 1, inset));
    skeleton
}

/// Returns the gridded house with two windows cut in one bay, `inset` apart along it.
///
/// **The pair is the control**: `second` inset of `0.25` from the bay's own edges makes the second
/// hole meet the first exactly at `u = 1.25` and share no area, which is what a facade divided and
/// then opened produces on purpose; `second` inset of `1.0` overlaps it by a quarter of a metre and
/// is the fault.
fn two_windows_in_one_bay(first: Inset, second: Inset) -> BuildingSkeleton {
    let mut skeleton = gridded_house();
    let face = FaceRef::new(MassId(0), 0);
    skeleton.openings.push(Opening::new(face, 0, 0, first));
    skeleton.openings.push(Opening::new(face, 0, 0, second));
    skeleton
}

/// Returns a 4 × 4 courtyard in a 10 × 10 mass with a rhythm laid on **both** of one ring edge's
/// loops, and one window authored on the rooms-facing one.
///
/// **The pair is what makes the fault a statement about the loop rather than about the courtyard.** The
/// far face carries a grid of its own and holds a bay the window's slot resolves against, so nothing
/// here is malformed in any way a bay can be — and the hole is refused anyway because the ring's walk
/// cuts from the loop the author drew. The near loop of the same ring carries the same window in
/// [`far_loop_hole_on_the_near_loop`] and is cut.
fn far_loop_hole() -> BuildingSkeleton {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(
            vec![
                PlanNode::new([-5.0, -5.0], 0.0),
                PlanNode::new([5.0, -5.0], 0.0),
                PlanNode::new([5.0, 5.0], 0.0),
                PlanNode::new([-5.0, 5.0], 0.0),
            ],
            3.2,
        )
        .with_wall(0.3)
        .with_slab(0.4)
        .with_voids(vec![vec![
            PlanNode::new([-2.0, -2.0], 0.0),
            PlanNode::new([2.0, -2.0], 0.0),
            PlanNode::new([2.0, 2.0], 0.0),
            PlanNode::new([-2.0, 2.0], 0.0),
        ]]),
    ];
    for face in skeleton.court_faces_of(MassId(0), 0) {
        split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 2 },
        );
    }
    for face in skeleton.room_faces_of(MassId(0), 0) {
        split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 2 },
        );
    }
    skeleton.openings.push(Opening::new(
        FaceRef::court_rooms(MassId(0), 0, 0),
        1,
        1,
        Inset::new(0.3, 0.3, 0.3, 0.3),
    ));
    skeleton
}

/// Returns [`far_loop_hole`]'s document with the same window on the loop the walk reads.
///
/// The control: nothing about the two rectangles differs but the face they were authored on, so the one
/// is cut and reads `0` violations and the other is refused by name.
fn far_loop_hole_on_the_near_loop() -> BuildingSkeleton {
    let mut skeleton = far_loop_hole();
    skeleton.openings = Openings::new();
    skeleton.openings.push(Opening::new(
        FaceRef::court(MassId(0), 0, 0),
        1,
        1,
        Inset::new(0.3, 0.3, 0.3, 0.3),
    ));
    skeleton
}

/// Prints the numbers the boundary reading is made from, so a control is read against the comparison
/// rather than against the word "outside": the strip the hole has to stay inside, the hole inside it,
/// and how far each of the hole's four edges stands from the boundary it may not reach.
///
/// **The ground line is re-derived here from the mass rather than asked of the validator**, so the two
/// readings are independent: `(node elevation - slab)` at each end of the edge, linear between them.
fn report_strips(skeleton: &BuildingSkeleton) {
    for (index, opening) in skeleton.openings.iter().enumerate() {
        let face = opening.face;
        let (Some(mass), Some(length)) = (skeleton.mass(face.mass), skeleton.face_length(face))
        else {
            println!("  opening {index}: the wall it names is not in the document");
            continue;
        };
        let Some((_, extents)) = skeleton.aperture(index) else {
            println!(
                "  opening {index} on edge {}: no rectangle, so there is nothing to place on the wall",
                face.edge
            );
            continue;
        };
        let count = mass.outline.len();
        let under = |station: f32| {
            let from = mass.outline[face.edge].elevation - mass.slab;
            let to = mass.outline[(face.edge + 1) % count].elevation - mass.slab;
            from + (to - from) * (station / length)
        };
        let floor = under(extents.from).max(under(extents.to));
        println!(
            "  opening {index} on edge {}: the strip runs u 0.0000..{length:.4}, its wall top is y {:.4}, and the\n    \
             ground line under this hole is at y {floor:.4}; the hole is u {:.4}..{:.4} by y {:.4}..{:.4}",
            face.edge, mass.top, extents.from, extents.to, extents.low, extents.high
        );
        println!(
            "    margins, each of which must stand above {GRID_TOLERANCE:.4} m: start {:.4}, end {:.4}, head {:.4}, sill {:.4}",
            extents.from,
            length - extents.to,
            mass.top - extents.high,
            extents.low - floor
        );
    }
}

fn main() {
    println!("# the reference volume: a wall ring and the plate under it, both wound outward");
    report_mesh("welded ring and plate", &Skeleton::default(), &cube());

    println!("\n# positive controls: the reference damaged one way each");
    report_mesh(
        "winding reversed on every face",
        &Skeleton::default(),
        &inverted_cube(),
    );
    report_mesh(
        "one face's winding reversed",
        &Skeleton::default(),
        &one_face_turned_inside_out(),
    );
    report_mesh(
        "one face deleted",
        &Skeleton::default(),
        &one_face_missing(),
    );
    report_mesh(
        "one face duplicated",
        &Skeleton::default(),
        &one_face_duplicated(),
    );
    report_mesh(
        "one corner lifted 0.25 off its planes",
        &Skeleton::default(),
        &one_corner_lifted(),
    );
    report_mesh(
        "two plates welded into one solid",
        &Skeleton::default(),
        &two_plates_one_solid(),
    );

    println!("\n# the fitter against a plane that is not a coordinate plane");
    // The patch is two triangles rather than a solid, so only its planarity is read here: the
    // closure predicates are answering about the absence of a solid, not about the fit.
    for (name, lift) in [
        ("four corners exactly on the sloped plane z = x + y", 0.0),
        ("the same, one corner lifted 0.25 along its normal", 0.25),
    ] {
        let built = sloped_quad(lift);
        let reading: Vec<String> = mesh(&Skeleton::default(), &built)
            .iter()
            .filter(|violation| matches!(violation, Violation::FaceNotPlanar { .. }))
            .map(|violation| format!("{violation:?}"))
            .collect();
        println!(
            "  {name}: the corner stands {lift:.4} m off the plane, the fit reads {}",
            if reading.is_empty() {
                "nothing over tolerance".to_string()
            } else {
                reading.join(", ")
            }
        );
    }

    println!("\n# a facet of no area: two classes, and only one of them is the author's");
    report_mesh(
        "one corner walked onto its own face's line (three distinct collinear corners)",
        &Skeleton::default(),
        &one_corner_on_the_line(),
    );
    let built = one_corner_on_the_line();
    println!(
        "  the outer wall's first facet measures {:.9} m2 and it is NOT reported: a collinear node is what\n  \
         `ops::attach_mass` inserts when it splits a host, and the facet closing the ring there is structural",
        triangle_area(&built, 0)
    );
    report_mesh(
        "two corners at one point (a collapsed ridge)",
        &Skeleton::default(),
        &two_corners_at_one_point(),
    );
    let built = two_corners_at_one_point();
    println!(
        "  the outer wall's first facet measures {:.9} m2 and it IS reported",
        triangle_area(&built, 0)
    );

    println!("\n# document controls");
    report_document("a house: one mass and a two plane roof", &house());

    let mut reversed = house();
    reversed.masses[0].outline.reverse();
    report_document("mass outline walked the other way", &reversed);

    let mut flat = house();
    flat.masses[0].top = -1.0;
    report_document("mass wall top below its outline", &flat);

    let mut stub = house();
    stub.masses[0].outline.truncate(2);
    report_document("mass outline of two nodes", &stub);

    report_document(
        "two masses overlapping with nothing attached",
        &overlapping_pair(None),
    );
    report_document(
        "the same pair, one naming the other as its host",
        &overlapping_pair(Some(MassId(0))),
    );

    a_mass_outline_that_meets_itself();
    plans_the_clipper_cannot_compare();

    let mut lost_host = house();
    lost_host.masses[0].attachment = Some(Attachment {
        host: MassId(9),
        station: 0.25,
    });
    report_document("mass naming a host that does not exist", &lost_host);

    let mut lost_roof_host = house();
    lost_roof_host.roofs[0].host = MassId(9);
    report_document("roof naming a host that does not exist", &lost_roof_host);

    let mut short_eave = house();
    short_eave.roofs[0].eaves.truncate(2);
    report_document("roof eave loop of two nodes", &short_eave);

    let mut one_row = house();
    one_row.roofs[0].planes = vec![RoofPlane::new(vec![RoofRow::new(0, EaveStation::new(0.0))])];
    report_document("roof plane of one row", &one_row);

    let mut lost_crest = house();
    for row in lost_crest.roofs[0].planes[0].rows.iter_mut() {
        row.crest = 7;
    }
    report_document("roof row naming a crest that does not exist", &lost_crest);

    println!("\n# coverage: a starter, and the same starter with a corner or a base strip missing");
    let whole = hip_starter();
    let built = Building::new(whole.clone())
        .build()
        .expect("the starter expands");
    report_mesh("hip 7x11", &whole, &built);

    let holed = hip_starter_missing_an_end_plane();
    let built = Building::new(holed.clone())
        .build()
        .expect("a roof with a corner missing still expands: it is a closed solid");
    report_mesh("hip 7x11 with one end plane deleted", &holed, &built);

    let (skeleton, built) = hip_starter_base_short();
    report_mesh(
        "hip 7x11 with one base strip triangle deleted",
        &skeleton,
        &built,
    );

    println!(
        "\n# the plate's coverage: a mass's own plate against the floor it stands on, and the two\n\
         # hosted slabs that are not plates"
    );
    report_building(
        "the hip starter, its plate as derive authored it",
        &hip_starter(),
    );
    let shrunk = hip_starter_plate_too_small(0.5);
    report_building(
        "the same starter, its plate pulled half a metre in on every side",
        &shrunk,
    );
    println!(
        "  the plate reads 5.4000 x 9.4000 = 50.7600 against a floor of 6.4000 x 10.4000 = 66.5600 — the mass's\n  \
         inner loop, which is what its wall ring leaves it — and that loop's own perimeter of 33.6000 m allows\n  \
         only {:.4} m2 of it to be missing, so 15.8000 m2 is a fault by three decades of area rather than by a\n  \
         rounding error",
        0.001 * 33.6
    );
    report_building(
        "the same starter with an intermediate floor on the inner loop",
        &hip_starter_with_a_floor(),
    );
    report_building(
        "the same starter with that floor walked down to 0.5 mm above the ground line",
        &hip_starter_with_a_floor_at_the_ground_line(),
    );
    println!(
        "  the two differ in nothing but the name the document gives the slab and the half-millimetre it \
         stands above the ground: same loop pulled in half a metre on all four sides, same 15.8000 m2 \
         short of the same floor. A rule that reads what a slab IS answers nothing here; a rule that \
         reads where it sits has to draw the line somewhere, and it drew it a millimetre above the \
         ground rather than on it"
    );
    report_building(
        "the same starter with no plate at all, which is not a fault",
        &hip_starter_with_no_plate(),
    );

    println!(
        "\n# the other half of the slab's host question: the mass itself is not there, and all three\n\
         # roles say the same thing about it"
    );
    report_building(
        "a plate, a floor and a flat roof, every one of them hosted on the mass the document holds",
        &three_hosted_roles(MassId(0)),
    );
    report_building(
        "the same three slabs, every one of them hosted on a mass the document does not hold",
        &three_hosted_roles(MassId(9)),
    );
    println!(
        "  the second document is what the first staging swallowed: `check_plate_coverage` returns on a\n  \
         host it cannot find, so a plate, a floor and a flat roof all named a mass that was gone and none of\n  \
         them said so. The three answers differ in nothing but the role the document gave the slab — the\n  \
         plate is seated on the host's inner loop, the floor on the same loop at a storey, and the flat roof\n  \
         over the eave, which is outside the outline — and the fault is the same in all three, because it is\n  \
         in the reference and not in the seating: a missing mass takes nothing away from a slab's own plan\n  \
         or its two elevations, and there is nothing of its left to measure"
    );
    println!(
        "  an unhoused slab is not this fault and is not reported by it: a step, a plinth and a canopy are\n  \
         plates that stand on nothing, which is what `Slab::host` documents, and the starter with no plate\n  \
         at all — read above — is the same answer from the other side"
    );

    println!("\n# openings: a bay, a hole, and the four ways to be wrong");
    report_document(
        "the gridded house: three bays across, two storeys up",
        &gridded_house(),
    );
    let windowed = windowed_house();
    report_document("a window and a door, inset on every side", &windowed);
    report_strips(&windowed);

    println!(
        "\n# a door is a hole in the bottom row: neither sill is a fault, and both are measured"
    );
    for (name, sill) in [
        (
            "a door, its sill at the elevation the mass is authored at",
            0.0,
        ),
        (
            "the same door, its sill raised by the mass's slab thickness",
            0.4,
        ),
    ] {
        let door = bottom_row_hole(sill);
        report_document(name, &door);
        report_strips(&door);
    }

    let off = openings_off_the_grid();
    report_document(
        "one opening on a wall with no rhythm, one past the stops",
        &off,
    );
    report_strips(&off);

    let eaten = openings_eating_their_bays();
    report_document("three openings that cannot be cut in their bay", &eaten);
    report_strips(&eaten);

    let reaching = openings_reaching_the_wall();
    report_document(
        "three holes reaching a corner, the far corner and the wall top",
        &reaching,
    );
    report_strips(&reaching);

    let into_ground = sloped_house(Inset::new(0.25, 0.25, 0.5, 0.5));
    report_document(
        "a hole run into the ground line of a sloping wall",
        &into_ground,
    );
    report_strips(&into_ground);

    let jamb = sloped_house(Inset::new(0.25, 0.25, 2.65, 0.5));
    report_document(
        "the same wall, a hole whose far jamb alone clears the line",
        &jamb,
    );
    report_strips(&jamb);
    println!(
        "  the ground line under that hole reads 2.6000 at its middle and 3.3500 under its far jamb,\n  \
         so the sill at 2.6500 clears a midpoint reading by {:.4} m and reaches the wall's foot at the jamb",
        2.65 - 2.6
    );

    let overlapping = two_windows_in_one_bay(
        Inset::new(0.25, 0.75, 0.5, 0.5),
        Inset::new(1.0, 0.25, 0.5, 0.5),
    );
    report_document(
        "two windows in one bay sharing a quarter of a metre",
        &overlapping,
    );
    report_strips(&overlapping);

    let touching = two_windows_in_one_bay(
        Inset::new(0.25, 0.75, 0.5, 0.5),
        Inset::new(1.25, 0.25, 0.5, 0.5),
    );
    report_document(
        "two windows in one bay meeting exactly along a mullion",
        &touching,
    );
    report_strips(&touching);
    println!(
        "  the two holes meet at u = 1.2500 and remove no wall twice, so no OpeningOverlaps is named"
    );

    // **The one document the far loop's name makes writable that this crate will not honour**, with the
    // identical window on the near loop beside it: the two rectangles differ in nothing but the face
    // they were authored on, and only one of them is cut.
    let on_the_far_loop = far_loop_hole();
    report_document(
        "a window on a courtyard wall's rooms-facing loop",
        &on_the_far_loop,
    );
    let on_the_near_loop = far_loop_hole_on_the_near_loop();
    report_document(
        "the same window on the loop the walk reads",
        &on_the_near_loop,
    );
    println!(
        "  the rooms-facing edge is {:.4} m and the void loop's own is {:.4} m, so the two bays the\n  \
         windows stand in are not the same span of the same wall",
        on_the_far_loop
            .face_length(FaceRef::court_rooms(MassId(0), 0, 0))
            .unwrap_or(f32::NAN),
        on_the_far_loop
            .face_length(FaceRef::court(MassId(0), 0, 0))
            .unwrap_or(f32::NAN),
    );

    let mut no_rhythm = house();
    no_rhythm.openings.push(Opening::new(
        FaceRef::new(MassId(0), 0),
        0,
        0,
        Inset::new(0.25, 0.25, 0.5, 0.5),
    ));
    report_document(
        "a house with one opening and no rhythm anywhere",
        &no_rhythm,
    );
    println!(
        "  the four grid checks read the rhythms the document holds, and there is none to read"
    );

    println!("\n# a skeleton that cannot expand at all");
    let mut wide = house();
    wide.masses[0].wall = 3.5;
    report_building(
        "a 6x6 mass whose wall is wider than half its outline",
        &wide,
    );
    report_building("the same house with a 0.3 wall", &house());

    println!("\n# the order the answer comes back in");
    let broken = one_face_missing();
    let first = mesh(&Skeleton::default(), &broken);
    let second = mesh(&Skeleton::default(), &broken);
    println!("two runs over one mesh agree: {}", first == second);
    println!("the list is {:?}", first);
    let whole = Building::new(hip_starter());
    println!(
        "two runs over one building agree: {}",
        whole.validate() == whole.validate()
    );
}

/// Prints what `Building::validate` answers, which is the composition an agent calls.
fn report_building(name: &str, skeleton: &BuildingSkeleton) {
    println!(
        "{name}: {} masses, {} roofs",
        skeleton.masses.len(),
        skeleton.roofs.len()
    );
    report(&Building::new(skeleton.clone()).validate());
}

/// Returns the cube with every face's winding reversed: the whole solid inside out.
fn inverted_cube() -> BuildingMesh {
    let mut built = cube();
    for face in built.faces.iter_mut() {
        face.indices.swap(1, 2);
    }
    built
}

/// Returns two masses whose plans share area, with an attachment from the second to the first when
/// one is named.
fn overlapping_pair(attachment: Option<MassId>) -> BuildingSkeleton {
    let first = Mass::new(square((0.0, 0.0), (6.0, 6.0), 0.0), 3.0);
    let second = Mass::new(square((3.0, 3.0), (9.0, 9.0), 0.0), 3.0);
    let second = match attachment {
        Some(host) => second.with_attachment(Attachment {
            host,
            station: 0.25,
        }),
        None => second,
    };
    BuildingSkeleton::new(vec![first, second], Vec::new())
}

/// Asks whether a **mass's own outline** that meets itself has a name, which is the last loop in
/// this module that nothing asked about.
///
/// # Invariants
///
/// - **Two shapes must be named and two must not**, or the check is not a gate but a width: a needle
///   and a crossing are the two ways [`SelfMeeting`] reads, and an authored pinch and a collinear node
///   are the two ways an outline this crate writes on purpose looks. The last of those is
///   [`ops::split_outline`](whiting_building::ops::split_outline)'s own insertion, so a gate that caught
///   it would refuse a document the crate builds itself.
/// - **Both entry points are printed for every row**, because the fault this section is about is one
///   that arrived only through [`Building::validate`]'s `DidNotExpand`: `report_document` is
///   [`validate::document`] alone, and a name that exists in one and not the other is not the shape the
///   author has to fix.
/// - **The clip reading is printed too**, because "the check said nothing" cannot tell a plan that was
///   measured and found disjoint from a plan that could not be measured at all — which is the whole of
///   what the overlap subtraction used to be asked about.
fn a_mass_outline_that_meets_itself() {
    println!(
        "\n# a mass outline that meets itself: the two shapes that meet, and the two that must not"
    );
    for (name, plan) in [
        ("a needle cut into the north edge", needled_square()),
        ("a spike crossing its own south edge", crossed_square()),
        ("two 3 x 3 squares sharing one corner", pinched_pair()),
        (
            "a square with one node written on its east edge",
            collinear_square(),
        ),
        (
            "a plain 10 x 10 square",
            square((0.0, 0.0), (10.0, 10.0), 0.0),
        ),
    ] {
        let mut skeleton = BuildingSkeleton::default();
        skeleton.masses = vec![
            Mass::new(plan.clone(), 3.2).with_wall(0.3).with_slab(0.4),
            Mass::new(square((5.0, 0.0), (15.0, 10.0), 0.0), 3.2)
                .with_wall(0.3)
                .with_slab(0.4),
        ];
        let plan_of =
            |mass: &Mass| -> Vec<Vec2> { mass.outline.iter().map(|n| n.position).collect() };
        let shared = whiting_building::clip::shared_area(
            &plan_of(&skeleton.masses[0]),
            &plan_of(&skeleton.masses[1]),
        );
        println!(
            "\n{name}: the two plans share {}, of a true 50.0000",
            match shared {
                Some(area) => format!("{area:.4} m2"),
                None => "nothing, because `clip` cannot measure the first plan".to_owned(),
            }
        );
        report_document(&format!("{name}, through validate::document"), &skeleton);
        report_building(&format!("{name}, through Building::validate"), &skeleton);
    }
}

/// Asks what the overlap subtraction does with a plan [`whiting_building::clip`] cannot measure,
/// which is the case no gate in front of it covers.
///
/// # Invariants
///
/// - **The first row is a control and the other two are what keep it honest.** An authored pinch is
///   the one meeting [`Violation::OutlineNotSimple`] is deliberately silent about, so a pinched
///   plan reaches [`Violation::UnattachedOverlap`] with nothing in front of it; the second row is
///   the same plan against a neighbour it does not touch, where the two plans really are apart and
///   must stay silent; and the third is a plan this crate's own [`ops::split_outline`]
///   (whiting_building::ops::split_outline) writes against the control's own neighbour, where the
///   subtraction works and must go on reporting.
/// - **The true area is printed beside every row**, because "`clip` cannot measure the first plan"
///   cannot tell a pair that really does share area from a pair that really does not — and the
///   first two rows are one plan and two neighbours, so that difference is the whole content.
/// - **`OutlineNotSimple` is what shows the gate did not skip the pair.** Its absence from the first
///   row is the evidence that the pair reached the subtraction at all.
fn plans_the_clipper_cannot_compare() {
    println!(
        "\n# the overlap subtraction itself: a plan `clip` cannot measure, and the two pairs beside it"
    );
    for (name, plan, neighbour, truth) in [
        (
            "the control: two 3 x 3 squares sharing a corner, standing inside an 8 x 8 neighbour",
            pinched_pair(),
            square((-4.0, -4.0), (4.0, 4.0), 0.0),
            "18.0000",
        ),
        (
            "the same plan against a neighbour whose plan it does not touch",
            pinched_pair(),
            square((5.0, 0.0), (15.0, 10.0), 0.0),
            "0.0000",
        ),
        (
            "a square with one node written on its own east edge, against the control's neighbour",
            collinear_square(),
            square((-4.0, -4.0), (4.0, 4.0), 0.0),
            "16.0000",
        ),
    ] {
        let mut skeleton = BuildingSkeleton::default();
        skeleton.masses = vec![
            Mass::new(plan, 3.2).with_wall(0.3).with_slab(0.4),
            Mass::new(neighbour, 3.2).with_wall(0.3).with_slab(0.4),
        ];
        let plan_of =
            |mass: &Mass| -> Vec<Vec2> { mass.outline.iter().map(|n| n.position).collect() };
        let shared = whiting_building::clip::shared_area(
            &plan_of(&skeleton.masses[0]),
            &plan_of(&skeleton.masses[1]),
        );
        println!(
            "\n{name}: the two plans really share {truth} m2, and `clip` reads {}",
            match shared {
                Some(area) => format!("{area:.4} m2"),
                None => "nothing, because it cannot measure the first plan".to_owned(),
            }
        );
        report_document(&format!("{name}, through validate::document"), &skeleton);
    }
    println!(
        "  the first row carries no OutlineNotSimple, so the gate in front of the subtraction did not \
         skip this pair and the pair's reading is whatever the subtraction returned. The second row's \
         two plans were compared and are apart: their bounding boxes miss, which is a measurement \
         rather than a refusal, so the row must stay silent. The third row is the same neighbour \
         against a plan `clip` can measure"
    );
}

/// Returns a `10 × 10` square with a needle cut into its north edge and straight back out, the same
/// shape [`courtyard_probe`](whiting_building) cuts into a void loop.
///
/// Two of its nodes stand at one place and a third stands inside the edge they run over, so the loop
/// is not simple in a way no reader has to be told about: it encloses `50.0000 m²` a person can check.
fn needled_square() -> Vec<PlanNode> {
    vec![
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([10.0, 0.0], 0.0),
        PlanNode::new([10.0, 10.0], 0.0),
        PlanNode::new([6.0, 10.0], 0.0),
        PlanNode::new([8.0, 10.0], 0.0),
        PlanNode::new([0.0, 10.0], 0.0),
    ]
}

/// Returns a square with a spike running back across its own south edge: the second reading
/// [`SelfMeeting`] carries, and the one a pinch cannot stand in for.
fn crossed_square() -> Vec<PlanNode> {
    vec![
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([6.0, 0.0], 0.0),
        PlanNode::new([6.0, 6.0], 0.0),
        PlanNode::new([3.0, 6.0], 0.0),
        PlanNode::new([3.0, -3.0], 0.0),
        PlanNode::new([0.0, 6.0], 0.0),
    ]
}

/// Returns two `3 × 3` squares sharing exactly one corner: the **authored** pinch, which builds a
/// sound solid and which every simplicity reading in this crate declines on purpose.
fn pinched_pair() -> Vec<PlanNode> {
    vec![
        PlanNode::new([-3.0, -3.0], 0.0),
        PlanNode::new([0.0, -3.0], 0.0),
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([3.0, 0.0], 0.0),
        PlanNode::new([3.0, 3.0], 0.0),
        PlanNode::new([0.0, 3.0], 0.0),
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([-3.0, 0.0], 0.0),
    ]
}

/// Returns a square with one node standing on its own edge, which is what
/// [`ops::split_outline`](whiting_building::ops::split_outline) inserts at every junction.
///
/// A node on an edge is not a meeting: the two segments that meet there are the node's **own**, and
/// naming it would refuse every document this crate splits.
fn collinear_square() -> Vec<PlanNode> {
    vec![
        PlanNode::new([0.0, 0.0], 0.0),
        PlanNode::new([10.0, 0.0], 0.0),
        PlanNode::new([10.0, 5.0], 0.0),
        PlanNode::new([10.0, 10.0], 0.0),
        PlanNode::new([0.0, 10.0], 0.0),
    ]
}
