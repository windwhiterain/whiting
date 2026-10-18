//! The roof profile's acceptance: a 举架 slope, measured strip by strip against arithmetic.
//!
//! A profile is a list of [`Step`](whiting_building::Step)s — a fraction of the slope's plan run and a
//! rise above the row's own eave — and what it must produce is a **chain of planes**, not a curve: the
//! surface bends at every purlin, so every strip is its own planar face, its own shading group, and
//! the crease between two strips is a measurable angle rather than a smoothed normal.
//!
//! What this probe checks, and why each one is a claim the design makes:
//!
//! - **every strip is planar**, to the crate's own tolerance, because a strip that is not is invisible
//!   to the closure checks and visible in the frame;
//! - **the strips tile the eave loop**, so the skin's plan area is the roof's own plan area and no gap
//!   opened where the profile was cut in;
//! - **the slope steepens toward the ridge**, which is the whole of 举架: four 步架 of equal plan width
//!   and rising coefficients;
//! - **the purlin a step names stands where the step says**, at the eave's own elevation plus the rise;
//! - **the solid still closes**, because a roof remains a solid slab of skin, soffit and fascia.
//!
//! Run with `cargo run -p whiting-building --example roof_profile_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::mesh::Element;
use whiting_building::roof::{Roof, RoofId, RoofPlane, Step};
use whiting_building::{Building, SolidId};

/// The house the roof stands on: ten by six, one storey, a gable over it.
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

/// The profile: four 步架 of equal plan width and the 举架 coefficients 五举, 六举, 七五举, and the
/// ridge's own step above them.
///
/// Each slope runs `depth / 2 + overhang` = 3.6 m in plan, so a 步架 is 0.9 m, and the four steps take
/// the coefficients **五举 0.50, 六举 0.60, 七五举 0.75 and 八举 0.80** — the ladder steepens all the way
/// to the ridge, which is the whole of 举架. The house's own ridge is therefore placed by the
/// profile and not the other way round: `0.9 × (0.5 + 0.6 + 0.75 + 0.8) = 2.385 m`, and a starter
/// whose `rise` disagreed with that sum would put the ridge at a height no purlin leads to.
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

fn main() {
    let mut building = starter(&STARTER);
    let before = Building::new(building.skeleton.clone())
        .build()
        .expect("the flat-roofed house expands");
    building.skeleton.roofs[0] = profiled(building.skeleton.roofs[0].clone());
    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("the profiled house expands");
    let roof = &building.skeleton.roofs[0];
    let eave = roof.eaves[0].elevation;

    println!(
        "# a {:.0} x {:.0} gable with a {}-step profile, {:.2} m of plan a 步架",
        STARTER.width,
        STARTER.depth,
        PROFILE.len(),
        BEAM
    );
    println!(
        "  the eave stands at {eave:.4} and the crest at {:.4}, so the slope's own rise is {:.4} m",
        roof.crest[0].elevation,
        roof.crest[0].elevation - eave
    );

    // The ladder, read back off the profile rather than off a diagram: each step's own slope, and the
    // steps of the ladder it belongs to.
    println!("\n# the ladder, steepening toward the ridge");
    let mut previous_fraction = 0.0;
    let mut previous_rise = 0.0;
    for (index, step) in PROFILE.iter().enumerate() {
        let width = (step.fraction - previous_fraction) * slope_run(roof);
        let rise = step.rise - previous_rise;
        println!(
            "  step {index}: plan {width:.4} m, rise {rise:.4} m, coefficient {:.3} 举, angle {:.2}°",
            rise / width,
            (rise / width).atan().to_degrees()
        );
        previous_fraction = step.fraction;
        previous_rise = step.rise;
    }
    let top = (roof.crest[0].elevation - eave) - previous_rise;
    let top_width = (1.0 - previous_fraction) * slope_run(roof);
    println!(
        "  the ridge's own step: plan {top_width:.4} m, rise {top:.4} m, coefficient {:.3} 举, angle \
         {:.2}°",
        top / top_width,
        (top / top_width).atan().to_degrees()
    );

    // Every strip is one element, so grouping by element is grouping by strip.
    println!("\n# every strip is one planar face and they tile the loop");
    let mut worst = 0.0_f32;
    let mut strips = 0;
    for faces in by_element(&built) {
        let departure = departure_of(&built, &faces);
        worst = worst.max(departure);
        strips += 1;
    }
    let skin: Vec<usize> = (0..built.faces.len())
        .filter(|face| {
            matches!(
                built.faces[*face].element,
                Element::Roof(_, whiting_building::mesh::RoofSurface::Skin { .. })
            )
        })
        .collect();
    let before_skin: Vec<usize> = (0..before.faces.len())
        .filter(|face| {
            matches!(
                before.faces[*face].element,
                Element::Roof(_, whiting_building::mesh::RoofSurface::Skin { .. })
            )
        })
        .collect();
    println!(
        "  {strips} strip(s) of roof, {} skin triangle(s) against {} for one straight slope, and the \
         largest departure from a plane is {:.2e} m",
        skin.len(),
        before_skin.len(),
        worst
    );
    println!(
        "  the skin's plan area is {:.6} m² against the eave loop's {:.6} m², a gap of {:.2e}",
        built.plan_area_of(&skin),
        roof.plan_area(),
        (built.plan_area_of(&skin).abs() - roof.plan_area().abs()).abs()
    );
    assert!(worst < 1.0e-5, "a strip of a roof must be planar: {worst}");
    assert!(
        (built.plan_area_of(&skin).abs() - roof.plan_area().abs()).abs() < 1.0e-3,
        "the strips must tile the loop"
    );

    // The purlin a step names is a place on the row, and its elevation is the eave's plus the rise.
    println!("\n# the purlins stand where the steps name them");
    let mut on_the_ladder = 0;
    for point in &built.points {
        for step in PROFILE {
            if (point[1] - (eave + step.rise)).abs() < 1.0e-4 {
                on_the_ladder += 1;
            }
        }
    }
    println!(
        "  {on_the_ladder} mesh point(s) stand at an eave plus a step's rise, of {:.1} m and {:.1} m \
         for the two purlins the middle steps name",
        eave + PROFILE[0].rise,
        eave + PROFILE[1].rise
    );

    println!("\n# the solid still closes, and it is still a solid");
    let solid = SolidId::Roof(RoofId(0));
    let violations = Building::new(building.skeleton.clone()).validate();
    println!(
        "  {} faces, loops {}, euler {}, manifold {}, volume {:.6} m³, {} violation(s)",
        built.faces_of(solid).len(),
        built.solid_boundary_loops(solid),
        built.solid_euler_characteristic(solid),
        built.solid_is_manifold(solid),
        built.solid_signed_volume(solid),
        violations.len()
    );
    assert_eq!(
        violations.len(),
        0,
        "a profiled roof is a document like any other"
    );

    println!("\n# a profile that is not a profile is refused by name");
    for (label, profile) in [
        (
            "out of order",
            vec![
                Step {
                    fraction: 0.5,
                    rise: 0.3,
                },
                Step {
                    fraction: 0.25,
                    rise: 0.6,
                },
            ],
        ),
        (
            "above the crest",
            vec![Step {
                fraction: 0.5,
                rise: 9.0,
            }],
        ),
    ] {
        let mut broken = starter(&STARTER);
        let mut plane: RoofPlane = broken.skeleton.roofs[0].planes[0].clone();
        plane = plane.with_profile(profile);
        broken.skeleton.roofs[0].planes[0] = plane;
        let faults = Building::new(broken.skeleton.clone()).validate();
        let expanded = Building::new(broken.skeleton.clone()).build();
        println!(
            "  {label}: validate says {faults:?}, and build says {:?}",
            expanded.err().map(|problems| problems.problems.len())
        );
        assert!(!faults.is_empty(), "{label} must be reported");
    }
}

