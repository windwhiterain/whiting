//! Reports what a **mass of four sections** expands to, and where it fails to close.
//!
//! ```text
//! cargo run -p whiting-building --example band_probe
//! ```
//!
//! This is a report and not a test: it prints one hand-authored mass whose sections are a 骑楼's — the foot's plan,
//! the same plan at the floor, a plan whose front has moved out at the floor's own level (the soffit), and that
//! projecting plan up to the eaves — together with the boundary edges the expansion leaves unclaimed. The question it
//! answers is whether the band walk can close such a solid **at all**, in the smallest document that has one.

use std::collections::HashMap;

use whiting_building::Building;
use whiting_building::expand::build;
use whiting_building::mass::{Level, Mass, MassId, PlanNode};
use whiting_building::mesh::BuildingMesh;
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
    let building = Building::new(skeleton.clone());

    let mesh = build(&skeleton);
    match &mesh {
        Ok(built) => {
            println!("expands: {} faces", built.faces.len());
            report_unclosed(built);
        }
        Err(error) => println!("refused: {error:?}"),
    }
    let violations = building.validate();
    println!("violations: {}", violations.len());
    for violation in &violations {
        println!("  {violation:?}");
        if let (whiting_building::validate::Violation::FaceNotPlanar { face, .. }, Ok(built)) =
            (violation, &mesh)
        {
            let held = built.faces[*face];
            println!("    element {:?}", held.element);
            for index in held.indices {
                println!("      {:?}", built.points[index as usize]);
            }
        }
    }
    let _ = MassId(0);
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
            if open <= 12 {
                let at = |index: u32| mesh.points[index as usize];
                println!("  open {edge:?}: {:?} -> {:?}", at(edge.0), at(edge.1));
            }
        }
    }
    println!("open boundary edges: {open}");
}
