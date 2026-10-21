//! Reports what a **mass of four sections** expands to — with and without openings — and where it fails to close.
//!
//! ```text
//! cargo run -p whiting-building --example band_probe            # the plain mass
//! cargo run -p whiting-building --example band_probe -- cut     # the same mass with two holes in it
//! ```
//!
//! This is a report and not a test. The mass's sections are a 骑楼's: the foot's plan, the same plan at the floor, a
//! plan whose front has moved out at the floor's own level (the soffit), and that projecting plan up to the eaves. It
//! prints three things about the solid it becomes:
//!
//! - **the unbalanced edges**, counted apart as holes (one claim, no partner) and doubled faces (two claims the same
//!   way round, no partner either), because the two demand different fixes;
//! - **the planarity of every element group**, measured here rather than read from the validator, because a group is
//!   what an element names and the departure that matters is across all of its faces;
//! - **the seams**, which is what the unbalanced edges have in common: the elevation they sit at.
//!
//! `cut` opens a shopfront in the foot's band and a window in the projecting one — the two holes a 骑楼 has, in the two
//! bands a mass of four sections puts them in.

use std::collections::HashMap;

use whiting_building::expand::build;
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::mass::{Level, Mass, PlanNode};
use whiting_building::mesh::{BuildingMesh, Element};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::skeleton::BuildingSkeleton;

const WIDTH: f32 = 6.0;
const DEPTH: f32 = 5.0;
const FLOOR: f32 = 3.0;
const SOFFIT: f32 = 0.16;
const HEIGHT: f32 = 6.2;
const PROJECT: f32 = 2.2;

fn main() {
    let cut = std::env::args().any(|argument| argument == "cut");
    let plan = [
        [-WIDTH * 0.5, -DEPTH * 0.5],
        [WIDTH * 0.5, -DEPTH * 0.5],
        [WIDTH * 0.5, DEPTH * 0.5],
        [-WIDTH * 0.5, DEPTH * 0.5],
    ];
    // The front is the `+z` edge, so its two corners are the ones that move.
    let projected: Vec<[f32; 2]> = plan
        .iter()
        .map(|at| match at[1] > 0.0 {
            true => [at[0], at[1] + PROJECT],
            false => *at,
        })
        .collect();
    let level = |plan: &[[f32; 2]], elevation: f32| {
        Level::flat(
            &plan
                .iter()
                .map(|at| PlanNode::new(*at, 0.0))
                .collect::<Vec<_>>(),
            elevation,
        )
    };
    let mut mass = Mass::of_sections(vec![
        level(&plan, 0.0),
        level(&plan, FLOOR),
        level(&projected, FLOOR + SOFFIT),
        level(&projected, HEIGHT),
    ]);
    mass.wall = 0.4;
    mass.slab = 0.35;
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    skeleton.masses.push(mass);
    if cut {
        cut_the_front(&mut skeleton);
    }

    println!("mode: {}", if cut { "cut" } else { "plain" });
    match build(&skeleton) {
        Ok(built) => {
            println!("expands: {} faces", built.faces.len());
            report_unbalanced(&built);
            report_seams(&built);
            report_planarity(&built);
        }
        Err(error) => println!("refused: {error:?}"),
    }
    let violations = whiting_building::Building::new(skeleton.clone()).validate();
    println!("violations: {}", violations.len());
    for violation in violations.iter().take(6) {
        println!("  {violation:?}");
    }
}

/// Cuts the two holes a 骑楼 has: a shopfront in the foot's band, a window in the projecting one.
fn cut_the_front(skeleton: &mut BuildingSkeleton) {
    // The front is the edge whose two nodes are the ones the projection moved, which is `+z`.
    let mass = &skeleton.masses[0];
    let Some(edge) = mass
        .outline()
        .iter()
        .position(|node| node.position[1] > DEPTH * 0.5 - 1.0e-3)
    else {
        println!("  no front edge found");
        return;
    };
    let face = FaceRef::new(whiting_building::MassId(0), edge);
    let ground = face;
    let upper = face.in_band(2);
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 1 };
    if ops::split_face(skeleton, ground, &columns, &rows).is_none() {
        println!("  the foot's face took no rhythm");
        return;
    }
    if ops::split_face(skeleton, upper, &columns, &rows).is_none() {
        println!("  the projecting band's face took no rhythm");
        return;
    }
    let _ = ops::add_opening(
        skeleton,
        Opening::new(ground, 0, 1, Inset::new(0.45, 0.45, 0.0, FLOOR - 2.4)),
    );
    let _ = ops::add_opening(
        skeleton,
        Opening::new(
            upper,
            0,
            1,
            Inset::new(0.40, 0.40, 0.95, (HEIGHT - FLOOR - SOFFIT) - 1.45),
        ),
    );
}

