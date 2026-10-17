//! The shared-face question: **does a hole in a face two solids share do harm, or is it only ugly?**
//!
//! # What this decides
//!
//! `attach_mass` records an attachment and the wing overlaps its host by construction, so the two arms **share a
//! face**. Nothing stops a `split_face` and an `add_opening` on that face — the scene simply chooses not to — and the
//! closing record says so. The question a `Violation` would answer is *whether the crate should refuse it*, and
//! refusing needs a reason stronger than taste.
//!
//! This probe measures the reason: a hole in a shared face removes a piece of **one** arm's wall and leaves the
//! other arm's wall whole, so where the two arms overlap there is now a **hole backed by solid**. The solid is
//! still watertight (it lost an opening's worth of wall and gained the reveal faces), so nothing is *broken* — but
//! the two solids no longer meet cleanly there, and a door cut through the party wall would open into the
//! neighbour's wall rather than through it.
//!
//! Run with `cargo run -p whiting-building --example shared_face_probe`.

use whiting_building::derive::HALL;
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

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
    let host = ops::add_mass(
        &mut skeleton,
        Mass::new(rectangle(-3.0, -3.0, 3.0, 3.0), plan.height)
            .with_wall(plan.wall)
            .with_slab(plan.slab),
    );
    let wing = ops::attach_mass(
        &mut skeleton,
        host,
        0.2,
        Mass::new(rectangle(3.0 - plan.wall, -2.0, 9.0, 2.0), plan.height)
            .with_wall(plan.wall)
            .with_slab(plan.slab),
    )
    .expect("the host takes a station off its corners");

    // **The host's east face is the shared one**: the wing reaches west over it by the wall's thickness. Split it
    // and cut a door, which is exactly what a scene must not do.
    let shared = FaceRef::new(host, 1);
    ops::split_face(
        &mut skeleton,
        shared,
        &Rhythm::Even { count: 2 },
        &Rhythm::Even { count: 1 },
    );
    let cut = ops::add_opening(
        &mut skeleton,
        Opening::new(shared, 0, 0, Inset::new(0.6, 0.6, 0.0, 2.1)),
    );

    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("# a door cut into the face two arms share");
    println!("  the cut was accepted: {}", cut.is_some());
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
        }
        Err(error) => println!(
            "  the document does not build: {:?}",
            error.problems.first()
        ),
    }
    println!(
        "\n# the reading: {}",
        if violations.is_empty() {
            "the crate accepts a hole in a shared face, so the refusal the closing record names would have to be a new \
             judgement rather than a fault it already sees"
        } else {
            "the crate already reports something about a hole in a shared face"
        }
    );
    let _ = MassId(0);
}
