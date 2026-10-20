//! Where a building plate's build time actually goes.
//!
//! The question: the building series measured 250–580 ms per build, which is already small, so
//! an incremental split has to be aimed at the *stages an edit reaches* rather than at a slow
//! phase. Which phase is which, and how much does each cost?
//!
//! Four phases of one plate: the plan (skeleton plus the scene's edits), the expansion to a
//! closed mesh, the per-element point lists the document draws, and the document itself.
//!
//! It prints and asserts nothing: `cargo run -p whiting-gallery --example building_cost_probe`,
//! and again with `--release`, because the plate is small enough that the debug constant is the
//! one an author actually pays.

use std::time::Instant;

use building::{Building, BuildingMesh};
use whiting_gallery::building_mesh;

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

/// The plan a hall plate is drawn from.
fn plan() -> Building {
    whiting_gallery::hall::document()
}

fn main() {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("hall, {profile} build");

    let phase = Instant::now();
    let planned = plan();
    let plan_ms = ms(phase);
    println!(
        "  plan        {plan_ms:8.1} ms   {} violations",
        planned.validate().len()
    );

    let phase = Instant::now();
    let built: BuildingMesh = planned.build().expect("the hall expands");
    let build_ms = ms(phase);
    println!(
        "  build       {build_ms:8.1} ms   {} points, {} faces, {} solids",
        built.points.len(),
        built.faces.len(),
        building_mesh::solids(&built).len(),
    );

    let phase = Instant::now();
    let mut triangles = 0usize;
    for solid in building_mesh::solids(&built) {
        let points = building_mesh::points_of(&built, solid);
        triangles += points.len();
    }
    let points_ms = ms(phase);
    println!("  points      {points_ms:8.1} ms   {triangles} packed elements");

    let phase = Instant::now();
    let document = whiting_gallery::hall::hall();
    let document_ms = ms(phase);
    println!(
        "  document    {document_ms:8.1} ms   {} top-level fields",
        document.as_struct().map_or(0, |fields| fields.len())
    );

    println!(
        "  ---- plan + build + points = {:.1} ms; document alone = {document_ms:.1} ms",
        plan_ms + build_ms + points_ms
    );
}