/// Prints the unbalanced edges: a hole has one claim and no partner, a doubled face two claims and no partner either.
fn report_unbalanced(mesh: &BuildingMesh) {
    let mut used: HashMap<(u32, u32), usize> = HashMap::new();
    for face in &mesh.faces {
        for edge in [
            (face.indices[0], face.indices[1]),
            (face.indices[1], face.indices[2]),
            (face.indices[2], face.indices[0]),
        ] {
            *used.entry(edge).or_insert(0) += 1;
        }
    }
    let (mut holes, mut doubled) = (0, 0);
    for (edge, count) in &used {
        let reverse = used.get(&(edge.1, edge.0)).copied().unwrap_or(0);
        if *count <= reverse {
            continue;
        }
        let at = |index: u32| mesh.points[index as usize];
        match *count {
            1 => {
                holes += 1;
                if holes <= 6 {
                    println!("  hole {edge:?}: {:?} -> {:?}", at(edge.0), at(edge.1));
                }
            }
            _ => {
                doubled += 1;
                if doubled <= 6 {
                    println!(
                        "  doubled {edge:?} x{count}: {:?} -> {:?}",
                        at(edge.0),
                        at(edge.1)
                    );
                }
            }
        }
    }
    println!(
        "unbalanced edges: {} (holes {holes}, doubled {doubled})",
        holes + doubled
    );
}

/// Prints what the unbalanced edges have in common: the elevation each sits at.
fn report_seams(mesh: &BuildingMesh) {
    let mut used: HashMap<(u32, u32), usize> = HashMap::new();
    for face in &mesh.faces {
        for edge in [
            (face.indices[0], face.indices[1]),
            (face.indices[1], face.indices[2]),
            (face.indices[2], face.indices[0]),
        ] {
            *used.entry(edge).or_insert(0) += 1;
        }
    }
    let mut heights: HashMap<i32, usize> = HashMap::new();
    for (edge, count) in &used {
        if *count > used.get(&(edge.1, edge.0)).copied().unwrap_or(0) {
            let at = mesh.points[edge.0 as usize];
            *heights.entry((at[1] * 100.0).round() as i32).or_insert(0) += 1;
        }
    }
    let mut seams: Vec<(i32, usize)> = heights.into_iter().collect();
    seams.sort();
    for (hundredths, count) in seams {
        println!(
            "  seam at y {:.2}: {count} unbalanced",
            hundredths as f32 / 100.0
        );
    }
}

/// Prints, for every element group, how far its own points leave the plane of its first triangle.
fn report_planarity(mesh: &BuildingMesh) {
    let mut groups: HashMap<Element, Vec<[f32; 3]>> = HashMap::new();
    for face in &mesh.faces {
        let corners = face.indices.map(|index| mesh.points[index as usize]);
        groups.entry(face.element).or_default().extend(corners);
    }
    let mut worst: Vec<(f32, Element)> = groups
        .iter()
        .map(|(element, points)| (departure(points), *element))
        .collect();
    worst.sort_by(|one, two| two.0.total_cmp(&one.0));
    println!("element groups: {}", worst.len());
    for (departure, element) in worst.iter().take(3) {
        println!("  {departure:.4}  {element:?}");
    }
}

/// Returns how far a group of points leaves the plane through its first three, in metres.
fn departure(points: &[[f32; 3]]) -> f32 {
    if points.len() < 4 {
        return 0.0;
    }
    let [first, second, third] = [points[0], points[1], points[2]];
    let along = [
        second[0] - first[0],
        second[1] - first[1],
        second[2] - first[2],
    ];
    let across = [
        third[0] - first[0],
        third[1] - first[1],
        third[2] - first[2],
    ];
    let normal = [
        along[1] * across[2] - along[2] * across[1],
        along[2] * across[0] - along[0] * across[2],
        along[0] * across[1] - along[1] * across[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length <= 1.0e-9 {
        return 0.0;
    }
    points
        .iter()
        .map(|at| {
            let offset = [at[0] - first[0], at[1] - first[1], at[2] - first[2]];
            (offset[0] * normal[0] + offset[1] * normal[1] + offset[2] * normal[2]).abs() / length
        })
        .fold(0.0_f32, f32::max)
}
