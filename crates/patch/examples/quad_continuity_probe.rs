//! Probes the continuity layer on a real multi-face surface: soft edges, hard edges, and whether
//! editing a seam keeps it a seam.
//!
//! Run with `cargo run -p whiting-patch --example quad_continuity_probe`. An example and not a gate,
//! following `facet_value_probe`. **It exits non-zero**, because measurement 3 fails and the reason
//! is written out below and in `crate::continuity`.
//!
//! 1. An all-hard model reproduces its subdivided mesh, quad by quad. Passes.
//! 2. A shared boundary is one curve on both faces — a hard edge is a crease, never a crack. Passes.
//! 3. A soft edge is tangent-plane continuous and every hard edge is a crease. **Fails**, and the
//!    diagnosis is below.
//! 4. Both kinds exist in the sample, so the measurement is comparing two things.
//! 5. A drag on one side of a soft edge moves the other side; across a hard edge it does not.
//! 6. Elevation preserves every tag.
//! 7. Dragging a shared boundary moves both faces and keeps the tag.
//! 8. `mu` places the slave's private row exactly, and the plane gap follows from it.
//! 9. A point two soft edges own is refused, and a refused edit changes nothing.
//!
//! ## Why measurement 3 fails, and what it costs
//!
//! The soft-edge invariant is enforced **exactly**: measurement 8 shows a residual of zero, and the
//! along-edge derivatives of the two faces are bit-identical. What is wrong is what the invariant
//! does *not* cover.
//!
//! A patch's cross-boundary derivative at the boundary is
//!
//! ```text
//! D(v) = degree * sum over j of B(degree-1, j, v) * (A[j] - C[j])
//! ```
//!
//! — the **whole** adjacent row, `j = 0..=degree`. The invariant runs over `j = 1..degree` only,
//! because positions 0 and degree are points of the perpendicular edge's shared boundary and
//! forcing them would demand `mu == -1`. But `B(degree-1, 0, v)` and `B(degree-1, degree, v)` are
//! positive at **every** `v` in the interior, so those two unconstrained offsets enter `D` along the
//! whole edge, not only at its ends.
//!
//! The measured consequence at degree 3 is a tangent-plane gap of **4.8e-2 rad** at the interior
//! positions and **1.3e-1 rad** at the ends, against **1.8e-1 rad** for a hard edge. So a soft edge
//! is measurably smoother than a crease and is not smooth. Raising the degree does not fix it: the
//! end weights shrink but never vanish.
//!
//! Fixing it means solving the **corner stencil**: at a mesh vertex the corner-adjacent control
//! points satisfy `q_i = -mu_i * q_{i-1}` around the cycle of incident faces, where `q` is measured
//! from the vertex itself. Closing the cycle needs `(-1)^valence * product(mu) == 1`. A Doo–Sabin
//! centroid has valence 3, so all-`mu == 1` forces `(-1)^3 == 1`, which fails; the only consistent
//! solution is `q == 0`, i.e. every corner-adjacent point collapsed onto the vertex — smooth, and a
//! pinch. That is a real constraint on what `mu` a user may choose, not something a formula can
//! absorb.
//!
//! Until that is built, the implementation deliberately leaves the row ends to boundary sharing
//! rather than applying the invariant over them. A crack is worse than a crease.

mod support;
use std::collections::BTreeSet;

use support::{ANGLE_FLOOR, COLUMNS, EXACT, grid};

use whiting_patch::Point3;
use whiting_patch::subdivide::{Origin, QuadMesh, subdivide};
use whiting_patch::topology::{
    EdgeKey, EdgeSlot, boundary_index, edge_endpoints, interior_index, mirrored_position,
};
use whiting_patch::{Edit, PatchMesh, RectNet};

/// A gap this small is indistinguishable from continuous, whatever the arithmetic says.
const CONTINUOUS: f64 = 1.0e-6;

/// A gap this large is unambiguously a crease.
const CREASE: f64 = 1.0e-2;

/// A distance this small is indistinguishable from nothing happening.
const MOVED: f64 = 1.0e-6;

/// How far into a patch measurement 5 samples. At degree 3 a control point row at `u = 1` carries
/// about 42% of a point at `u = 0.25`, so a soft edge cannot be missed by sampling there.
const INSIDE: f64 = 0.25;

