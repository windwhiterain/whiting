//! The courtyard's question: **can a mass be concave, and can it have a hole?**
//!
//! # Two things are being asked
//!
//! 1. **A hole.** A courtyard closed on all four sides is a room whose plan is a ring, and a `Mass` is authored as
//!    **one simple polygon** — a ring is not simple, so a closed courtyard is not expressible by construction. What
//!    *is* expressible is a courtyard **open on one side**: a U-shaped outline.
//! 2. **A concave outline.** A U has two reflex corners, and the crate's walls, miters, slab, reveals and roof all
//!    read the outline — so whether a U expands into one watertight solid is a real question, not a formality.
//!
//! This probe measures the second, because the first is a vocabulary answer rather than a measurement.
//!
//! Run with `cargo run -p whiting-building --example courtyard_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns a U-shaped outline: 10 × 10 outside, with a 4 × 6 courtyard open to the north.
///
/// The nodes run clockwise from the south-west corner, up the west side, round the courtyard's three sides, and
/// back down the east side — eight nodes, of which the two at the courtyard's mouth are reflex.
fn u_shape() -> Vec<PlanNode> {
    let elevation = 0.0;
    [
        [-5.0, -5.0],
        [5.0, -5.0],
        [5.0, 5.0],
        [2.0, 5.0],
        [2.0, -1.0],
        [-2.0, -1.0],
        [-2.0, 5.0],
        [-5.0, 5.0],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, elevation))
    .collect()
}

fn main() {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![Mass::new(u_shape(), 3.2).with_wall(0.3).with_slab(0.4)];
    println!("# a U-shaped mass: 10 x 10 outside, a 4 x 6 courtyard open to the north");
    println!("  the outline is {} nodes, two of them reflex", u_shape().len());

    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }

    match building.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            println!(
                "  the mass: manifold {}, boundary loops {}, volume {:.4}",
                built.solid_is_manifold(solid),
                built.solid_boundary_loops(solid),
                built.solid_signed_volume(solid)
            );
            let (low, high) = built.bounds();
            println!(
                "  its bounds: x {:.2}..{:.2}, y {:.2}..{:.2}, z {:.2}..{:.2}",
                low[0], high[0], low[1], high[1], low[2], high[2]
            );
        }
        Err(error) => println!("  the document does not build: {error:?}"),
    }

    println!(
        "\n# the vocabulary answer: a closed courtyard needs a plan with a hole, and a mass is authored as one\n# simple polygon, so the crate draws a courtyard that is open on one side and not one that is closed."
    );
}
