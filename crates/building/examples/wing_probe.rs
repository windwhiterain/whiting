//! An L-shaped building, built the way the vocabulary intends: **a host and an attached wing**.
//!
//! # The decision this probe records
//!
//! An L can be one concave outline (which the strict crossing test now builds) or **two rectangles, the wing
//! attached to the host**. The second is this crate's own idiom: `attach_mass` writes an `Attachment { host, station
//! }` into the wing, and `check_overlap`'s own docs say an attachment **ends the overlap question** because "a wing
//! overlaps its host by construction". It also gives each arm **its own gable**, which is how an L is roofed in
//! practice: two ridges meeting at a valley, not one saddle over both.
//!
//! What this probe measures is that the two-mass reading actually builds, validates and is watertight per solid —
//! because a scene should not be written on an assumption the crate has not confirmed.
//!
//! Run with `cargo run -p whiting-building --example wing_probe`.

use whiting_building::derive::{self, HALL};
use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, RoofId, SolidId};

/// Returns a rectangular outline.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, 0.0))
        .collect()
}

fn main() {
    let plan = HALL.starter(6.0, 6.0, 1);
    let mut skeleton = BuildingSkeleton::default();
    // The host: a 6 x 6 room centred on the origin.
    let host = whiting_building::ops::add_mass(
        &mut skeleton,
        Mass::new(rectangle(-3.0, -3.0, 3.0, 3.0), plan.height)
            .with_wall(plan.wall)
            .with_slab(plan.slab),
    );
    // The wing: 6 x 4, reaching east from the host's east wall, overlapping it by the wall's thickness so the
    // attachment covers the shared leaf.
    let wing = whiting_building::ops::attach_mass(
        &mut skeleton,
        host,
        0.2,
        Mass::new(rectangle(3.0 - plan.wall, -2.0, 9.0, 2.0), plan.height)
            .with_wall(plan.wall)
            .with_slab(plan.slab),
    )
    .expect("the host's outline takes a station");

    // One gable a mass, each derived from its own outline.
    for id in [host, wing] {
        if let Some(roof) = derive::gable_roof(
            id,
            &skeleton.masses[id.0],
            plan.overhang,
            plan.rise,
            plan.sink,
            plan.roof_thickness,
        ) {
            whiting_building::ops::add_roof(&mut skeleton, roof);
        }
    }

    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("# a host and an attached wing, each with its own gable");
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    match building.build() {
        Ok(built) => {
            for (id, label) in [(host, "host"), (wing, "wing")] {
                let solid = SolidId::Mass(id);
                println!(
                    "  the {label}: manifold {}, boundary loops {}, volume {:.4}",
                    built.solid_is_manifold(solid),
                    built.solid_boundary_loops(solid),
                    built.solid_signed_volume(solid)
                );
            }
            for roof in [0usize, 1] {
                let solid = SolidId::Roof(RoofId(roof));
                println!(
                    "  roof {roof}: manifold {}, boundary loops {}",
                    built.solid_is_manifold(solid),
                    built.solid_boundary_loops(solid)
                );
            }
        }
        Err(error) => println!(
            "  the document does not build: {:?}",
            error.problems.first()
        ),
    }
    println!(
        "\n# the reading: two rectangles, one attached, one gable each — the shape the vocabulary intends"
    );
    let _ = MassId(0);
}
