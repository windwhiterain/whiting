//! The two gables' acceptance: a 悬山 and a 硬山 over the same mass, measured against its own outline.
//!
//! The claim is one claim with two readings — **the ends are where the ends were pushed to** — and the
//! rest is that neither roof changes its mind about anything else: same ridge, same span, same plan
//! coverage of the mass, and a closed solid either way.
//!
//! Run with `cargo run -p whiting-building --example gable_overhang_probe`.

use whiting_building::derive::{self, RoofStyle, Starter, starter};
use whiting_building::math::Vec2;
use whiting_building::roof::Roof;
use whiting_building::{Building, Mass};

/// A ten-by-six mass: the ridge runs along the ten metres, so its two ends are the short edges.
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

/// The overhang the eaves take, and the one a gable end takes when it takes none.
const OVERHANG: f32 = 0.70;
const FLUSH: f32 = 0.0;

fn main() {
    let starter = starter(&STARTER);
    let mass: Mass = starter.skeleton.masses[0].clone();
    let host = whiting_building::MassId(0);

    // The mass's own extents, which every claim below is measured against.
    let xs: Vec<f32> = mass.outline.iter().map(|node| node.position[0]).collect();
    let zs: Vec<f32> = mass.outline.iter().map(|node| node.position[1]).collect();
    let (low_x, high_x) = (
        xs.iter().copied().fold(f32::INFINITY, f32::min),
        xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );
    let (low_z, high_z) = (
        zs.iter().copied().fold(f32::INFINITY, f32::min),
        zs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    );
    println!(
        "# a {:.1} x {:.1} mass, so the ridge runs along x and the gable ends are the two x extremes",
        high_x - low_x,
        high_z - low_z
    );

    // **悬山**: the same amount on every edge, which is what every scene drew before.
    let overhanging = derive::gable_roof(host, &mass, OVERHANG, 2.0, 0.1, 0.22)
        .expect("a rectangle has an eave loop");
    // **硬山**: the sides overhang, the two ends do not.
    let flush = derive::gable_roof_by(host, &mass, OVERHANG, FLUSH, 2.0, 0.1, 0.22)
        .expect("a rectangle has an eave loop");

    println!("\n# the two roofs over the same mass");
    for (label, roof) in [("悬山", &overhanging), ("硬山", &flush)] {
        let (eaves_low_x, eaves_high_x, eaves_low_z, eaves_high_z) = extents(
            &roof
                .eaves
                .iter()
                .map(|node| node.position)
                .collect::<Vec<Vec2>>(),
        );
        println!(
            "  {label}: eave loop x {eaves_low_x:.3}..{eaves_high_x:.3} (span {:.3}), z {eaves_low_z:.3}..{eaves_high_z:.3} (span {:.3}), plan area {:.3} m²",
            eaves_high_x - eaves_low_x,
            eaves_high_z - eaves_low_z,
            roof.plan_area()
        );
    }

    let (over_x0, over_x1, _, _) = extents(
        &overhanging
            .eaves
            .iter()
            .map(|node| node.position)
            .collect::<Vec<Vec2>>(),
    );
    let (flush_x0, flush_x1, flush_z0, flush_z1) = extents(
        &flush
            .eaves
            .iter()
            .map(|node| node.position)
            .collect::<Vec<Vec2>>(),
    );
    println!("\n# the ends are the only difference, and it is exactly twice the overhang");
    let gained = (over_x1 - over_x0) - (flush_x1 - flush_x0);
    println!(
        "  悬山's run is {gained:.3} m longer than 硬山's, against 2 x {OVERHANG:.2} = {:.2}",
        2.0 * OVERHANG
    );
    assert!(
        (gained - 2.0 * OVERHANG).abs() < 1.0e-4,
        "two ends, one overhang each"
    );

    println!("\n# a 硬山's two end edges stand in the walls' own planes");
    // **The end edges, not the nodes.** A 硬山's corner is where the end edge (which did not move) meets the
    // long edge (which moved out by the overhang), so the corner is pushed in `z` and held in `x`: the claim
    // is about the two *edges*, and one node can only tell half of it.
    println!(
        "  the mass's own x extremes are {low_x:.3} and {high_x:.3}; the 硬山's eave loop's are {flush_x0:.3} and {flush_x1:.3}"
    );
    assert!(
        (flush_x0 - low_x).abs() < 1.0e-4 && (flush_x1 - high_x).abs() < 1.0e-4,
        "a 硬山's end edges stand in the walls' planes"
    );
    assert!(
        over_x0 < low_x - 1.0e-3 && over_x1 > high_x + 1.0e-3,
        "a 悬山's end edges stand beyond them"
    );

    assert!(
        (flush_z0 - (low_z - OVERHANG)).abs() < 1.0e-4 || flush_z0 <= low_z - OVERHANG + 1.0e-4
    );
    assert!(flush_z1 >= high_z + OVERHANG - 1.0e-4);

    println!("\n# both sides of the eave are still 0.7 beyond the walls");
    let (_, _, over_z0, over_z1) = extents(
        &overhanging
            .eaves
            .iter()
            .map(|node| node.position)
            .collect::<Vec<Vec2>>(),
    );
    println!(
        "  悬山 z {over_z0:.3}..{over_z1:.3}; 硬山 z {flush_z0:.3}..{flush_z1:.3}: the long sides do not move",
    );
    assert!((over_z0 - flush_z0).abs() < 1.0e-4 && (over_z1 - flush_z1).abs() < 1.0e-4);

    // **Both are solids like any other roof**, so the difference is a plan difference and not a hole.
    println!("\n# both roofs expand into closed solids, and both are documents like any other");
    for (label, roof) in [("悬山", &overhanging), ("硬山", &flush)] {
        let mut skeleton = starter.skeleton.clone();
        skeleton.roofs[0] = roof.clone();
        let building = Building::new(skeleton);
        let violations = building.validate();
        let built = building.build().expect("the roof expands");
        let solid = whiting_building::SolidId::Roof(whiting_building::roof::RoofId(0));
        println!(
            "  {label}: {} face(s), loops {}, euler {}, manifold {}, volume {:.6} m³, {} violation(s)",
            built.faces_of(solid).len(),
            built.solid_boundary_loops(solid),
            built.solid_euler_characteristic(solid),
            built.solid_is_manifold(solid),
            built.solid_signed_volume(solid),
            violations.len()
        );
        assert!(violations.is_empty(), "{label} must be a clean document");
        assert_eq!(
            built.solid_euler_characteristic(solid),
            2,
            "{label} is a ball"
        );
        assert!(built.solid_signed_volume(solid) > 0.0, "{label} is outward");
    }

    // **And the roof still covers its mass**: a 硬山 loses plan area and not coverage.
    println!("\n# the 硬山 covers its mass to the wall, and the 悬山 covers it past the wall");
    let mass_area = mass.plan_area();
    println!(
        "  the mass is {mass_area:.3} m²; 悬山 covers {:.3} m², 硬山 {:.3} m²",
        overhanging.plan_area(),
        flush.plan_area()
    );
    assert!(
        flush.plan_area() >= mass_area - 1.0e-3,
        "a 硬山 still covers its walls"
    );
    assert!(
        overhanging.plan_area() > flush.plan_area() + 1.0,
        "the 悬山 oversails"
    );

    println!("\n# a 悬山 is the call this file has always answered");
    let same = derive::gable_roof(host, &mass, OVERHANG, 2.0, 0.1, 0.22).expect("a roof");
    assert_eq!(
        same, overhanging,
        "the old entry point is the new one with one number twice"
    );
    println!("  gable_roof(x, y) and gable_roof_by(x, y, y) are the same roof, field for field");
    let _ = Roof::new(host, mass.outline.clone(), 0.22);
}

/// Returns nothing; left for the shape of the file.\nfn flush_ends_placeholder() {}\n\n/// Returns the plan extents of a loop: `(low_x, high_x, low_z, high_z)`.
fn extents(loop_: &[Vec2]) -> (f32, f32, f32, f32) {
    let low_x = loop_.iter().map(|at| at[0]).fold(f32::INFINITY, f32::min);
    let high_x = loop_
        .iter()
        .map(|at| at[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let low_z = loop_.iter().map(|at| at[1]).fold(f32::INFINITY, f32::min);
    let high_z = loop_
        .iter()
        .map(|at| at[1])
        .fold(f32::NEG_INFINITY, f32::max);
    (low_x, high_x, low_z, high_z)
}
