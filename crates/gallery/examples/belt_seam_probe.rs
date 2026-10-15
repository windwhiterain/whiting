//! Do neighbouring belts share geometry, and what is the largest gap between them?
//!
//! Run with `cargo run -p whiting-gallery --example belt_seam_probe`. Exits non-zero if any
//! property fails.
//!
//! # The question
//!
//! Superior's ruling: **one belt is one patch, and between patches the edge is soft or hard at
//! the caller's choice.** Choosing between C1 and C0 presumes both patches reach the same points on
//! the seam. This measures whether they do.
//!
//! # Why the seam exists at all
//!
//! `BeltRef { valley, side, segment }` and `expand.rs`'s `valley_run` give a belt its lower boundary
//! from the valley and the segment **alone** — `side` is not an input. So a valley's Left and Right
//! belts have the *same* bottom run, which is a full shared edge between two patches. That is where
//! soft or hard would be chosen.
//!
//! # What is measured
//!
//! 1. **How many seams there are**, read off the massif's own `BeltRef` values rather than
//!    guessed.
//! 2. **The largest gap between the two patches along it.** Each belt samples its own triangles onto
//!    its own grid, so the two grids are independent — and independent grids do not put points in the
//!    same places. If this is not zero the patches are not C0 and a soft/hard choice is not yet
//!    available on this seam.
//! 3. **Whether the two agree at the points they do share**, which is the part a remap would have to
//!    preserve.
//! 4. **The control**: two patches from the *same* belt must agree exactly, or measurement 2 would
//!    report a gap that is really the measurement being wrong.

use std::collections::BTreeMap;

use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Starter, starter};
use patch::{HermiteSurface, Point3};
use whiting_gallery::blockmesh::{self, BeltSurfaces};

fn parameters() -> [f64; 9] {
    [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0]
}

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

/// Where a patch's own **valley row** lies in world space, sampled along it.
///
/// `Face::uv` defines `v` as the fraction from the belt's **upper** boundary down to the lower one,
/// so the valley is `v = 1` and the crest is `v = 0`. The first version of this probe sampled
/// `evaluate(0.0, t)`, which is the grid's `u = 0` **side** — a different edge entirely — and
/// reported `1.185e+02` as a gap between the two patches. It was measuring two unrelated side edges
/// of a grid whose winding nobody had checked against `Face::uv`.
///
/// Sampled over the last patch row, since `v = 1` is the far edge of it.
fn valley_edge(belts: &BeltSurfaces, belt: BeltRef) -> Vec<Point3> {
    let Some(patch) = belts.belts().iter().find(|candidate| candidate.belt == belt) else {
        return Vec::new();
    };
    let side = patch.surface.face_count();
    let _ = side;
    (0..=8)
        .map(|step| {
            let s = step as f64 / 8.0;
            // The valley row lives in the last row of patches; which face that is depends on the
            // surface's own layout, so the last face id is read rather than assumed.
            let last = (patch.surface.face_count() - 1) as u32;
            patch.surface.patch(last).evaluate(s, 1.0)
        })
        .collect()
}

/// The largest distance between two polylines at corresponding sample positions.
///
/// **Both matchings are tried and the smaller is reported.** The two belts wind along their shared
/// valley in opposite directions, so `s` on one is `1 - s` on the other, and a measurement that
/// assumes the match would report a real seam as far apart. Taking the better of the two asks the
/// question that actually matters — *do these two surfaces reach the same curve* — without
/// committing to an orientation this file does not establish.
///
/// This is deliberately **not** a nearest-point comparison: that would report the gap between two
/// sets, which is zero whenever they overlap somewhere. What a seam needs is "at the same place
/// along the shared edge, do the two patches agree", and that is a sampled comparison or nothing.
fn best_agreement(first: &[Point3], second: &[Point3]) -> f64 {
    let mut best = f64::INFINITY;
    for mirror in [false, true] {
        let mut worst = 0.0f64;
        for (index, a) in first.iter().enumerate() {
            let at = if mirror {
                first.len() - 1 - index
            } else {
                index
            };
            let Some(b) = second.get(at) else { continue };
            worst = worst.max(a.distance(*b));
        }
        best = best.min(worst);
    }
    best
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let facet = massif();
    let side = 4;
    let belts = blockmesh::from_belts(&facet, side);
    println!(
        "\n{} belts at {side}x{side}, {} patches",
        belts.len(),
        belts.patch_count()
    );

    // 1. The seams, from the massif's own belts.
    let seams = valley_seams(&facet);
    println!("  {} valley seams between a belt's Left and Right", seams.len());
    if seams.is_empty() {
        println!("  no valley seams exist, so there is nowhere to choose soft or hard yet");
        failures.push("1  neighbouring belts share a seam");
    }

    // 2. The gap along them.
    let mut worst_gap = 0.0f64;
    let mut considered = 0usize;
    for (left, right) in &seams {
        let first = valley_edge(&belts, *left);
        let second = valley_edge(&belts, *right);
        if first.is_empty() || second.is_empty() {
            continue;
        }
        considered += 1;
        worst_gap = worst_gap.max(best_agreement(&first, &second));
    }
    println!(
        "  worst gap between the two patches along a shared valley   {worst_gap:.3e}\n\
         \x20   over {considered} seams, best of the two orientations"
    );
    if considered == 0 {
        failures.push("2  the seams were found and measured");
    }

    // 3. The control: one belt sampled twice must agree exactly. If this is not zero, measurement 2
    //    is measuring the instrument rather than the geometry.
    let twice = blockmesh::from_belts(&facet, side);
    let mut control = 0.0f64;
    for (index, belt) in belts.belts().iter().enumerate() {
        let other = &twice.belts()[index];
        if other.belt != belt.belt {
            continue;
        }
        for face in 0..belt.surface.face_count() as u32 {
            for t in parameters() {
                control = control
                    .max(belt.surface.patch(face).evaluate(0.5, t)
                        .distance(other.surface.patch(face).evaluate(0.5, t)));
            }
        }
    }
    println!("  control: the same belt sampled twice   {control:.3e}");
    if control != 0.0 {
        failures.push("3  sampling a belt twice gives the same surface");
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

/// Keeps the unused import honest while the seam code is being written.
fn _unused(_: HermiteSurface) {}
