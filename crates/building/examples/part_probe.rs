//! The parts slice's acceptance: what a template occupies, where a placement puts it, and what is
//! refused.
//!
//! The interesting measurement is not that a part exists but that it lands **in the hole it was made
//! for**: a door leaf is authored in its own frame, placed by a bay slot and a pin, and the numbers
//! here put its box against the opening's own extents — in one frame, because a part that fits its bay
//! and misses its hole is a door in the wall beside the doorway.
//!
//! Run with `cargo run -p whiting-building --example part_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::parts::{Pin, Placement, Template};
use whiting_building::{Building, Mass, MassId, PlanNode};

/// Returns a door leaf: a hollow panel `width` across, `height` tall, `thickness` through.
///
/// # Invariants
///
/// It is authored the way the frame a placement maps into says to author one: `x` across the leaf,
/// `y` up it, `z` through it. Its own frame's origin is its bottom outer corner, so a pin puts that
/// corner where the author wants the leaf to stand.
fn door_leaf(width: f32, height: f32, thickness: f32) -> Template {
    let wall = 0.010;
    let slab = 0.020;
    // The outline stands at the elevation of its own slab, so the template's local origin is its
    // **floor** and its bounds start at `0`: that is what makes `Pin::new(along, 0.0)` stand the leaf
    // on the bay's own sill rather than a slab below it.
    let outline = vec![
        PlanNode::new([0.0, 0.0], slab),
        PlanNode::new([width, 0.0], slab),
        PlanNode::new([width, thickness], slab),
        PlanNode::new([0.0, thickness], slab),
    ];
    Template::new(
        "door leaf",
        vec![Mass::new(outline, height).with_wall(wall).with_slab(slab)],
        Vec::new(),
    )
}