const STEP: f64 = 1.0e-5;

const DEGREE: usize = 3;

/// The original edge a derived edge lies on, if it lies on one.
fn original_edge_at(derived: &QuadMesh, edge: &EdgeKey) -> Option<EdgeKey> {
    [edge.0, edge.1]
        .into_iter()
        .find_map(|index| match derived.origin[index as usize] {
            Origin::EdgeMidpoint(key) => Some(key),
            _ => None,
        })
}

/// Horizontal original edges. Derived edges inherit the tag of the original edge they lie on.
fn is_soft(derived: &QuadMesh, edge: &EdgeKey) -> bool {
    original_edge_at(derived, edge)
        .is_some_and(|key| key.0 / COLUMNS as u32 == key.1 / COLUMNS as u32)
}

/// Soft edges with at most one per face, so a patch's private interior is claimed once and the
/// invariant can be measured without a second edge writing the same point.
fn matching_edges(mesh: &PatchMesh) -> Vec<EdgeKey> {
    let mut claimed: BTreeSet<u32> = BTreeSet::new();
    let mut chosen = Vec::new();
    for (key, faces) in &mesh.topology().edge_faces {
        if faces.len() != 2 || claimed.contains(&faces[0].0) || claimed.contains(&faces[1].0) {
            continue;
        }
        claimed.insert(faces[0].0);
        claimed.insert(faces[1].0);
        chosen.push(*key);
    }
    chosen
}

fn build_with(derived: &QuadMesh, edges: &[EdgeKey]) -> PatchMesh {
    let mut mesh = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for edge in edges {
        mesh.set_soft(*edge);
    }
    for _ in 1..DEGREE {
        mesh.elevate();
    }
    mesh
}

fn interior_edges(mesh: &PatchMesh) -> Vec<(EdgeKey, [(u32, EdgeSlot); 2])> {
    mesh.topology()
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 2)
        .map(|(key, faces)| (*key, [faces[0], faces[1]]))
        .collect()
}

/// The along-edge and into-the-face derivative vectors at one boundary position, by finite
/// difference. Second-order one-sided across the edge, central along it.
fn edge_frame(
    net: &RectNet,
    degree: usize,
    slot: EdgeSlot,
    t: usize,
    step: f64,
) -> (Point3, Point3) {
    let (i, j) = boundary_index(degree, slot, t);
    let scale = degree as f64;
    let at = [i as f64 / scale, j as f64 / scale];

    // Index 0 is `u` and index 1 is `v`. The sign is which way is *into* this face.
    let (along, across, inward) = match slot {
        EdgeSlot::MinU => (1, 0, 1.0),
        EdgeSlot::MaxU => (1, 0, -1.0),
        EdgeSlot::MinV => (0, 1, 1.0),
        EdgeSlot::MaxV => (0, 1, -1.0),
    };

    let mut before = at;
    before[along] -= step;
    let mut after = at;
    after[along] += step;
    let along_vector =
        (net.evaluate(after[0], after[1]) - net.evaluate(before[0], before[1])) / (2.0 * step);

    let mut one_step = at;
    one_step[across] += inward * step;
    let mut two_steps = at;
    two_steps[across] += inward * step * 2.0;
    let across_vector = (net.evaluate(at[0], at[1]) * -3.0
        + net.evaluate(one_step[0], one_step[1]) * 4.0
        - net.evaluate(two_steps[0], two_steps[1]))
        / (2.0 * step);

    (along_vector, across_vector)
}

fn edge_normal(net: &RectNet, degree: usize, slot: EdgeSlot, t: usize, step: f64) -> Point3 {
    let (along, across) = edge_frame(net, degree, slot, t, step);
    along.cross(across)
}

/// The angle between two directions, in radians, treating a line as unoriented.
///
/// Two tangent planes are the same plane whether their normals agree or oppose, so the raw angle
/// is folded into `[0, pi/2]`. The same folding applies to two cross-boundary derivatives, which
/// point *into* their own patches and so oppose across a seam.
fn folded_angle(first: Point3, second: Point3) -> f64 {
    let raw = first.angle(second);
    if raw.is_nan() {
        return f64::INFINITY;
    }
    raw.min(std::f64::consts::PI - raw)
}

