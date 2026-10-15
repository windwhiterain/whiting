//! What the erosion solve does to this workspace's own massif, measured.
//!
//! Run it with `cargo run -p whiting-gallery --example erode_probe`. It builds the
//! `landscape` scene's massif, reads it onto lattices at three spacings to show what the
//! spacing is worth, and then runs the scene's own [`Erosion`] over the range the scene
//! draws — the books, the displacement that reaches the mesh, and the cost of both.

use std::time::Instant;

use mountain::FacetMesh;
use terrain::erode::{self, Hydraulic};
use terrain::{Books, ErodeParams, HeightField, erode_traced, rasterize};
use whiting_gallery::landscape::{Erosion, RANGES, massif, stood_at};

fn main() {
    let authored = massif();
    let mesh = authored.build().expect("the authored massif expands");
    let (low, high) = mesh.bounds();
    println!("massif");
    println!("  faces          {}", mesh.faces.len());
    println!("  points         {}", mesh.points.len());
    println!("  violations     {}", authored.validate().len());
    println!(
        "  plan extent    {:.1} x {:.1}",
        high[0] - low[0],
        high[2] - low[2]
    );
    println!("  relief         {:.1}", high[1] - low[1]);
    println!("  shortest edge  {:.2}", mesh.shortest_edge());

    println!("\nwhat the lattice's spacing is worth, at the reference parameters");
    for cell in [24.0_f32, 12.0, 6.0] {
        sweep(&mesh, cell);
    }

    println!("\nthe scene's own erosion, over the range it draws");
    scene(&mesh);
}

/// Runs one lattice at the reference parameters and prints what came out of it.
fn sweep(mesh: &FacetMesh, cell: f32) {
    let started = Instant::now();
    let read = rasterize(mesh, cell).expect("the massif covers its own lattice");
    let field = &read.field;
    let scanned = started.elapsed();

    let started = Instant::now();
    let (eroded, books) =
        erode_traced(&ErodeParams::default(), field, &Hydraulic).expect("a field is a field");
    let solve = started.elapsed();

    println!(
        "  at {cell:.0} units: {} x {} cells, {} covered, {scanned:?} to read, {solve:?} to solve",
        field.width(),
        field.height(),
        field.covered(),
    );
    println!("    {}", displacement(field, &eroded));
    println!("    {}", ledger(&books));
    println!("    {}", mass_line(field, &eroded, &books));
    println!("    {}", sinks(&eroded));
}

/// Runs what [`Erosion`] does to the near range, which is the authored massif stood at the
/// scene's own placement.
fn scene(mesh: &FacetMesh) {
    let range = &RANGES[0];
    let (low, high) = mesh.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let stood = stood_at(
        mesh,
        [range.centre[0] * radius, range.centre[1] * radius],
        range.scale,
        range.turn,
    );

    let started = Instant::now();
    let erosion = Erosion::of(&stood);
    let eroded = erosion.applied(&stood);
    let elapsed = started.elapsed();

    println!(
        "  spacing        {} units",
        whiting_gallery::landscape::EROSION_CELL
    );
    println!(
        "  lattice        {} cells, {} covered",
        erosion.field.cells(),
        erosion.field.covered()
    );
    println!("  solve and mesh {elapsed:?}");
    println!("  books          {}", ledger(&erosion.books));
    println!(
        "  mass line      {}",
        mass_line(&erosion.field, &erosion.eroded, &erosion.books)
    );
    println!("  {}", applied(&stood, &eroded));
    println!(
        "  displacement by distance from the rim:\n{}",
        by_distance_from_the_rim(&erosion)
    );
}

