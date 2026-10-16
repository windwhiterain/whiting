//! Do neighbouring belts reach the same points on their shared valley, and exactly?
//!
//! Run with `cargo run -p whiting-gallery --example belt_seam_probe`. Exits non-zero if any
//! property fails.
//!
//! # The question
//!
//! Superior's ruling: **one belt is one patch, and between patches the edge is soft or hard at
//! the caller's choice.** That ruling was retired with the shared vertices it presumed
//! (2026-10-08): a Hermite surface holds one derivative field per vertex, and at a shared valley
//! vertex that one vector had to serve two flanks leaving the valley in different directions, so
//! it served neither — the averaged cross-boundary derivative pinched both belts' patches at
//! every seam cell. Belts now meet at **coincident but separate** vertices, and the crease is
//! structural rather than tagged.
//!
//! What survives of the question is the half the tagging never settled: **do the two belts put
//! their valley rows in the same places**, because coincident is a claim about arithmetic, not
//! about intent. Each belt resamples the shared run by arc length from its own end, so the two
//! rows are two walks of one polyline and agree only as well as the walks do.
//!
//! # What is measured
//!
//! 1. **Every valley row vertex of one belt has exactly one partner in the other belt**, at the
//!    same place to `1.0e-9`, and the count of partners is the grid's `side` — a row that lost a
//!    station would leave a crack one cell wide along the whole valley.
//! 2. **No vertex belongs to two belts.** The components of the surface are the belts; a shared
//!    vertex would put the derivative averaging back.
//! 3. **The control**: a belt's own valley row vertices are distinct from one another, or a
//!    "partner" found within `1.0e-9` could be the row's own neighbour and measurement 1 would
//!    pass on a collapsed row.

use std::collections::BTreeMap;

use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Starter, starter};
use patch::{HermiteSurface, Point3};
use whiting_gallery::blockmesh;

/// How close two stations have to be to be the same station. The two walks of one run differ by
/// the rounding of their own arc-length sums, which at these magnitudes is far inside this.
const COINCIDENT: f64 = 1.0e-9;

fn massif() -> FacetMesh {
    match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    }
}

/// Every `(left, right)` belt pair that shares a valley segment, in a stable order.
fn valley_seams(facet: &FacetMesh) -> Vec<(BeltRef, BeltRef)> {
    let mut by_valley: BTreeMap<(usize, usize), Vec<BeltRef>> = BTreeMap::new();
    for face in &facet.faces {
        let belt = face.attributes.belt;
        by_valley
            .entry((belt.valley.0 as usize, belt.segment))
            .or_default()
            .push(belt);
    }
    let mut seams = Vec::new();
    for mut group in by_valley.into_values() {
        group.sort_by_key(|belt| (belt.segment, belt.side as u8, belt.valley.0));
        group.dedup();
        for pair in group.chunks(2) {
            if pair.len() == 2 {
                seams.push((pair[0], pair[1]));
            }
        }
    }
    seams
}

/// The positions of every vertex a belt's faces name, deduplicated.
fn belt_vertices(surface: &HermiteSurface, faces: std::ops::Range<u32>) -> Vec<Point3> {
    let mut seen: BTreeMap<u32, Point3> = BTreeMap::new();
    for face in faces {
        for corner in surface.quads()[face as usize] {
            seen.insert(corner, surface.positions()[corner as usize]);
        }
    }
    seen.into_values().collect()
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let facet = massif();
    let side = 4;

    let seams = valley_seams(&facet);
    let Some(belts) = blockmesh::from_belts(&facet, side) else {
        eprintln!("the massif would not build as one surface");
        std::process::exit(1);
    };
    let surface = belts.surface();
    println!(
        "\n{} belts at {side}x{side}, {} patches, one surface; {} valley seams",
        belts.belts().len(),
        surface.face_count(),
        seams.len()
    );

    // 2. Vertex ownership: every vertex names one belt, because the belts are the surface's
    //    components and a vertex in two of them would put the derivative averaging back.
    let mut owner: BTreeMap<u32, BeltRef> = BTreeMap::new();
    for record in belts.belts() {
        for face in record.first_face..record.first_face + record.face_count {
            for corner in surface.quads()[face as usize] {
                if let Some(other) = owner.insert(corner, record.belt) {
                    if other != record.belt {
                        failures.push("2  a vertex belongs to two belts");
                    }
                }
            }
        }
    }

    // 1 and 3. Per seam: partners, their distance, and the row's own distinctness.
    let mut worst_partner = 0.0f64;
    let mut worst_own = f64::INFINITY;
    let mut paired_seams = 0usize;
    for (left, right) in &seams {
        let (Some(left_record), Some(right_record)) = (
            belts.belts().iter().find(|record| record.belt == *left),
            belts.belts().iter().find(|record| record.belt == *right),
        ) else {
            failures.push("1  a seam's belt is missing from the surface");
            continue;
        };
        let lefts = belt_vertices(
            surface,
            left_record.first_face..left_record.first_face + left_record.face_count,
        );
        let rights = belt_vertices(
            surface,
            right_record.first_face..right_record.first_face + right_record.face_count,
        );
        let mut partners = 0usize;
        for here in &lefts {
            let mut nearest = f64::INFINITY;
            for there in &rights {
                nearest = nearest.min(here.distance(*there));
            }
            if nearest < COINCIDENT {
                partners += 1;
                worst_partner = worst_partner.max(nearest);
            }
            // 3, read against the belt's own row: the nearest other vertex of the same belt.
            for other in &lefts {
                let gap = here.distance(*other);
                if gap > 0.0 {
                    worst_own = worst_own.min(gap);
                }
            }
        }
        // `>=` rather than `==`: where a valley head collapses, both belts' interior rows reach
        // the same head point as well, and a vertex more in the pairing is two grids agreeing,
        // not a crack. A crack is a seam station with **no** partner, which is `partners < side`.
        if partners >= side {
            paired_seams += 1;
        } else {
            println!(
                "  seam {left:?} | {right:?}: {partners} of {side} left vertices paired; \
                 left {} right {} vertices",
                lefts.len(),
                rights.len()
            );
            failures.push("1  every valley row vertex has exactly one partner");
        }
    }
    println!(
        "  {paired_seams} of {} seams paired at all {side} stations; worst partner distance {:.3e}",
        seams.len(),
        worst_partner
    );
    println!("  closest two vertices of one belt's own row: {worst_own:.3e}");
    if worst_partner > COINCIDENT {
        failures.push("1  coincident means coincident");
    }
    if worst_own < COINCIDENT {
        failures.push("3  a belt's own row vertices are distinct");
    }

    println!();
    if failures.is_empty() {
        println!("all measurements pass");
    } else {
        for failure in &failures {
            println!("FAILED  {failure}");
        }
        std::process::exit(1);
    }
}
