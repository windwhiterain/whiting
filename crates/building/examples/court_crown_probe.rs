//! The crowned court's acceptance: a rising gable over a light well, measured against the crown itself.
//!
//! The claim is one claim — **the wall top is one surface** — and it has three readings here: a crowned
//! mass with a court is a clean document at last, the court's ring reaches the crown's high side and not
//! the level top, and both solids close with the characteristic a ring has.
//!
//! Run with `cargo run -p whiting-building --example court_crown_probe`.

use whiting_building::math::Vec2;
use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// The mass: twelve by ten, so the ridge of anything over it runs along `x`.
const WIDTH: f32 = 12.0;
const DEPTH: f32 = 10.0;
const HEIGHT: f32 = 3.2;

/// The court: four by three, inside the walls.
const COURT: [f32; 4] = [-2.0, -1.5, 2.0, 1.5];

fn main() {
    println!("# a levelled mass with a court, which is what the crate could already draw");
    let level = document(&[HEIGHT; 4]);
    let level_report = report(&level, "levelled");
    let flat_top = level_report.1;
    println!(
        "  the court's wall top is {flat_top:.4}, which is the mass's own top of {HEIGHT:.1}: {}",
        (flat_top - HEIGHT).abs() < 1.0e-4
    );
    assert!(
        (flat_top - HEIGHT).abs() < 1.0e-4,
        "a levelled court keeps the level top"
    );

    println!("\n# a crowned mass with a court: the two node counts no rule joined");
    // The two `+z` nodes rise to 5.0 and the two `-z` nodes stay at 3.2: a gable rising to the north,
    // which is the direction the crown varies in, so the court's two z sides are the two readings.
    let crowned = document(&[HEIGHT, HEIGHT, 5.0, 5.0]);
    let crowned_report = report(&crowned, "crowned");
    let high_top = crowned_report.1;
    println!(
        "  the court's wall top reaches {high_top:.4} against the crown's high side of 5.0: the court rises with the gable"
    );
    assert!(
        high_top > HEIGHT + 0.5,
        "a crowned court must reach the crown and not the level top: {high_top}"
    );
    assert!(
        high_top <= 5.0 + 1.0e-4,
        "and it cannot reach past the crown: {high_top}"
    );

    // **The court's top is the crown read at the court's own nodes**, so its low side must be above the
    // mass's level top and below the crown's high side: a court interrupted by a rising gable crosses the
    // whole slope between the two.
    println!("\n# the court's four sides are the crown read at the court's own nodes");
    let (south, north) = court_tops(&crowned);
    println!("  the court's south wall tops at {south:.4}, its north wall at {north:.4}");
    assert!(
        (south - HEIGHT).abs() < 1.0e-3,
        "the court's low side must be the mass's own level top: {south}"
    );
    assert!(
        (north - 5.0).abs() < 1.0e-3,
        "and its high side the crown's own high side: {north}"
    );

    println!("\n# both are clean documents, and both are rings");
    for (label, building) in [("levelled", &level), ("crowned", &crowned)] {
        let violations = Building::new(building.skeleton.clone()).validate();
        let built = Building::new(building.skeleton.clone())
            .build()
            .expect("a court with a crown expands");
        let mass = SolidId::Mass(MassId(0));
        let court = SolidId::Void {
            mass: MassId(0),
            void: 0,
        };
        println!(
            "  {label}: {} violation(s); mass {} face(s) euler {}, court {} face(s) euler {}, both manifold {}",
            violations.len(),
            built.faces_of(mass).len(),
            built.solid_euler_characteristic(mass),
            built.faces_of(court).len(),
            built.solid_euler_characteristic(court),
            built.solid_is_manifold(court) && built.solid_is_manifold(mass)
        );
        if !violations.is_empty() {
            for violation in &violations {
                println!("    {violation:?}");
            }
        }
        assert!(violations.is_empty(), "{label} must be a clean document");
        assert_eq!(
            built.solid_euler_characteristic(mass),
            0,
            "{label}'s mass is a ring"
        );
        assert_eq!(
            built.solid_euler_characteristic(court),
            0,
            "{label}'s court is a ring"
        );
    }
    println!(
        "\n  a crowned mass and a crowned court are one document now, and neither shape changed"
    );
}

/// Returns a document: one mass with one court, and the four crowns it is given.
fn document(crowns: &[f32; 4]) -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let corners: [Vec2; 4] = [
        [-WIDTH * 0.5, -DEPTH * 0.5],
        [WIDTH * 0.5, -DEPTH * 0.5],
        [WIDTH * 0.5, DEPTH * 0.5],
        [-WIDTH * 0.5, DEPTH * 0.5],
    ];
    let mut mass = Mass::new(
        corners.iter().map(|at| PlanNode::new(*at, 0.0)).collect(),
        HEIGHT,
    );
    mass.wall = 0.4;
    mass.slab = 0.3;
    for (node, crown) in mass.crown_mut().outline.iter_mut().zip(crowns) {
        node.elevation = *crown;
    }
    mass.voids.push(
        vec![
            [COURT[0], COURT[1]],
            [COURT[2], COURT[1]],
            [COURT[2], COURT[3]],
            [COURT[0], COURT[3]],
        ]
        .into_iter()
        .map(|at| PlanNode::new(at, 0.0))
        .collect(),
    );
    skeleton.masses.push(mass);
    Building::new(skeleton)
}

/// Returns the document's violations and the court solid's highest elevation.
fn report(building: &Building, _label: &str) -> (usize, f32) {
    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("a court expands");
    let court = SolidId::Void {
        mass: MassId(0),
        void: 0,
    };
    let (_, high) = built.bounds_of(court);
    let violations = Building::new(building.skeleton.clone()).validate().len();
    (violations, high[1])
}

/// Returns the highest point of the court's wall on its `-z` side and on its `+z` side.
///
/// **One number per side, read off the court's own faces**: the ring's face over a side of the loop tops
/// at that side's crown, so the two sides of a court under a rising gable differ by the whole rise.
fn court_tops(building: &Building) -> (f32, f32) {
    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("a court expands");
    let court = SolidId::Void {
        mass: MassId(0),
        void: 0,
    };
    let (mut low, mut high) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for index in built.faces_of(court) {
        let corners = built.triangle(&built.faces[index]);
        let middle_z = (corners[0][2] + corners[1][2] + corners[2][2]) / 3.0;
        let top = corners
            .iter()
            .map(|corner| corner[1])
            .fold(f32::NEG_INFINITY, f32::max);
        // **Only the ring's outer band counts.** The face over a court edge is the top of that wall, and it
        // stands at the outer loop's own `z`; a face nearer the middle is the *inner wall* over the same
        // edge, which is a trapezoid running from the low crown to the high one and would answer `5` on
        // both sides.
        if middle_z.abs() < 1.7 {
            continue;
        }
        if middle_z < 0.0 {
            low = low.max(top);
        } else {
            high = high.max(top);
        }
    }
    (low, high)
}