/// Returns how the displacement is distributed by distance from the rim.
///
/// The rim is where a massif is **cut off** rather than where it ends, so whatever the solve
/// does near it is its answer to an artificial boundary and not to a landscape. This is the
/// measurement that shows whether the boundary is leaking: a band of cells standing above
/// where they were, all the way round, is a toe the water piled up because it ran out of
/// mountain — and it stands above the plain the scene levels to meet it, so it casts a
/// shadow onto it.
fn by_distance_from_the_rim(erosion: &Erosion) -> String {
    let field = &erosion.field;
    let mut bands: [(f64, usize, f32, f32); 5] = Default::default();
    let limits = [1.5, 3.5, 6.5, 12.5];
    for at in 0..field.cells() {
        if !field.is_inside(at) {
            continue;
        }
        let band = limits
            .iter()
            .position(|limit| erosion.margin[at] < *limit)
            .unwrap_or(limits.len());
        let delta = erosion.eroded.height_at(at) - field.height_at(at);
        let entry = &mut bands[band];
        entry.0 += delta as f64;
        entry.1 += 1;
        entry.2 = entry.2.min(delta);
        entry.3 = entry.3.max(delta);
    }
    let named = [
        "under 1.5 cells",
        "1.5 to 3.5",
        "3.5 to 6.5",
        "6.5 to 12.5",
        "beyond 12.5",
    ];
    named
        .iter()
        .zip(bands.iter())
        .filter(|(_, entry)| entry.1 > 0)
        .map(|(name, entry)| {
            format!(
                "  {name:<16} {:>7} cells, mean {:>7.2}, range {:>7.2} to {:>7.2}",
                entry.1,
                entry.0 / entry.1 as f64,
                entry.2,
                entry.3
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns how far the solve moved the rock before it was applied back to the mesh, split by
/// whether it moved it at the rim.
///
/// The split is the whole of what decides how the scene may use the answer: the rim is where
/// the massif's own mesh meets the plain the scene stands it on, so a displacement there is a
/// crack between two surfaces rather than a feature of the mountain.
fn displacement(field: &HeightField, eroded: &HeightField) -> String {
    let (worst, worst_at_rim, total, moved) = movement(field, eroded);
    format!(
        "raw displacement: worst {worst:.2}, worst at the rim {worst_at_rim:.2}, \
         mean {:.3}, over {moved} cells",
        total / field.covered() as f32
    )
}

/// Returns what reached the mesh's own vertices, which is the displacement after the rim
/// fade and is the number the scene actually draws.
fn applied(before: &FacetMesh, after: &FacetMesh) -> String {
    let mut worst = 0.0_f32;
    let mut total = 0.0;
    let mut moved = 0;
    for (from, to) in before.points.iter().zip(after.points.iter()) {
        let delta = (to[1] - from[1]).abs();
        total += delta;
        if delta > 0.0 {
            moved += 1;
        }
        worst = worst.max(delta);
    }
    let (low, high) = after.bounds();
    format!(
        "on the mesh: worst {worst:.2} over {moved} of {} vertices, mean {:.2}; \
         relief now {:.1}..{:.1}",
        before.points.len(),
        total / before.points.len() as f32,
        low[1],
        high[1]
    )
}

/// Returns the worst, the worst at the rim, the total and how many cells moved.
fn movement(field: &HeightField, eroded: &HeightField) -> (f32, f32, f32, usize) {
    let mut worst = 0.0_f32;
    let mut worst_at_rim = 0.0_f32;
    let mut total = 0.0;
    let mut moved = 0;
    for cell in 0..field.cells() {
        if !field.is_inside(cell) {
            continue;
        }
        let delta = (eroded.height_at(cell) - field.height_at(cell)).abs();
        total += delta;
        if delta > 0.0 {
            moved += 1;
        }
        worst = worst.max(delta);
        if erode::grid::is_outlet(field, cell) {
            worst_at_rim = worst_at_rim.max(delta);
        }
    }
    (worst, worst_at_rim, total, moved)
}

/// Returns how many cells are sinks, split into the ones on the rim and the ones that are
/// not. A rim sink is a valley mouth and spills; an interior one is a lake, and how many of
/// them there are is the measure of whether the lattice is too coarse for the terrain.
fn sinks(field: &HeightField) -> String {
    let heights = field.heights();
    let mut interior = 0;
    let mut rim = 0;
    for at in 0..field.cells() {
        if !field.is_inside(at) {
            continue;
        }
        if erode::grid::receiver(field, heights, at).is_none() {
            if erode::grid::is_outlet(field, at) {
                rim += 1;
            } else {
                interior += 1;
            }
        }
    }
    format!("sinks: {interior} interior, {rim} on the rim")
}

/// Returns both sides of the mass line, so a reader can see whether they agree.
///
/// The height book and the material book are separate identities and either can close while
/// the other does not, so both are printed: the height that left the rock, and the load that
/// left in something else.
fn mass_line(field: &HeightField, eroded: &HeightField, books: &Books) -> String {
    let moved: f64 = field
        .heights()
        .iter()
        .zip(eroded.heights())
        .map(|(from, to)| *from as f64 - *to as f64)
        .sum();
    let carried = books.suspended + books.spilled;
    format!(
        "mass line: height {:.2} against load {carried:.2}; material {:.2} against load {carried:.2}",
        moved,
        books.eroded - books.deposited
    )
}

/// Returns the books as the one line every reader of them wants.
fn ledger(books: &Books) -> String {
    format!(
        "books: eroded {:.0}, deposited {:.0}, suspended {:.1}, spilled {:.1}, \
         thickest deposit {:.2}",
        books.eroded,
        books.deposited,
        books.suspended,
        books.spilled,
        books.thickness.iter().copied().fold(0.0, f32::max),
    )
}
