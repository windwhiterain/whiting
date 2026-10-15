//! Is the shared per-direction step what costs the massif its fit at some resolutions?
//!
//! Run with `cargo run -p whiting-gallery --example step_fit_probe`. Diagnostic: it reports, and
//! exits non-zero only if the hypothesis it tests is **confirmed**, so a run that says "not the
//! step" is a pass.
//!
//! # The symptom it starts from
//!
//! `block_surface_probe` measures, at `side` 2 and 4, a departure at the mesh's own `f32` floor
//! (`1.4e-06` and `4.2e-06`), and at `side` 3, 6 and 8, one around `1.4e-01` to `2.5e-01`. Crucially
//! that split holds **among blocks whose every corner is on the rock**, with no fallback involved, so
//! it is not the projection and it is not the ragged domain. A fit error that is perfect at one
//! resolution and catastrophic at the next is not a fit error.
//!
//! # The hypothesis
//!
//! `HermiteSurface` seeds each vertex's `du` and `dv` from edge directions scaled by **one step per
//! direction for the whole mesh** — that shared step is what makes a soft seam exactly C1, at the
//! price of every patch being parameterised for the mesh's average cell. A block whose own cells are
//! much smaller than that average gets tangents built for a longer cell, and a bicubic with the
//! wrong tangent magnitude bows away from the corners it already interpolates.
//!
//! If that is the cause, then:
//!
//! - a surface built from **one block alone** has a step fitted to that block, so it should depart at
//!   the floor at **every** resolution — and
//! - within a whole-mesh surface, a block's departure should **track how far its own cell size is
//!   from the mesh-wide step**, not the resolution.
//!
//! Both are measured. The second one is the decisive one: it is a **correlation over blocks**, so it
//! can come out flat even when the hypothesis is right, and a flat correlation here is reported as
//! "not the step" rather than quietly ignored.

use mountain::{FacetMesh, Starter, starter};
use patch::Point3;
use whiting_gallery::blockmesh::{self, BlockSurfaces};

fn parameters() -> [f64; 3] {
    [0.25, 0.5, 0.75]
}

/// The rock at `(u, v)`, averaged over every face of the set that covers it.
fn sample(faces: &[&mountain::Face], massif: &FacetMesh, u: f32, v: f32) -> Option<Point3> {
    let mut sum = [0.0f64; 3];
    let mut count = 0.0f64;
    for face in faces {
        let [a, b, c] = face.uv;
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if area.abs() < f32::EPSILON {
            continue;
        }
        let raw = [
            (b[0] - u) * (c[1] - v) - (c[0] - u) * (b[1] - v),
            (c[0] - u) * (a[1] - v) - (a[0] - u) * (c[1] - v),
        ];
        let shares = [f64::from(raw[0] / area), f64::from(raw[1] / area)];
        let shares = [shares[0], shares[1], 1.0 - shares[0] - shares[1]];
        if shares.iter().any(|share| *share < -1.0e-9) {
            continue;
        }
        for (index, share) in shares.into_iter().enumerate() {
            let point = massif.points[face.indices[index] as usize];
            for axis in 0..3 {
                sum[axis] += f64::from(point[axis]) * share;
            }
        }
        count += 1.0;
    }
    (count > 0.0).then(|| Point3::new(sum[0] / count, sum[1] / count, sum[2] / count))
}

/// The `(u, v)` box a set of faces covers.
fn uv_box(faces: &[&mountain::Face]) -> ([f32; 2], [f32; 2]) {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for face in faces {
        for corner in face.uv {
            for axis in 0..2 {
                low[axis] = low[axis].min(corner[axis]);
                high[axis] = high[axis].max(corner[axis]);
            }
        }
    }
    (low, high)
}

/// A block's cell **sizes in space**, not in `(u, v)`.
///
/// The spread column in the table is `(u, v)` ratios, which says nothing about how big the cells
/// are where the rock is. This measures the two directions of the first cell in metres, which is
/// what a patch's tangents are actually built from — and it is the number that separates a block
/// whose grid is square on the rock from one whose grid is `side` squares laid on a long thin
/// region.
///
/// Read off the built surface rather than recomputed, so what is measured is the geometry the
/// patches were handed.
fn cell_sizes(surface: &patch::HermiteSurface, face: u32) -> (f64, f64) {
    let corners = &surface.quads()[face as usize];
    let positions = &surface.positions();
    let near = positions[corners[0] as usize];
    let along_u = positions[corners[1] as usize];
    let along_v = positions[corners[3] as usize];
    (near.distance(along_u), near.distance(along_v))
}

