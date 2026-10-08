//! Probes the hierarchy's first real caller: sampling a surface into an indexed mesh.
//!
//! Run with `cargo run -p whiting-patch --example tessellation_probe`. An example and not a gate.
//! **It exits non-zero on any failure.**
//!
//! Everything else in this crate reads control points. `PatchMesh::tessellate` is the first thing
//! here that reads the **surface**, so the questions change shape: not "is this control point right"
//! but "does summing the levels at a parameter give what the caller would draw".
//!
//! 1. **The two read-outs agree.** `evaluate` sums the levels at a parameter; `composed` materialises
//!    the sum and evaluates one patch. They are different code and a caller will use whichever is
//!    convenient, so if they disagree the choice of function changes the answer.
//! 2. **A displacement moves what is sampled.** A sum path that is wired but inert would pass
//!    everything else here.
//! 3. **Sharing is structural, not numerical.** A shared boundary is sampled once and both faces
//!    hold the *same index*, so no tolerance is involved in "they agree".
//! 4. **What independent sampling would cost.** The hazard this design avoids, measured: two faces
//!    sampling the same boundary separately sum the same Bernstein terms in opposite orders, and
//!    `f64` does not round two orders the same way.
//! 5. **Elevation still holds still** through a whole tessellation, on the real massif and not the
//!    sample.
//!
//! Measurement 4 is the one that justifies the module. 3 says the sharing works; 4 says what it is
//! worth.

mod support;

use std::collections::BTreeMap;

use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::topology::{boundary_index, edge_endpoints, mirrored_position};
use whiting_patch::{FaceId, PatchMesh, Point3, Tessellation};

use support::{EXACT, scramble, split_over_shared_edges};

/// Samples per edge, so each face becomes 4 by 4 quads.
///
/// Small on purpose: the questions here are about which points exist and who holds them, and a
/// bigger grid only makes the same answer slower to arrive at.
///
/// **Not finer than the patch carries.** A degree-`d` edge holds `d + 1` positions, so `SIDE - 1`
/// must not exceed the degree; `tessellate` refuses otherwise rather than collapsing samples onto
/// one another. At `DEGREE` 4 this is the finest grid the patch can carry at all, which makes it the
/// interesting case rather than an arbitrary one.
const SIDE: usize = 5;

/// The degree the massif is elevated to before anything is sampled.
///
/// Higher than 1, and the reason is that a degree-1 row carries two positions: summing two terms
/// forwards and backwards is exact in `f64`, so a measurement about summation order reports
/// `0.000e+00` and means nothing. A null result that cannot fail is not a measurement — and
/// measurement 4 was exactly that before this was raised.
const DEGREE: usize = 4;

/// Two read-outs that both round differently may still differ from each other. This is the floor,
/// not a budget: the arithmetic is a sum of the same terms in a different grouping.
const READOUT_ROUNDING: f64 = 1.0e-12;

/// Elevation on an all-hard mesh is exact to rounding. See `facet_front_end_probe`, which measures
/// the same thing on a surface rather than a tessellation of it.
const ELEVATION_ROUNDING: f64 = 1.0e-12;

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

/// The authored massif every measurement here runs on, elevated so `SIDE` samples across a face have
/// distinct positions to land on.
fn real_massif() -> PatchMesh {
    let massif = starter(11, Starter::default());
    let facet: FacetMesh = match massif.build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the sample massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };
    let mut model = match patchmesh::from_facet(&facet) {
        Ok(model) => model,
        Err(problem) => {
            eprintln!("the sample massif is not convertible: {problem}");
            std::process::exit(1);
        }
    };
    for _ in 1..DEGREE {
        model.elevate();
    }
    model
}

/// The largest distance between any two sampled points of `first` and `second`.
///
/// Compared by index, because that is how a tessellation is consumed: the same index is the same
/// point. Two tessellations of the same surface at the same resolution hold the same points in the
/// same order, so this is well defined.
fn worst_movement(first: &Tessellation, second: &Tessellation) -> f64 {
    first
        .points()
        .iter()
        .zip(second.points())
        .map(|(was, now)| was.distance(*now))
        .fold(0.0f64, f64::max)
}

