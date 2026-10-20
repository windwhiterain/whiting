//! Where a building plate's build time actually goes, stage by stage.
//!
//! The question a cache answers is "which stages does an edit re-run", so the four phases of one
//! plate are timed as the four stages a lichen program names: the **plan** (the skeleton plus the
//! scene's ops), its **expansion** to a closed mesh, the **parts** (the identity table and the
//! packed point lists the document draws), and the **document** itself. Two more readings sit
//! beside them because they decide what a cache is worth: what re-reading a reused `parts` costs,
//! and what `--census`'s own half of its wall clock is.
//!
//! It prints and asserts nothing: `cargo run -p whiting-gallery --example building_cost_probe`,
//! and again with `--release`, because the plate is small enough that the debug constant is the
//! one an author actually pays.

use std::time::Instant;

use building::BuildingMesh;
use building::derive::HALL;
use scene::standard::POINTS;
use whiting_gallery::Gallery;

/// The document's mesh list, by the field name the standard schema writes it under.
///
/// # Invariant
///
/// [`scene::standard::POINTS`] is re-exported and this is not, so the one name this probe spells
/// out is the list's own; a reader who wants it as a constant finds `scene_schema::MESHES`.
const MESHES: &str = "meshes";

/// The footprint this plate is drawn at, and the storey count it is drawn with.
const WIDTH: f32 = 10.0;
const DEPTH: f32 = 16.0;
const STOREYS: usize = 1;

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

/// How many packed elements the document's meshes carry between them.
fn packed_elements(document: &definition::Value) -> usize {
    document
        .as_struct()
        .and_then(|fields| fields.get(MESHES))
        .and_then(definition::Value::as_list)
        .map_or(0, |meshes| {
            meshes
                .iter()
                .filter_map(definition::Value::as_struct)
                .filter_map(|mesh| mesh.get(POINTS))
                .filter_map(definition::Value::as_packed)
                .map(definition::PackedList::len)
                .sum()
        })
}

fn main() {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("hall, {profile} build");

    let phase = Instant::now();
    let planned = whiting_gallery::hall::document_at(WIDTH, DEPTH, STOREYS);
    let plan_ms = ms(phase);
    println!(
        "  plan        {plan_ms:8.1} ms   {} violations",
        planned.validate().len()
    );

    let phase = Instant::now();
    let built: BuildingMesh = planned.build().expect("the hall expands");
    let build_ms = ms(phase);
    let solids = whiting_gallery::building_mesh::solids(&built);
    println!(
        "  built       {build_ms:8.1} ms   {} points, {} faces, {} solids",
        built.points.len(),
        built.faces.len(),
        solids.len(),
    );

    let phase = Instant::now();
    let mut per_solid = 0usize;
    for solid in &solids {
        per_solid += whiting_gallery::building_mesh::points_of(&built, *solid).len();
    }
    let points_ms = ms(phase);
    println!(
        "  points      {points_ms:8.1} ms   {per_solid} elements, one solid at a time, no ids"
    );

    let starter = HALL.starter(WIDTH, DEPTH, STOREYS);
    let phase = Instant::now();
    let parts = whiting_gallery::hall::parts(&starter, &planned, &built);
    let parts_ms = ms(phase);
    println!("  parts       {parts_ms:8.1} ms   the identity table and the four packed lists");

    let phase = Instant::now();
    let _re_read = parts.clone();
    let re_read_ms = ms(phase);
    println!(
        "  re-read     {re_read_ms:8.1} ms   what a reused parts cell costs the document stage"
    );

    let phase = Instant::now();
    let view = whiting_gallery::hall::hall_view(1.0, [0.0; 3]);
    let document = whiting_gallery::hall::hall_document_over_parts(&parts, &view);
    let document_ms = ms(phase);
    println!(
        "  document    {document_ms:8.1} ms   {} top-level fields, {} elements over the meshes",
        document.as_struct().map_or(0, |fields| fields.len()),
        packed_elements(&document),
    );

    println!(
        "  ---- plan + built + parts = {:.1} ms; document alone = {document_ms:.1} ms",
        plan_ms + build_ms + parts_ms
    );
    println!(
        "  ---- the whole plate = {:.1} ms",
        plan_ms + build_ms + parts_ms + document_ms
    );

    // **`--census` builds the scene twice**, once through `Gallery::census` and once through
    // `Gallery::bounds`, so this reading is both of its document halves and neither of the
    // rasteriser the harness builds before it prints anything.
    let phase = Instant::now();
    let census = Gallery::census("hall");
    let bounds = Gallery::bounds("hall");
    let census_ms = ms(phase);
    println!(
        "  census      {census_ms:8.1} ms   {} + {} lines, two whole documents",
        census.lines().count(),
        bounds.lines().count(),
    );
}
