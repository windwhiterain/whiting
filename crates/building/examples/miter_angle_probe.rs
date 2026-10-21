//! Measures what `Problem::MiterTooLong` now says: **the corner's own angle**, not only the length the mitre reached.
//!
//! # What this probe answers
//!
//! *Which corner of which plan breaks the join limit, and by how much?* The reach is `wall / sin(interior / 2)`, so a square corner reaches
//! `1.41 × wall` and never breaks it, while a `20°` corner reaches `5.76 × wall` and always does. The probe builds one rectangle and one
//! wedge of `20°` with the same wall, and prints both verdicts.

use whiting_building::mass::{Mass, PlanNode};
use whiting_building::ops;
use whiting_building::skeleton::{Building, BuildingSkeleton};

/// The wall the probe gives both plans: the reach is reported in multiples of it.
const WALL: f32 = 0.30;

fn main() {
    // **A rectangle's corner is 90°**, and its mitre reaches `WALL / sin(45°) = 1.41 × WALL` — under the limit of four.
    let square = Mass::new(
        ops::wound(vec![[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [-2.0, 2.0]])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
        2.4,
    );
    // **A wedge of 20°** at the origin: its mitre reaches `WALL / sin(10°) = 5.76 × WALL`, which no wall thickness can pay for.
    let apex = [[0.0_f32, 0.0_f32], [6.0, -1.05], [6.0, 1.05]];
    let wedge = Mass::new(
        ops::wound(vec![apex[0], apex[1], apex[2]])
            .into_iter()
            .map(|at| PlanNode::new(at, 0.0))
            .collect(),
        2.4,
    );

    for (name, mut mass) in [("square", square), ("wedge", wedge)] {
        mass.wall = WALL;
        let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
        skeleton.masses.push(mass);
        let problems = Building::new(skeleton).validate();
        if problems.is_empty() {
            println!(
                "{name}: expands clean, a corner of 90 degrees reaches {:.2} x wall",
                1.41
            );
        } else {
            for problem in problems {
                println!("{name}: {problem:?}");
            }
        }
    }
}
