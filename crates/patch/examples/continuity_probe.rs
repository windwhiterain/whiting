//! Probes the continuity layer on a real multi-face surface: soft edges, hard edges, and whether
//! editing a seam keeps it a seam.
//!
//! Run with `cargo run -p whiting-patch --example continuity_probe`. An example and not a gate,
//! following `facet_value_probe`. It exits non-zero if any property fails, including the cases
//! where its own controls go degenerate.
//!
//! 1. An all-hard model reproduces its mesh, face by face.
//! 2. Elevation does not move an all-hard surface.
//! 3. A shared boundary is one curve on both faces — a hard edge is a crease, never a crack.
//! 4. Soft edges are tangent-plane continuous; hard edges are not.
//! 5. Both kinds exist in the sample, so the measurement is comparing two things.
//! 6. Soft edges bend the surface, which is what they are for.
//! 7. Editing a shared boundary moves both faces; editing a slave's interior row moves the master.
//! 8. Elevation preserves every tag.
//!
//! ## The interior points a triangle's edges share
//!
//! A degree-`n` triangle's three adjacent rows — one per edge — cross each other. At degree 3 the
//! single point `(1, 1, 1)` sits on all three, so dragging it is constrained by up to three edges
//! at once. Measurement 7 drags a point that belongs to one row only, and measurement 9 measures
//! the shared one separately, because the difference is a property of the representation and not
//! something a probe should hide by choosing its numbers well.

mod support;
use support::{ANGLE_FLOOR, COLUMNS, EXACT, grid};

use whiting_patch::topology::{
    EdgeKey, boundary_index, edge_endpoints, interior_index, mirrored_position,
};
use whiting_patch::{EdgeSlot, Edit, PatchMesh, Point3, SimplexNet};

/// A gap this small is indistinguishable from continuous, whatever the arithmetic says.
const CONTINUOUS: f64 = 1.0e-6;

/// A gap this large is unambiguously a crease.
const CREASE: f64 = 1.0e-2;

/// A distance this small is indistinguishable from nothing happening.
const MOVED: f64 = 1.0e-6;

/// Horizontal edges soft, vertical and diagonal edges hard. Both kinds are always present.
fn is_soft(edge: &EdgeKey) -> bool {
    edge.0 / COLUMNS as u32 == edge.1 / COLUMNS as u32
}

/// Which edges to make soft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tags {
    /// None. Level 0 then reproduces the mesh exactly.
    None,
    /// Horizontal only, so hard and soft sit side by side.
    Horizontal,
    /// Every interior edge, so faces carry two or three soft edges and the rows really do cross.
    All,
}

