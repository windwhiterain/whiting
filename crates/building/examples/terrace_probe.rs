//! The terrace's first question: does the vocabulary accept two masses that **touch**?
//!
//! **This measures no camera, and it is not a measurement of the `terrace` scene.** The name is the
//! terrace *idea*, not `gallery`'s `terrace`; nothing here draws a frame and there is no camera in
//! this file. What it reads is `Building::validate` and the built solid's manifoldness, boundary
//! loops and signed volume. A reader who came for the scene's picture is in the wrong file, and
//! that is a fact about the name worth one line rather than four files of reading.
//!
//! # What a party wall has to be
//!
//! A terrace is houses side by side sharing one wall. The crate's cheapest honest reading is **two leaves**:
//! each house is its own closed solid with its own wall, and the party wall is what two leaves look like when
//! their outer faces meet — 0.15 + 0.15 = 0.3 m of wall that reads as one, exactly as a real party wall often is.
//!
//! That reading costs no new vocabulary, and it turns the design question into a measurable one: **does
//! `validate` refuse two solids whose faces are coincident but whose interiors are disjoint?** A zero-volume
//! meeting is not an overlap, but a checker that tested for "faces in the same place" would say it is, and the
//! answer decides whether a terrace is expressible today or needs a decision first.
//!
//! The masses here carry no roofs on purpose: `validate` has no violation for a mass without one, so nothing is
//! hidden, and the question is about the walls.
//!
//! Run with `cargo run -p whiting-building --example terrace_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns a rectangular outline from `(x0, z0)` to `(x1, z1)` at the given elevation.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32, elevation: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, elevation))
        .collect()
}

fn main() {
    // Two houses, 6 x 8 each, flush at x = 0: the left one occupies -6..0 and the right one 0..6, and each has
    // a 0.15 m wall, so the party wall is the two leaves together.
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(rectangle(-6.0, -4.0, 0.0, 4.0, 0.0), 3.2).with_wall(0.15),
        Mass::new(rectangle(0.0, -4.0, 6.0, 4.0, 0.0), 3.2).with_wall(0.15),
    ];

    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("# two 6 x 8 houses flush at x = 0, 0.15 m wall each");
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }

    if let Ok(built) = building.build() {
        for (index, label) in [(0usize, "left"), (1, "right")] {
            let solid = SolidId::Mass(MassId(index));
            println!(
                "  the {label} house: manifold {}, boundary loops {}, volume {:.4}",
                built.solid_is_manifold(solid),
                built.solid_boundary_loops(solid),
                built.solid_signed_volume(solid)
            );
        }
        println!(
            "  the party wall is two leaves of 0.15 m meeting at x = 0, so the wall reads as 0.30 m of wall"
        );
    } else {
        println!("  the document does not build");
    }

    println!(
        "\n# the reading: {}",
        if violations.is_empty() {
            "touching is allowed, so a terrace is two closed solids whose leaves meet"
        } else {
            "touching is refused, so the vocabulary has no terrace until a decision is made"
        }
    );

    // **The boundary of that answer: one shared leaf instead of two.** Overlap the two houses by the leaf's
    // thickness, so the strip `-0.15 .. 0` is wall to *both* of them. Two closed solids then share a volume, which
    // is the case a terrace's two leaves exist to avoid — and the case the crate should refuse rather than draw,
    // because a pixel of that strip belongs to two buildings and no id can say which.
    let mut shared = BuildingSkeleton::default();
    shared.masses = vec![
        Mass::new(rectangle(-6.0, -4.0, 0.0, 4.0, 0.0), 3.2).with_wall(0.15),
        Mass::new(rectangle(-0.15, -4.0, 6.0, 4.0, 0.0), 3.2).with_wall(0.15),
    ];
    let shared_building = Building::new(shared);
    let shared_violations = shared_building.validate();
    println!("\n# the other reading: one leaf, shared by both houses");
    println!(
        "  the document reports {} violation(s)",
        shared_violations.len()
    );
    for violation in &shared_violations {
        println!("    {violation:?}");
    }
    println!(
        "  the boundary reading: {}",
        if shared_violations.is_empty() {
            "a shared leaf is accepted, which the id channel cannot name: the strip belongs to two buildings"
        } else {
            "a shared leaf is refused, so the domain stops exactly where one pixel would have two owners"
        }
    );
}
