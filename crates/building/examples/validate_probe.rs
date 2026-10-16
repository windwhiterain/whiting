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
use whiting_building::mass::{Attachment, Mass, MassId, PlanNode};
use whiting_building::mesh::{
    BuildingFace, BuildingMesh, Element, MassSurface, RoofSurface, SolidId,
};
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
        Violation::FaceNotPlanar { .. } => "FaceNotPlanar",
        Violation::FaceDegenerate { .. } => "FaceDegenerate",
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
