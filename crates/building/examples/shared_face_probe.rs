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
use whiting_building::{BuildingSkeleton, Mass, MassId, PlanNode};

/// Returns a rectangular outline.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, 0.0))
        .collect()
}

/// Answers whether one split-and-cut on a mass face is accepted.
fn cut_on(skeleton: &mut BuildingSkeleton, mass: MassId, face: usize) -> bool {
    let held = FaceRef::new(mass, face);
    ops::split_face(
        skeleton,
        held,
        &Rhythm::Even { count: 2 },
        &Rhythm::Even { count: 1 },
    );
    ops::add_opening(
        skeleton,
        Opening::new(held, 0, 0, Inset::new(0.6, 0.6, 0.0, 2.1)),
    )
    .is_some()
}

/// Returns a 6 x 6 room and a 6 x 4 wing attached to it, the pair the shared face belongs to.
fn pair(plan: &whiting_building::derive::Starter) -> (BuildingSkeleton, MassId, MassId) {
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
    (skeleton, host, wing)
}

fn main() {
    let plan = HALL.starter(6.0, 6.0, 1);
    println!("# the same split and the same 2.1 m door, on three faces");

    // 1. A lone 6 x 6 room: nothing is attached to it, so nothing is shared.
    let mut alone = BuildingSkeleton::default();
    let lone = ops::add_mass(
        &mut alone,
        Mass::new(rectangle(-3.0, -3.0, 3.0, 3.0), plan.height)
            .with_wall(plan.wall)
            .with_slab(plan.slab),
    );
    let lone_cut = cut_on(&mut alone, lone, 1);

    // 2. The pair, cut on the wing outer face: not shared by anything.
    let (mut outer, _host, wing) = pair(&plan);
    let outer_cut = cut_on(&mut outer, wing, 1);

    // 3. The pair, cut on the host east face: the one the wing overlaps.
    let (mut shared, host, _wing) = pair(&plan);
    let shared_cut = cut_on(&mut shared, host, 1);

    println!("  a lone room, its east face:        accepted {lone_cut}");
    println!("  an attached pair, a wing outer face: accepted {outer_cut}");
    println!("  an attached pair, the shared face:   accepted {shared_cut}");
    println!(
        "\n# the reading: {}",
        if lone_cut && outer_cut && !shared_cut {
            "only the shared face is refused, so the refusal is attachment-aware"
        } else if !shared_cut && !(lone_cut && outer_cut) {
            "the refusal is not about the attachment: the same cut fails on faces nothing shares"
        } else {
            "the shared face accepts the cut, so nothing guards it"
        }
    );
}
