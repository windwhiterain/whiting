//! Reports what a built building is and what an author has got wrong, measured rather than
//! asserted.
//!
//! This is the reporting vehicle for the `building` crate's `validate` module, and the pattern is
//! `mountain`'s census probe: it prints facts, never asserts intent. Two things make it worth
//! running rather than a list of pass messages:
//!
//! - The **base case** is a hand-built welded cube, because a check that is only ever shown on a
//!   derived building shows itself working only where `derive` happens to be right. The cube is
//!   what a correct expansion of a mass looks like, so zero violations on it is the reference every
//!   other number is read against.
//! - Every check has a **positive control** beside it: the same cube damaged one way, or the same
//!   starter broken one way, so a check that cannot be made to fire is visible as such rather than
//!   as silence.
//!
//! ```text
//! cargo run -p whiting-building --example validate_probe
//! ```

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, GRID_TOLERANCE, Rhythm};
use whiting_building::mass::{Attachment, Mass, MassId, PlanNode};
use whiting_building::mesh::{
    BuildingFace, BuildingMesh, Element, MassSurface, RoofSurface, SolidId,
};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops::{clear_face, split_face};
use whiting_building::roof::{CrestPoint, EaveStation, Roof, RoofPlane, RoofRow};
use whiting_building::skeleton::{Building, BuildingSkeleton};
use whiting_building::validate::{Violation, document, mesh};
use whiting_building::{BuildingSkeleton as Skeleton, RoofId};

/// The cube's points, welded: eight corners, each written down once.
const CUBE_POINTS: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [2.0, 0.0, 0.0],
    [2.0, 0.0, 2.0],
    [0.0, 0.0, 2.0],
    [0.0, 2.0, 0.0],
    [2.0, 2.0, 0.0],
    [2.0, 2.0, 2.0],
    [0.0, 2.0, 2.0],
];

/// The cube's triangles, one row per side, each wound so the right handed normal points out.
const CUBE_FACES: [([u32; 3], usize); 12] = [
    // the floor, normal down
    ([0, 1, 2], 5),
    ([0, 2, 3], 5),
    // the wall at z = 0, normal -z
    ([0, 4, 1], 0),
    ([1, 4, 5], 0),
    // the wall at x = 2, normal +x
    ([1, 5, 2], 1),
    ([2, 5, 6], 1),
    // the wall at z = 2, normal +z
    ([2, 6, 3], 2),
    ([3, 6, 7], 2),
    // the wall at x = 0, normal -x
    ([3, 7, 0], 3),
    ([0, 7, 4], 3),
    // the wall top, normal up
    ([4, 6, 5], 4),
    ([4, 7, 6], 4),
];

/// The element each side of the cube came from: one authored surface per side.
///
/// The underside is [`MassSurface::SlabSoffit`] — the mass's floor is a disc across the whole
/// outline, one slab below, and it is a different element from [`MassSurface::FloorTop`] because
/// the two are different planes and one element is one shading group.
fn cube_element(side: usize) -> Element {
    let surface = match side {
        0 => MassSurface::OuterWall { edge: 0 },
        1 => MassSurface::OuterWall { edge: 1 },
        2 => MassSurface::OuterWall { edge: 2 },
        3 => MassSurface::OuterWall { edge: 3 },
        4 => MassSurface::WallTop { edge: 0 },
        _ => MassSurface::SlabSoffit,
    };
    Element::Mass(MassId(0), surface)
}

/// Builds the reference cube: twelve triangles, outward, one element per side.
fn cube() -> BuildingMesh {
    let mut built = BuildingMesh::new();
    built.points = CUBE_POINTS.to_vec();
    built.faces = CUBE_FACES
        .iter()
        .map(|(indices, side)| BuildingFace {
            indices: *indices,
            element: cube_element(*side),
        })
        .collect();
    built
}

/// Returns the cube with one face's winding reversed: an inconsistent surface, closed and
/// edge-manifold, which is the only way to open a surface without breaking manifoldness.
fn one_face_turned_inside_out() -> BuildingMesh {
    let mut built = cube();
    built.faces[0].indices.swap(1, 2);
    built
}

