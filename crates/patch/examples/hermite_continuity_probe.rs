//! Does a Hermite surface with a per-edge hard/soft tag give what the tag promises?
//!
//! Run with `cargo run -p whiting-patch --example hermite_continuity_probe`. An example and not a
//! gate, following the other probes in this crate. It exits non-zero if any property fails.
//!
//! **This measures the crate.** [`HermiteSurface`] and [`HermitePatch`] are the code under test and
//! are imported, which is the opposite of the other probes in this directory: those keep their own
//! evaluator so the check is independent of the implementation, and this one does not need that,
//! because the Hermite implementation is *the* thing being measured and a hand-written second copy
//! of it would only prove that two copies agree. What is kept independent is the **Bernstein basis**
//! in measurement 4 — a different set of functions of the same degree, evaluated the long way from
//! the binomial definition — so "a Hermite boundary is the same curve as the Bernstein row it
//! replaces" is compared rather than assumed.
//!
//! 0. **Control.** Break one edge and require the C1 measurement to fire on it, and to stay silent
//!    everywhere else. Two arms, and the second one is the point: perturbing a surface everywhere at
//!    once and measuring it measures nothing, because both faces on each touched seam move
//!    together.
//! 1. Every edge soft: **exactly C1** across every interior seam.
//! 2. What the tag buys. The same seam, soft and then hard. `split` and `along` stay exact in both;
//!    only `across` moves.
//! 3. A hard edge: shared boundary, free interiors. A drag on either side moves that side, and the
//!    seam neither opens nor loses the crease.
//! 4. The boundary is the same curve as the Bernstein row it replaces, and the interior is free of
//!    it.

mod support;

use whiting_patch::topology::{EdgeKey, canonical, edge_endpoints};
use whiting_patch::{FaceId, HermiteSurface, Jet, Join, Point3, VertexId};

use support::ANGLE_FLOOR;

/// A ceiling on an angle that should be zero.
///
/// The floor is about `1.4e-8`, from `acos` on a dot product `f64` cannot resolve below `1 - 1e-16`.
/// A **normal** reads two to three times that rather than exactly it, because it is a cross product
/// of two vectors that are not unit length and the error in their directions is what is being
/// measured. That is rounding, not geometry, so the ceiling sits a decade above the floor and no
/// tighter: tightening it against the floor is how a probe ends up reporting its own instrument.
const C1_CEILING: f64 = 1.0e-7;

/// The degree-3 Bernstein weights, built from the binomial definition.
///
/// Deliberately **not** [`whiting_patch::hermite::hermite_basis`]. The two are different sets of
/// functions of the same degree, and the first version of this probe used the Bernstein weights
/// where the Hermite ones belonged and reported every seam `2.501e0` apart — it was placing a
/// tangent where a corner goes, and reading it as a surface that does not meet.
fn bernstein_degree_three(t: f64) -> [f64; 4] {
    let mut weights = [0.0f64; 4];
    for (k, weight) in weights.iter_mut().enumerate() {
        let binomial: f64 = match k {
            0 => 1.0,
            1 => 3.0,
            2 => 3.0,
            _ => 1.0,
        };
        *weight = binomial * t.powi(k as i32) * (1.0 - t).powi((3 - k) as i32);
    }
    weights
}

/// The parameters measured along every seam: corners, the middle and three interior points.
///
/// Corners included on purpose. A seam's ends are where a per-position rule and a per-edge rule
/// disagree, and this design's claim is that there is no such distinction.
fn parameters() -> [f64; 7] {
    [0.0, 0.25, 0.5, 0.75, 1.0, 0.125, 0.875]
}

/// A measurement that came out NaN cannot be compared, and folding it into a maximum silently turns
/// a broken run into a passing one.
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        f64::INFINITY
    }
}

// -- the sample ----------------------------------------------------------------------------------