/// Returns the roof with the profile, on every plane.
fn profiled(mut roof: Roof) -> Roof {
    for plane in &mut roof.planes {
        plane.profile = PROFILE.to_vec();
    }
    roof
}

/// Returns the plan run one slope covers, from its eave to its crest.
fn slope_run(roof: &Roof) -> f32 {
    let eave = roof.eaves[0];
    let crest = roof.crest[0];
    let along = [
        crest.position[0] - eave.position[0],
        crest.position[1] - eave.position[1],
    ];
    (along[0] * along[0] + along[1] * along[1]).sqrt()
}

/// Returns one list of face indices per roof element, which for a profiled plane is one per strip.
fn by_element(built: &whiting_building::mesh::BuildingMesh) -> Vec<Vec<usize>> {
    let mut groups: Vec<(Element, Vec<usize>)> = Vec::new();
    for (index, face) in built.faces.iter().enumerate() {
        match groups
            .iter_mut()
            .find(|(element, _)| *element == face.element)
        {
            Some((_, faces)) => faces.push(index),
            None => groups.push((face.element, vec![index])),
        }
    }
    groups
        .into_iter()
        .filter(|(element, _)| {
            matches!(
                element,
                Element::Roof(_, whiting_building::mesh::RoofSurface::Skin { .. })
            )
        })
        .map(|(_, faces)| faces)
        .collect()
}

/// Returns how far the farthest corner of these faces stands from the plane they nearly lie in.
fn departure_of(built: &whiting_building::mesh::BuildingMesh, faces: &[usize]) -> f32 {
    let first = &built.faces[faces[0]];
    let corners = built.triangle(first);
    let normal = built.normal(first);
    let mut worst = 0.0_f32;
    for face in faces {
        for corner in built.triangle(&built.faces[*face]) {
            let mut offset = [0.0_f64; 3];
            for axis in 0..3 {
                offset[axis] = f64::from(corner[axis]) - f64::from(corners[0][axis]);
            }
            let along = offset[0] * f64::from(normal[0])
                + offset[1] * f64::from(normal[1])
                + offset[2] * f64::from(normal[2]);
            worst = worst.max(along.abs() as f32);
        }
    }
    worst
}
