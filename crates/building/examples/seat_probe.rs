//! The seating slice's acceptance: a building put on a ground, against a ground whose answer is known.
//!
//! The ground here is the analytic plane `z = 0.3 x`, so every number has a closed form: the highest
//! ground under a 5 × 8 outline is at its own `x = 2.5` corner, and the seating level must be exactly
//! that. What the probe checks beyond it is that the seat is **rigid** — the walls keep their height and
//! the roof keeps its rise, so the volume is unchanged and the bounds move by exactly the delta — which
//! is the one way a seat can be silently wrong.
//!
//! Run with `cargo run -p whiting-building --example seat_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::site::{SeatReport, seat};
use whiting_building::{Building, MassId, RoofId, SolidId};

/// The ground: a plane rising to the east at thirty degrees' worth of slope.
fn slope(plan: [f32; 2]) -> f32 {
    0.3 * plan[0]
}

/// Returns the highest the slope reaches under a 5 × 8 outline centred on the origin: its own corner.
const ANALYTIC_HIGHEST: f32 = 0.3 * 2.5;
/// And the lowest, at the other corner.
const ANALYTIC_LOWEST: f32 = -0.3 * 2.5;

fn shed() -> Starter {
    Starter {
        width: 5.0,
        depth: 8.0,
        height: 2.8,
        roof: RoofStyle::Gable,
        ..Starter::default()
    }
}

fn main() {
    let plan = shed();
    let mut skeleton = starter(&plan).skeleton.clone();
    let before = skeleton.clone();
    let report: SeatReport = seat(&mut skeleton, slope, 0.0);

    println!("# a 5 x 8 shed on the plane z = 0.3 x");
    let seated = &report.masses[0];
    println!(
        "  level {:.6} (analytic {:.6}), lowest {:.6} (analytic {:.6}), delta {:.6}, fall {:.6}",
        seated.level,
        ANALYTIC_HIGHEST,
        seated.lowest,
        ANALYTIC_LOWEST,
        seated.delta,
        seated.fall()
    );
    println!(
        "  every node stands at the level: {}",
        skeleton.masses[0]
            .outline
            .iter()
            .all(|node| (node.elevation - seated.level).abs() < 1.0e-6)
    );
    println!(
        "  the level is the highest sampled ground plus the offset: {}",
        (seated.level - (seated.highest + 0.0)).abs() < 1.0e-6
    );

    println!("\n# the seat is rigid: the walls keep their height and the roof its rise");
    let unseated = Building::new(before.clone())
        .build()
        .expect("the shed expands on a plane");
    let seated_built = Building::new(skeleton.clone())
        .build()
        .expect("the seated shed expands");
    let mass = SolidId::Mass(MassId(0));
    let roof = SolidId::Roof(RoofId(0));
    let (low_before, high_before) = unseated.bounds_of(mass);
    let (low_after, high_after) = seated_built.bounds_of(mass);
    println!(
        "  mass volume {:.6} before, {:.6} after (the seat moves it, it does not resize it)",
        unseated.solid_signed_volume(mass),
        seated_built.solid_signed_volume(mass)
    );
    println!(
        "  mass bounds moved by {:.6} low and {:.6} high, delta was {:.6}",
        low_after[1] - low_before[1],
        high_after[1] - high_before[1],
        seated.delta
    );
    let (roof_low_before, roof_high_before) = unseated.bounds_of(roof);
    let (roof_low_after, roof_high_after) = seated_built.bounds_of(roof);
    println!(
        "  roof rise kept: {:.6} before, {:.6} after",
        roof_high_before[1] - roof_low_before[1],
        roof_high_after[1] - roof_low_after[1]
    );
    println!(
        "  and the solids are still closed: mass loops {}, roof loops {}, both manifold {}",
        seated_built.solid_boundary_loops(mass),
        seated_built.solid_boundary_loops(roof),
        seated_built.solid_is_manifold(mass) && seated_built.solid_is_manifold(roof)
    );

    println!("\n# two masses on one slope are two terraces, not one");
    let mut pair = skeleton.clone();
    let mut second = pair.masses[0].clone();
    let shift = [12.0_f32, 0.0];
    for node in &mut second.outline {
        node.position[0] += shift[0];
        node.position[1] += shift[1];
    }
    pair.masses.push(second);
    let pair_report = seat(&mut pair, slope, 0.0);
    println!(
        "  levels {:?}, and the analytic difference is {:.6}",
        pair_report
            .masses
            .iter()
            .map(|seat| seat.level)
            .collect::<Vec<_>>(),
        0.3 * shift[0]
    );
    println!(
        "  deepest fall on the site: {:.6}, highest level {:.6}",
        pair_report.deepest_fall(),
        pair_report.highest_level().unwrap_or_default()
    );

    println!("\n# an offset lifts the whole site by itself");
    let lifted = seat(&mut skeleton.clone(), slope, 1.5);
    println!(
        "  level {:.6} against the un-offset {:.6}",
        lifted.masses[0].level, seated.level
    );

    println!("\n# seating twice is seating once: a ground that has not moved");
    let mut twice = starter(&plan).skeleton.clone();
    seat(&mut twice, slope, 0.0);
    let once = twice.clone();
    seat(&mut twice, slope, 0.0);
    println!(
        "  the second seat changed nothing: {}",
        twice.masses == once.masses
    );

    println!("\n# a ground that does not answer under the whole outline is refused, not half-used");
    let mut unsure = starter(&plan).skeleton.clone();
    let partly = seat(
        &mut unsure,
        |plan| {
            if plan[0] > 0.0 { f32::NAN } else { 0.0 }
        },
        0.0,
    );
    println!(
        "  a ground unknown over one half seated {} of {} masses, and the document still stands at \
         its own elevations: {}",
        partly.len(),
        unsure.masses.len(),
        unsure.masses[0]
            .outline
            .iter()
            .all(|node| node.elevation == 0.0)
    );
    let mut unknown = starter(&plan).skeleton.clone();
    let nothing = seat(&mut unknown, |_| f32::NAN, 0.0);
    println!(
        "  a ground that answers nothing seated {} masses, and the document is untouched: {}",
        nothing.len(),
        unknown.masses[0]
            .outline
            .iter()
            .all(|node| node.elevation == 0.0)
    );

    println!("\n# the document the seat produced is clean");
    let violations = Building::new(skeleton).validate();
    println!("  violations: {}", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
}
