//! Probes the hierarchy: a base level and displacement levels above it, enforced one level at a time.
//!
//! Run with `cargo run -p whiting-patch --example layers_probe`. An example and not a gate. **It exits
//! non-zero on any failure.**
//!
//! 1. A built mesh holds no displacement levels, and a fresh one is exactly zero — before *and* after
//!    enforcement.
//! 2. Enforcement reaches the displacement levels. The positive control for 1: a level broken on
//!    purpose is repaired by the same pass.
//! 3. **Levels are independent**: enforcing each level and adding gives the same surface as enforcing
//!    the sum. This is the load-bearing claim of the whole layered design.
//! 4. Clearing a level returns the surface to the base exactly, and dropping it is the difference
//!    between clearing and not having it.
//! 5. Elevation raises every level together. On an all-hard model it moves the surface by rounding,
//!    hierarchy included; with soft edges it does not, and that is the accepted gap again.
//! 6. Enforcement leaves every level **whole**: no shared edge split, and one value per mesh vertex
//!    across all the faces around it. This one found a fault — see below.
//!
//! ## 1 and 3 are one argument in two halves
//!
//! Write enforcement as a map `E`. Enforcing levels separately gives `E(base) + E(offset)`; enforcing
//! their sum gives `E(base + offset)`. For an **affine** `E(x) = M x + c` those agree only when
//! `c = 0`, because the first carries the constant twice and the second once. So the property that
//! makes a hierarchy storable as displacements is not affineness but **homogeneity**, and that is two
//! separate claims: measurement 1 shows zero in, zero out — which is `c = 0`, the only evidence there
//! is for it — and measurement 3 shows the composition that depends on it. Neither alone is the
//! argument; a probe asserting just one of them would pass on a design that is broken.
//!
//! This was the one proposition in the design argued but never measured, and this session has already
//! been wrong once by believing an argument instead of a number.
//!
//! ## What measurement 6 found
//!
//! A face's corner is the end of the shared boundary row of **each** of the two edges meeting at
//! that vertex, so two sharings write it and the later one wins. On a base level every face's corner
//! is the same `positions` entry to begin with, the copies start out equal, and the ordering never
//! shows. On a displacement level nothing forces them to start out equal, and what was left over
//! measured `3.666e-1` across 19 corners — every one of them at a row end, and nowhere else in the
//! row, which is the fingerprint of a contested vertex rather than of a mis-shared edge.
//!
//! `enforce` now reconciles each vertex the way `set_control_point` always has, and measurement 5's
//! all-hard row with a displacement went from `1.223e-2` to rounding. A probe that had only ever run
//! on a base level could not have found this.

mod support;
use std::collections::BTreeSet;

use support::{EXACT, grid, level_distance, level_magnitude, scramble, split_over_shared_edges};

use whiting_patch::enforce::enforce as enforce_level;
use whiting_patch::subdivide::subdivide;
use whiting_patch::topology::EdgeKey;
use whiting_patch::{FaceId, Level, PatchMesh, Point3};

/// A distance this small is indistinguishable from nothing having happened.
const MOVED: f64 = 1.0e-6;

/// The largest gap between enforcing the levels and enforcing their sum that floating-point rounding
/// alone explains.
///
/// The two routes agree in exact arithmetic and are **not** bit-identical, because `f(x) + g(y)` and
/// `f(x + y)` round differently. The scrambled coordinates reach about `5e0`, so a handful of
/// operations put the rounding near `1e-15`. The ceiling leaves three orders of headroom and is still
/// eleven orders below the `1e0` a genuine disagreement would produce.
const ROUNDING: f64 = 1.0e-12;

/// The most elevating a model with soft edges is allowed to move its surface.
///
/// **A regression guard, not a target.** Elevating a model that carries soft edges is *not* a
/// shape-preserving operation in this design, and the number is not a bug to be chased to zero: it is
/// the accepted gap of `crate::continuity`, dragged inward. The soft invariant leaves the two ends
/// of a slave's row un-derived because they belong to the perpendicular edge's shared boundary, and
/// elevation blends those un-derived ends into the derived ones, so the re-derived row sits slightly
/// off where the surface was.
///
/// The contrast that localises it is measurement 5's two rows: with every edge hard, and a scrambled
/// displacement level present, elevation moves the surface by rounding alone.
const ELEVATION_SEAM_MOVE: f64 = 2.0e-1;