/// A bowed heightfield grid, the same shape the other probes in this crate use.
///
/// The bow is the point: a planar sample cannot tell a surface that is continuous from one that is
/// flat, because a flat surface's two tangents are equal everywhere for reasons that have nothing
/// to do with the join.
fn sample(columns: usize, rows: usize) -> HermiteSurface {
    let mut positions = Vec::with_capacity((columns + 1) * (rows + 1));
    for row in 0..=rows {
        for column in 0..=columns {
            let x = column as f64 / columns as f64;
            let y = row as f64 / rows as f64;
            positions.push(Point3::new(
                4.0 * x - 2.0,
                3.0 * y - 1.5,
                0.6 * (x * x + y * y),
            ));
        }
    }
    let mut quads = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let base = (row * (columns + 1) + column) as VertexId;
            let right = base + 1;
            let up = base + columns as VertexId + 1;
            let both = up + 1;
            // The crate's winding: `(0,0) (1,0) (1,1) (0,1)`.
            quads.push([base, right, both, up]);
        }
    }
    HermiteSurface::build(positions, quads).expect("a grid mesh is representable")
}

fn face_of(column: usize, row: usize, columns: usize) -> FaceId {
    (row * columns + column) as FaceId
}

/// The interior edge between two cells side by side, at global `u = column + 1`.
fn seam_between(column: usize, row: usize, columns: usize, surface: &HermiteSurface) -> EdgeKey {
    let _ = (columns, surface);
    let left = face_of(column, row, columns) as usize;
    let _ = left;
    unreachable!()
}

// -- measuring ----------------------------------------------------------------------------------

/// Everything measured at one point on one shared edge.
#[derive(Clone, Copy, Debug, Default)]
struct Reading {
    /// How far the two faces put the shared point. A crease must still read zero here.
    split: f64,
    /// Angle between the two faces' derivatives **along** the edge. Shared on both tags.
    along_edge: f64,
    /// Angle between the two faces' derivatives **across** it. This is what the tag decides.
    across_edge: f64,
    /// Angle between the two faces' normals.
    normal: f64,
    /// Readings that came out non-finite.
    non_finite: usize,
}

impl Reading {
    /// The worse of two readings, field by field: a seam is only as good as its worst point.
    fn worse(self, other: Self) -> Self {
        Self {
            split: self.split.max(other.split),
            along_edge: self.along_edge.max(other.along_edge),
            across_edge: self.across_edge.max(other.across_edge),
            normal: self.normal.max(other.normal),
            non_finite: self.non_finite + other.non_finite,
        }
    }
}

/// Reads the relationship between two faces at one point along the seam between them.
///
/// The two faces sit at **opposite ends of their own domain** — `s = 1` against `s = 0` for a
/// `u = constant` seam — and reading both at the same parameter is the error that makes a seam look
/// continuous when the two faces do not even meet.
fn read_seam_point(
    along: f64,
    across_u: bool,
    first: FaceId,
    second: FaceId,
    surface: &HermiteSurface,
) -> Reading {
    let (first_uv, second_uv) = if across_u {
        ((1.0, along), (0.0, along))
    } else {
        ((along, 1.0), (along, 0.0))
    };
    let first_patch = surface.patch(first);
    let second_patch = surface.patch(second);
    let (first_u, first_v) = (
        first_patch.tangent_u(first_uv.0, first_uv.1),
        first_patch.tangent_v(first_uv.0, first_uv.1),
    );
    let (second_u, second_v) = (
        second_patch.tangent_u(second_uv.0, second_uv.1),
        second_patch.tangent_v(second_uv.0, second_uv.1),
    );
    let place = first_patch
        .evaluate(first_uv.0, first_uv.1)
        .distance(second_patch.evaluate(second_uv.0, second_uv.1));
    let (along_first, along_second) = if across_u {
        (first_v, second_v)
    } else {
        (first_u, second_u)
    };
    let (across_first, across_second) = if across_u {
        (first_u, second_u)
    } else {
        (first_v, second_v)
    };
    let along = along_first.angle(along_second);
    let across = across_first.angle(across_second);
    let normal = first_u.cross(first_v).angle(second_u.cross(second_v));
    let readings = [place, along, across, normal];
    Reading {
        split: place,
        along_edge: finite(along),
        across_edge: finite(across),
        normal: finite(normal),
        // Counted before the reductions, because `f64::max` returns the non-NaN operand and a
        // broken run would otherwise report a clean zero.
        non_finite: readings.iter().filter(|value| !value.is_finite()).count(),
    }
}

