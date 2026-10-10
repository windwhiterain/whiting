//! What the per-patch model does when every junction is as regular as the model can express.
//!
//! Run with `cargo run -p whiting-patch --example junction_probe`. A diagnostic, not a test.
//!
//! The question is separable into three places a seam can fail, and they are measured apart here:
//!
//! 1. **The middle of an edge**, where the mirror invariant applies and nothing contests it.
//! 2. **A valence-4 vertex**, where four patches meet and their tangent planes must all agree.
//! 3. **A T-junction**, where three patches meet and one quad's edge runs past the vertex.
//!
//! **The answer is no to all three, and the reason for each is different.** Restricting the mesh to
//! regular junctions does not buy smoothness, because a seam is already discontinuous away from its
//! ends; and a T-junction is not merely hard to make smooth, it is **not representable** — an
//! `EdgeKey` is a pair of endpoint ids, so a vertex lying in the middle of an edge has nowhere to be
//! recorded and the edge reads as a boundary edge with one face.
//!
//! A **lone seam** runs first as the control: two patches, one soft edge, nothing to contest. Every
//! number below is measured against it, so the cost of crowding is separated from the baseline
//! rather than guessed at. The absolute figures depend on how curved the sampled surface is; what
//! does not depend on the surface is that none of them is zero.

use std::collections::BTreeMap;

use whiting_patch::topology::{EdgeKey, EdgeSlot};
use whiting_patch::{Continuity, PatchMesh, Point3, VertexId};

/// How many edges meet at each mesh vertex, counted over **distinct** edges.
///
/// Counting per slot overcounts by exactly two: a vertex is an endpoint of two of its own face's
/// four edges, which is correct for `edge_endpoints` and wrong for a valence. The mesh's own
/// `edge_faces` keys are the distinct edges, so each endpoint is counted once per key.
fn valence(quads: &[[VertexId; 4]]) -> BTreeMap<VertexId, usize> {
    let mut count: BTreeMap<VertexId, usize> = BTreeMap::new();
    let mut distinct: BTreeMap<EdgeKey, ()> = BTreeMap::new();
    for corners in quads {
        for slot in EdgeSlot::ALL {
            distinct.insert(
                whiting_patch::topology::canonical(whiting_patch::topology::edge_endpoints(
                    *corners, slot,
                )),
                (),
            );
        }
    }
    for (edge, _) in distinct {
        for end in [edge.0, edge.1] {
            *count.entry(end).or_default() += 1;
        }
    }
    count
}

/// A curved height field, so that no patch is flat and a zero reading means continuity rather than
/// a degenerate net.
fn height(x: f64, z: f64) -> f64 {
    0.3 * ((x * 1.7).sin() * (z * 2.1).cos() + 0.4 * (x * 3.3 + z * 1.1).sin())
}

fn at(i: i64, j: i64) -> Point3 {
    Point3::new(i as f64, height(i as f64, j as f64), j as f64)
}

/// The vertex at grid cell `(i, j)`, on the same row-major numbering the quads use.
fn lattice(stride: i64) -> impl Fn(i64, i64) -> VertexId {
    move |i, j| (j * stride + i) as VertexId
}

/// One position per lattice site, whether or not a quad still refers to it.
///
/// The merged cell below orphans `(1, 0)`; leaving its position in the buffer costs nothing and
/// avoids reconstructing grid coordinates from edge adjacency, which is a way to introduce a bug
/// into a measurement.
fn positions(stride: i64, columns: i64, rows: i64) -> Vec<Point3> {
    let mut positions = Vec::new();
    for j in 0..=rows {
        for i in 0..=columns {
            positions.push(at(i, j));
        }
    }
    let _ = stride;
    positions
}

