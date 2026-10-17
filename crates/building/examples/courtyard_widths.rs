//! The courtyard's question, resolved by sweeping the one variable that distinguishes the two answers.
//!
//! # What the U-shape refusal could mean
//!
//! `courtyard_probe` measured that a U-shaped mass at a 0.3 m wall is refused with `OffsetSelfIntersecting`. Two
//! readings were left open:
//!
//! - the offset's **reflex-corner handling is wrong**, so no concave building is expressible at any thickness; or
//! - the offset is right and the refusal is a **limit** of this notch against this wall, so a thinner wall — or a
//!   wider notch — builds.
//!
//! **Sweeping the wall tells the two apart without reading the offset loop at all.** A systematic error in the
//! miter fails at every thickness; a geometric limit succeeds as the wall shrinks. The notch's width is swept the
//! same way, because a limit should also move when the courtyard widens.
//!
//! Run with `cargo run -p whiting-building --example courtyard_widths`.

use whiting_building::{Building, BuildingSkeleton, Mass, PlanNode};

/// Returns a U-shaped outline: 10 × 10 outside, with a `notch`-wide courtyard open to the north.
fn u_shape(notch: f32) -> Vec<PlanNode> {
    let half = notch * 0.5;
    [
        [-5.0, -5.0],
        [5.0, -5.0],
        [5.0, 5.0],
        [half, 5.0],
        [half, -1.0],
        [-half, -1.0],
        [-half, 5.0],
        [-5.0, 5.0],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, 0.0))
    .collect()
}

/// Answers whether a U of this notch and this wall expands, and what it says if not.
fn try_u(notch: f32, wall: f32) -> (bool, usize, String) {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(u_shape(notch), 3.2)
            .with_wall(wall)
            .with_slab(0.4),
    ];
    let building = Building::new(skeleton);
    let violations = building.validate();
    let detail = match building.build() {
        Ok(_) => String::from("builds"),
        Err(error) => format!("{:?}", error.problems.first()),
    };
    (violations.is_empty(), violations.len(), detail)
}

fn main() {
    println!("# the wall swept, at a 4 m notch");
    // **Monotonicity is the test.** A geometric limit refuses thin walls and accepts thick ones; a defect can
    // refuse 0.05 and accept 0.10, which no geometry does. So the outcomes are recorded in order and the answer
    // is whether they ever go the other way.
    let mut outcomes: Vec<bool> = Vec::new();
    for wall in [0.05f32, 0.1, 0.15, 0.2, 0.3] {
        let (clean, count, detail) = try_u(4.0, wall);
        outcomes.push(clean);
        println!("  wall {wall:.2}: {count} violation(s), {detail}");
    }

    println!("\n# the notch swept, at a 0.3 m wall");
    for notch in [4.0f32, 6.0, 8.0] {
        let (_clean, count, detail) = try_u(notch, 0.3);
        println!("  notch {notch:.1}: {count} violation(s), {detail}");
    }

    // The sweep must be monotone in the wall for the refusal to be a limit: thin builds, thick refuses. Any other
    // pattern is a defect, and it is the pattern the first run produced.
    let monotone = outcomes.windows(2).all(|pair| pair[0] || !pair[1]);
    println!(
        "\n# the reading: the sweep is {}, so {}",
        if monotone {
            "monotone (thin builds, thick refuses)"
        } else {
            "NOT monotone"
        },
        if monotone {
            "the refusal is a geometric limit of this notch against this wall"
        } else {
            "the offset has a defect: no geometry refuses a 0.05 m wall and accepts a 0.10 m one"
        }
    );
}