const DEGREE: usize = 3;

/// How many interior edges the sample makes soft. Enough that sharing and the soft invariant both run
/// over every corner of the mesh.
const SOFT_EDGES: usize = 32;

/// How much two faces of a shared hard edge disagree, in one line for a report.
fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

/// The sample every probe in this crate uses, subdivided once, at degree 3, with the first `soft`
/// interior edges soft and no displacement levels.
///
/// Deterministic throughout — the edges come out of a `BTreeMap` in key order and the grid is fixed —
/// so two calls build identical models. Measurement 3 starts both of its routes from a state built
/// this way, and identical starting states are what makes its comparison mean anything.
fn sample(soft: usize) -> PatchMesh {
    let (positions, triangles) = grid();
    let derived = subdivide(&positions, &triangles);
    let mut mesh = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for edge in interior_edges(&mesh).into_iter().take(soft) {
        mesh.set_soft(edge);
    }
    for _ in 1..DEGREE {
        mesh.elevate();
    }
    mesh
}

fn interior_edges(mesh: &PatchMesh) -> Vec<EdgeKey> {
    mesh.topology()
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 2)
        .map(|(key, _)| *key)
        .collect()
}

/// `(face, u, v)` triples spread over the mesh, for comparing surfaces rather than control points.
///
/// Levels of different degrees hold different numbers of points, so a surface that survived an
/// elevation can only be compared where it is evaluated.
fn surface_samples(mesh: &PatchMesh) -> Vec<(FaceId, f64, f64)> {
    let mut samples = Vec::new();
    for face in 0..mesh.level().len() {
        for (u, v) in [(0.25f64, 0.25f64), (0.5, 0.75), (0.75, 0.125)] {
            samples.push((face as FaceId, u, v));
        }
    }
    samples
}

fn evaluate_all(mesh: &PatchMesh, samples: &[(FaceId, f64, f64)]) -> Vec<Point3> {
    samples
        .iter()
        .map(|&(face, u, v)| mesh.evaluate(face, u, v))
        .collect()
}

fn worst_distance(first: &[Point3], second: &[Point3]) -> f64 {
    first
        .iter()
        .zip(second)
        .map(|(was, now)| was.distance(*now))
        .fold(0.0f64, f64::max)
}

/// The worst disagreement between the faces around a mesh vertex about what that vertex is.
///
/// A vertex is shared by every face around it and by each incident edge, so it is the one place a
/// level can be internally inconsistent while every individual shared edge still agrees with itself.
fn worst_vertex_split(level: &Level, quads: &[[u32; 4]]) -> f64 {
    let degree = level.degrees().0;
    let mut vertices: BTreeSet<u32> = BTreeSet::new();
    for corners in quads {
        vertices.extend(corners.iter().copied());
    }
    let mut worst = 0.0f64;
    for vertex in vertices {
        let held: Vec<Point3> = quads
            .iter()
            .enumerate()
            .filter_map(|(face, corners)| {
                corners
                    .iter()
                    .position(|corner| *corner == vertex)
                    .map(|corner| {
                        let (i, j) = match corner {
                            0 => (0, 0),
                            1 => (degree, 0),
                            2 => (degree, degree),
                            _ => (0, degree),
                        };
                        level.face(face).expect("a face resolves")[(i, j)]
                    })
            })
            .collect();
        if let Some(first) = held.first().copied() {
            for other in &held {
                worst = worst.max(first.distance(*other));
            }
        }
    }
    worst
}

/// How far one elevation moves this mesh's surface, sampled over its own faces.
///
/// A surface is the only thing comparable across degrees — a level before and after holds different
/// numbers of points — so this is the measurement elevation has to be judged by.
fn elevation_moves(mesh: &mut PatchMesh) -> f64 {
    let samples = surface_samples(mesh);
    let before = evaluate_all(mesh, &samples);
    mesh.elevate();
    worst_distance(&before, &evaluate_all(mesh, &samples))
}