/// How much the two faces of every shared hard edge disagree at **sample** positions.
///
/// This is the cost of not sharing: each face evaluates its own copy of the boundary row, and the
/// two rows are mirrored, so the same Bernstein terms are summed in opposite orders.
fn independent_boundary_split(mesh: &PatchMesh, side: usize) -> (f64, usize) {
    let degree = mesh.degree();
    let mut worst = 0.0f64;
    let mut positions = 0usize;
    for faces in mesh.topology().edge_faces.values() {
        if faces.len() != 2 {
            continue;
        }
        let (owner, follower) = (faces[0], faces[1]);
        let owner_ends = edge_endpoints(mesh.quads()[owner.0 as usize], owner.1);
        let follower_ends = edge_endpoints(mesh.quads()[follower.0 as usize], follower.1);
        for k in 0..side {
            let t = k * degree / (side - 1);
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            worst = worst
                .max(row_sample(mesh, owner, t).distance(row_sample(mesh, follower, follower_t)));
            positions += 1;
        }
    }
    (worst, positions)
}

/// The surface at edge position `t` of `face`'s row.
fn row_sample(mesh: &PatchMesh, face: (FaceId, whiting_patch::EdgeSlot), t: usize) -> Point3 {
    let degree = mesh.degree();
    let (i, j) = boundary_index(degree, face.1, t);
    mesh.evaluate(face.0, i as f64 / degree as f64, j as f64 / degree as f64)
}

/// Every mesh vertex, so a caller can see whether one was sampled once or once per incident edge.
fn vertex_count(mesh: &PatchMesh) -> usize {
    let mut vertices: BTreeMap<VertexIdOf, ()> = BTreeMap::new();
    for corners in mesh.quads() {
        for vertex in corners {
            vertices.insert(*vertex, ());
        }
    }
    vertices.len()
}

type VertexIdOf = u32;