/// Measures one seam over every sampled parameter along it.
fn measure_seam(
    across_u: bool,
    first: FaceId,
    second: FaceId,
    surface: &HermiteSurface,
) -> Reading {
    let mut worst = Reading::default();
    for along in parameters() {
        worst = worst.worse(read_seam_point(along, across_u, first, second, surface));
    }
    worst
}

/// Every interior seam of a grid, as `(across_u, first face, second face)`.
fn interior_seams(columns: usize, rows: usize) -> Vec<(bool, FaceId, FaceId)> {
    let mut found = Vec::new();
    for row in 0..rows {
        for column in 1..columns {
            found.push((
                true,
                face_of(column - 1, row, columns),
                face_of(column, row, columns),
            ));
        }
    }
    for column in 0..columns {
        for row in 1..rows {
            found.push((
                false,
                face_of(column, row - 1, columns),
                face_of(column, row, columns),
            ));
        }
    }
    found
}

/// Measures every interior seam, with an optional predicate selecting which ones to include.
fn measure_all(
    columns: usize,
    rows: usize,
    surface: &HermiteSurface,
    skip: impl Fn(bool, FaceId, FaceId) -> bool,
) -> Reading {
    let mut worst = Reading::default();
    for (across_u, first, second) in interior_seams(columns, rows) {
        if skip(across_u, first, second) {
            continue;
        }
        worst = worst.worse(measure_seam(across_u, first, second, surface));
    }
    worst
}