/// Returns the cube with one face taken away: a hole with no area, so its plan area cannot see it.
fn one_face_missing() -> BuildingMesh {
    let mut built = cube();
    built.faces.remove(6);
    built
}

/// Returns the cube with one face written twice: an edge three faces use, which no orientation of
/// a closed surface can fix.
fn one_face_duplicated() -> BuildingMesh {
    let mut built = cube();
    built.faces.insert(6, built.faces[0]);
    built
}

/// Returns the cube with one corner lifted a quarter of a metre off the planes it belongs to.
fn one_corner_lifted() -> BuildingMesh {
    let mut built = cube();
    built.points[6][1] += 0.25;
    built
}

/// Returns two cubes welded into one solid: closed, manifold, outward, and two spheres.
fn two_cubes_one_solid() -> BuildingMesh {
    let first = cube();
    let offset = first.points.len() as u32;
    let mut built = cube();
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

/// Returns the starter hip with one soffit triangle deleted: a slab that does not reach the whole
/// outline, which is the mass-side reading of the same fault.
fn hip_starter_slab_short() -> (BuildingSkeleton, BuildingMesh) {
    let skeleton = hip_starter();
    let mut built = Building::new(skeleton.clone())
        .build()
        .expect("the starter expands");
    let soffit: Vec<usize> = built
        .faces_of(SolidId::Mass(MassId(0)))
        .into_iter()
        .filter(|index| {
            matches!(
                built.faces[*index].element,
                Element::Mass(_, MassSurface::SlabSoffit)
            )
        })
        .collect();
    built.faces.remove(soffit[0]);
    (skeleton, built)
}

/// Returns the cube with one corner walked onto the line through its own face's other two corners:
/// the floor's first facet has no area, and nothing else has changed.
///
/// This is the fault a planarity check cannot see either — the other three corners of the wall that
/// corner belonged to are still in their planes — and it is the shape a node written one step along
/// the edge behind it takes. Closure is untouched by it on purpose: the edges are welded by index,
/// so a corner standing on a line is still a corner every neighbour shares.
fn one_corner_on_the_line() -> BuildingMesh {
    let mut built = cube();
    built.points[2] = [4.0, 0.0, 0.0];
    built
}

/// Returns the cube with one corner walked onto **the position of another corner of its own faces**:
/// two corners at one point, so the facets between them have no area.
///
/// **This is the class `FaceDegenerate` reports**, and it is the other class from
/// [`one_corner_on_the_line`]: the phase's own square hip roof stood two crest points at one spot —
/// points are welded by authored identity, so they are two indices — and the triangle between them
/// measured exactly zero while closure, Euler, manifoldness and orientation were all green. A
/// collinear triple is not reported, because a split host needs it to close; a coincidence is always
/// a fault, because no authored element has two corners in one place on purpose.
fn two_corners_at_one_point() -> BuildingMesh {
    let mut built = cube();
    built.points[2] = built.points[1];
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

/// Prints the solid's four measurements and every violation the mesh check found.
fn report_mesh(name: &str, built: &BuildingMesh) {
    report_solid(name, &Skeleton::default(), built, SolidId::Mass(MassId(0)));
}

/// Prints one solid's four measurements, the two areas the coverage check compares, and every
/// violation the mesh check found.
fn report_solid(name: &str, skeleton: &BuildingSkeleton, built: &BuildingMesh, solid: SolidId) {
    let faces = built.faces_of(solid);
    let violations = mesh(skeleton, built);
    println!(
        "{name}: {} faces, {} points, solid volume {:.4}, euler {}, boundary loops {}, manifold {}",
        built.faces.len(),
        built.points.len(),
        built.solid_signed_volume(solid),
        built.solid_euler_characteristic(solid),
        built.solid_boundary_loops(solid),
        built.solid_is_manifold(solid),
    );
    println!("  solid holds {} faces", faces.len());
    report_areas(skeleton, built, solid);
    report(&violations);
}

/// Prints the coverage reading: the plan area of the surface the check measures and the plan area
/// of the loop it has to reach, which are the two numbers the violation carries.
fn report_areas(skeleton: &BuildingSkeleton, built: &BuildingMesh, solid: SolidId) {
    let faces = built.faces_of(solid);
    let measured: Vec<usize> = faces
        .iter()
        .copied()
        .filter(|index| {
            matches!(
                built.faces[*index].element,
                Element::Mass(_, MassSurface::SlabSoffit)
                    | Element::Roof(_, RoofSurface::Skin { .. })
            )
        })
        .collect();
    let covered = built.plan_area_of(&measured);
    let authored = match solid {
        SolidId::Mass(id) => skeleton.mass(id).map(|mass| mass.plan_area()),
        SolidId::Roof(id) => skeleton.roof(id).map(|roof| roof.plan_area()),
    };
    println!(
        "  coverage: measured {:.4} against authored loop {}, over {} faces",
        covered.abs(),
        authored
            .map(|area| format!("{area:.4}"))
            .unwrap_or_default(),
        measured.len(),
    );
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
        Violation::MassDoesNotCover { .. } => "MassDoesNotCover",
        Violation::RoofDoesNotCover { .. } => "RoofDoesNotCover",
        Violation::RoofSkinNotADisc { .. } => "RoofSkinNotADisc",
        Violation::RoofSkinLeavesTheEave { .. } => "RoofSkinLeavesTheEave",
        Violation::GridFaceMissing { .. } => "GridFaceMissing",
        Violation::GridTooShort { .. } => "GridTooShort",
        Violation::GridNotOrdered { .. } => "GridNotOrdered",
        Violation::GridOutsideFace { .. } => "GridOutsideFace",
        Violation::OpeningBayMissing { .. } => "OpeningBayMissing",
        Violation::OpeningExceedsBay { .. } => "OpeningExceedsBay",
        Violation::OpeningOutsideWall { .. } => "OpeningOutsideWall",
        Violation::OpeningOverlaps { .. } => "OpeningOverlaps",
        Violation::FaceNotPlanar { .. } => "FaceNotPlanar",
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
    println!("# the reference cube: one element per side, wound outward");
    report_mesh("welded cube", &cube());

    println!("\n# positive controls: the cube damaged one way each");
    report_mesh("winding reversed on every face", &inverted_cube());
    report_mesh("one face's winding reversed", &one_face_turned_inside_out());
    report_mesh("one face deleted", &one_face_missing());
    report_mesh("one face duplicated", &one_face_duplicated());
    report_mesh(
        "one corner lifted 0.25 off its planes",
        &one_corner_lifted(),
    );
    report_mesh("two cubes welded into one solid", &two_cubes_one_solid());

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
        &one_corner_on_the_line(),
    );
    let built = one_corner_on_the_line();
    println!(
        "  the floor's first facet measures {:.9} m2 and it is NOT reported: a collinear node is what\n  \
         `ops::attach_mass` inserts when it splits a host, and the facet closing the ring there is structural",
        triangle_area(&built, 0)
    );
    report_mesh(
        "two corners at one point (a collapsed ridge)",
        &two_corners_at_one_point(),
    );
    let built = two_corners_at_one_point();
    println!(
        "  the floor's first facet measures {:.9} m2 and it IS reported",
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

    println!("\n# coverage: a starter, and the same starter with a corner or a slab missing");
    let whole = hip_starter();
    let built = Building::new(whole.clone())
        .build()
        .expect("the starter expands");
    report_solid("hip 7x11", &whole, &built, SolidId::Roof(RoofId(0)));
    report_solid("hip 7x11", &whole, &built, SolidId::Mass(MassId(0)));

    let holed = hip_starter_missing_an_end_plane();
    let built = Building::new(holed.clone())
        .build()
        .expect("a roof with a corner missing still expands: it is a closed solid");
    report_solid(
        "hip 7x11 with one end plane deleted",
        &holed,
        &built,
        SolidId::Roof(RoofId(0)),
    );

    let (skeleton, built) = hip_starter_slab_short();
    report_solid(
        "hip 7x11 with one soffit triangle deleted",
        &skeleton,
        &built,
        SolidId::Mass(MassId(0)),
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