/// A regular `columns` by `rows` quad grid. Every interior vertex is valence 4.
fn regular(columns: i64, rows: i64) -> PatchMesh {
    let stride = columns + 1;
    let id = lattice(stride);
    let mut quads = Vec::new();
    for j in 0..rows {
        for i in 0..columns {
            quads.push([id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    PatchMesh::build(positions(stride, columns, rows), quads)
}

/// The same grid with cells `(0, 0)` and `(1, 0)` merged into one wide quad.
///
/// This leaves vertex `(1, 1)` in the middle of the merged quad's upper edge, where three patches
/// meet: a **T-junction**. Nothing else changes, so the difference from [`regular`] is the junctions
/// and only the junctions.
fn with_t_junction(columns: i64, rows: i64) -> PatchMesh {
    let stride = columns + 1;
    let id = lattice(stride);
    let mut quads = vec![[id(0, 0), id(2, 0), id(2, 1), id(0, 1)]];
    for j in 0..rows {
        for i in 0..columns {
            if j == 0 && (i == 0 || i == 1) {
                continue;
            }
            quads.push([id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    PatchMesh::build(positions(stride, columns, rows), quads)
}

/// The angle between the two faces of `edge`, at `along` of its length.
///
/// The slot names the parameter the edge **fixes**, so the other is what `along` moves.
fn fold_where(mesh: &PatchMesh, edge: &EdgeKey, along: f64) -> Option<f64> {
    let faces = mesh.topology().edge_faces.get(edge)?;
    if faces.len() != 2 {
        return None;
    }
    let normal_of = |(face, slot): (whiting_patch::FaceId, EdgeSlot)| {
        let (fixed, moves_u) = match slot {
            EdgeSlot::MinV => (0.0, true),
            EdgeSlot::MaxV => (1.0, true),
            EdgeSlot::MinU => (0.0, false),
            EdgeSlot::MaxU => (1.0, false),
        };
        mesh.composed().face(face as usize).map(|net| {
            if moves_u {
                net.normal(along, fixed)
            } else {
                net.normal(fixed, along)
            }
        })
    };
    let first = normal_of(faces[0])?;
    let second = normal_of(faces[1])?;
    Some(first.angle(second))
}

struct Summary {
    middle: Vec<f64>,
    valence_four: Vec<f64>,
    valence_three: Vec<f64>,
    other: Vec<f64>,
}

fn measure(mesh: &PatchMesh) -> Summary {
    let degrees = valence(mesh.quads());
    let mut summary = Summary {
        middle: Vec::new(),
        valence_four: Vec::new(),
        valence_three: Vec::new(),
        other: Vec::new(),
    };
    for (edge, faces) in &mesh.topology().edge_faces {
        if faces.len() != 2 || !mesh.continuity(edge).is_some_and(Continuity::is_soft) {
            continue;
        }
        if let Some(mid) = fold_where(mesh, edge, 0.5) {
            summary.middle.push(mid);
        }
        // `along = 0` and `along = 1` are the edge's two ends, which are mesh vertices. The fold is
        // read there as well, because that is where the junction question lives.
        for (end, at) in [(0.0f64, edge.0), (1.0f64, edge.1)] {
            let Some(fold) = fold_where(mesh, edge, end) else {
                continue;
            };
            match degrees.get(&at).copied() {
                Some(4) => summary.valence_four.push(fold),
                Some(3) => summary.valence_three.push(fold),
                _ => summary.other.push(fold),
            }
        }
    }
    summary
}

fn worst(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0f64, f64::max)
}

fn line(label: &str, values: &[f64]) {
    if values.is_empty() {
        println!("  {label:<34} none");
        return;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    println!(
        "  {label:<34} n {:>4}   median {:.3e}   worst {:.3e} rad ({:.2} deg)",
        values.len(),
        median,
        worst(values),
        worst(values).to_degrees()
    );
}

fn report(label: &str, mesh: &PatchMesh) {
    let degrees = valence(mesh.quads());
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for degree in degrees.values() {
        *histogram.entry(*degree).or_default() += 1;
    }
    let summary = measure(mesh);
    println!("\n{label}");
    print!("  mesh vertex valence ");
    for (degree, count) in &histogram {
        print!("{degree}:{count}  ");
    }
    println!();
    line("edge middle", &summary.middle);
    line("at a valence-4 vertex", &summary.valence_four);
    // On this mesh the valence-3 vertices are the grid's own boundary, not junctions: a real
    // T-junction cannot be built at all, which measurement 3 shows. Labelling them "T" would read
    // as a claim about junctions that this mesh does not support.
    line("at a valence-3 boundary vertex", &summary.valence_three);
    line("at any other vertex", &summary.other);
}

fn all_soft(mesh: &mut PatchMesh) {
    let interior: Vec<EdgeKey> = mesh
        .topology()
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 2)
        .map(|(edge, _)| *edge)
        .collect();
    for edge in interior {
        mesh.set_soft(edge);
    }
}

fn main() {
    println!("\n== 1  a lone seam, the control ==\n");
    let mut lone = regular(2, 1);
    let seam: EdgeKey = lone
        .topology()
        .edge_faces
        .iter()
        .find(|(_, faces)| faces.len() == 2)
        .map(|(edge, _)| *edge)
        .expect("two patches share one edge");
    lone.set_soft(seam);
    let degrees = valence(lone.quads());
    for along in [0.0, 0.25, 0.5, 0.75, 1.0] {
        if let Some(fold) = fold_where(&lone, &seam, along) {
            let vertex = if along == 0.0 {
                seam.0
            } else if along == 1.0 {
                seam.1
            } else {
                u32::MAX
            };
            let where_ = if vertex == u32::MAX {
                format!("along {along:.2} (interior)")
            } else {
                format!("along {along:.2} (a valence-{} vertex)", degrees[&vertex])
            };
            println!(
                "  at {where_:<28} {fold:.3e} rad ({:.3} deg)",
                fold.to_degrees()
            );
        }
    }

    println!("\n== 2  a regular grid, every interior vertex valence 4 ==\n");
    let mut plain = regular(4, 4);
    all_soft(&mut plain);
    report("4 by 4, all interior edges soft", &plain);

    println!("\n== 3  what a T-junction does to this topology ==\n");
    let tacked = with_t_junction(4, 4);
    let topology = tacked.topology();
    let hanging: Vec<_> = topology
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 1)
        .map(|(edge, _)| *edge)
        .collect();
    println!(
        "  a 4 by 4 grid with cells (0,0) and (1,0) merged into one wide quad\n\
         \x20  edges with only one face   {}\n\
         \x20  of them                    {:?}\n\
         \n\
         A T-junction is by definition a vertex **inside** another quad's edge. That edge runs from\n\
         lattice `(0, 1)` to `(2, 1)`, so vertex `(1, 1)` is not one of its endpoints and the edge\n\
         never learns the vertex exists. `EdgeKey` is a pair of endpoint ids, so there is nowhere to\n\
         record that `(1, 1)` lies in the middle of it, and the edge reads as a **boundary** edge:\n\
         one face, not two.\n\
         \n\
         So this is not a gap that tuning closes. The four-corner quad model cannot represent a\n\
         T-junction at all — the fold readings above are a *cracked mesh*, not a continuous one.",
        hanging.len(),
        &hanging[..hanging.len().min(4)]
    );
}
