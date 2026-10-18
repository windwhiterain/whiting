//! Reports what a massif is, before anything draws it.
//!
//! This is the authoring loop's cheap half: a massif that will not validate and will not expand
//! is a document, and a document costs no gpu work at all. The number that matters most is the
//! last one — the mesh's plan area against the footprint's — because a hole and a doubled surface
//! are the two failures a picture of a mountain cannot show you.
//!
//! ```text
//! cargo run -p whiting-mountain --example mountain_census           the default seed
//! cargo run -p whiting-mountain --example mountain_census 23        another mountain
//! ```

use whiting_mountain::derive::Starter;
use whiting_mountain::{Massif, ValleyId, derive};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse::<i64>().ok())
        .unwrap_or(Starter::default().seed);

    let massif = Massif::starter(seed, Starter::default());
    println!(
        "seed {seed}: {} ridges, {} valleys, {} footprint nodes, step {}",
        massif.skeleton.ridges.len(),
        massif.skeleton.valleys.len(),
        massif.skeleton.footprint.len(),
        massif.tessellation.step,
    );
    println!(
        "ridge lines: {} connected piece(s), so they are {}",
        massif.skeleton.ridge_components(),
        match massif.skeleton.ridge_components() {
            0 | 1 => "a tree or a network",
            _ => "a scatter, which is neither",
        },
    );

    let violations = massif.validate();
    println!("violations: {}", violations.len());
    for violation in &violations {
        println!("  {violation:?}");
    }

    match massif.build() {
        Ok(mesh) => {
            let (low, high) = mesh.bounds();
            println!(
                "mesh: {} faces, {} points, shortest edge {:.3}",
                mesh.faces.len(),
                mesh.points.len(),
                mesh.shortest_edge(),
            );
            println!(
                "plan area {:.1} against footprint {:.1}, difference {:.3}",
                mesh.plan_area(),
                massif.footprint_area(),
                mesh.plan_area() - massif.footprint_area(),
            );
            // What the surface *is*, which the area cannot say: a hole with no area leaves the
            // difference above at zero and the surface open.
            println!(
                "surface: {} boundary loop(s), euler {}, manifold {}",
                mesh.boundary_loops(),
                mesh.euler_characteristic(),
                mesh.is_manifold(),
            );
            println!(
                "bounds x [{:.1}, {:.1}], y [{:.1}, {:.1}], z [{:.1}, {:.1}]",
                low[0], high[0], low[1], high[1], low[2], high[2],
            );
            for valley in 0..massif.skeleton.valleys.len() {
                let faces = derive::faces(&massif.skeleton, ValleyId(valley));
                println!("  valley {valley} faces {faces:?}");
            }
        }
        Err(failure) => println!("build failed: {failure}"),
    }
}
