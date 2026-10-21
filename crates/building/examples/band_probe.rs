//! Reports what a **mass of four sections** expands to, and which group of faces is not one plane.
//!
//! ```text
//! cargo run -p whiting-building --example band_probe
//! ```
//!
//! This is a report and not a test. It builds one mass whose sections are a 骑楼's — the foot's plan, the same plan at
//! the floor, a plan whose front has moved out at the floor's own level (the soffit), and that projecting plan up to
//! the eaves — and prints three things about the solid it becomes:
//!
//! - **the boundary edges nothing claims twice**, which is what an open solid is made of;
//! - **the planarity of every element group**, measured here rather than read from the validator, because a group is
//!   what an element names and the departure that matters is across all of its faces;
//! - **the group with the greatest departure and every corner of it**, so the reading is a shape and not an index.

use std::collections::HashMap;

use whiting_building::expand::build;
use whiting_building::mass::{Level, Mass, PlanNode};
use whiting_building::mesh::{BuildingMesh, Element};
use whiting_building::skeleton::BuildingSkeleton;

const WIDTH: f32 = 6.0;
const DEPTH: f32 = 5.0;
const FLOOR: f32 = 3.0;
const SOFFIT: f32 = 0.16;
const HEIGHT: f32 = 6.2;
const PROJECT: f32 = 2.2;

fn main() {
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

    match build(&skeleton) {
        Ok(built) => {
            println!("expands: {} faces", built.faces.len());
            report_unclosed(&built);
            report_planarity(&built);
            report_seams(&built);
        }
        Err(error) => println!("refused: {error:?}"),
    }
}

/// Prints the boundary edges of a mesh that nothing claims twice, which is what an open solid is made of.
fn report_unclosed(mesh: &BuildingMesh) {
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
    let mut open = 0;
    for (edge, count) in &used {
        if *count > used.get(&(edge.1, edge.0)).copied().unwrap_or(0) {
            open += 1;
            if open <= 8 {
                let at = |index: u32| mesh.points[index as usize];
                println!("  open {edge:?}: {:?} -> {:?}", at(edge.0), at(edge.1));
            }
        }
    }
    println!("open boundary edges: {open}");
}

/// Prints what the open boundary edges have in common: the elevations they sit at, and how many there are.
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
            "  seam at y {:.2}: {count} open edges",
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
    let mut worst: Vec<(f32, Element, Vec<[f32; 3]>)> = groups
        .into_iter()
        .map(|(element, points)| (departure(&points), element, points))
        .collect();
    worst.sort_by(|one, two| two.0.total_cmp(&one.0));
    println!("element groups: {}", worst.len());
    for (departure, element, _) in worst.iter().take(3) {
        println!("  {departure:.4}  {element:?}");
        for face in &mesh.faces {
            if face.element != *element {
                continue;
            }
            let corners: Vec<[f32; 3]> = face
                .indices
                .iter()
                .map(|at| mesh.points[*at as usize])
                .collect();
            println!("      {corners:?}");
        }
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
