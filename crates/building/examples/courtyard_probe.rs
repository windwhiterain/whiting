//! The concave shapes, measured as a matrix rather than one case.
//!
//! # What this pins
//!
//! `courtyard_probe` measured one U-shaped mass and found it refused; `courtyard_widths` proved the refusal was a
//! defect and not a boundary, and the strict crossing test fixed it. This probe is the record of what the fix
//! bought: **three concave plans the crate could not draw before**, each with the wall the crate would use for it.
//!
//! - **U** — a courtyard open to the north: two reflex corners at the mouth.
//! - **L** — a wing off a square: one reflex corner.
//! - **porch** — a shallow notch two metres deep: the case a cottage's front door wants.
//!
//! Each must report **no violations, a manifold solid and zero boundary loops**. A volume is printed for the
//! record rather than asserted: what makes a solid a solid is the first three readings, and the volume is what a
//! later change would move if it moved the geometry.
//!
//! Run with `cargo run -p whiting-building --example courtyard_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns an outline from plan corners at one elevation.
fn outline(corners: &[[f32; 2]]) -> Vec<PlanNode> {
    corners
        .iter()
        .map(|corner| PlanNode::new(*corner, 0.0))
        .collect()
}

/// Answers whether one concave plan expands into one watertight solid.
fn measure(label: &str, corners: &[[f32; 2]], wall: f32) {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(outline(corners), 3.2)
            .with_wall(wall)
            .with_slab(0.4),
    ];
    let building = Building::new(skeleton);
    let violations = building.validate();
    match building.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            println!(
                "  {label:<7} {} node(s), wall {wall:.2}: {} violation(s), manifold {}, loops {}, volume {:.4}",
                corners.len(),
                violations.len(),
                built.solid_is_manifold(solid),
                built.solid_boundary_loops(solid),
                built.solid_signed_volume(solid)
            );
        }
        Err(error) => println!(
            "  {label:<7} {} node(s), wall {wall:.2}: {} violation(s), does not build: {:?}",
            corners.len(),
            violations.len(),
            error.problems.first()
        ),
    }
}

fn main() {
    println!("# three concave plans, each with the wall a cottage would use");
    // A courtyard open to the north: 10 x 10 outside, a 4 x 6 notch.
    measure(
        "U",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, 5.0],
            [2.0, 5.0],
            [2.0, -1.0],
            [-2.0, -1.0],
            [-2.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );
    // A wing off a square: 10 x 10 with a 4 x 4 corner taken out.
    measure(
        "L",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, -1.0],
            [1.0, -1.0],
            [1.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );
    // A shallow porch two metres deep: the notch a front door wants.
    measure(
        "porch",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, 5.0],
            [2.0, 5.0],
            [2.0, 3.0],
            [-2.0, 3.0],
            [-2.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );

    println!(
        "\n# a closed courtyard would need a plan with a hole, and a mass is one simple polygon — so the crate draws\n# a courtyard open on one side and not one that is closed."
    );

    // **The identity table of a concave mass**, which is what a scene would draw through: every face of the
    // courtyard must name itself, and the two sides of the mouth must be two different elements.
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![Mass::new(outline(&[
        [-5.0, -5.0],
        [5.0, -5.0],
        [5.0, 5.0],
        [2.0, 5.0],
        [2.0, -1.0],
        [-2.0, -1.0],
        [-2.0, 5.0],
        [-5.0, 5.0],
    ]), 3.2)
    .with_wall(0.3)
    .with_slab(0.4)];
    let built = Building::new(skeleton).build().expect("the U expands");
    let table = whiting_building::identity::Identity::of(&built, 1);
    println!();
    println!("# the concave mass identity table");
    println!("  it holds {} row(s): one per face an element names", table.len());
    for row in table.rows().iter().take(4) {
        println!("    id {:>3} to {}", row.id, row.path);
    }
    let last = table.rows().last().expect("a row");
    println!("    id {:>3} to {} (last)", last.id, last.path);
    println!("  every row names something: {}", table.rows().iter().all(|row| !row.path.is_empty()));
}