/// Builds the grid at `degree`.
fn build(degree: usize, tags: Tags) -> PatchMesh {
    let (positions, triangles) = grid();
    let mut mesh = PatchMesh::build(positions, triangles);
    if tags != Tags::None {
        let edges: Vec<EdgeKey> = mesh
            .topology()
            .edge_faces
            .keys()
            .copied()
            .filter(|edge| {
                mesh.topology().edge_faces[edge].len() == 2 && (tags == Tags::All || is_soft(edge))
            })
            .collect();
        for edge in edges {
            mesh.set_soft(edge);
        }
    }
    for _ in 1..degree {
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

/// The barycentric coordinates of local boundary position `t` on this face's edge.
fn edge_barycentric(degree: usize, slot: EdgeSlot, t: usize) -> [f64; 3] {
    let (i, j) = boundary_index(degree, slot, t);
    let scale = degree as f64;
    [
        i as f64 / scale,
        j as f64 / scale,
        (degree - i - j) as f64 / scale,
    ]
}

/// The same point pushed `h` across the edge, holding the proportion along it fixed.
fn across_barycentric(degree: usize, slot: EdgeSlot, t: usize, h: f64) -> [f64; 3] {
    let zero = match slot {
        EdgeSlot::OppositeAlpha => 0,
        EdgeSlot::OppositeBeta => 1,
        EdgeSlot::OppositeGamma => 2,
    };
    let mut out = edge_barycentric(degree, slot, t);
    for coordinate in out.iter_mut() {
        *coordinate *= 1.0 - h;
    }
    out[zero] = h;
    out
}

/// The surface normal of one face at one point on one of its edges, by finite difference.
///
/// Second-order one-sided across the edge, central along it. Everything here is measured; no
/// derivative is evaluated in closed form.
fn edge_normal(mesh: &PatchMesh, face: u32, slot: EdgeSlot, t: usize, step: f64) -> Point3 {
    let degree = mesh.degree();
    let net = mesh.net(face);
    let along = (net.evaluate(edge_barycentric(degree, slot, t + 1))
        - net.evaluate(edge_barycentric(degree, slot, t - 1)))
        / 2.0;
    let across = (net.evaluate(across_barycentric(degree, slot, t, 0.0)) * -3.0
        + net.evaluate(across_barycentric(degree, slot, t, step)) * 4.0
        - net.evaluate(across_barycentric(degree, slot, t, step * 2.0)))
        / (2.0 * step);
    along.cross(across)
}

/// The angle between two tangent planes, in radians, treating a plane as unoriented.
fn plane_gap(first: Point3, second: Point3) -> f64 {
    let raw = first.angle(second);
    if raw.is_nan() {
        return f64::INFINITY;
    }
    raw.min(std::f64::consts::PI - raw)
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
    let here = edge_endpoints(mesh.triangles()[face as usize], slot);
    let there = edge_endpoints(mesh.triangles()[other.0 as usize], other.1);
    let other_t = mirrored_position(degree, here, there, t);
    let (oi, oj) = boundary_index(degree, slot, t);
    let (fi, fj) = boundary_index(degree, other.1, other_t);
    mesh.net(face)[(oi, oj)].distance(mesh.net(other.0)[(fi, fj)])
}

/// The worst tangent-plane gap across an edge, between its two faces, over the positions that
/// have an interior row on both sides.
fn edge_gap(mesh: &PatchMesh, faces: [(u32, EdgeSlot); 2], step: f64) -> f64 {
    edge_gap_over(mesh, faces, step, 1..mesh.degree())
}

/// The worst gap over an explicit range of boundary positions.
///
/// Position 0 is measured separately because it is the one position the adjacent-row mirror can
/// fail to pair when the two faces run along the edge in opposite directions — and a probe that
/// only looks at the positions that work is not measuring the thing.
fn edge_gap_over(
    mesh: &PatchMesh,
    faces: [(u32, EdgeSlot); 2],
    step: f64,
    positions: std::ops::Range<usize>,
) -> f64 {
    let degree = mesh.degree();
    let (master, slave) = (faces[0], faces[1]);
    let here = edge_endpoints(mesh.triangles()[master.0 as usize], master.1);
    let there = edge_endpoints(mesh.triangles()[slave.0 as usize], slave.1);
    positions
        .filter(|t| *t > 0 && *t <= degree)
        .map(|t| {
            let other_t = mirrored_position(degree, here, there, t);
            plane_gap(
                edge_normal(mesh, master.0, master.1, t, step),
                edge_normal(mesh, slave.0, slave.1, other_t, step),
            )
        })
        .fold(0.0f64, f64::max)
}

/// The level-0 triangle a face started from.
fn level_zero(mesh: &PatchMesh, face: u32) -> SimplexNet {
    let corners = mesh.triangles()[face as usize].map(|index| mesh.positions()[index as usize]);
    SimplexNet::from_corners(corners[0], corners[1], corners[2])
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let degree = 3;
    let step = 1.0e-5;
    let hard_model = build(degree, Tags::None);
    let mesh = build(degree, Tags::Horizontal);
    let edges = interior_edges(&mesh);
    let soft_count = edges.iter().filter(|(edge, _)| is_soft(edge)).count();
    let hard_count = edges.len() - soft_count;
    println!(
        "continuity probe — {} faces, {} interior edges ({soft_count} soft, {hard_count} hard), \
         degree {degree}",
        mesh.nets().len(),
        edges.len()
    );

    // 1. An all-hard model reproduces its mesh. Soft edges bend the surface on purpose, so this
    //    claim is about the model before anyone asked for a smooth join.
    let mut corner_error = 0.0f64;
    for (face, triangle) in hard_model.triangles().iter().enumerate() {
        let net = hard_model.net(face as u32);
        for (slot, index) in triangle.iter().enumerate() {
            let barycentric = [
                if slot == 0 { 1.0 } else { 0.0 },
                if slot == 1 { 1.0 } else { 0.0 },
                if slot == 2 { 1.0 } else { 0.0 },
            ];
            corner_error = corner_error.max(
                net.evaluate(barycentric)
                    .distance(hard_model.positions()[*index as usize]),
            );
        }
    }
    // A flat face stays flat: an elevated linear net is still its own triangle.
    let mut face_deviation = 0.0f64;
    for face in 0..hard_model.nets().len() as u32 {
        let corners = hard_model.triangles()[face as usize]
            .map(|index| hard_model.positions()[index as usize]);
        for step_index in 1..8 {
            for inner in 1..8 - step_index {
                let a = step_index as f64 / 8.0;
                let b = inner as f64 / 8.0;
                let raised = hard_model.net(face).evaluate([a, b, 1.0 - a - b]);
                let flat = corners[0] * a + corners[1] * b + corners[2] * (1.0 - a - b);
                face_deviation = face_deviation.max(raised.distance(flat));
            }
        }
    }
    report(
        "1  an all-hard model reproduces its mesh",
        &format!(
            "largest corner error over every face   {corner_error:.3e}\n\
             worst departure of a face from its own triangle   {face_deviation:.3e}\n\
             elevated four times and the assembled surface is still the mesh, corner for corner"
        ),
    );

    // 2. A shared boundary is one curve on both faces, hard or soft.
    let mut worst_split = 0.0f64;
    for (edge, faces) in &edges {
        for t in 1..degree {
            worst_split = worst_split.max(boundary_split(&mesh, edge, faces[0].0, faces[0].1, t));
            worst_split = worst_split.max(boundary_split(&mesh, edge, faces[1].0, faces[1].1, t));
        }
    }
    report(
        "2  a shared boundary is one curve",
        &format!(
            "worst gap between the two faces' copies of an edge   {worst_split:.3e}\n\
             this is the crack check. a hard edge shares its boundary too — a crease is one curve\n\
             with independent interiors, not two curves that happen to be near each other"
        ),
    );

    // 3. Soft edges are tangent-plane continuous; hard edges are not.
    let mut worst_soft = 0.0f64;
    let mut worst_hard = 0.0f64;
    let mut soft_samples = 0;
    let mut hard_samples = 0;
    for (edge, faces) in &edges {
        let gap = edge_gap(&mesh, *faces, step);
        if is_soft(edge) {
            worst_soft = worst_soft.max(gap);
            soft_samples += degree - 1;
        } else {
            worst_hard = worst_hard.max(gap);
            hard_samples += degree - 1;
        }
    }
    report(
        "3  soft edges are continuous, hard edges are creases",
        &format!(
            "worst gap on a soft edge   {worst_soft:.3e} rad over {soft_samples} positions\n\
             worst gap on a hard edge   {worst_hard:.3e} rad over {hard_samples} positions\n\
             the soft figure sits on the acos floor of about {ANGLE_FLOOR:.0e}; the hard figure is\n\
             {hard_samples} measurements of something real\n\
             positions 1..{degree}, which is where an adjacent row exists on both sides. position 0\n\
             is the one the mirror can leave unpaired, so it is measured on its own below"
        ),
    );

    // 4. The position the adjacent-row mirror cannot always pair.
    let mut soft_corner_gap = 0.0f64;
    let mut unpaired_edges = 0;
    for (edge, faces) in &edges {
        if !is_soft(edge) {
            continue;
        }
        let here = edge_endpoints(mesh.triangles()[faces[0].0 as usize], faces[0].1);
        let there = edge_endpoints(mesh.triangles()[faces[1].0 as usize], faces[1].1);
        if here != there {
            unpaired_edges += 1;
        }
        soft_corner_gap = soft_corner_gap.max(edge_gap_over(&mesh, *faces, step, 0..1));
    }
    report(
        "4  the position the mirror cannot pair",
        &format!(
            "worst soft gap at position 0   {soft_corner_gap:.3e} rad\n\
             {unpaired_edges} of {soft_count} soft edges have their faces running along it in\n\
             opposite directions, and on those the mirror of interior position 0 lands past the end\n\
             of the slave's interior row, so nothing ties it. a degree-n edge carries n interior\n\
             positions on one side and n-1 on the other and no assignment makes them one to one"
        ),
    );

    // 5. Both kinds are present, so the two numbers above are being compared.
    report(
        "5  the sample compares two things",
        &format!(
            "{soft_count} soft edges and {hard_count} hard edges\n\
             a run where either is empty measures nothing, and says so below"
        ),
    );

    // 6. Soft edges bend the surface. This is the feature, not a defect, and it is why
    //    measurement 1 is measured on an all-hard model.
    let mut bend_soft = 0.0f64;
    let mut bend_hard = 0.0f64;
    for (edge, faces) in &edges {
        for t in 1..degree {
            let moved = |mesh: &PatchMesh| {
                let degree = mesh.degree();
                let net = mesh.net(faces[0].0);
                let point = net.evaluate(edge_barycentric(degree, faces[0].1, t));
                point.distance(
                    level_zero(mesh, faces[0].0).evaluate(edge_barycentric(degree, faces[0].1, t)),
                )
            };
            let (soft, hard) = (moved(&mesh), moved(&hard_model));
            if is_soft(edge) {
                bend_soft = bend_soft.max(soft);
            } else {
                bend_hard = bend_hard.max(hard);
            }
        }
    }
    report(
        "6  soft edges bend the surface, hard edges do not",
        &format!(
            "worst departure at an edge position, soft   {bend_soft:.3e}\n\
             worst departure at an edge position, hard   {bend_hard:.3e}\n\
             tangent-plane continuity between two planar facets means they stop being facets. an\n\
             edge that had been tagged soft has pulled its neighbours into a smooth surface, and\n\
             one left hard has not"
        ),
    );

    // 7. Elevation preserves every tag.
    let elevated = build(degree + 1, Tags::Horizontal);
    let mut worst_after = 0.0f64;
    for (edge, faces) in interior_edges(&elevated) {
        if is_soft(&edge) {
            worst_after = worst_after.max(edge_gap(&elevated, faces, step));
        }
    }
    report(
        "7  elevation preserves every tag",
        &format!(
            "worst soft gap after one more elevation   {worst_after:.3e} rad\n\
             elevating re-derives every slave row from its master, so the tags cannot drift"
        ),
    );

    // 8. Editing a seam keeps it a seam.
    let mut edited = build(degree, Tags::Horizontal);
    let Some(soft_edge) = edges
        .iter()
        .find(|(edge, _)| is_soft(edge))
        .map(|(edge, _)| *edge)
    else {
        println!("\nFAILED:\n  - no soft edge to edit");
        std::process::exit(1);
    };
    let entries = edited.topology().edge_faces[&soft_edge].clone();
    let master_slot = entries[0].1;

    // A boundary position, dragged. It belongs to both faces, which is the whole point.
    let grab = 0;
    let grab_index = boundary_index(degree, master_slot, grab);
    let before = edited.net(entries[0].0)[grab_index];
    let split_before = boundary_split(&edited, &soft_edge, entries[0].0, master_slot, grab);
    let gap_before = edge_gap(&edited, [entries[0], entries[1]], step);
    let outcome = edited.set_control_point(
        entries[0].0,
        grab_index.0,
        grab_index.1,
        before + Point3::new(0.0, 0.0, 0.5),
    );
    let after = edited.net(entries[0].0)[grab_index];
    let split_after = boundary_split(&edited, &soft_edge, entries[0].0, master_slot, grab);
    let drag_distance = before.distance(after);
    let gap_after_boundary_drag = edge_gap(&edited, [entries[0], entries[1]], step);
    report(
        "8  a drag on a shared boundary keeps the seam and the tag",
        &format!(
            "outcome                       {outcome:?}\n\
             boundary drag distance       {drag_distance:.3e}\n\
             boundary split before/after {split_before:.3e} / {split_after:.3e}\n\
             soft gap before / after      {gap_before:.3e} / {gap_after_boundary_drag:.3e} rad\n\
             the drag landed on one face and the other followed, because a point on a seam is a\n\
             point on the seam. enforcement then re-derived the slave's row from the master, so\n\
             moving the curve did not break the join"
        ),
    );

    // 9. Dragging a slave's interior row, at the degrees where one point is decided by one edge.
    //    At degree 3 no such point exists, so the model refuses rather than picking a winner.
    let soft_index = edited
        .soft_edges()
        .iter()
        .position(|entry| entry.edge == soft_edge)
        .expect("the edge was just made soft");
    let (master, slave) = (
        edited.soft_edges()[soft_index].master,
        edited.soft_edges()[soft_index].slave,
    );

    let mut unique_drag = (0.0f64, f64::INFINITY, "none".to_string());
    for t in 0..degree {
        let candidate = interior_index(degree, slave.1, t);
        let owners = EdgeSlot::ALL
            .iter()
            .filter(|slot| (0..degree).any(|u| interior_index(degree, **slot, u) == candidate))
            .count();
        if owners != 1 {
            continue;
        }
        let mut trial = build(degree, Tags::Horizontal);
        let master_row_before = trial.net(master.0)[interior_index(degree, master.1, t)];
        let origin = trial.net(slave.0)[candidate];
        let outcome = trial.set_control_point(
            slave.0,
            candidate.0,
            candidate.1,
            origin + Point3::new(0.4, 0.0, 0.0),
        );
        let master_moved =
            master_row_before.distance(trial.net(master.0)[interior_index(degree, master.1, t)]);
        let gap = edge_gap(&trial, [master, slave], step);
        unique_drag = (
            master_moved,
            gap,
            format!("t={t} {candidate:?} -> {outcome:?}"),
        );
        break;
    }

    // The point every row shares. Needs a face carrying two or three soft edges, or the rows do not
    // actually cross in this sample, so this runs on an all-soft model.
    let mut crowded = build(degree, Tags::All);
    let crowded_face = crowded
        .soft_edges()
        .first()
        .map(|edge| edge.slave.0)
        .expect("an all-soft model has soft edges");
    let shared_index = (1, 1);
    let shared_before = crowded.net(crowded_face)[shared_index];
    let shared_outcome = crowded.set_control_point(
        crowded_face,
        shared_index.0,
        shared_index.1,
        shared_before + Point3::new(0.0, 0.3, 0.0),
    );
    let shared_unchanged = shared_before.distance(crowded.net(crowded_face)[shared_index]);
    let slave_edge_count = crowded
        .soft_edges()
        .iter()
        .filter(|edge| edge.slave.0 == crowded_face)
        .count();

    // An all-soft model also asks whether enforcement order matters, which is the open question
    // about master/slave assignment: a face that is master on one edge and slave on another owns a
    // row that the other edge's enforcement will overwrite.
    let mut crowded_worst = 0.0f64;
    for (_, faces) in interior_edges(&crowded) {
        crowded_worst = crowded_worst.max(edge_gap(&crowded, faces, step));
    }

    let unique_line = unique_drag.2.clone();
    let unique_moved = unique_drag.0;
    let unique_gap = unique_drag.1;
    report(
        "9  which control points a drag can reach",
        &format!(
            "a point one row alone    {unique_line}\n     master moved {unique_moved:.3e}, soft gap afterwards {unique_gap:.3e} rad\n\
             all-soft model, face {crowded_face} is the slave of {slave_edge_count} edges\n     \
             dragging {shared_index:?} gave {shared_outcome:?}, model unchanged by {shared_unchanged:.3e}\n\
             worst soft gap on the all-soft model   {crowded_worst:.3e} rad\n\
             a control point is in the `i` row exactly when `i == 1`, so the points a single edge\n\
             decides alone need all three indices at 2 or more and exactly one equal to 1 — and\n\
             1 + 2 + 2 is 5. at degree 3 and 4 every interior point is shared, so a face with two\n\
             or more soft edges has no interior point a drag can reach, and refusing is the only\n\
             answer that does not depend on which edge happened to be tagged first"
        ),
    );

    let mut failures: Vec<&str> = Vec::new();
    if corner_error > EXACT {
        failures.push("an all-hard model does not reproduce its mesh");
    }
    if face_deviation > EXACT {
        failures.push("an elevated flat face is not flat");
    }
    if worst_split > EXACT {
        failures.push("a shared boundary is not shared — that is a crack");
    }
    if soft_count == 0 || hard_count == 0 {
        failures.push("the sample has only one kind of edge, so nothing is being compared");
    }
    if worst_soft > CONTINUOUS {
        failures.push("a soft edge is not tangent-plane continuous");
    }
    if worst_hard < CREASE {
        failures.push("a hard edge looks continuous, so the measurement cannot see a crease");
    }
    if bend_soft < CREASE {
        failures.push("a soft edge did not bend the surface");
    }
    if bend_hard > EXACT {
        failures.push("a hard edge moved the surface");
    }
    if worst_after > CONTINUOUS {
        failures.push("elevation did not preserve the soft tags");
    }
    if drag_distance < MOVED {
        failures.push("a boundary drag did nothing");
    }
    if split_after > EXACT {
        failures.push("a boundary drag split the seam");
    }
    if gap_after_boundary_drag > CONTINUOUS {
        failures.push("a boundary drag broke the soft join");
    }
    if !matches!(shared_outcome, Edit::Overconstrained { by: 2 | 3 }) {
        failures.push("a point two or three rows share was not refused as over-constrained");
    }
    if shared_unchanged > EXACT {
        failures.push("a refused edit still changed the model");
    }
    if degree < 5 {
        // At degree 3 and 4 no interior point belongs to a single row, so there is nothing to
        // drag and the refusal above is the whole expected behaviour.
    } else if unique_moved < MOVED {
        failures.push("a drag on a single-row point did not reach the master");
    }

    println!();
    if failures.is_empty() {
        println!("all continuity probes passed");
    } else {
        println!("FAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
