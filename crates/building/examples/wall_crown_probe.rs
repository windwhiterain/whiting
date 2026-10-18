//! The wall crown's acceptance: a gable wall that rises to its roof, measured against arithmetic.
//!
//! A mass's wall top is level, and a gable is not: the 山墙 under a pitched roof is a **triangle**, and
//! a wall that stops at one elevation shows open air under the ridge. A crown is one elevation per
//! outline node, so a gable is authored by splitting the gable edge at the point under the ridge and
//! raising that node — and what must then hold is that the ring is still one closed solid, that each
//! strip of wall top over each edge is still **planar**, and that the shape the ring closes is the
//! triangle the crown names.
//!
//! Run with `cargo run -p whiting-building --example wall_crown_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::mesh::Element;
use whiting_building::ops;
use whiting_building::{Building, BuildingSkeleton, MassId, SolidId};

/// The house: ten by six, one storey, a gable over it. The ridge runs along `x`, so the two gable
/// ends are the edges at `x = ±5`.
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

fn main() {
    let mut building = starter(&STARTER);
    let mass = MassId(0);
    let flat = Building::new(building.skeleton.clone()).validate();
    let apex = apex_of(&building.skeleton);

    println!(
        "# a {:.0} x {:.0} gable whose wall top is {:.4}, under a roof whose ridge is at {:.4}",
        STARTER.width,
        STARTER.depth,
        building.skeleton.masses[0].top,
        apex + STARTER.roof_thickness
    );
    println!(
        "  the flat-topped document reports {} violation(s)",
        flat.len()
    );

    // The two gable edges, split under the ridge, crown the node that was added. A split is named by
    // an arc-length station of the **closed** loop, so the two stations are computed before either
    // insertion and applied from the far end, where an insert cannot move the other.
    let stations = gable_stations(&building.skeleton.masses[0], STARTER.width * 0.5);
    println!(
        "  the gable ends are at stations {stations:?} of the closed loop, which has {} node(s)",
        building.skeleton.masses[0].outline.len()
    );
    for station in stations.iter().copied() {
        ops::split_outline(&mut building.skeleton.masses[0], station)
            .expect("the gable edge splits at its middle");
    }
    let nodes = building.skeleton.masses[0].outline.len();
    let mut crowns = vec![building.skeleton.masses[0].top; nodes];
    for (node, crown) in crowns.iter_mut().enumerate() {
        let at = building.skeleton.masses[0].outline[node].position;
        // The middle of each gable end is the one plan point under the ridge.
        if at[1].abs() < 1.0e-4 && (at[0].abs() - STARTER.width * 0.5).abs() < 1.0e-4 {
            *crown = apex;
        }
    }
    building.skeleton.masses[0].crowns = crowns.clone();
    println!(
        "  {nodes} nodes, crowns {:?}",
        crowns
            .iter()
            .map(|top| (*top * 100.0).round() / 100.0)
            .collect::<Vec<_>>()
    );

    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("the crowned mass expands");
    let violations = Building::new(building.skeleton.clone()).validate();
    for violation in &violations {
        println!("    violation: {violation:?}");
        if let whiting_building::Violation::FaceNotPlanar { face, .. } = violation {
            println!(
                "      that face's element is {:?}",
                built.faces[*face].element
            );
        }
    }

    println!("\n# the ring is still one closed solid");
    let solid = SolidId::Mass(mass);
    println!(
        "  {} faces, loops {}, euler {} (a ring, not a box), manifold {}, volume {:.6} m³, {} \
         violation(s)",
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
        "a crowned mass is a document like any other"
    );

    println!("\n# every element of wall top is still a plane, and the tall ones lean");
    let mut worst = 0.0_f32;
    let mut tilted = 0;
    let mut halves = 0;
    let mut elements: Vec<(Element, Vec<usize>)> = Vec::new();
    for (index, face) in built.faces.iter().enumerate() {
        if !matches!(
            face.element,
            Element::Mass(_, whiting_building::mesh::MassSurface::WallTop { .. })
        ) {
            continue;
        }
        match elements
            .iter_mut()
            .find(|(element, _)| *element == face.element)
        {
            Some((_, faces)) => faces.push(index),
            None => elements.push((face.element, vec![index])),
        }
    }
    for (element, faces) in &elements {
        if let Element::Mass(_, whiting_building::mesh::MassSurface::WallTop { half, .. }) = element
        {
            if *half > 0 {
                halves += 1;
            }
        }
        let departure = departure_of(&built, faces);
        let normal = built.normal(&built.faces[faces[0]]);
        if normal[1].abs() < 0.999 {
            tilted += 1;
        }
        worst = worst.max(departure);
        println!(
            "  {element:?}: {} triangle(s), departure {:.2e} m, normal {:.4} up",
            faces.len(),
            departure,
            normal[1]
        );
    }
    println!(
        "  {} element(s) of wall top, {halves} of them the second half of a **warped** strip",
        elements.len()
    );
    assert!(
        worst < 1.0e-5,
        "an element of wall top must be planar: {worst}"
    );
    assert!(tilted > 0, "a crowned strip leans, and none of these did");
    assert!(
        halves > 0,
        "a crowned strip at a corner warps, so at least one must have been split"
    );

    println!("\n# the gable is a triangle, not a box");
    let (low, high) = built.bounds_of(solid);
    println!(
        "  the mass now reaches y {:.4} .. {:.4}, which is the level top plus {:.4}",
        low[1],
        high[1],
        high[1] - STARTER.height
    );
    assert!(
        (high[1] - apex).abs() < 1.0e-3,
        "the apex is where the crown put it"
    );

    println!("\n# a crown that is not a crown is refused by name");
    for (label, crowns, expect) in [
        (
            "one elevation short",
            vec![STARTER.height; 3],
            "CrownCountMismatch",
        ),
        ("a crown under its base", vec![-9.0; 5], "CrownBelowBase"),
    ] {
        let mut broken = starter(&STARTER);
        broken.skeleton.masses[0].crowns = crowns;
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
        let named = found.iter().any(|name| name == expect);
        println!("  {label}: {found:?} (named {expect}: {named})");
        assert!(named, "{label} must be reported as {expect}");
    }

    // A ritual on the crowned edge is a bay model that has no rectangle to divide.
    let mut gridded = starter(&STARTER);
    gridded.skeleton.masses[0].crowns = vec![STARTER.height; 4];
    let face = whiting_building::grid::FaceRef::new(mass, 0);
    let _ = ops::split_face(
        &mut gridded.skeleton,
        face,
        &whiting_building::grid::Rhythm::Even { count: 2 },
        &whiting_building::grid::Rhythm::Even { count: 1 },
    );
    let found: Vec<String> = Building::new(gridded.skeleton.clone())
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
    println!(
        "  a rhythm on a crowned face: {found:?} (named GridOnACrownedFace: {})",
        found.iter().any(|name| name == "GridOnACrownedFace")
    );
}

