//! TEMPORARY. What valence does our own input actually have?
//!
//! The whole "arbitrary topology" question turns on this: if every interior vertex were valence 4,
//! the mesh is a regular quad grid and the per-patch model already covers it. Everything else is
//! outside what that model can express smoothly.

use std::collections::BTreeMap;

use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::topology::VertexId;

/// How many edges meet at each mesh vertex, over a mesh's quads.
fn valence(quads: &[[VertexId; 4]]) -> BTreeMap<usize, usize> {
    let mut count: BTreeMap<VertexId, usize> = BTreeMap::new();
    for corners in quads {
        for slot in whiting_patch::topology::EdgeSlot::ALL {
            let ends = whiting_patch::topology::edge_endpoints(*corners, slot);
            for end in ends {
                *count.entry(end).or_default() += 1;
            }
        }
    }
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for degree in count.values() {
        *histogram.entry(*degree).or_default() += 1;
    }
    histogram
}

fn report(label: &str, quads: &[[VertexId; 4]]) {
    let histogram = valence(quads);
    let vertices: usize = histogram.values().sum();
    let regular = histogram.get(&4).copied().unwrap_or(0);
    println!("\n{label}");
    println!("  quads {}   vertices {vertices}", quads.len());
    print!("  valence histogram ");
    for (degree, count) in &histogram {
        print!("{degree}:{count}  ");
    }
    println!();
    println!(
        "  valence 4 (regular interior): {regular} of {vertices} = {:.1}%",
        100.0 * regular as f64 / vertices as f64
    );
    let irregular: usize = histogram
        .iter()
        .filter(|(degree, _)| **degree != 4)
        .map(|(_, count)| count)
        .sum();
    let distinct = histogram
        .iter()
        .filter(|(degree, _)| **degree != 4)
        .map(|(degree, _)| *degree)
        .collect::<Vec<_>>();
    println!("  valence != 4: {irregular}   distinct valences {distinct:?}");
}

/// How many authored blocks touch each skeleton node, from each block's own four corners.
///
/// A block is not a quad — it holds `top.len() + bottom.len() - 2` faces, two when both runs are
/// two points — so this is the junction count of the coarser candidate front end rather than a
/// valence over a quad mesh.
fn report_blocks(facet: &FacetMesh) {
    let mut degree_of: BTreeMap<u32, usize> = BTreeMap::new();
    let mut quads = 0usize;
    for block in &facet.blocks {
        quads += block.faces.end as usize - block.faces.start as usize;
        let corners = [
            block.top[0],
            *block.top.last().expect("a block's top run is never empty"),
            *block
                .bottom
                .last()
                .expect("a block's bottom run is never empty"),
            block.bottom[0],
        ];
        for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            for end in [corners[a], corners[b]] {
                *degree_of.entry(end).or_default() += 1;
            }
        }
    }
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for degree in degree_of.values() {
        *histogram.entry(*degree).or_default() += 1;
    }
    let nodes: usize = histogram.values().sum();
    println!(
        "\nauthored blocks, junctions counted over each block's four corners\n  faces {quads}   \
         nodes {nodes}"
    );
    print!("  blocks meeting at a node: ");
    for (degree, count) in &histogram {
        print!("{degree}:{count}  ");
    }
    println!();
}

fn main() {
    let facet: FacetMesh = match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the sample massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };
    let FacetMesh { ref faces, .. } = facet;
    println!("source: {} triangles", faces.len());

    // The triangle mesh's own vertex valence, before anything.
    let mut triangle_valence: BTreeMap<u32, usize> = BTreeMap::new();
    for face in faces {
        for index in face.indices {
            *triangle_valence.entry(index).or_default() += 1;
        }
    }
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for degree in triangle_valence.values() {
        *histogram.entry(*degree).or_default() += 1;
    }
    print!("\nsource triangle mesh\n  valence histogram ");
    for (degree, count) in &histogram {
        print!("{degree}:{count}  ");
    }
    println!();
    println!(
        "  distinct valences {:?}",
        histogram.keys().collect::<Vec<_>>()
    );

    let model = patchmesh::from_facet(&facet).expect("the massif converts");
    let quads: Vec<[VertexId; 4]> = model.quads().to_vec();
    report(
        "after Doo-Sabin (what tessellate hands the patch model)",
        &quads,
    );
    report_blocks(&facet);
}