fn main() {
    let mut failures: Vec<String> = Vec::new();

    // 1 -- an unused hierarchy costs nothing, and a fresh level is exactly zero.
    let mut mesh = sample(SOFT_EDGES);
    let empty_count = mesh.offset_count();
    let index = mesh.add_offset();
    let zero_before = level_magnitude(&mesh.offsets()[index]);
    mesh.enforce();
    let zero_after = level_magnitude(&mesh.offsets()[index]);
    let zero_surface = level_distance(&mesh.level().clone(), &mesh.composed());

    if empty_count != 0 {
        failures.push("1  a built mesh starts with no displacement levels".to_string());
    }
    if zero_before != 0.0 || zero_after != 0.0 {
        failures.push(format!(
            "1  a fresh level is exactly zero (before {zero_before:.3e}, after {zero_after:.3e})"
        ));
    }
    if zero_surface != 0.0 {
        failures.push(format!(
            "1  summing a zero level changes nothing (moved {zero_surface:.3e})"
        ));
    }
    report(
        "1  an unused hierarchy, and a fresh level that stays zero",
        &format!(
            "levels at build            {empty_count}\n\
             largest control point      {zero_before:.3e} before enforcement\n\
             \x20                          {zero_after:.3e} after enforcement\n\
             surface moved by summing  {zero_surface:.3e}"
        ),
    );

    // 2 -- the positive control. A zero result in 1 is indistinguishable from a loop that skipped the
    // offsets, so break one on purpose and watch the same pass repair it.
    let mut mesh = sample(SOFT_EDGES);
    let index = mesh.add_offset();
    scramble(&mut mesh.offsets_mut()[index], 1.0);
    let broken = mesh.offsets()[index].clone();
    mesh.enforce();
    let repaired = &mesh.offsets()[index];
    let repaired_by = level_distance(&broken, repaired);
    let still_there = level_magnitude(repaired);

    if repaired_by <= MOVED {
        failures.push(format!(
            "2  enforcement reached the displacement level (moved {repaired_by:.3e})"
        ));
    }
    if still_there <= MOVED {
        failures.push(format!(
            "2  enforcement edits a displacement level rather than wiping it (left {still_there:.3e})"
        ));
    }
    report(
        "2  the pass really does reach a displacement level",
        &format!(
            "control points written     {repaired_by:.3e}\n\
             displacement left behind  {still_there:.3e}"
        ),
    );

    // 3 -- the load-bearing one. Enforcing each level and adding, against enforcing the sum.
    //
    // Both the base and the displacement are broken first, so the map is doing real work on both
    // sides and a pass that quietly skipped either one would show up as a gap rather than as
    // agreement.
    let mut per_level = sample(SOFT_EDGES);
    let index = per_level.add_offset();
    scramble(per_level.level_mut(), 0.0);
    scramble(&mut per_level.offsets_mut()[index], 1.0);
    per_level.enforce();
    let levelwise = per_level.composed();

    let mut whole = sample(SOFT_EDGES);
    let index = whole.add_offset();
    scramble(whole.level_mut(), 0.0);
    scramble(&mut whole.offsets_mut()[index], 1.0);
    let mut summed = whole.composed();
    enforce_level(&whole.rules(), &mut summed);

    let independent = level_distance(&levelwise, &summed);
    if independent > ROUNDING {
        failures.push(format!(
            "3  levels are independent: enforcing each is not the same as enforcing the sum \
             (off by {independent:.3e})"
        ));
    }
    report(
        "3  enforcing each level is the same as enforcing the sum",
        &format!(
            "base and displacement both broken first, so the map has work on both sides\n\
             worst difference            {independent:.3e}\n\
             ceiling (rounding only)     {ROUNDING:.3e}"
        ),
    );

    // 4 -- clearing a level takes the surface back to the base, exactly.
    let mut mesh = sample(SOFT_EDGES);
    let index = mesh.add_offset();
    scramble(&mut mesh.offsets_mut()[index], 1.0);
    mesh.enforce();
    let loaded = mesh.composed();
    let gain = level_distance(&mesh.level().clone(), &loaded);

    let cleared_ok = mesh.clear_offset(index);
    let cleared = mesh.composed();
    let undone = level_distance(&loaded, &cleared);
    let is_base = cleared == mesh.level().clone();

    let dropped_ok = mesh.drop_offset(index);
    let dropped_out_of_range = mesh.clear_offset(0);
    let count_after = mesh.offset_count();

    // The layer has to have done something before "clearing it undoes that" means anything.
    if gain <= MOVED {
        failures.push(format!(
            "4  the displacement did something before clearing it (moved {gain:.3e})"
        ));
    }
    if !cleared_ok || !is_base || undone <= MOVED {
        failures.push(format!(
            "4  clearing a level returns the surface to the base (undid {undone:.3e}, is base \
             {is_base})"
        ));
    }
    if !dropped_ok || dropped_out_of_range || count_after != 0 {
        failures.push(format!(
            "4  dropping a level is not the same as clearing one (count {count_after}, \
             out-of-range clear reported {dropped_out_of_range})"
        ));
    }
    report(
        "4  clearing a level returns to the base, exactly",
        &format!(
            "surface moved by the level   {gain:.3e}\n\
             undone by clearing           {undone:.3e}\n\
             cleared level is the base    {is_base}, compared with `==` rather than a tolerance\n\
             after dropping it            {count_after} levels"
        ),
    );

    // 5 -- elevation raises every level together, and what it does to the surface.
    //
    // The degrees move in step; that is structural and always true. Whether the *surface* stays put
    // is a different question, and it is asked as a two-by-two: soft edges or not, a displacement
    // level or not. Anything that moves the surface in one corner of that table and not another is
    // attributed by the table rather than by an argument.
    let mut bare_hard = sample(0);
    let hard_bare = elevation_moves(&mut bare_hard);

    let mut hier_hard = sample(0);
    hier_hard.add_offset();
    scramble(&mut hier_hard.offsets_mut()[0], 1.0);
    hier_hard.enforce();
    let hard_hier = elevation_moves(&mut hier_hard);

    let mut bare_soft = sample(SOFT_EDGES);
    let soft_bare = elevation_moves(&mut bare_soft);

    let mut hier_soft = sample(SOFT_EDGES);
    hier_soft.add_offset();
    scramble(&mut hier_soft.offsets_mut()[0], 1.0);
    hier_soft.enforce();
    let soft_hier = elevation_moves(&mut hier_soft);

    let base_degree = hier_soft.level().degrees().0;
    let offset_degree = hier_soft.offsets()[0].degrees().0;

    if base_degree != DEGREE + 1 || offset_degree != DEGREE + 1 {
        failures.push(format!(
            "5  elevation raises every level (base {base_degree}, displacement {offset_degree})"
        ));
    }
    // The all-hard row is the control: it is what an exact elevation looks like, and it is what says
    // the hierarchy adds nothing to it.
    if hard_bare > EXACT || hard_hier > EXACT {
        failures.push(format!(
            "5  elevating an all-hard model moves nothing, hierarchy included (bare \
             {hard_bare:.3e}, with a displacement {hard_hier:.3e})"
        ));
    }
    if soft_bare > ELEVATION_SEAM_MOVE || soft_hier > ELEVATION_SEAM_MOVE {
        failures.push(format!(
            "5  elevating a model with soft edges stays within the seam's own scale (bare \
             {soft_bare:.3e}, with a displacement {soft_hier:.3e})"
        ));
    }
    report(
        "5  elevation raises every level; soft edges are what moves the surface",
        &format!(
            "degrees                      base {base_degree}, displacement {offset_degree}\n\
             \n\
             \x20                          no displacement   one displacement\n\
             every edge hard            {hard_bare:.3e}         {hard_hier:.3e}\n\
             {SOFT_EDGES} soft edges           {soft_bare:.3e}         {soft_hier:.3e}\n\
             \n\
             Worst movement of any sampled surface point, over {} samples each. The all-hard row is\n\
             rounding, so a hierarchy does not make elevation lossy. The soft row does not go to zero\n\
             with or without a hierarchy, and that is the same accepted gap the two rows share: the\n\
             invariant leaves a slave's row ends un-derived, elevation blends them inward, and the\n\
             re-derived row sits slightly off where the surface was.",
            surface_samples(&hier_soft).len()
        ),
    );

    // 6 -- enforcement leaves every level whole: no shared edge split, and one value per mesh
    // vertex across all the faces around it.
    //
    // This is the measurement that found a real fault. A face's corner is the end of the shared
    // boundary row of *each* of the two edges meeting at that vertex, so two sharings write it and
    // the later one wins. On a base level the copies start out equal — every face's corner is the
    // same `positions` entry — and the ordering never shows. On a displacement level nothing forces
    // them to start out equal, and what was left over was measured at `3.666e-1` across 19 corners,
    // every one of them at a row end and nowhere else in the row.
    let mut mesh = sample(0);
    mesh.add_offset();
    scramble(mesh.level_mut(), 0.0);
    scramble(&mut mesh.offsets_mut()[0], 1.0);
    let before_base_split = split_over_shared_edges(&mesh, mesh.level());
    let before_offset_split = split_over_shared_edges(&mesh, &mesh.offsets()[0]);
    let before_vertex_split = worst_vertex_split(mesh.level(), mesh.quads());

    mesh.enforce();

    let after_base_split = split_over_shared_edges(&mesh, mesh.level());
    let after_offset_split = split_over_shared_edges(&mesh, &mesh.offsets()[0]);
    let base_vertex_split = worst_vertex_split(mesh.level(), mesh.quads());
    let offset_vertex_split = worst_vertex_split(&mesh.offsets()[0], mesh.quads());

    let mut settled = mesh.offsets()[0].clone();
    enforce_level(&mesh.rules(), &mut settled);
    let idempotent = level_distance(&mesh.offsets()[0], &settled);

    if after_base_split.worst > EXACT || after_offset_split.worst > EXACT {
        failures.push(format!(
            "6  every shared hard edge is one curve on every level (base {:.3e}, displacement \
             {:.3e})",
            after_base_split.worst, after_offset_split.worst
        ));
    }
    if before_base_split.positions == 0 || before_base_split.worst <= EXACT {
        failures.push(
            "6  the levels were already whole before enforcement, so this ran nothing".to_string(),
        );
    }
    if base_vertex_split > EXACT || offset_vertex_split > EXACT {
        failures.push(format!(
            "6  a mesh vertex holds one value on every level (base {base_vertex_split:.3e}, \
             displacement {offset_vertex_split:.3e})"
        ));
    }
    if idempotent != 0.0 {
        failures.push(format!(
            "6  enforcement is a fixed point (moved {idempotent:.3e})"
        ));
    }
    report(
        "6  enforcement leaves every level whole",
        &format!(
            "disagreement between the two faces of a shared hard edge:\n\
             \x20                                 base {before_base_split}, displacement {before_offset_split}, before\n\
             \x20                                 base {after_base_split}, displacement {after_offset_split}, after\n\
             \n\
             one value per mesh vertex: {before_vertex_split:.3e} before, {base_vertex_split:.3e} on the base\n\
             \x20                             and {offset_vertex_split:.3e} on the displacement, after\n\
             enforcing a second time: {idempotent:.3e}\n\
             \n\
             Every shared edge and every vertex is whole after one pass, on both levels, and the pass\n\
             is a fixed point. Before it, each level carried {} split boundary positions and no\n\
             vertex agreement at all.",
            before_base_split.positions
        ),
    );

    if failures.is_empty() {
        println!("\nall 6 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        println!(
            "\n3 is the claim the layered design rests on, and it only holds because 1 shows the\n\
             enforcement map has no constant term. A run that fails 3 while passing 1 is rounding\n\
             or an ordering change; a run that fails 1 has lost homogeneity and 3 is meaningless."
        );
        std::process::exit(1);
    }
}