/// Returns the elevation the roof's underside reaches at the ridge: the crest less the thickness.
fn apex_of(skeleton: &BuildingSkeleton) -> f32 {
    let roof = &skeleton.roofs[0];
    let crest = roof
        .crest
        .iter()
        .map(|point| point.elevation)
        .fold(f32::NEG_INFINITY, f32::max);
    crest - roof.thickness
}

/// Returns the arc-length stations at which the two **gable ends** are widest, in descending order.
///
/// A gable end is an edge whose two ends share an `x` of magnitude `half_width` — the walls a pitched
/// roof's rake stands over. Its middle is where the ridge meets it, which is the node the crown needs.
fn gable_stations(mass: &whiting_building::Mass, half_width: f32) -> Vec<f32> {
    let count = mass.outline.len();
    let lengths: Vec<f32> = (0..count)
        .map(|edge| {
            let from = mass.outline[edge].position;
            let to = mass.outline[(edge + 1) % count].position;
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt()
        })
        .collect();
    let perimeter: f32 = lengths.iter().sum();
    let mut stations = Vec::new();
    let mut walked = 0.0_f32;
    for (edge, length) in lengths.iter().enumerate() {
        let from = mass.outline[edge].position;
        let to = mass.outline[(edge + 1) % count].position;
        let gable = (from[0] - to[0]).abs() < 1.0e-4 && (from[0].abs() - half_width).abs() < 1.0e-4;
        if gable {
            stations.push((walked + length * 0.5) / perimeter);
        }
        walked += length;
    }
    stations.sort_by(|left, right| right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal));
    stations
}

/// Returns how far the farthest corner of these faces stands from the plane the first one lies in.
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
