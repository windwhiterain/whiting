//! The openings slice's acceptance: what a hole measures, what it removes, and what is refused.
//!
//! Measured off the document and off the expanded mesh, because a hole has one fact in each: the
//! rectangle it cuts (a bay's own extents moved inward, in arc length and elevation) and the wall it
//! takes away. The strongest number here is the third column — a wall with a hole holds less material
//! by exactly `hole area × wall thickness`, and that is a check no predicate about closure can make.
//!
//! Run with `cargo run -p whiting-building --example opening_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening, RevealSide};
use whiting_building::ops;
use whiting_building::{Building, Element, MassId, MassSurface, SolidId};

fn main() {
    let plan = Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    };
    let base = starter(&plan);
    let uncut = base.skeleton.clone();
    let mut skeleton = uncut.clone();
    let mass = MassId(0);
    let wall = skeleton.masses[0].wall;
    let slab = skeleton.masses[0].slab;
    let top = skeleton.masses[0].highest_top();

    println!(
        "# the house: {:.1} x {:.1}, wall top {top:.4}, wall {wall:.4} thick, slab {slab:.4}",
        plan.width, plan.depth
    );

    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }
    let face = FaceRef::new(mass, 1);
    let (low, high) = skeleton.face_extent(face).expect("the face is there");
    println!(
        "# face {} of mass {mass:?}: {:.4} long, floor at {low:.4} (the wall runs from {:.4}), top {high:.4}",
        face.edge,
        skeleton.face_length(face).expect("the face is there"),
        low - slab
    );

    println!("\n# a window, a door, and a window above the door");
    // **The jambs' insets must clear the miter's own offset, not merely the edge's end.** A hole's
    // rim is projected onto the *inner* edge, whose ends the miter moved inward by one wall's
    // thickness at a right-angle corner, and an inner wall's corner edge runs the full height without
    // being split at a hole's elevations — so a jamb landing on that corner would leave the reveal's
    // inner edge a sub-segment nothing welds to, and the mass open. The expander refuses it, which is
    // why the door below is inset 0.5 m on an edge whose miter moved the inner rim 0.3 m, and why the
    // boundary case is measured as a refusal further down rather than written as a window.
    let openings = [
        (
            "a window in the middle bay",
            Opening::new(face, 1, 1, Inset::new(0.6, 0.6, 0.9, 0.2)),
        ),
        (
            "a door in the first bay, its sill at ground level",
            Opening::new(face, 0, 0, Inset::new(0.5, 0.3, 0.0, 0.2)),
        ),
        (
            "a window above it, in the second storey",
            Opening::new(face, 1, 0, Inset::new(0.5, 0.3, 0.2, 0.5)),
        ),
    ];
    let mut cut = Vec::new();
    for (name, opening) in &openings {
        let index = ops::add_opening(&mut skeleton, *opening).expect("the slot is in the grid");
        cut.push(index);
        let bay = skeleton
            .bay(opening.face, opening.row, opening.column)
            .expect("the slot is in the grid");
        let (_, extents) = skeleton.aperture(index).expect("the slot is in the grid");
        println!(
            "  {name}: index {index}, bay [{:.4}, {:.4}] x [{:.4}, {:.4}], hole [{:.4}, {:.4}] x \
             [{:.4}, {:.4}], {:.4} m² of wall",
            bay.from,
            bay.to,
            bay.low,
            bay.high,
            extents.from,
            extents.to,
            extents.low,
            extents.high,
            extents.area()
        );
    }

    let removed: f32 = cut
        .iter()
        .map(|index| skeleton.aperture(*index).expect("just built").1.area())
        .sum::<f32>()
        * wall;
    println!("\n# the material three holes take out of this one wall: {removed:.4} m³");

    println!("\n# refusals at the verb");
    let refused = [
        (
            "a slot the grid does not hold",
            Opening::new(face, 9, 0, Inset::new(0.3, 0.3, 0.3, 0.3)),
        ),
        (
            "an inset that eats the bay",
            Opening::new(face, 0, 2, Inset::new(0.0, 4.0, 0.1, 0.1)),
        ),
        (
            "a negative inset",
            Opening::new(face, 0, 2, Inset::new(-0.2, 0.3, 0.1, 0.1)),
        ),
    ];
    for (name, opening) in refused {
        println!("  {name}: {:?}", ops::add_opening(&mut skeleton, opening));
    }

    println!("\n# the mesh: the same house with the holes and without them");
    let uncut_mesh = Building::new(uncut.clone())
        .build()
        .expect("the shell expands");
    let cut_mesh = Building::new(skeleton.clone())
        .build()
        .expect("the shell expands");
    for (name, mesh) in [
        ("without openings", &uncut_mesh),
        ("with openings", &cut_mesh),
    ] {
        let solid = SolidId::Mass(mass);
        println!(
            "  {name}: mass {} faces, loops {}, euler {}, manifold {}, volume {:.4}",
            mesh.faces_of(solid).len(),
            mesh.solid_boundary_loops(solid),
            mesh.solid_euler_characteristic(solid),
            mesh.solid_is_manifold(solid),
            mesh.solid_signed_volume(solid)
        );
    }
    let before = uncut_mesh.solid_signed_volume(SolidId::Mass(mass));
    let after = cut_mesh.solid_signed_volume(SolidId::Mass(mass));
    println!(
        "  removed {:.4} m³ against the {:.4} the holes' own areas say (off by {:.6})",
        before - after,
        removed,
        (before - after - removed).abs()
    );

    let reveals = cut_mesh
        .faces
        .iter()
        .filter(|face| matches!(face.element, Element::Mass(_, MassSurface::Reveal { .. })))
        .count();
    let sides: Vec<RevealSide> = RevealSide::ALL.to_vec();
    println!(
        "  reveal faces {reveals} for {} holes ({} sides each), and {sides:?} is every side",
        cut.len(),
        sides.len()
    );

    println!("\n# the domain's own edge: a jamb exactly on the miter's offset");
    let mut boundary = uncut.clone();
    for at in boundary.faces_of(mass) {
        ops::split_face(&mut boundary, at, &columns, &rows);
    }
    let on_the_corner = Opening::new(face, 0, 0, Inset::new(wall, 0.3, 0.0, 0.2));
    ops::add_opening(&mut boundary, on_the_corner).expect("the slot is in the grid");
    match Building::new(boundary).build() {
        Ok(_) => println!(
            "  a jamb at {wall:.4} m, the offset the miter moved the inner rim by: BUILT, which means \
             the reveal's inner edge is a sub-segment of the inner corner and the mass is open"
        ),
        Err(error) => println!(
            "  a jamb at {wall:.4} m, the offset the miter moved the inner rim by: refused with {:?}",
            error.problems
        ),
    }

    println!("\n# the document the mesh was built from");
    let violations = Building::new(skeleton).validate();
    println!("  violations: {}", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
}
