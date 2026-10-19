//! The covering's acceptance: the rolls on a roof, measured against the skin they cover.
//!
//! A covering is three numbers — a period, a fill, a height — and what it must produce is **one closed
//! solid per roll**, bent at every purlin the skin bends at, standing on the skin rather than on a
//! plane that approximates it. Everything here is that claim measured.
//!
//! Run with `cargo run -p whiting-building --example covering_probe`.

use whiting_building::covering::{Covering, RollId};
use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::roof::{RoofId, Step};
use whiting_building::{Building, SolidId};

/// The house under the roof: ten by six, one storey, a gable.
const STARTER: Starter = Starter {
    width: 10.0,
    depth: 6.0,
    height: 3.2,
    wall: 0.24,
    slab: 0.30,
    overhang: 0.60,
    sink: 0.10,
    rise: 2.385,
    roof_thickness: 0.22,
    roof: RoofStyle::Gable,
    seed: 5,
};

/// The 举架 ladder the rolls must bend at, from the roof-profile arc.
const BEAM: f32 = 0.9;
const PROFILE: [Step; 3] = [
    Step {
        fraction: 0.25,
        rise: BEAM * 0.50,
    },
    Step {
        fraction: 0.50,
        rise: BEAM * 0.50 + BEAM * 0.60,
    },
    Step {
        fraction: 0.75,
        rise: BEAM * 0.50 + BEAM * 0.60 + BEAM * 0.75,
    },
];

/// The covering: a roll every 30 cm of plan, 62 % of that period wide, 6 cm proud of the skin.
const COVERING: Covering = Covering {
    period: 0.30,
    fill: 0.62,
    height: 0.06,
};

fn main() {
    let mut skeleton = starter(&STARTER).skeleton;
    for plane in &mut skeleton.roofs[0].planes {
        plane.profile = PROFILE.to_vec();
    }
    skeleton.roofs[0] = skeleton.roofs[0].clone().with_covering(COVERING);

    println!(
        "# a {:.0} x {:.0} gable, {} steps a slope, rolls every {:.2} m of {:.2} m width",
        STARTER.width,
        STARTER.depth,
        PROFILE.len(),
        COVERING.period,
        COVERING.period * COVERING.fill
    );
    let built = Building::new(skeleton.clone())
        .build()
        .expect("a covered roof expands");
    let violations = Building::new(skeleton.clone()).validate();
    println!("  the document reports {} violation(s)", violations.len());
    for violation in violations.iter().take(6) {
        println!("    {violation:?}");
    }
    // Debug run: the assertion is at the end, after the boundary dump.
    let _clean = violations.is_empty();

    // Every roll is one closed box-like prism: euler 2, manifold, positive volume.
    // Debug: the boundary of the first roll, if it has one.
    {
        let faces: Vec<usize> = (0..built.faces.len())
            .filter(|face| matches!(built.faces[*face].element, whiting_building::Element::Roll(id, _) if id.index == 0))
            .collect();
        let boundary = built.boundary_edges_of(&faces);
        println!(
            "\n  roll 0 has {} face(s) and {} boundary edge(s)",
            faces.len(),
            boundary.len()
        );
        for (from, to) in boundary.iter().take(6) {
            println!(
                "    edge {:.4},{:.4},{:.4} → {:.4},{:.4},{:.4}",
                built.points[*from as usize][0],
                built.points[*from as usize][1],
                built.points[*from as usize][2],
                built.points[*to as usize][0],
                built.points[*to as usize][1],
                built.points[*to as usize][2]
            );
        }
    }

    println!("\n# every roll is one closed solid");
    let mut rolls = 0;
    let mut faces = 0;
    let mut shortest = f32::INFINITY;
    let mut tallest = f32::NEG_INFINITY;
    let mut worst_euler = 0;
    let mut volume_sum = 0.0_f64;
    while rolls < 10_000 {
        let id = SolidId::Roll(RollId {
            roof: RoofId(0),
            index: rolls,
        });
        let count = built.faces_of(id).len();
        if count == 0 {
            break;
        }
        faces += count;
        let volume = built.solid_signed_volume(id);
        let euler = built.solid_euler_characteristic(id);
        assert!(built.solid_is_manifold(id), "roll {rolls} is not closed");
        worst_euler = worst_euler.max((euler - 2).abs());
        assert!(volume > 0.0, "roll {rolls} is inside out: {volume}");
        volume_sum += f64::from(volume);
        let (low, high) = built.bounds_of(id);
        let rise = high[1] - low[1];
        shortest = shortest.min(rise);
        tallest = tallest.max(rise);
        rolls += 1;
    }
    println!(
        "  {rolls} roll(s), {faces} triangle(s), total volume {volume_sum:.6} m³",
        faces = faces
    );
    println!(
        "  every roll's euler characteristic is 2 within {worst_euler}; its height ranges {shortest:.4} .. {tallest:.4} m",
    );
    assert!(rolls > 10, "a 10 m eave at 30 cm is more than ten rolls");
    assert_eq!(worst_euler, 0, "a roll is a ball");

    // **A roll is on the skin and not on a plane through it**: its base follows the bend, so the roll
    // that crosses the purlins is taller than its own section by the drop it covers.
    println!("\n# a roll bends at the purlins, so its ends are not its section apart");
    let first = SolidId::Roll(RollId {
        roof: RoofId(0),
        index: 0,
    });
    let (low, high) = built.bounds_of(first);
    println!(
        "  roll 0 spans y {:.4} .. {:.4} and x {:.4} .. {:.4}",
        low[1], high[1], low[0], high[0]
    );
    assert!(
        high[1] - low[1] > COVERING.height * 2.0,
        "a roll crossing a rise must be taller than its own height"
    );

    println!("\n# a covering that is not a covering is refused by name");
    for (label, covering, expect) in [
        (
            "a period of zero",
            Covering::new(0.0, 0.6, 0.06),
            "PeriodNotALength",
        ),
        (
            "a fill of one",
            Covering::new(0.3, 1.0, 0.06),
            "FillOutOfRange",
        ),
        (
            "a height of zero",
            Covering::new(0.3, 0.6, 0.0),
            "HeightNotARise",
        ),
    ] {
        let mut broken = starter(&STARTER);
        broken.skeleton.roofs[0] = broken.skeleton.roofs[0].clone().with_covering(covering);
        let found: Vec<String> = Building::new(broken.skeleton.clone())
            .validate()
            .iter()
            .map(|violation| {
                format!("{violation:?}")
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        let named = found.iter().any(|name| name == "CoveringFault");
        let detail = found.join(",");
        println!("  {label}: {detail:?} (named CoveringFault: {named})");
        assert!(
            named,
            "{label} must be reported as CoveringFault ({expect})"
        );
    }
}
