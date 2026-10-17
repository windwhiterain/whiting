//! The courtyard's first step: **the vocabulary can say it, and the crate refuses to pretend it drew it.**
//!
//! # What this measures
//!
//! `Mass::voids` lets a document author a courtyard — a loop inside the outline that the mass has a hole into. The
//! drawing does not exist yet, so the crate must **refuse** rather than expand the mass as if the void were absent:
//! a solid that quietly ignored an authored courtyard would be worse than no feature at all.
//!
//! Two readings, and the first is the control:
//!
//! 1. a mass with **no** void expands, and `validate` is silent;
//! 2. the same mass with a 4 × 4 void reports **`DidNotExpand { VoidNotDrawn }`** and does not build.
//!
//! Run with `cargo run -p whiting-building --example patio_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns a rectangular outline.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, 0.0))
        .collect()
}

fn main() {
    let outline = rectangle(-5.0, -5.0, 5.0, 5.0);

    // 1. The control: no void, and the mass is what it always was.
    let mut plain = BuildingSkeleton::default();
    plain.masses = vec![Mass::new(outline.clone(), 3.2).with_wall(0.3).with_slab(0.4)];
    let control = Building::new(plain);
    let control_violations = control.validate();
    let control_volume = match control.build() {
        Ok(built) => format!("{:.4}", built.solid_signed_volume(SolidId::Mass(MassId(0)))),
        Err(error) => format!("does not build: {:?}", error.problems.first()),
    };
    println!("# a 10 x 10 mass with no courtyard");
    println!(
        "  {} violation(s), volume {control_volume}",
        control_violations.len()
    );

    // 2. The same mass with a courtyard authored: refused, with the reason.
    let mut patio = BuildingSkeleton::default();
    patio.masses = vec![Mass::new(outline, 3.2)
        .with_wall(0.3)
        .with_slab(0.4)
        .with_voids(vec![rectangle(-2.0, -2.0, 2.0, 2.0)])];
    let with_patio = Building::new(patio);
    let violations = with_patio.validate();
    println!("\n# the same mass with a 4 x 4 courtyard authored");
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    match with_patio.build() {
        Ok(_) => println!("  it built, which it must not do until the drawing exists"),
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    println!(
        "\n# the reading: {}",
        if violations.is_empty() {
            "the crate accepted a courtyard it cannot draw, which would be a lie"
        } else {
            "the crate says it cannot draw a courtyard yet, which is the refusal the slice needs until it can"
        }
    );
}