fn row(reading: &Reading) -> String {
    format!(
        "split {:.3e}   along {:.3e}   across {:.3e}   normal {:.3e} rad",
        reading.split, reading.along_edge, reading.across_edge, reading.normal
    )
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

const COLUMNS: usize = 5;
const ROWS: usize = 4;

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let mut surface = sample(COLUMNS, ROWS);
    println!(
        "\nsample: {} by {} nodes, {} cells, {} interior seams, every edge soft",
        COLUMNS + 1,
        ROWS + 1,
        COLUMNS * ROWS,
        interior_seams(COLUMNS, ROWS).len()
    );

    // -- 1. Every edge soft is exactly C1. ------------------------------------------------------------
    let soft = measure_all(COLUMNS, ROWS, &surface, |_, _, _| false);
    report(
        "1  every edge soft: exactly C1",
        &format!(
            "{}\n\
             the corner jets are shared, so the two faces are the same surface there rather than\n\
             two surfaces held in agreement. nothing is enforced and nothing can fall out of step:\n\
             there is no pass and no ordering. the ceiling is {:.1e}, a decade above the acos floor\n\
             of {:.1e}, because a normal reads a small multiple of it",
            row(&soft),
            C1_CEILING,
            ANGLE_FLOOR
        ),
    );
    if soft.non_finite > 0 || soft.split > support::EXACT {
        failures.push("1  a soft seam is one shared boundary");
    }
    if soft.normal > C1_CEILING {
        failures.push("1  a soft seam is exactly C1");
    }

    // -- 0. The control. -----------------------------------------------------------------------------
    //
    // Break one edge by giving its two faces different twists, which is the smallest change that
    // must break exactly one seam. The second arm is the point: a change applied to *both* sides
    // moves both faces together and leaves the surface perfectly smooth, so a control that cannot
    // do that is a control that cannot fail.
    let broken_seam = (true, face_of(1, 0, COLUMNS), face_of(2, 0, COLUMNS));
    let mut broken = surface.clone();
    break_seam(&mut broken, broken_seam.1, broken_seam.2);
    let control_on = measure_seam(broken_seam.0, broken_seam.1, broken_seam.2, &broken);
    let control_elsewhere = measure_all(COLUMNS, ROWS, &broken, |across_u, a, b| {
        (across_u, a, b) == broken_seam
    });
    report(
        "0  control: the probe can see a discontinuity",
        &format!(
            "the broken seam\n{}\nevery other seam, same surface\n{}\n\
             the second arm is the one a perturbation-only control reports, and it is the floor:\n\
             both faces moved together, so nothing is discontinuous. the first arm is what makes\n\
             measurement 1's near-zero a property of the surface rather than of the instrument --\n\
             it is {:.0e} times larger, on the same function and the same sample",
            row(&control_on),
            row(&control_elsewhere),
            control_on.normal / control_elsewhere.normal
        ),
    );
    if control_on.normal < 1.0e-2 || control_elsewhere.normal > C1_CEILING {
        failures.push("0  the control fires on one seam and stays silent on the rest");
    }

    // -- 2. What the tag buys. ------------------------------------------------------------------------
    let soft_seam = measure_seam(broken_seam.0, broken_seam.1, broken_seam.2, &surface);
    let hard_seam = measure_seam(broken_seam.0, broken_seam.1, broken_seam.2, &broken);
    report(
        "2  a hard tag, against a soft one, on the same seam",
        &format!(
            "soft\n{}\nhard\n{}\n\
             split and along-edge are identical in both arms and exact in both: positions and the\n\
             tangential derivative are shared whatever the tag says, so a crease can never open\n\
             into a crack. only `across` moves, by {:.3e} rad. that is the whole content of the\n\
             tag, and it is why 'the two faces agree about the edge' is not the same claim as\n\
             'the join is C1'",
            row(&soft_seam),
            row(&hard_seam),
            hard_seam.normal - soft_seam.normal
        ),
    );
    if soft_seam.normal > C1_CEILING {
        failures.push("2a  a soft edge gives an exact C1 join");
    }
    if hard_seam.split > support::EXACT || hard_seam.along_edge > C1_CEILING {
        failures.push("2b  a hard edge is still C0 with a matched tangent");
    }
    if hard_seam.normal < 1.0e-2 {
        failures.push("2b  a hard edge does fold (the control for 2a)");
    }

    // -- 3. A hard edge keeps its boundary while the interiors move. ------------------------------------
    //
    // Both faces twist independently, and the crease must survive it: the boundary is still one
    // curve, and the two sides still disagree about the normal by about as much as they did.
    let mut dragged = broken.clone();
    drag_around(&mut dragged, broken_seam.1, broken_seam.2);
    let dragged_reading = measure_seam(broken_seam.0, broken_seam.1, broken_seam.2, &dragged);
    let dragged_elsewhere = measure_all(COLUMNS, ROWS, &dragged, |across_u, a, b| {
        (across_u, a, b) == broken_seam
    });

    let twist_moved = dragged.twist(broken_seam.1, 0) != broken.twist(broken_seam.1, 0)
        && dragged.twist(broken_seam.2, 2) != broken.twist(broken_seam.2, 2);
    report(
        "3  a hard edge: shared boundary, free interiors",
        &format!(
            "the broken seam after both sides were twisted independently\n{}\n\
             every other seam, unchanged\n{}\n\
             both sides' twists actually moved\n{}\n\
             the crease is {:.3e} rad before and {:.3e} rad after, and the split stays at {:.3e}.\n\
             a hard edge is a shared curve with independent interiors: the same sentence\n\
             crate::continuity uses today, reached without an enforcement pass to keep it true",
            row(&dragged_reading),
            row(&dragged_elsewhere),
            if twist_moved {
                "yes"
            } else {
                "no, so this arm proves nothing"
            },
            control_on.normal,
            dragged_reading.normal,
            dragged_reading.split
        ),
    );
    if dragged_reading.split > support::EXACT {
        failures.push("3  a drag on either side of a hard edge does not open the seam");
    }
    if dragged_reading.along_edge > C1_CEILING {
        failures.push("3  a hard edge keeps its two sides' tangential derivative matched");
    }
    if !twist_moved || dragged_elsewhere.normal > C1_CEILING {
        failures.push("3  the interiors really were free, and the rest of the surface was not");
    }

    // -- 4. The boundary is the shared Bernstein row, and the interior is free of it. -------------------
    // The face whose interior is twisted, and the shared boundary that must not notice.
    let probe_face = face_of(0, 0, COLUMNS);
    let before = surface.corner_jet(probe_face, 2);
    let mut moved = surface.clone();
    moved.set_twist(probe_face, 2, Point3::new(0.4, -0.3, 0.9));
    let after = moved.corner_jet(probe_face, 2);

    let patch = surface.patch(probe_face);
    let moved_patch = moved.patch(probe_face);
    let mut worst_curve = 0.0f64;
    for t in parameters() {
        let weights = bernstein_degree_three(t);
        let control = HermitePatchBezier::of(patch.boundary(true, false));
        let assembled = control
            .iter()
            .enumerate()
            .fold(Point3::ZERO, |sum, (index, point)| {
                sum + *point * weights[index]
            });
        worst_curve = worst_curve.max(finite(patch.evaluate(0.0, t).distance(assembled)));
    }

    // The boundary curve is shared geometry, so a twist inside one face must not move it — and the
    // face's own middle must move, or the twist did nothing at all. Both are read from the same
    // two patches, so the pair is a measurement rather than two plausible stories.
    let boundary_shift = (0..4)
        .map(|index| {
            finite(
                patch.boundary(true, false)[index]
                    .distance(moved_patch.boundary(true, false)[index]),
            )
        })
        .fold(0.0f64, f64::max);
    let interior_shift = finite(
        patch
            .evaluate(0.5, 0.5)
            .distance(moved_patch.evaluate(0.5, 0.5)),
    );
    let twist_shift = after.duv.distance(before.duv);
    report(
        "4  the boundary is the shared Bernstein row, independent of the interior",
        &format!(
            "Hermite boundary against an independently written Bernstein basis  {worst_curve:.3e}\n\
             the twist moved by                                                    {twist_shift:.3e}\n\
             the shared boundary moved by                                         {boundary_shift:.3e}\n\
             the face's own interior moved by                                     {interior_shift:.3e}\n\
             the first line is the identity that makes the shared boundary of this representation\n\
             the same curve as the shared Bernstein row it replaces -- a reparameterization of the\n\
             shared geometry rather than a change to it. the other three say a face's interior is\n\
             free without touching the boundary it shares with anybody",
        ),
    );
    if worst_curve > support::EXACT
        || boundary_shift > support::EXACT
        || interior_shift <= 0.0
        || twist_shift <= 0.0
    {
        failures.push("4  the boundary is one cubic and the interior is free of it");
    }

    surface = moved;
    let _ = surface;

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

/// Gives the two faces on a seam different twists, which is what a hard tag lets a caller do.
fn break_seam(surface: &mut HermiteSurface, left: FaceId, right: FaceId) {
    let seam = shared_edge(surface, left, right);
    surface.set_join(seam, Join::Hard);
    let push = Point3::new(0.0, 0.0, 0.8);
    surface.set_twist(left, 1, push);
    surface.set_twist(right, 3, -push);
}

/// Twists both sides a second time, in different directions, to show the interior is free.
fn drag_around(surface: &mut HermiteSurface, left: FaceId, right: FaceId) {
    surface.set_twist(left, 0, Point3::new(0.1, 0.0, 0.35));
    surface.set_twist(right, 2, Point3::new(0.0, -0.2, 0.25));
}

/// The edge two adjacent faces share.
fn shared_edge(surface: &HermiteSurface, left: FaceId, right: FaceId) -> EdgeKey {
    for slot in 0..4 {
        let key = canonical(edge_endpoints(
            surface.quads()[left as usize],
            slot_of(slot),
        ));
        if surface
            .topology()
            .edge_faces
            .get(&key)
            .is_some_and(|faces| faces.len() == 2 && faces.iter().any(|(face, _)| *face == right))
        {
            return key;
        }
    }
    panic!("the two faces share no edge, so this sample is not a grid");
}

fn slot_of(index: usize) -> whiting_patch::topology::EdgeSlot {
    whiting_patch::topology::EdgeSlot::ALL[index]
}

/// The Bezier control points of a Hermite boundary, `(P_0, P_0 + d_0/3, P_1 - d_1/3, P_1)`.
struct HermitePatchBezier;

impl HermitePatchBezier {
    fn of(boundary: [Point3; 4]) -> [Point3; 4] {
        [
            boundary[0],
            boundary[0] + boundary[1] / 3.0,
            boundary[2] - boundary[3] / 3.0,
            boundary[2],
        ]
    }
}

/// Keeps the unused import honest: `Jet` is what a caller of `corner_jet` receives.
fn _assert_jet_is_used(_: Jet) {}