fn main() {
    let mut failures: Vec<String> = Vec::new();

    // A displacement, so every measurement below is about a hierarchy rather than a single level.
    let mut mesh = real_massif();
    let level = mesh.add_offset();
    scramble(&mut mesh.offsets_mut()[level], 1.0);
    mesh.enforce();

    // 1 -- the two read-outs.
    let composed = mesh.composed();
    let mut readout_gap = 0.0f64;
    let mut samples = 0usize;
    for face in 0..mesh.quads().len() as FaceId {
        for (i, j) in [(0usize, 0usize), (1, 2), (3, 1), (4, 4)] {
            let (u, v) = (i as f64 / (SIDE - 1) as f64, j as f64 / (SIDE - 1) as f64);
            let summed = mesh.evaluate(face, u, v);
            let materialised = composed
                .face(face as usize)
                .expect("a face below the face count resolves")
                .evaluate(u, v);
            readout_gap = readout_gap.max(summed.distance(materialised));
            samples += 1;
        }
    }
    if readout_gap > READOUT_ROUNDING {
        failures.push(format!(
            "1  evaluate and composed agree (worst {readout_gap:.3e} over {samples} points)"
        ));
    }
    report(
        "1  the two read-outs agree",
        &format!(
            "evaluate vs composed     {readout_gap:.3e} over {samples} points on {}\n\
             ceiling                  {READOUT_ROUNDING:.3e}\n\
             \n\
             One sums the levels at a parameter, the other materialises the sum and evaluates one\n\
             patch. Same terms, different grouping, so the floor is rounding rather than a budget.",
            mesh.offset_count()
        ),
    );

    // 2 -- a displacement moves what is sampled. Otherwise everything below is about one level.
    let before = mesh.tessellate(SIDE);
    let mut moved = mesh.clone();
    let second = moved.add_offset();
    scramble(&mut moved.offsets_mut()[second], 2.0);
    moved.enforce();
    let after = moved.tessellate(SIDE);
    let displacement_moved = worst_movement(&before, &after);

    if displacement_moved <= EXACT {
        failures.push(format!(
            "2  a second displacement moves the sampled surface (moved {displacement_moved:.3e})"
        ));
    }
    report(
        "2  a displacement moves what is sampled",
        &format!(
            "levels                   {}, then {}\n\
             worst point movement     {displacement_moved:.3e} over {} points",
            mesh.offset_count(),
            moved.offset_count(),
            before.points().len(),
        ),
    );

    // 3 -- sharing is a fact about indices, so no tolerance is involved.
    //
    // The check is per edge rather than a count of shared points, because a count can be satisfied
    // by the wrong thing: a mesh vertex is the end sample of each incident edge, and two faces can
    // reach it through different edges. What has to hold is that the **two faces on an edge hold
    // the same indices**, in one order or the other.
    let uses = after.uses_per_point();
    let shared_points = uses.iter().filter(|count| **count > 1).count();
    let mut unshared_edges = 0usize;
    let mut interior_edges = 0usize;
    for faces in mesh.topology().edge_faces.values() {
        if faces.len() != 2 {
            continue;
        }
        interior_edges += 1;
        let (owner, follower) = (faces[0], faces[1]);
        let first = after.face(owner.0).expect("a face resolves").edge(owner.1);
        let second = after
            .face(follower.0)
            .expect("a face resolves")
            .edge(follower.1);
        let same = first == second || first.iter().rev().copied().eq(second.iter().copied());
        if !same {
            unshared_edges += 1;
        }
    }

    if unshared_edges != 0 {
        failures.push(format!(
            "3  the two faces on an edge hold the same samples ({unshared_edges} of {interior_edges} \
             interior edges do not)"
        ));
    }
    report(
        "3  sharing is structural",
        &format!(
            "faces                    {}\n\
             edges, interior          {interior_edges}\n\
             samples per edge         {SIDE}\n\
             interior edges whose two faces disagree on indices  {unshared_edges}\n\
             points total             {}\n\
             \x20  held by more than one  {shared_points}\n\
             mesh vertices            {}\n\
             \n\
             A shared boundary is one index in one buffer, so \"the two faces agree\" is a fact about\n\
             integers. The per-edge check is the one that means something: a total of shared points\n\
             could be satisfied while a mesh vertex is reached through two different edges by two\n\
             different faces, which is agreement in count and not in place.",
            after.face_count(),
            after.points().len(),
            vertex_count(&mesh),
        ),
    );

    // 4 -- what the sharing is worth: the same question answered the naive way.
    //
    // The control is the point of this measurement. A `0.000e+00` is only worth reading next to a
    // run of the same function that does **not** report zero, and breaking the base level's sharing
    // is the cheapest way to make the two faces' rows genuinely different.
    let (independent, independent_positions) = independent_boundary_split(&mesh, SIDE);
    let mut broken = mesh.clone();
    scramble(broken.level_mut(), 3.0);
    let (control, _) = independent_boundary_split(&broken, SIDE);
    if control <= 0.0 {
        failures.push(
            "4  the measurement can fail: breaking a shared row has to make it non-zero"
                .to_string(),
        );
    }
    report(
        "4  what sampling each face's boundary separately would cost",
        &format!(
            "worst disagreement        {independent:.3e} over {independent_positions} positions\n\
             the same function on a mesh whose rows disagree  {control:.3e}\n\
             \n\
             The control says the measurement can see a disagreement, and the row above says the two\n\
             faces did not have one: on this surface the rows come out equal to the last bit. So the\n\
             case for sampling an edge once is not a numerical one. It is that the alternative stores\n\
             a copy per side, spends the evaluation twice, and leaves a rasteriser to reconcile two\n\
             numbers that happen to match today."
        ),
    );

    // 5 -- elevation through a whole tessellation, all hard.
    let mut hard = real_massif();
    let before_elevation = hard.tessellate(SIDE);
    hard.elevate();
    let after_elevation = hard.tessellate(SIDE);
    let elevation_moved = worst_movement(&before_elevation, &after_elevation);
    let split = split_over_shared_edges(&hard, hard.level()).worst;

    if elevation_moved > ELEVATION_ROUNDING {
        failures.push(format!(
            "5  elevation moves no sampled point on an all-hard massif (moved {elevation_moved:.3e})"
        ));
    }
    if split > EXACT {
        failures.push(format!(
            "5  the elevated massif still shares every hard edge (split {split:.3e})"
        ));
    }
    report(
        "5  elevation holds still through a whole tessellation",
        &format!(
            "worst point movement     {elevation_moved:.3e} over {} points\n\
             worst control-point split {split:.3e}\n\
             degree                   {}",
            before_elevation.points().len(),
            hard.degree(),
        ),
    );

    if failures.is_empty() {
        println!("\nall 5 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
