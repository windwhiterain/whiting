//! The courtyard that closes: **what the crate does with a ring authored as one polygon**.
//!
//! # What is being asked
//!
//! A room whose plan is a ring — a closed courtyard, a patio, a siheyuan — is the one building type this crate
//! cannot express, and the closing record says why: a `Mass` is authored as **one simple polygon** and a ring is not
//! simple. The question this probe answers is what happens when someone writes one anyway: does the crate **refuse
//! it with a reason**, or draw something wrong? A refusal with a reason is a boundary; a wrong drawing is a hole in
//! the vocabulary that no reader would guess.
//!
//! The outline below is a ring written as a single loop: it goes round the outside, steps inward at one corner, runs
//! round the inside in the opposite direction, and closes — the classic way a hole is smuggled into a single
//! polygon.
//!
//! Run with `cargo run -p whiting-building --example ring_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns a 10 × 10 ring with a 4 × 4 hole, written as one loop that crosses itself at the hole's neck.
fn ring() -> Vec<PlanNode> {
    [
        [-5.0, -5.0],
        [5.0, -5.0],
        [5.0, 5.0],
        [1.0, 5.0],
        // Into the hole, round it the other way, and back out through the same neck.
        [1.0, 1.0],
        [-1.0, 1.0],
        [-1.0, -1.0],
        [1.0, -1.0],
        [1.0, 1.0],
        [-1.0, 5.0],
        [-5.0, 5.0],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, 0.0))
    .collect()
}

fn main() {
    let skeleton = BuildingSkeleton::new(
        vec![Mass::new(ring(), 3.2).with_wall(0.3).with_slab(0.4)],
        Vec::new(),
    );
    println!("# a 10 x 10 ring with a 4 x 4 hole, written as one loop with a neck");
    println!(
        "  the outline is {} nodes and crosses itself at the neck",
        ring().len()
    );

    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    match building.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            let (low, high) = built.bounds();
            println!(
                "  it built anyway: manifold {}, loops {}, faces {}, volume {:.4}",
                built.solid_is_manifold(solid),
                built.solid_boundary_loops(solid),
                built.faces_of(solid).len(),
                built.solid_signed_volume(solid)
            );
            println!(
                "  its bounds: x {:.2}..{:.2}, y {:.2}..{:.2}, z {:.2}..{:.2}",
                low[0], high[0], low[1], high[1], low[2], high[2]
            );
        }
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }
    println!(
        "\n# the reading: {}",
        if violations.is_empty() {
            "the crate draws the ring without a word, so the hole in the vocabulary is silent"
        } else {
            "the crate names the ring as a fault, so the boundary is stated rather than discovered"
        }
    );
}
