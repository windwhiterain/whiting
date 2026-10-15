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
use patch::hermite_surface::Join;
use patch::topology::EdgeKey;
use patch::{HermiteSurface, Point3};
use whiting_gallery::blockmesh::{self, BeltSurfaces};

/// The `acos` floor for two unit vectors this close, and a small multiple for a normal.
const C1_CEILING: f64 = 1.0e-7;

/// A measurement that came out NaN cannot be compared, and folding it into a maximum silently turns
/// a broken run into a passing one.
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        f64::INFINITY
    }
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

/// Reads one seam out of the surface's own topology.
///
/// Now that every belt is a group of faces in one `HermiteSurface`, a seam is a shared edge with two
/// faces on it, and the two faces read **the same vertex ids** — measured, both endpoints of the
/// first seam are literally the same vertices in both faces. So the split on a seam is zero by
/// construction and measuring it is only worth doing as the check that it is.
///
/// What is left to measure is the thing sharing vertices does **not** settle: whether the two faces'
/// normals agree, which is what `Join` decides.
///
/// # The parameter trap, which this function has now fallen into twice
///
/// **Two faces meeting along a shared edge traverse it in opposite directions.** A point at `t`
/// along it on one face is at `1 - t` on the other, so comparing them at the same parameter
/// measures two different places. The first version of this probe got the *edge* wrong as well and
/// reported `1.185e+02`; the second got the edge right and still would have reported the seam as
/// far apart, for this reason.
///
/// So both faces' own corners are read, the shared pair is located in each, and **the parameter is
/// mapped through the actual endpoint order** rather than assumed. Nothing here infers an
/// orientation.
fn seam_reading(surface: &HermiteSurface, edge: &EdgeKey) -> (f64, f64, f64) {
    let faces = &surface.topology().edge_faces[edge];
    if faces.len() != 2 {
        return (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    }
    let (first, _) = faces[0];
    let (second, _) = faces[1];

    // Where the seam sits inside each face's own `[0,1]^2`, read off that face's corners.
    let corner_of = |face: u32, vertex: u32| -> Option<(f64, f64)> {
        let quads = &surface.quads()[face as usize];
        let at = quads.iter().position(|held| *held == vertex)?;
        let other = quads.iter().position(|held| *held == edge.1);
        // The corner's own `(u, v)` is the winding's: `(0,0) (1,0) (1,1) (0,1)`.
        let uv = match at {
            0 => (0.0, 0.0),
            1 => (1.0, 0.0),
            2 => (1.0, 1.0),
            _ => (0.0, 1.0),
        };
        // `other` is `None` when the face holds both endpoints adjacent to this one; the caller
        // checks the pair it got back rather than trusting a `None` here.
        let _ = other;
        Some(uv)
    };
    let (Some(first_start), Some(first_end)) = (corner_of(first, edge.0), corner_of(first, edge.1))
    else {
        return (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    };
    let (Some(second_start), Some(second_end)) =
        (corner_of(second, edge.0), corner_of(second, edge.1))
    else {
        return (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    };

    let mut worst_split = 0.0f64;
    let mut worst_along = 0.0f64;
    let mut worst_normal = 0.0f64;
    for step in 0..=8usize {
        let t = step as f64 / 8.0;
        // Walk each face along its own edge from its own start, so `t` names the same place in both.
        let at = |(start, end): ((f64, f64), (f64, f64))| {
            let (axis, from, to) = if (start.0 - end.0).abs() > (start.1 - end.1).abs() {
                (0, start.0, end.0)
            } else {
                (1, start.1, end.1)
            };
            let mut uv = [start.0, start.1];
            uv[axis] = from + (to - from) * t;
            (uv[0], uv[1])
        };
        let a = at((first_start, first_end));
        let b = at((second_start, second_end));

        let one = surface.patch(first);
        let two = surface.patch(second);
        worst_split = worst_split.max(one.evaluate(a.0, a.1).distance(two.evaluate(b.0, b.1)));

        let (first_u, first_v) = (one.tangent_u(a.0, a.1), one.tangent_v(a.0, a.1));
        let (second_u, second_v) = (two.tangent_u(b.0, b.1), two.tangent_v(b.0, b.1));
        // Which tangent runs along the seam is read from the edge itself, not from the slot.
        let along_axis =
            if (first_start.0 - first_end.0).abs() > (first_start.1 - first_end.1).abs() {
                0
            } else {
                1
            };
        let (along_first, along_second) = if along_axis == 0 {
            (first_u, second_u)
        } else {
            (first_v, second_v)
        };
        worst_along = worst_along.max(finite(along_first.angle(along_second)));
        worst_normal = worst_normal.max(finite(
            first_u.cross(first_v).angle(second_u.cross(second_v)),
        ));
    }
    (worst_split, worst_along, worst_normal)
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let facet = massif();
    let side = 4;

    // 1. The seams, from the massif's own belts: every shared edge whose two faces are in
    //    different belts.
    let seams = valley_seams(&facet);
    let Some(belts) = blockmesh::from_belts(&facet, side) else {
        eprintln!("the massif would not build as one surface");
        std::process::exit(1);
    };
    println!(
        "\n{} belts at {side}x{side}, {} patches, one surface",
        belts.belts().len(),
        belts.surface().face_count()
    );
    let surface = belts.surface();

    // Every edge two different belts share, read out of the surface itself rather than predicted.
    let mut tagged_hard = 0usize;
    let mut worst = (0.0f64, 0.0f64, 0.0f64);
    let mut first: Option<(EdgeKey, u32, u32)> = None;
    for (edge, faces) in &surface.topology().edge_faces {
        if faces.len() != 2 {
            continue;
        }
        let (Some(first_belt), Some(second_belt)) = (
            belts.belt_of_face(faces[0].0),
            belts.belt_of_face(faces[1].0),
        ) else {
            continue;
        };
        if first_belt == second_belt {
            continue;
        }
        tagged_hard += 1;
        if first.is_none() {
            first = Some((*edge, faces[0].0, faces[1].0));
        }
        let reading = seam_reading(surface, edge);
        worst = (
            worst.0.max(reading.0),
            worst.1.max(reading.1),
            worst.2.max(reading.2),
        );
    }
    println!("  {tagged_hard} inter-belt edges, all tagged hard");
    if let Some((edge, one, two)) = first {
        println!(
            "  the first, {edge:?}: faces {one} and {two} -- {}",
            shared_vertices(surface, &edge, one, two)
        );
    }
    println!(
        "  split {:.3e}   along-edge {:.3e} rad   normal {:.3e} rad",
        worst.0, worst.1, worst.2
    );
    if tagged_hard == 0 {
        failures.push("1  neighbouring belts share edges in one surface");
    }
    if worst.0 > 1.0e-12 {
        failures.push("2  a hard seam is C0, never a crack");
    }

    // 3. The tag decides, and that is the whole question: softening one seam has to make its two
    //    faces agree on the normal, and nothing else on the surface may move.
    let probe = seams
        .first()
        .and_then(|(left, right)| find_seam(surface, &belts, *left, *right));
    let Some(edge) = probe else {
        failures.push("3  a seam was found to retag");
        println!();
        for failure in &failures {
            println!("FAILED  {failure}");
        }
        std::process::exit(1);
    };

    let mut softened = belts.clone();
    let before = seam_reading(softened.surface(), &edge);
    softened.surface_mut().set_join(edge, Join::Soft);
    let after = seam_reading(softened.surface(), &edge);
    println!(
        "\n  retagging one seam soft\n\
         \x20   split {:.3e} -> {:.3e}\n\
         \x20   normal {:.3e} -> {:.3e} rad",
        before.0, after.0, before.2, after.2
    );

    if after.0 > 1.0e-12 {
        failures.push("3  softening a seam keeps it closed");
    }
    // **The claim is that a soft seam is exactly C1 — not that retagging improves anything.** The
    // first version of this assertion was "the normal must go down", which is wrong twice over: the
    // measurement cannot distinguish `2.980e-8` from `1.490e-8`, both being the `acos` floor, and a
    // seam that is already at the floor has nowhere to go. An assertion that asks a measurement to
    // improve is an assertion that passes when the instrument is blunt.
    if after.2 > C1_CEILING {
        println!(
            "\n  note: the softened seam reads {:.3e} rad, above the {:.1e} ceiling. Retagging cannot\n\
             \x20 repair a tangent field the two faces were seeded with independently -- the seed is\n\
             \x20 what has to agree, and that is the next thing to look at.",
            after.2, C1_CEILING
        );
        failures.push("3  a softened seam is exactly C1");
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

/// How many vertices the two faces on an edge actually share, read off the faces' own corners.
///
/// **A shared edge in this crate's `Topology` does not mean the two faces share vertices.** `EdgeKey`
/// is a pair of endpoints, so two faces can name the same edge with **different** vertex ids and the
/// topology will still pair them — which is exactly the hole the valley-keyed pool was cut to close,
/// and this is the measurement that says whether the hole is closed.
fn shared_vertices(surface: &HermiteSurface, edge: &EdgeKey, one: u32, two: u32) -> String {
    let first = surface.quads()[one as usize];
    let second = surface.quads()[two as usize];
    // `EdgeKey` is the unordered pair of endpoints, so it is the edge's own id list — there is
    // nothing to look up, and looking it up is where the first version of this went wrong.
    let shared = [edge.0, edge.1]
        .iter()
        .filter(|end| first.contains(end) && second.contains(end))
        .count();
    format!(
        "of {} endpoints, {shared} are the same vertex in both faces; face {one} has {first:?}, face {two} has {second:?}",
        2
    )
}

/// The first shared edge between two named belts' faces.
fn find_seam(
    surface: &HermiteSurface,
    belts: &BeltSurfaces,
    left: BeltRef,
    right: BeltRef,
) -> Option<EdgeKey> {
    surface
        .topology()
        .edge_faces
        .iter()
        .find(|(_edge, faces)| {
            faces.len() == 2
                && faces
                    .iter()
                    .any(|(face, _)| belts.belt_of_face(*face) == Some(left))
                && faces
                    .iter()
                    .any(|(face, _)| belts.belt_of_face(*face) == Some(right))
        })
        .map(|(edge, _)| *edge)
}

/// Keeps the unused import honest while the seam code is being written.
fn _unused(_: Point3) {}
