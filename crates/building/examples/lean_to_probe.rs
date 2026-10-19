//! The lean-to's acceptance: **a roof with one slope**, over a porch that stands against a wall.
//!
//! A 夏子 is a porch with its own little roof shedding away from the 正房, and that roof is one plane: it
//! runs from the wall, where it is high, down to the porch's front edge, where it is an eave. The crate's
//! derivations build gables and hips — both of which put the crest **inside** the eave loop — so the
//! question this probe settles is whether a crest **on** the loop's own back edge is expressible, and what
//! it costs if it is not.
//!
//! Run with `cargo run -p whiting-building --example lean_to_probe`.

use whiting_building::derive;
use whiting_building::mesh::RoofSurface;
use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// The porch: nine metres along the wall, 1.2 out from it.
const HALF: f32 = 4.5;
const DEPTH: f32 = 1.2;

/// The wall's top, the eave's height, and how far the roof stands over its own porch.
const WALL_TOP: f32 = 3.9;
const EAVE: f32 = 2.9;
const OVERHANG: f32 = 0.15;
const THICKNESS: f32 = 0.12;

fn main() {
    println!("# a lean-to: one plane from a wall down to a porch's eave");
    for (label, inset) in [
        ("the crest on the wall line", 0.0_f32),
        ("the crest 5 cm short of it", 0.05),
    ] {
        println!("\n# {label}");
        match attempt(inset) {
            Ok(report) => println!("  {report}"),
            Err(report) => println!("  refused: {report}"),
        }
    }
}

/// Returns a one-line report, or the refusal, for a lean-to whose crest stands `inset` in front of the wall.
fn attempt(inset: f32) -> Result<String, String> {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    // The wall this lean-to stands against: a 0.45 m mass two metres deep, tall enough to carry the roof.
    let mut wall = Mass::new(
        // **The wall is 1.8 m deep, which is thicker than twice its own 0.45 m section**: an inner loop
        // that inverts is a wall with no inside, and that is what a 0.75 m wall of 0.45 m thickness is.
        vec![
            PlanNode::new([-HALF - 0.5, -2.2], 0.0),
            PlanNode::new([HALF + 0.5, -2.2], 0.0),
            PlanNode::new([HALF + 0.5, -0.4], 0.0),
            PlanNode::new([-HALF - 0.5, -0.4], 0.0),
        ],
        WALL_TOP,
    );
    wall.wall = 0.45;
    wall.slab = 0.3;
    skeleton.masses.push(wall);

    // **The derivation, not a hand-authored roof**: `lean_to` builds the loop, the crest and the one
    // plane, and the probe says whether the *model* accepts what it builds.
    let back = -0.4;
    let front = back + DEPTH + OVERHANG;
    let roof = derive::lean_to(
        MassId(0),
        [
            [-HALF - OVERHANG, front],
            [HALF + OVERHANG, front],
            [HALF + OVERHANG, back + inset],
            [-HALF - OVERHANG, back + inset],
        ],
        EAVE,
        WALL_TOP,
        THICKNESS,
    )
    .expect("a porch rectangle is a lean-to");
    skeleton.roofs.push(roof);
    let building = Building::new(skeleton);
    let violations = building.validate();
    println!("  one plane, two rows: the eave's own two ends up to the crest");
    let built = building.build().map_err(|error| format!("{error:?}"))?;
    let solid = SolidId::Roof(whiting_building::roof::RoofId(0));
    let area: f32 = built
        .faces_of(solid)
        .iter()
        .filter(|face| {
            matches!(
                built.faces[**face].element,
                whiting_building::Element::Roof(_, RoofSurface::Skin { .. })
            )
        })
        .map(|face| {
            let corners = built.triangle(&built.faces[*face]);
            let twice = (corners[1][0] - corners[0][0]) * (corners[2][2] - corners[0][2])
                - (corners[2][0] - corners[0][0]) * (corners[1][2] - corners[0][2]);
            (twice * 0.5).abs()
        })
        .sum();
    Ok(format!(
        "{} violation(s); skin plan area {area:.4} m² against {:.4}; euler {}; manifold {}; volume {:.6}",
        violations.len(),
        (2.0 * (HALF + OVERHANG)) * (DEPTH + OVERHANG),
        built.solid_euler_characteristic(solid),
        built.solid_is_manifold(solid),
        built.solid_signed_volume(solid)
    ))
}
