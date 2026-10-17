//! Measures seating: what a ground function does to a plant's base, and what a floating or sunk base reports.
//!
//! The rows are `.agents/proposals/agentic-vegetation-generation.md`'s P3 item *"seating on a `terrain::HeightField`
//! by the `site::seat` precedent"*, read off placements rather than off the ground's own numbers.
//!
//! ```text
//! cargo run -p whiting-vegetation --example ground_probe
//! ```

use whiting_vegetation::derive::bamboo;
use whiting_vegetation::ground::{footing, on_the_ground, seat};
use whiting_vegetation::plant::Placement;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("a plant is seated", a_plant_is_seated()));
    checks.push(("only the height moves", only_the_height_moves()));
    checks.push((
        "a flat ground changes nothing",
        a_flat_ground_changes_nothing(),
    ));
    checks.push(("a floating base is reported", a_floating_base_is_reported()));
    checks.push(("a sunk base is reported", a_sunk_base_is_reported()));
    checks.push(("a grove is seated together", a_grove_is_seated_together()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<30} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// A swell: the ground a grove of this probe stands on, gentle enough to be a meadow and steep enough to read.
fn swell(plan: [f32; 2]) -> f32 {
    0.28 * (0.9 * plan[0] + 0.4).sin() * (0.7 * plan[1]).cos()
}

/// A ground that is flat at zero, which is every still scene's floor.
fn flat(_plan: [f32; 2]) -> f32 {
    0.0
}

/// A planting at a plan position, turned.
fn planted_at(x: f32, z: f32, turn: f32) -> Placement {
    let (sin, cos) = turn.sin_cos();
    Placement {
        origin: [x, 0.0, z],
        x: [cos, 0.0, -sin],
        y: [0.0, 1.0, 0.0],
        z: [sin, 0.0, cos],
    }
}

/// A plant is seated at the ground's own height under it, and reports itself on the ground.
fn a_plant_is_seated() -> bool {
    let plant = bamboo(7);
    let base = plant
        .plant
        .get(&whiting_vegetation::handle::Handle::root())
        .expect("the trunk resolves")
        .taper_now()
        .at(0.0);
    let under = planted_at(0.62, -0.35, 0.7);
    let seated = seat(under, swell);
    let height = swell([0.62, -0.35]);
    let read = footing(&seated, swell);
    println!("\n# a seated plant");
    println!(
        "  the ground under (0.62, -0.35) is {height:.6} high: the base went {:.6} → {:.6}, and its footing reads {read:.3e}",
        under.origin[1], seated.origin[1]
    );
    println!(
        "  the control is the base's own radius: a plant of a {base:.6} m base is on the ground within that"
    );
    (seated.origin[1] - height).abs() < 1e-6
        && read.abs() < 1e-6
        && on_the_ground(&seated, base, swell)
}

/// Seating moves the height and nothing else: the plan position and the turn are the caller's.
fn only_the_height_moves() -> bool {
    let under = planted_at(-0.4, 0.9, 2.1);
    let seated = seat(under, swell);
    let plan = [
        seated.origin[0] - under.origin[0],
        seated.origin[2] - under.origin[2],
    ];
    let turn = [
        seated.x[0] - under.x[0],
        seated.x[2] - under.x[2],
        seated.z[0] - under.z[0],
        seated.z[2] - under.z[2],
    ];
    println!("\n# what seating may move");
    println!(
        "  the plan position moved {:.3e} across and {:.3e} deep, and the frame's own axes moved {:.3e} at most",
        plan[0].abs(),
        plan[1].abs(),
        turn.iter()
            .fold(0.0f32, |worst, step| worst.max(step.abs()))
    );
    println!(
        "  the control is the height: a seat that also turned a plant would make two neighbours' seam unpredictable"
    );
    plan[0] == 0.0 && plan[1] == 0.0 && turn.iter().all(|step| *step == 0.0)
}

/// A flat ground is the identity, which is what lets a scene add a ground and keep its picture.
fn a_flat_ground_changes_nothing() -> bool {
    let under = planted_at(1.1, -0.8, 0.3);
    let seated = seat(under, flat);
    println!("\n# a flat ground");
    println!(
        "  a planting at y = {:.6} on a ground at zero comes back at y = {:.6}",
        under.origin[1], seated.origin[1]
    );
    println!(
        "  the control is the whole placement: a seat that wrote a default anywhere would move a scene that never asked for one"
    );
    seated == under
}

/// A base hanging above the ground is reported with its own distance, and is not on the ground.
fn a_floating_base_is_reported() -> bool {
    let plant = bamboo(7);
    let base = plant
        .plant
        .get(&whiting_vegetation::handle::Handle::root())
        .expect("the trunk resolves")
        .taper_now()
        .at(0.0);
    let mut lifted = planted_at(0.2, 0.3, 0.0);
    lifted.origin[1] = swell([0.2, 0.3]) + 0.2;
    let read = footing(&lifted, swell);
    let grounded = on_the_ground(&lifted, base, swell);
    println!("\n# a base that floats");
    println!(
        "  it hangs {read:.6} above the surface, and a base of {base:.6} m is on the ground: {grounded}"
    );
    println!(
        "  the control is the radius: a tolerance of a metre would call this planted, and a tolerance of zero would call every culm floating"
    );
    (read - 0.2).abs() < 1e-6 && !grounded
}

/// A base pushed into the ground is reported too: the defect has two signs and one report.
fn a_sunk_base_is_reported() -> bool {
    let plant = bamboo(7);
    let base = plant
        .plant
        .get(&whiting_vegetation::handle::Handle::root())
        .expect("the trunk resolves")
        .taper_now()
        .at(0.0);
    let mut sunk = planted_at(-0.7, -0.2, 0.0);
    sunk.origin[1] = swell([-0.7, -0.2]) - 0.15;
    let read = footing(&sunk, swell);
    let grounded = on_the_ground(&sunk, base, swell);
    println!("\n# a base that is sunk");
    println!("  it stands {read:.6} below the surface, and it is on the ground: {grounded}");
    println!(
        "  the control is the sign: a report that only measured height above the surface would call a buried root planted"
    );
    (read + 0.15).abs() < 1e-6 && read < 0.0 && !grounded
}

/// A whole grove is seated by one call per plant and every base lands inside its own radius.
fn a_grove_is_seated_together() -> bool {
    let places = [
        (-1.2f32, 0.4f32),
        (-0.5, -0.5),
        (0.1, 0.3),
        (0.7, -0.3),
        (1.3, 0.5),
    ];
    let mut worst = 0.0f32;
    let mut grounded = 0;
    for (index, (x, z)) in places.iter().enumerate() {
        let plant = bamboo(20 + index as u64);
        let base = plant
            .plant
            .get(&whiting_vegetation::handle::Handle::root())
            .expect("the trunk resolves")
            .taper_now()
            .at(0.0);
        let seated = seat(planted_at(*x, *z, index as f32 * 1.3), swell);
        worst = worst.max(footing(&seated, swell).abs());
        if on_the_ground(&seated, base, swell) {
            grounded += 1;
        }
    }
    println!("\n# five plants on one ground");
    println!(
        "  {grounded} of {} are on the ground, and the worst footing is {worst:.3e}",
        places.len()
    );
    println!(
        "  the control is the count: a grove seated one plant at a time is five chances to leave one hanging"
    );
    grounded == places.len() && worst < 1e-6
}