/// The parameters of a point on `slot`'s boundary at position `t`, pushed `h` into the face.
fn inside_parameters(degree: usize, slot: EdgeSlot, t: usize, h: f64) -> (f64, f64) {
    let (i, j) = boundary_index(degree, slot, t);
    let scale = degree as f64;
    let (u, v) = (i as f64 / scale, j as f64 / scale);
    match slot {
        EdgeSlot::MinU => (h, v),
        EdgeSlot::MaxU => (1.0 - h, v),
        EdgeSlot::MinV => (u, h),
        EdgeSlot::MaxV => (u, 1.0 - h),
    }
}

/// The worst tangent-plane gap across an edge over an explicit set of boundary positions.
fn edge_gap_over(
    mesh: &PatchMesh,
    faces: [(u32, EdgeSlot); 2],
    step: f64,
    positions: impl IntoIterator<Item = usize>,
) -> f64 {
    let degree = mesh.degree();
    let (master, slave) = (faces[0], faces[1]);
    let here = edge_endpoints(mesh.quads()[master.0 as usize], master.1);
    let there = edge_endpoints(mesh.quads()[slave.0 as usize], slave.1);
    positions
        .into_iter()
        .filter(|t| *t <= degree)
        .map(|t| {
            let other_t = mirrored_position(degree, here, there, t);
            folded_angle(
                edge_normal(mesh.net(master.0), degree, master.1, t, step),
                edge_normal(mesh.net(slave.0), degree, slave.1, other_t, step),
            )
        })
        .fold(0.0f64, f64::max)
}

