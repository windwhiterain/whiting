//! Can the Hermite surface take the authored massif, or is the per-patch Bernstein model the only
//! thing that can?
//!
//! Run with `cargo run -p whiting-patch --example hermite_massif_probe`. It exits non-zero if the
//! surface cannot be built, **because that is a result and not a failure of the probe** — the whole
//! point of this file is to answer whether the refactor can finish, and "no" is an answer.
//!
//! `crates/gallery/src/patchmesh.rs` turns `mountain`'s `FacetMesh` into an editable patch surface by
//! Doo–Sabin subdividing the triangles into quads. That module is **the last production caller of
//! [`crate::PatchMesh`]**, so the per-patch model cannot be deleted until this one is answered.
//!
//! `HermiteSurface::build` refuses a mesh where the two faces on an edge disagree about **which kind**
//! of edge it is, because a per-vertex `du`/`dv` field only means something if every face at a vertex
//! reads it the same way round. The handoff records that our own front end's output is **89.4 %
//! extraordinary**, valences 2 to 40, so this is worth measuring rather than assuming either way.
//!
//! 0. What the conversion produces — the shape of the question before the answer.
//! 1. Whether `HermiteSurface::build` accepts it, and if not, how many edges refused it and whether a
//!    positive control shows the check can fire at all.


use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::{HermiteSurface, SurfaceError};

/// `mountain`'s authored massif, reached the same way every other front-end probe in this
/// repository reaches it.
fn massif() -> FacetMesh {
    match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the sample massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    }
}

fn main() {
    println!("\nconverting the authored massif with gallery::patchmesh::from_facet");
    let facet = massif();

    let mesh = match patchmesh::from_facet(&facet) {
        Ok(mesh) => mesh,
        Err(error) => {
            println!("  the conversion refused it: {error}");
            std::process::exit(1);
        }
    };
    let positions = mesh.positions().to_vec();
    let quads = mesh.quads().to_vec();
    println!(
        "  derived mesh: {} vertices, {} quads",
        positions.len(),
        quads.len()
    );

    // 0. What the mesh looks like, because the refusal below means different things on different
    // shapes and "build failed" is not a diagnosis without it.
    let mut valences: std::collections::BTreeMap<usize, usize> = Default::default();
    for quad in &quads {
        for slot in 0..4 {
            let ends = whiting_patch::topology::edge_endpoints(*quad, whiting_patch::topology::EdgeSlot::ALL[slot]);
            for end in ends {
                *valences.entry(end as usize).or_default() += 1;
            }
        }
    }
    let extraordinary = valences.values().filter(|valence| **valence != 4).count();
    println!(
        "  {} vertices, {} of them not valence 4 ({:.1} %)",
        valences.len(),
        extraordinary,
        100.0 * extraordinary as f64 / valences.len().max(1) as f64
    );
    let valences_present: Vec<usize> = valences.values().copied().collect();
    let _ = valences_present;

    // 1. The answer.
    //
    // Counted over the whole mesh rather than naming the first refusal, because "edge (0, 1414)" says
    // something is wrong and a count says whether it is one edge or the mesh. `HermiteSurface::build`
    // stops at the first, which is right for a caller and wrong for a measurement.
    let mut disagreeing = 0usize;
    let mut shared = 0usize;
    let mut odd_valence = 0usize;
    for (_edge, faces) in &mesh.topology().edge_faces {
        if let [(first, a), (second, b)] = faces.as_slice() {
            let _ = (first, second);
            shared += 1;
            if a.varies_along_u() != b.varies_along_u() {
                disagreeing += 1;
            }
        }
    }
    for valence in valences.values() {
        if valence % 2 == 1 {
            odd_valence += 1;
        }
    }
    println!(
        "\n  {shared} shared edges, {disagreeing} of them traversed as different kinds of edge\n\
         {odd_valence} vertices of odd valence",
    );

    match HermiteSurface::build(positions, quads.clone()) {
        Ok(surface) => {
            println!(
                "\nACCEPTED: {} faces, {} interior edges",
                surface.face_count(),
                surface
                    .topology()
                    .edge_faces
                    .values()
                    .filter(|faces| faces.len() == 2)
                    .count()
            );
            let (spread_u, spread_v) = surface.step_spread();
            println!(
                "  cell spread: {:.4} along u, {:.4} along v",
                spread_u, spread_v
            );
            println!(
                "\nthe Hermite surface takes the massif. gallery::patchmesh can be ported and \
                 crate::PatchMesh deleted."
            );
        }
        Err(error) => {
            println!("\nREFUSED: {error}");
            if let SurfaceError::InconsistentWinding { edge, first, second } = error {
                println!("  edge {edge:?}: slot {first:?} on one face, {second:?} on the other");
            }
            if let SurfaceError::NonManifold { edges } = error {
                println!("  {edges} edges carry three or more faces");
            }
            println!("\nreported, not asserted: this is a measurement, and it is why the old model");
            println!("cannot simply be deleted. See .agents/handoffs/hermite-surface.md.");

            // The control: a mesh that does fail must fail *here*, or the refusal above is a property
            // of the sample rather than of the check.
            let mut broken = quads.clone();
            broken[0] = [broken[0][0], broken[0][2], broken[0][1], broken[0][3]];
            let control = HermiteSurface::build(vec![whiting_patch::Point3::ZERO; 4], broken);
            println!(
                "\ncontrol: one quad wound the other way round gives {}",
                match control {
                    Ok(_) => "a surface, so the check does not fire".to_owned(),
                    Err(error) => format!("{error}"),
                }
            );
            std::process::exit(1);
        }
    }
}