fn main() {
    let plan = Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }
    let face = FaceRef::new(mass, 1);

    // The doorway this leaf is made for: bay 0 of the ground storey, cut from ground level.
    let doorway = Opening::new(face, 0, 0, Inset::new(0.5, 0.3, 0.0, 0.2));
    let opening = ops::add_opening(&mut skeleton, doorway).expect("the slot is in the grid");
    let (_, hole) = skeleton.aperture(opening).expect("just authored");
    println!(
        "# the doorway: [{:.4}, {:.4}] along the edge, [{:.4}, {:.4}] up it",
        hole.from, hole.to, hole.low, hole.high
    );

    let leaf = door_leaf(0.9, 1.35, 0.06);
    let bounds = leaf.bounds().expect("the leaf holds a mass");
    println!(
        "# the leaf in its own frame: x [{:.4}, {:.4}], y [{:.4}, {:.4}], z [{:.4}, {:.4}]",
        bounds.x.0, bounds.x.1, bounds.y.0, bounds.y.1, bounds.z.0, bounds.z.1
    );
    let template = ops::add_template(&mut skeleton, leaf);

    // The pin stands the leaf's own bottom outer corner on the bay's own sill and jamb, and a pin is
    // measured **from the bay's corner**: bay 0 starts at the edge's start, so here the leaf's `x` and
    // the hole's `from` are the same number and the general rule is easy to mistake for it.
    let doorway_bay = skeleton.bay(face, 0, 0).expect("the grid is there");
    let at = Pin::new(hole.from - doorway_bay.from, 0.0);
    let placed = ops::place_part(&mut skeleton, Placement::new(template, face, 0, 0, at))
        .expect("the template and the slot are both there");
    let standing = skeleton.placed(placed).expect("just placed");
    let bay = standing
        .bay
        .expect("the leaf stands in a bay rather than on a wall");
    let reach = standing.reach;
    println!(
        "# the leaf placed in bay [{:.4}, {:.4}] x [{:.4}, {:.4}]: x [{:.4}, {:.4}], y [{:.4}, {:.4}]",
        bay.from,
        bay.to,
        bay.low,
        bay.high,
        reach.x.0 + bay.from,
        reach.x.1 + bay.from,
        reach.y.0 + bay.low,
        reach.y.1 + bay.low
    );
    let inside_hole = reach.x.0 + bay.from >= hole.from - 1.0e-3
        && reach.x.1 + bay.from <= hole.to + 1.0e-3
        && reach.y.0 + bay.low >= hole.low - 1.0e-3
        && reach.y.1 + bay.low <= hole.high + 1.0e-3;
    println!("  the leaf fits its hole: {inside_hole}");

    println!("\n# a part that does not fit its bay is placed and reported, not refused");
    let tall = ops::add_template(&mut skeleton, door_leaf(0.9, 2.40, 0.06));
    let too_tall = ops::place_part(
        &mut skeleton,
        Placement::new(tall, face, 0, 1, Pin::new(0.6, 0.0)),
    )
    .expect("the bay is there");
    let big = skeleton.placed(too_tall).expect("just placed").reach;
    println!(
        "  a 2.40 m leaf in a {:.4} m storey: y [{:.4}, {:.4}], so it stands {:.4} m past the head",
        bay.height(),
        big.y.0,
        big.y.1,
        big.y.1 - bay.height()
    );

    println!("\n# refusals at the verb");
    let cases = [
        (
            "a template the document does not hold",
            Placement::new(99, face, 0, 0, Pin::new(0.0, 0.0)),
        ),
        (
            "a bay the grid does not hold",
            Placement::new(template, face, 7, 9, Pin::new(0.0, 0.0)),
        ),
    ];
    for (name, placement) in cases {
        println!("  {name}: {:?}", ops::place_part(&mut skeleton, placement));
    }

    println!("\n# the document");
    let violations = Building::new(skeleton.clone()).validate();
    println!("  violations: {}", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }

    println!("\n# the mesh: a part is a solid of its own");
    match Building::new(skeleton.clone()).build() {
        Ok(mesh) => {
            let mut solids: std::collections::BTreeSet<whiting_building::SolidId> =
                std::collections::BTreeSet::new();
            for face in &mesh.faces {
                solids.insert(face.element.solid());
            }
            for solid in solids {
                match solid {
                    whiting_building::SolidId::Mass(_)
                    | whiting_building::SolidId::Roof(_)
                    | whiting_building::SolidId::Wall(_)
                    | whiting_building::SolidId::Slab(_) => {}
                    whiting_building::SolidId::Part(_) => {
                        let (low, high) = mesh.bounds_of(solid);
                        println!(
                            "  {solid:?}: {} faces, loops {}, euler {}, manifold {}, volume {:.6}",
                            mesh.faces_of(solid).len(),
                            mesh.solid_boundary_loops(solid),
                            mesh.solid_euler_characteristic(solid),
                            mesh.solid_is_manifold(solid),
                            mesh.solid_signed_volume(solid)
                        );
                        println!(
                            "    it stands in plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], and from \
                             y {:.4} to {:.4}",
                            low[0], high[0], low[2], high[2], low[1], high[1]
                        );
                    }
                }
            }
            let (mass_low, mass_high) = mesh.bounds_of(whiting_building::SolidId::Mass(mass));
            println!(
                "  the mass it stands in: plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], y [{:.4}, {:.4}]",
                mass_low[0], mass_high[0], mass_low[2], mass_high[2], mass_low[1], mass_high[1]
            );
        }
        Err(error) => println!("  refused: {:?}", error.problems),
    }

    println!("\n# a placement whose slot has gone is named, not silently dropped");
    // Clearing the face's rhythm orphans the doorway **and** both leaves, and the crate's shape is
    // all-or-nothing: the problems come back as a list and no mesh is handed over. What the probe
    // measures is that the leaves are *in* that list rather than missing from it — a lookup that
    // answers `None` filters, and the companion that names what it dropped is the whole point.
    let mut orphaned = skeleton.clone();
    ops::clear_face(&mut orphaned, face);
    match Building::new(orphaned.clone()).build() {
        Ok(_) => {
            println!("  built with two orphaned placements and said nothing, which is the fault")
        }
        Err(error) => {
            let leaves = error
                .problems
                .iter()
                .filter(|problem| {
                    matches!(
                        problem,
                        whiting_building::Problem::PlacementUnresolved { .. }
                    )
                })
                .count();
            println!(
                "  {} problems, {leaves} of them leaves: {:?}",
                error.problems.len(),
                error.problems
            );
        }
    }
    let violations = Building::new(orphaned).validate();
    let named = violations
        .iter()
        .filter(|violation| {
            matches!(
                violation,
                whiting_building::Violation::PlacementBayMissing { .. }
            )
        })
        .count();
    println!("  and the document's own verdict names {named} placements, {violations:?}");
}