/// How far apart two points on the same edge are when read from its two faces.
fn boundary_split(mesh: &PatchMesh, edge: &EdgeKey, face: u32, slot: EdgeSlot, t: usize) -> f64 {
    let Some(other) = mesh.topology().edge_faces[edge]
        .iter()
        .find(|(id, _)| *id != face)
    else {
        return f64::NAN;
    };
    let degree = mesh.degree();
    let here = edge_endpoints(mesh.quads()[face as usize], slot);
    let there = edge_endpoints(mesh.quads()[other.0 as usize], other.1);
    let other_t = mirrored_position(degree, here, there, t);
    let (oi, oj) = boundary_index(degree, slot, t);
    let (fi, fj) = boundary_index(degree, other.1, other_t);
    mesh.net(face)[(oi, oj)].distance(mesh.net(other.0)[(fi, fj)])
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let (positions, triangles) = grid();
    let derived = subdivide(&positions, &triangles);
    let bare = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    let soft_edges = matching_edges(&bare);
    let mesh = build_with(&derived, &soft_edges);
    let mut hard_model = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for _ in 1..DEGREE {
        hard_model.elevate();
    }

    let mut failures: Vec<&str> = Vec::new();
    let edges = interior_edges(&mesh);
    let hard_count = edges.len() - soft_edges.len();

    println!(
        "quad continuity probe — {} triangles -> {} quads, {} interior edges, {} of them soft with \
         at most one per face, degree {DEGREE}",
        triangles.len(),
        derived.face_count(),
        edges.len(),
        soft_edges.len(),
    );

    // 1. An all-hard model reproduces its mesh.
    let mut corner_error = 0.0f64;
    for (face, quad) in hard_model.quads().iter().enumerate() {
        let net = hard_model.net(face as u32);
        for (index, corner) in quad.iter().enumerate() {
            let (u, v) = match index {
                0 => (0.0, 0.0),
                1 => (1.0, 0.0),
                2 => (1.0, 1.0),
                _ => (0.0, 1.0),
            };
            corner_error = corner_error.max(
                net.evaluate(u, v)
                    .distance(hard_model.positions()[*corner as usize]),
            );
        }
    }
    let mut face_deviation = 0.0f64;
    for (face, quad) in hard_model.quads().iter().enumerate() {
        let [q0, q1, q2, q3] = quad.map(|index| derived.positions[index as usize]);
        let base = RectNet::from_corners(q0, q1, q3, q2);
        let net = hard_model.net(face as u32);
        for step_u in 1..8 {
            for step_v in 1..8 {
                let (u, v) = (step_u as f64 / 8.0, step_v as f64 / 8.0);
                face_deviation =
                    face_deviation.max(net.evaluate(u, v).distance(base.evaluate(u, v)));
            }
        }
    }
    if corner_error > EXACT {
        failures.push("1  an all-hard model reproduces its mesh");
    }
    if face_deviation > EXACT {
        failures.push("1  an elevated flat patch is still flat");
    }
    report(
        "1  an all-hard model reproduces its mesh",
        &format!(
            "largest corner error over every quad   {corner_error:.3e}\n\
             worst departure of a patch from its own bilinear map   {face_deviation:.3e}\n\
             elevated {DEGREE} times and the assembled surface is still the mesh, corner for corner"
        ),
    );

    // 2. A shared boundary is one curve, hard or soft.
    let mut worst_split = 0.0f64;
    for (edge, faces) in &edges {
        for t in 1..DEGREE {
            for &(face, slot) in faces {
                worst_split = worst_split.max(boundary_split(&mesh, edge, face, slot, t));
            }
        }
    }
    if worst_split > EXACT {
        failures.push("2  a shared boundary is one curve");
    }
    report(
        "2  a shared boundary is one curve",
        &format!(
            "worst gap between the two faces' copies of an edge   {worst_split:.3e}\n\
             this is the crack check, and it is the constraint that shaped the implementation: the\n\
             soft invariant is not applied over the ends of a row, because a crack is worse than a\n\
             crease"
        ),
    );

    // 3. Soft edges are continuous and hard edges are creases.
    let mut soft_interior = 0.0f64;
    let mut soft_ends = 0.0f64;
    let mut hard_worst = 0.0f64;
    let mut along_worst = 0.0f64;
    for (edge, faces) in &edges {
        let interior = edge_gap_over(&mesh, *faces, STEP, 1..DEGREE);
        let ends = edge_gap_over(&mesh, *faces, STEP, [0, DEGREE]);
        if soft_edges.contains(edge) {
            soft_interior = soft_interior.max(interior);
            soft_ends = soft_ends.max(ends);
            for t in 1..DEGREE {
                let here = edge_endpoints(mesh.quads()[faces[0].0 as usize], faces[0].1);
                let there = edge_endpoints(mesh.quads()[faces[1].0 as usize], faces[1].1);
                let other_t = mirrored_position(DEGREE, here, there, t);
                let (master_along, _) =
                    edge_frame(mesh.net(faces[0].0), DEGREE, faces[0].1, t, STEP);
                let (slave_along, _) =
                    edge_frame(mesh.net(faces[1].0), DEGREE, faces[1].1, other_t, STEP);
                along_worst = along_worst.max(folded_angle(master_along, slave_along));
            }
        } else {
            hard_worst = hard_worst.max(interior.max(ends));
        }
    }
    if soft_interior > CONTINUOUS {
        failures.push("3  a soft edge is continuous along its interior");
    }
    if soft_ends > CONTINUOUS {
        failures.push("3  a soft edge is continuous at the two end positions");
    }
    if hard_worst < CREASE {
        failures.push("3  the sample has no crease to compare against");
    }
    report(
        "3  soft edges are smoother than creases, and are not continuous",
        &format!(
            "soft, positions 1..{DEGREE}   {soft_interior:.3e} rad\n\
             soft, positions 0 and {DEGREE}   {soft_ends:.3e} rad\n\
             hard, every position   {hard_worst:.3e} rad\n\
             the two faces' ALONG-edge derivatives agree to   {along_worst:.3e} rad\n\
             the along-edge agreement and the private-interior residual in measurement 8 say the\n\
             invariant is being enforced exactly. what it does not cover is the two ends of the row,\n\
             and the cross-boundary derivative sums over the WHOLE row: B(degree-1,0,v) and\n\
             B(degree-1,degree,v) are positive at every interior v, so two free offsets contaminate\n\
             the entire edge. that is why the interior figure is not at the acos floor of {ANGLE_FLOOR:.0e}\n\
             fixing it needs the corner stencil: around a vertex q_i = -mu_i * q_(i-1), which closes\n\
             only when (-1)^valence * product(mu) == 1, and a Doo-Sabin centroid has valence 3"
        ),
    );

    // 4. Both kinds are present, so the numbers above are being compared.
    if soft_edges.is_empty() || hard_count == 0 {
        failures.push("4  the sample compares two things");
    }
    report(
        "4  the sample compares two things",
        &format!(
            "{} soft edges and {hard_count} hard edges\n\
             a run where either is empty measures nothing",
            soft_edges.len()
        ),
    );

    // 5. A drag on one side of a soft edge moves the other side. This is the feature a soft edge
    //    exists for, and it has to be measured with a drag: on a planar level-0 mesh every value of
    //    mu leaves both patches planar, because the mirror of an in-plane offset is in-plane.
    let mut followed_soft = 0.0f64;
    let mut followed_hard = 0.0f64;
    for (edge, faces) in &edges {
        let t = DEGREE / 2;
        let (master, master_slot) = faces[0];
        let (i, j) = interior_index(DEGREE, master_slot, t);
        let mut trial = build_with(&derived, &soft_edges);
        let (u, v) = inside_parameters(DEGREE, faces[1].1, t, INSIDE);
        let before = trial.net(faces[1].0).evaluate(u, v);
        let origin = trial.net(master)[(i, j)];
        trial.set_control_point(master, i, j, origin + Point3::new(0.0, 0.0, 0.5));
        let moved = before.distance(trial.net(faces[1].0).evaluate(u, v));
        if soft_edges.contains(edge) {
            followed_soft = followed_soft.max(moved);
        } else {
            followed_hard = followed_hard.max(moved);
        }
    }
    if followed_soft < MOVED {
        failures.push("5  a drag across a soft edge moves the other side");
    }
    if followed_hard > MOVED {
        failures.push("5  a drag across a hard edge leaves the other side alone");
    }
    report(
        "5  a drag crosses a soft edge and stops at a hard one",
        &format!(
            "worst movement {INSIDE} inside the neighbour of a soft edge   {followed_soft:.3e}\n\
             worst movement {INSIDE} inside the neighbour of a hard edge   {followed_hard:.3e}\n\
             dragged the master's private interior, and read the slave's surface. measured with a drag\n\
             rather than from the starting geometry: on a planar level-0 mesh every mu leaves both\n\
             patches planar, since the mirror of an in-plane offset is in-plane, so nothing bends\n\
             until something is moved"
        ),
    );

    // 6. Elevation preserves every tag. The tag *is* the private-interior invariant, so that is what
    //    gets checked. The plane gap is not expected to be identical before and after: the two free
    //    row ends are re-placed by elevation, and they are what the gap is made of.
    let mut tall = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for edge in &soft_edges {
        tall.set_soft(*edge);
    }
    for _ in 1..DEGREE + 1 {
        tall.elevate();
    }
    let mut worst_residual = 0.0f64;
    for entry in tall.soft_edges() {
        let mu = tall
            .continuity(&entry.edge)
            .and_then(|c| c.mu())
            .unwrap_or(f64::NAN);
        let degree = tall.degree();
        let master_ends = edge_endpoints(tall.quads()[entry.master.0 as usize], entry.master.1);
        let slave_ends = edge_endpoints(tall.quads()[entry.slave.0 as usize], entry.slave.1);
        for t in 1..degree {
            let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
            let (bi, bj) = boundary_index(degree, entry.master.1, t);
            let (ai, aj) = interior_index(degree, entry.master.1, t);
            let (si, sj) = interior_index(degree, entry.slave.1, slave_t);
            let boundary = tall.net(entry.master.0)[(bi, bj)];
            let master_row = tall.net(entry.master.0)[(ai, aj)];
            let slave_row = tall.net(entry.slave.0)[(si, sj)];
            worst_residual =
                worst_residual.max(slave_row.distance(boundary + (boundary - master_row) * mu));
        }
    }
    let after_elevation = interior_edges(&tall)
        .iter()
        .filter(|(edge, _)| soft_edges.contains(edge))
        .map(|(_, faces)| edge_gap_over(&tall, *faces, STEP, 1..DEGREE + 1))
        .fold(0.0f64, f64::max);
    if worst_residual > EXACT {
        failures.push("6  elevation preserves every tag");
    }
    report(
        "6  elevation preserves every tag",
        &format!(
            "worst invariant residual at degree {}   {worst_residual:.3e} over every soft edge\n\
             worst soft gap at degree {}   {after_elevation:.3e} rad, against {soft_interior:.3e} at \
             degree {DEGREE}\n\
             the residual is the tag, and it survives elevation exactly. the gap is not expected to\n\
             match: elevation re-places the two free row ends, and those are what the gap is made of\n\
             mu needs no adjustment across an elevation, because it is one scalar for the whole edge\n\
             and the edge did not gain positions",
            DEGREE + 1,
            DEGREE + 1,
        ),
    );

    // 7. Dragging a shared boundary keeps the seam and the tag.
    let Some((soft_edge, soft_faces)) = edges
        .iter()
        .find(|(edge, _)| soft_edges.contains(edge))
        .map(|(edge, faces)| (*edge, *faces))
    else {
        println!("\nFAILED:\n  - no soft edge to edit");
        std::process::exit(1);
    };
    let (face, slot) = soft_faces[0];
    let grab = DEGREE / 2;
    let grab_index = boundary_index(DEGREE, slot, grab);
    let split_before = boundary_split(&mesh, &soft_edge, face, slot, grab);
    let gap_before = edge_gap_over(&mesh, soft_faces, STEP, 1..DEGREE);
    let mut edited = build_with(&derived, &soft_edges);
    let start = edited.net(face)[grab_index];
    let outcome = edited.set_control_point(
        face,
        grab_index.0,
        grab_index.1,
        start + Point3::new(0.0, 0.0, 0.6),
    );
    let drag_distance = start.distance(edited.net(face)[grab_index]);
    let split_after = boundary_split(&edited, &soft_edge, face, slot, grab);
    let gap_after = edge_gap_over(&edited, soft_faces, STEP, 1..DEGREE);
    if outcome != Edit::Applied {
        failures.push("7  a drag on a shared boundary is applied");
    }
    if drag_distance < MOVED {
        failures.push("7  a drag on a shared boundary actually moves the point");
    }
    if split_after > EXACT {
        failures.push("7  a drag on a shared boundary does not open a crack");
    }
    report(
        "7  a drag on a shared boundary keeps the seam",
        &format!(
            "outcome                       {outcome:?}\n\
             boundary drag distance       {drag_distance:.3e}\n\
             boundary split before/after {split_before:.3e} / {split_after:.3e}\n\
             soft gap before / after      {gap_before:.3e} / {gap_after:.3e} rad\n\
             the drag landed on one face and the other followed, because a point on a seam is a point\n\
             on the seam. the gap moves by {gap_delta:.3e}, which is the same order as the gap itself:\n\
             dragging does not break the invariant, it re-derives the row and the row ends stay free",
            gap_delta = (gap_after - gap_before).abs(),
        ),
    );

    // 8. mu places the slave's private row exactly.
    let mut mu_rows = String::new();
    for mu in [1.0, 0.6, 0.0] {
        let mut tuned = build_with(&derived, &soft_edges);
        assert!(tuned.set_mu(soft_edge, mu), "the edge was just made soft");
        let here = edge_endpoints(tuned.quads()[soft_faces[0].0 as usize], soft_faces[0].1);
        let there = edge_endpoints(tuned.quads()[soft_faces[1].0 as usize], soft_faces[1].1);
        let mut residual = 0.0f64;
        let mut ratio_error = 0.0f64;
        for t in 1..DEGREE {
            let slave_t = mirrored_position(DEGREE, here, there, t);
            let (bi, bj) = boundary_index(DEGREE, soft_faces[0].1, t);
            let (ai, aj) = interior_index(DEGREE, soft_faces[0].1, t);
            let (si, sj) = interior_index(DEGREE, soft_faces[1].1, slave_t);
            let boundary = tuned.net(soft_faces[0].0)[(bi, bj)];
            let master_row = tuned.net(soft_faces[0].0)[(ai, aj)];
            let slave_row = tuned.net(soft_faces[1].0)[(si, sj)];
            let wanted = boundary + (boundary - master_row) * mu;
            residual = residual.max(slave_row.distance(wanted));
            let offset = master_row - boundary;
            ratio_error =
                ratio_error.max(((slave_row - boundary).length() / offset.length() - mu).abs());
        }
        mu_rows.push_str(&format!(
            "   mu = {mu:.1}   worst residual against the invariant   {residual:.3e}\n\
             \x20            worst |slave offset ratio - mu|   {ratio_error:.3e}\n"
        ));
        if residual > EXACT {
            failures.push("8  mu places the slave's private row exactly");
        }
        if ratio_error > 1.0e-9 {
            failures.push("8  the slave's offset is mu times the master's, reversed");
        }
    }
    report(
        "8  mu places the slave's private row exactly",
        &format!(
            "{mu_rows}   the invariant is enforced to rounding, and the plane gap in measurement 3\n\
             follows from what it does not cover rather than from how well it is enforced"
        ),
    );

    // 9. A point two soft edges own is refused, and a refused edit changes nothing.
    let dense: Vec<EdgeKey> = interior_edges(&bare)
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| is_soft(&derived, key))
        .collect();
    let mut crowded = build_with(&derived, &dense);
    let busiest = crowded
        .soft_edges()
        .iter()
        .map(|entry| entry.slave.0)
        .max_by_key(|face| {
            crowded
                .soft_edges()
                .iter()
                .filter(|entry| entry.slave.0 == *face)
                .count()
        });
    let mut contested = (0usize, 0usize);
    let mut unique = (0usize, 0usize);
    if let Some(face) = busiest {
        for i in 1..DEGREE {
            for j in 1..DEGREE {
                let owners = crowded
                    .soft_edges()
                    .iter()
                    .filter(|entry| {
                        entry.slave.0 == face
                            && (1..DEGREE)
                                .any(|t| interior_index(DEGREE, entry.slave.1, t) == (i, j))
                    })
                    .count();
                match owners {
                    0 => {}
                    1 => unique = (i, j),
                    _ => contested = (i, j),
                }
            }
        }
    }
    let mut refused = None;
    if let Some(face) = busiest {
        if contested != (0, 0) {
            let before: Vec<Point3> = crowded
                .nets()
                .iter()
                .flat_map(|net| net.points())
                .copied()
                .collect();
            let origin = crowded.net(face)[contested];
            let outcome = crowded.set_control_point(
                face,
                contested.0,
                contested.1,
                origin + Point3::new(0.0, 0.0, 0.9),
            );
            let after: Vec<Point3> = crowded
                .nets()
                .iter()
                .flat_map(|net| net.points())
                .copied()
                .collect();
            refused = Some((outcome, before != after));
        }
    }
    match refused {
        None => failures.push("9  the crowded sample has a point two soft edges own"),
        Some((Edit::Overconstrained { .. }, _)) => {}
        Some(_) => failures.push("9  a point two soft edges own is refused as over-constrained"),
    }
    if refused.is_some_and(|(_, changed)| changed) {
        failures.push("9  a refused edit leaves the model untouched");
    }
    report(
        "9  two owners refuse, and the refusal changes nothing",
        &format!(
            "point owned by exactly one row   {unique:?}\n\
             point owned by two rows          {contested:?}   outcome {}\n\
             the model changed after the refusal: {}\n\
             a quad's four rows meet at (1,1), (1,degree-1), (degree-1,1) and (degree-1,degree-1), so two\n\
             adjacent soft edges on one face do share a point and cannot both hold there. the model\n\
             refuses the EDIT rather than guessing -- applying anyway would let the enforcement order\n\
             pick a winner, so the same drag would give a different answer depending on the order the\n\
             edges were tagged in. the TAGS are still allowed to coexist, because a user asking for a\n\
             smooth patch is entitled to more than one smooth seam",
            refused.map_or("none".to_string(), |(outcome, _)| format!("{outcome:?}")),
            refused.is_some_and(|(_, changed)| changed),
        ),
    );

    if failures.is_empty() {
        println!("\nall 9 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        println!(
            "\nmeasurement 3 is the known open problem and is meant to fail: see the module\n\
             documentation, `crate::continuity`, and .agents/handoffs/ for the corner stencil."
        );
        std::process::exit(1);
    }
}