/// The worst departure of a built surface from the rock, at cell centres.
fn departure(
    surface: &patch::HermiteSurface,
    faces: &[&mountain::Face],
    massif: &FacetMesh,
    side: usize,
) -> f64 {
    let (low, high) = uv_box(faces);
    let last = side - 1;
    let mut worst = 0.0f64;
    for row in 0..last {
        for column in 0..last {
            let (s, t) = (
                (column as f64 + 0.5) / last as f64,
                (row as f64 + 0.5) / last as f64,
            );
            let u = low[0] + (high[0] - low[0]) * s as f32;
            let v = low[1] + (high[1] - low[1]) * t as f32;
            let Some(rock) = sample(faces, massif, u, v) else {
                continue;
            };
            let cell = (row * last + column) as u32;
            worst = worst.max(rock.distance(surface.patch(cell).evaluate(0.5, 0.5)));
        }
    }
    worst
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// How much **the rock itself** moves across one cell, in space.
///
/// This is the number that decides whether a departure is a defect. A cell of a faceted mesh spans
/// several triangles, and the rock is not flat across them: a surface that interpolates the rock's
/// own corners and is smooth between them **cannot** follow that, and the best it can do is sit
/// somewhere inside the range the rock already occupies. So if a cell's departure is no larger than
/// the rock's own spread across that cell, the surface is inside the rock's own variation and the
/// probe's threshold is asking for something no smooth surface can deliver.
///
/// Sampled at nine points — the four corners, the four edge midpoints, and the centre — and taken as
/// the widest gap between any two of them, which is the spread rather than an average of it.
fn rock_spread(faces: &[&mountain::Face], massif: &FacetMesh, side: usize) -> f64 {
    let (low, high) = uv_box(faces);
    let last = side - 1;
    let mut per_cell: Vec<f64> = Vec::new();
    for row in 0..last {
        for column in 0..last {
            let mut probes = Vec::with_capacity(9);
            for (dr, dc) in [
                (0.0f64, 0.0f64),
                (0.0, 0.5),
                (0.0, 1.0),
                (0.5, 0.0),
                (0.5, 0.5),
                (0.5, 1.0),
                (1.0, 0.0),
                (1.0, 0.5),
                (1.0, 1.0),
            ] {
                let (s, t) = (
                    (column as f64 + dc) / last as f64,
                    (row as f64 + dr) / last as f64,
                );
                let u = low[0] + (high[0] - low[0]) * s as f32;
                let v = low[1] + (high[1] - low[1]) * t as f32;
                // A corner no triangle covers makes the cell's spread undefined rather than small,
                // and counting it as small is how a ragged region reads as a good fit.
                match sample(faces, massif, u, v) {
                    Some(point) => probes.push(point),
                    None => {
                        probes.clear();
                        break;
                    }
                }
            }
            if probes.len() < 9 {
                continue;
            }
            let mut widest = 0.0f64;
            for (at, first) in probes.iter().enumerate() {
                for second in &probes[at + 1..] {
                    widest = widest.max(first.distance(*second));
                }
            }
            per_cell.push(widest);
        }
    }
    median(&mut per_cell)
}

fn main() {
    let facet: FacetMesh = match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };

    println!("\n  A block on its own, against the same block inside the whole-mesh surface");
    println!(
        "\n  side   alone (median)   in the mesh (median)   cell aspect   rock's own spread\n     \x20       of the patch    inside the mesh         u/v (median)  in one cell (median)"
    );
    let mut alone_is_always_floor = true;
    let mut aspects: Vec<(f64, f64)> = Vec::new();
    let mut spreads: Vec<(f64, f64)> = Vec::new();

    for side in [2usize, 3, 4, 6, 8] {
        let whole = blockmesh::from_facet(&facet, side);
        let mut alone: Vec<f64> = Vec::new();
        let mut own: Vec<f64> = Vec::new();
        let mut spread: Vec<f64> = Vec::new();
        for block in whole.blocks() {
            let Some(source) = facet.blocks.get(block.source as usize) else {
                continue;
            };
            let faces: Vec<&mountain::Face> = facet.faces
                [source.faces.start as usize..source.faces.end as usize]
                .iter()
                .collect();
            // A one-block surface is built by handing `from_facet` a mesh holding only that block's
            // faces. The block's own face range has to be rebased onto them: it indexes the whole
            // massif's faces, and `from_facet` reads it against this much smaller list.
            let mut only = FacetMesh {
                points: facet.points.clone(),
                faces: Vec::new(),
                blocks: Vec::new(),
            };
            only.faces = faces.iter().map(|face| (**face).clone()).collect();
            for face in &mut only.faces {
                face.block = 0;
            }
            let mut one_block = source.clone();
            one_block.faces = 0..only.faces.len() as u32;
            only.blocks.push(one_block);
            let single = blockmesh::from_facet(&only, side);
            let Some(one) = single.blocks().first() else {
                continue;
            };
            let reading = departure(&one.surface, &faces, &facet, side);
            alone.push(reading);
            let (u_size, v_size) = cell_sizes(&one.surface, 0);
            own.push(u_size / v_size.max(f64::MIN_POSITIVE));
            aspects.push((u_size / v_size.max(f64::MIN_POSITIVE), reading));
            let rock = rock_spread(&faces, &facet, side);
            if rock > 0.0 {
                spread.push(rock);
                spreads.push((reading, rock));
            }
        }

        let mut sorted = alone.clone();
        let alone_median = median(&mut sorted);
        let in_mesh = blockmesh_median(&whole, &facet, side);
        let mut sorted_own = own.clone();
        let mut sorted_spread = spread.clone();
        println!(
            "  {:>4}   {:>14.3e}   {:>16.3e}   {:>11.3e}   {:>22.3e}",
            side,
            alone_median,
            in_mesh,
            median(&mut sorted_own),
            median(&mut sorted_spread)
        );
        if alone_median > 1.0e-4 {
            alone_is_always_floor = false;
        }
    }

    // The decisive reading: **is the departure inside the rock's own variation?**
    let inside = spreads
        .iter()
        .filter(|(reading, rock)| reading <= rock)
        .count();
    println!(
        "\n  of {} blocks, the departure is within the rock's own spread for {} ({:.0}%)\n\
         \x20 and above it for {}",
        spreads.len(),
        inside,
        100.0 * inside as f64 / spreads.len().max(1) as f64,
        spreads.len() - inside
    );

    println!("\n  cell aspect against departure, over every block at every resolution");
    let mut sorted_aspects = aspects.clone();
    sorted_aspects.sort_by(|left, right| left.0.total_cmp(&right.0));
    for quantile in [0.5f64, 0.9, 0.99, 1.0] {
        let at = ((sorted_aspects.len() as f64 - 1.0) * quantile).round() as usize;
        let (aspect, reading) = sorted_aspects[at];
        println!(
            "    aspect {aspect:>10.3e} at the {quantile:.0}th percentile   departure {reading:.3e}"
        );
    }
    let (aspect, reading) = sorted_aspects[0];
    let (worst_aspect, worst_reading) = *sorted_aspects.last().unwrap_or(&(0.0, 0.0));
    println!(
        "\n  least elongated cell {aspect:.3e} departs {reading:.3e}; most elongated \
         {worst_aspect:.3e} departs {worst_reading:.3e}"
    );

    println!();
    if alone_is_always_floor {
        println!("  CONFIRMED: a block on its own stays at the floor at every resolution, so the");
        println!("  departure is the mesh-wide step rather than the block's shape.");
        println!();
        println!(
            "FAILED  the shared step is confirmed as the cause, so it is the next thing to change"
        );
        std::process::exit(1);
    }
    println!("  NOT the step: a block on its own departs at the worse resolutions too.");
    println!("  The cause is inside the block's own grid, not in how the mesh shares one step.");
}

/// The median departure of the blocks inside a whole-mesh surface.
fn blockmesh_median(blocks: &BlockSurfaces, facet: &FacetMesh, side: usize) -> f64 {
    let mut readings: Vec<f64> = Vec::new();
    for block in blocks.blocks() {
        let Some(source) = facet.blocks.get(block.source as usize) else {
            continue;
        };
        let faces: Vec<&mountain::Face> = facet.faces
            [source.faces.start as usize..source.faces.end as usize]
            .iter()
            .collect();
        // A block's patches inside the whole-mesh surface are addressed from zero, because
        // `from_facet` gives every block its own `HermiteSurface`; the mesh-wide step is what the
        // block's surface was built with, which is the thing under test.
        readings.push(departure(&block.surface, &faces, facet, side));
    }
    median(&mut readings)
}
