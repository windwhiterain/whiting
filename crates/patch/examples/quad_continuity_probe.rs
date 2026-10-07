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
//! 10. The corner stencil's algebra, on the real mesh: how many independent chains a vertex has,
//!     what its consistency condition is, and what `mu` a valence-3 centroid actually needs.
//! 11. What a higher degree buys: **nothing**, on either count, and that is measured.
//! 12. What the two skipped row ends actually cost: **nothing at all**, unless two seams write the
//!     same shared place. The obstruction is one degree of freedom per such place.
//! 13. Whether the cheaper fix works: it does not. Deriving one master's corner at a contested place
//!     repairs **fewer** seams and puts six of thirteen corners exactly on their seam.
//!
//! **Measurements 3, 12 and 13 fail by design.** 3 is the known open problem, 12 is the same problem
//! measured on the full sample rather than on one seam, and 13 is the negative result that says the
//! zero-parameter route is closed and a corner scale is genuinely needed.
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
//! because positions 0 and degree are points of the perpendicular edge's shared boundary. But
//! `B(degree-1, 0, v)` and `B(degree-1, degree, v)` are positive at **every** `v` in the interior,
//! so those two unconstrained offsets enter `D` along the whole edge, not only at its ends.
//!
//! The measured consequence at degree 3 is a tangent-plane gap of **4.8e-2 rad** at the interior
//! positions and **1.3e-1 rad** at the ends, against **1.8e-1 rad** for a hard edge. So a soft edge
//! is measurably smoother than a crease and is not smooth.
//!
//! ## The two ends are not the obstruction, and an earlier claim to the contrary is withdrawn
//!
//! [`crate::continuity`] used to argue that enforcing the invariant at positions 0 and `degree` is
//! contradictory and forces `mu == -1`, so the ends were left to the boundary sharing and the gap
//! above was accepted as the price. **Measurement 12 refutes that.** Run the same invariant over the
//! whole row and a lone soft edge comes out exactly G1 — a gap of `0.000e0` at the interior
//! positions and `2.107e-8` at the ends, the latter being the `acos` floor, with the boundary sharing
//! reclaiming `5.0e-16` of what the soft write moved.
//!
//! What does fail is specific and countable. A row end is one step in from its own edge, which puts
//! it on the **perpendicular** edge's boundary row, one step along. When two soft seams meet at a
//! mesh vertex, their two row ends land on the *same place on the same shared edge*: two equations,
//! one value, and the boundary sharing can only give it to one of them. Measurement 12 counts those
//! places — none for a lone seam, one for the minimal two-seam case, thirteen on the full sample —
//! and across all three a seam is exactly continuous **if and only if** no shared place is written
//! by two seams.
//!
//! So the missing thing is not a parameter per edge. It is one degree of freedom per place where two
//! seams land on the same shared point, which is what a corner scale `kappa` would buy.
//!
//! Until that exists, the implementation deliberately leaves the row ends to boundary sharing rather
//! than applying the invariant over them. A crack is worse than a crease.

mod support;
use std::collections::{BTreeMap, BTreeSet};

use support::{ANGLE_FLOOR, COLUMNS, EXACT, grid};

use whiting_patch::Point3;
use whiting_patch::subdivide::{Origin, QuadMesh, subdivide};
use whiting_patch::topology::{
    EdgeKey, EdgeSlot, Topology, boundary_index, canonical, edge_endpoints, interior_index,
    mirrored_position,
};
use whiting_patch::{Continuity, Edit, PatchMesh, RectNet, slave_interior};

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
    build_at(derived, edges, DEGREE)
}

fn build_at(derived: &QuadMesh, edges: &[EdgeKey], degree: usize) -> PatchMesh {
    let mut mesh = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for edge in edges {
        mesh.set_soft(*edge);
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
///
/// Takes the nets rather than the model, so a probe can measure a scratch copy it has edited
/// directly. A measurement that can only see the enforced state cannot see what an alternative
/// would have been.
fn edge_gap_over(
    nets: &[RectNet],
    quads: &[[u32; 4]],
    degree: usize,
    faces: [(u32, EdgeSlot); 2],
    step: f64,
    positions: impl IntoIterator<Item = usize>,
) -> f64 {
    let (master, slave) = (faces[0], faces[1]);
    let here = edge_endpoints(quads[master.0 as usize], master.1);
    let there = edge_endpoints(quads[slave.0 as usize], slave.1);
    positions
        .into_iter()
        .filter(|t| *t <= degree)
        .map(|t| {
            let other_t = mirrored_position(degree, here, there, t);
            folded_angle(
                edge_normal(&nets[master.0 as usize], degree, master.1, t, step),
                edge_normal(&nets[slave.0 as usize], degree, slave.1, other_t, step),
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

/// The two edges of a quad meeting at `corner`, as (incoming, outgoing) going with the winding.
///
/// Corner `c` is reached along the edge from `c-1` and left along the edge to `c+1`, which is what
/// makes the walk below a walk and not a sort.
fn edges_at(corner: usize) -> [EdgeSlot; 2] {
    match corner {
        0 => [EdgeSlot::MinU, EdgeSlot::MinV],
        1 => [EdgeSlot::MinV, EdgeSlot::MaxU],
        2 => [EdgeSlot::MaxU, EdgeSlot::MaxV],
        _ => [EdgeSlot::MaxV, EdgeSlot::MinU],
    }
}

/// The faces meeting at `vertex`, in cyclic order, each with its corner index.
///
/// Built by walking across the outgoing edge, which needs no geometry and no sort: every face agrees
/// on the winding, so "the other face across this edge" is unambiguous.
fn faces_around(mesh: &PatchMesh, vertex: u32) -> Vec<(u32, usize)> {
    let mut order = Vec::new();
    let Some(start) = mesh.quads().iter().position(|quad| quad.contains(&vertex)) else {
        return order;
    };
    let start = start as u32;
    let mut face = start;
    while order.len() <= mesh.quads().len() {
        let quad = mesh.quads()[face as usize];
        let Some(corner) = quad.iter().position(|index| *index == vertex) else {
            break;
        };
        order.push((face, corner));
        let key = canonical(edge_endpoints(quad, edges_at(corner)[1]));
        let Some(faces) = mesh.topology().edge_faces.get(&key) else {
            break;
        };
        let Some(&(next, _)) = faces.iter().find(|(id, _)| *id != face) else {
            break;
        };
        if next == start {
            break;
        }
        face = next;
    }
    order
}

/// What the corner algebra says about every interior vertex of a mesh.
struct CornerSurvey {
    /// valence -> [interior vertices, non-degenerate at the model's mu, needing projection]
    by_valence: BTreeMap<usize, [usize; 3]>,
    /// vertices whose chain count disagreed with `gcd(2, valence)`.
    chain_mismatch: usize,
    /// the distinct projected `mu` triples seen at valence 3.
    centroid_mu: BTreeSet<String>,
}

impl CornerSurvey {
    fn non_degenerate_at_valence_3(&self) -> (usize, usize) {
        match self.by_valence.get(&3) {
            Some([total, non_degenerate, _]) => (*non_degenerate, *total),
            None => (0, 0),
        }
    }
}

/// Runs the corner recursion over every vertex where the walk closes, which is every vertex whose
/// incident edges are all interior. A vertex on the boundary cannot close the cycle, so it has no
/// corner condition to check and is skipped rather than half-checked.
fn corner_survey(mesh: &PatchMesh) -> CornerSurvey {
    let mut by_valence: BTreeMap<usize, [usize; 3]> = BTreeMap::new();
    let mut chain_mismatch = 0usize;
    let mut centroid_mu: BTreeSet<String> = BTreeSet::new();

    for vertex in 0..mesh.positions().len() as u32 {
        let around = faces_around(mesh, vertex);
        let n = around.len();
        if n < 2
            || n != mesh
                .quads()
                .iter()
                .filter(|quad| quad.contains(&vertex))
                .count()
        {
            continue;
        }
        let mut mu = vec![1.0f64; n];
        let mut master_is_left = vec![true; n];
        for (step, (face, corner)) in around.iter().enumerate() {
            let key = canonical(edge_endpoints(
                mesh.quads()[*face as usize],
                edges_at(*corner)[1],
            ));
            mu[step] = mesh.continuity(&key).and_then(|c| c.mu()).unwrap_or(1.0);
            master_is_left[step] = mesh.topology().edge_faces[&key][0].0 == *face;
        }

        let mut projected = mu.clone();
        let mut visited = vec![false; n];
        let mut chains = 0usize;
        let mut feasible = true;
        for start in 0..n {
            if visited[start] {
                continue;
            }
            chains += 1;
            let mut index = start;
            let mut factor = 1.0f64;
            let seam = loop {
                visited[index] = true;
                let seam = (index + 1) % n;
                let m = if master_is_left[seam] {
                    projected[seam]
                } else {
                    1.0 / projected[seam]
                };
                factor *= -m;
                let next = (index + 2) % n;
                index = next;
                if next == start {
                    break seam;
                }
            };
            if (factor - 1.0).abs() > 1.0e-12 {
                feasible = false;
                projected[seam] *= if master_is_left[seam] {
                    factor
                } else {
                    1.0 / factor
                };
            }
        }
        if chains != greatest_common_divisor(2, n) {
            chain_mismatch += 1;
        }
        let entry = by_valence.entry(n).or_insert([0, 0, 0]);
        entry[0] += 1;
        if feasible {
            entry[1] += 1;
        } else {
            entry[2] += 1;
        }
        if n == 3 {
            centroid_mu.insert(format!(
                "{:.3} {:.3} {:.3}",
                projected[0], projected[1], projected[2]
            ));
        }
    }
    CornerSurvey {
        by_valence,
        chain_mismatch,
        centroid_mu,
    }
}

fn greatest_common_divisor(a: usize, b: usize) -> usize {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

/// Makes every shared boundary identical on both sides, the first face on each edge winning.
///
/// This is [`PatchMesh::enforce`]'s hard pass, lifted out so a probe can run it on a scratch copy
/// of the nets. It is here because a whole-row soft write lands on a **boundary** point of the
/// perpendicular edge, and nothing is shared unless the neighbour is told.
fn share_every_boundary(mesh: &PatchMesh, nets: &mut [RectNet]) {
    let degree = mesh.degree();
    for faces in mesh.topology().edge_faces.values() {
        let (owner, follower) = match faces.as_slice() {
            [owner, follower] => (*owner, *follower),
            _ => continue,
        };
        let owner_ends = edge_endpoints(mesh.quads()[owner.0 as usize], owner.1);
        let follower_ends = edge_endpoints(mesh.quads()[follower.0 as usize], follower.1);
        for t in 0..=degree {
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            let (oi, oj) = boundary_index(degree, owner.1, t);
            let (fi, fj) = boundary_index(degree, follower.1, follower_t);
            let point = nets[owner.0 as usize][(oi, oj)];
            nets[follower.0 as usize].set_point(fi, fj, point);
        }
    }
}

/// The worst soft-edge gap of `mesh` at the interior positions and at the two ends, read from
/// `nets` rather than from the model, so a scratch copy can be measured.
fn soft_gap_pair(mesh: &PatchMesh, nets: &[RectNet]) -> (f64, f64) {
    let degree = mesh.degree();
    let mut interior = 0.0f64;
    let mut ends = 0.0f64;
    for edge in mesh.soft_edges() {
        let faces = [edge.master, edge.slave];
        interior = interior.max(edge_gap_over(
            nets,
            mesh.quads(),
            degree,
            faces,
            STEP,
            1..degree,
        ));
        ends = ends.max(edge_gap_over(
            nets,
            mesh.quads(),
            degree,
            faces,
            STEP,
            [0, degree],
        ));
    }
    (interior, ends)
}

/// The largest distance between two faces' copies of one point on a shared boundary.
fn worst_split_over_all(mesh: &PatchMesh, nets: &[RectNet]) -> f64 {
    let degree = mesh.degree();
    mesh.topology()
        .edge_faces
        .values()
        .filter_map(|faces| match faces.as_slice() {
            [owner, follower] => Some((*owner, *follower)),
            _ => None,
        })
        .flat_map(|(owner, follower)| (0..=degree).map(move |t| (owner, follower, t)))
        .map(|(owner, follower, t)| {
            boundary_split_over(nets, mesh.quads(), degree, [owner, follower], t)
        })
        .fold(0.0f64, f64::max)
}

/// The soft invariant over the **whole** row, `t = 0..=degree`, with boundaries re-shared between
/// rounds, and the largest single-point movement of each round.
///
/// The shipped model runs the invariant over `t = 1..degree` because `crate::continuity` argues the
/// two ends are contradictory and force `mu == -1`. That argument is what this measures: if the
/// feedback settles, there is no contradiction and the end positions were simply skipped; if it
/// does not, the argument stands and the movement per round says by how much.
/// The soft invariant over the **whole** row, `t = 0..=degree`, run in [`PatchMesh::enforce`]'s own
/// order: the boundaries are shared first, then the soft edges write, so a soft edge has the last
/// word exactly as it does in the shipped model.
///
/// Returns the state after each round, the distance the soft writes wanted to move, and the distance
/// the sharing took back. Those are two different questions and the shipped model's own order is the
/// only reason they can be asked apart: if the sharing gives nothing back, a row end is compatible
/// with shared geometry and the soft edge can keep it.
///
/// The other reason the order matters is that a row end is a **boundary** point of the perpendicular
/// edge, so writing one moves shared geometry. Writing after the sharing is what lets a measurement
/// see whether that geometry survives, rather than measuring only that the sharing undid it.
fn whole_row_enforcement(
    mesh: &PatchMesh,
    rounds: usize,
) -> (Vec<Vec<RectNet>>, Vec<f64>, Vec<f64>) {
    let degree = mesh.degree();
    let mut nets = mesh.nets().to_vec();
    let mut history = Vec::new();
    let mut written = Vec::new();
    let mut reclaimed = Vec::new();
    for _ in 0..rounds {
        share_every_boundary(mesh, &mut nets);
        let mut moved = 0.0f64;
        for edge in mesh.soft_edges() {
            let Some(mu) = mesh.continuity(&edge.edge).and_then(Continuity::mu) else {
                continue;
            };
            let master_ends = edge_endpoints(mesh.quads()[edge.master.0 as usize], edge.master.1);
            let slave_ends = edge_endpoints(mesh.quads()[edge.slave.0 as usize], edge.slave.1);
            for t in 0..=degree {
                let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
                let (mi, mj) = boundary_index(degree, edge.master.1, t);
                let (ai, aj) = interior_index(degree, edge.master.1, t);
                let (si, sj) = interior_index(degree, edge.slave.1, slave_t);
                let boundary = nets[edge.master.0 as usize][(mi, mj)];
                let master_interior = nets[edge.master.0 as usize][(ai, aj)];
                let target = slave_interior(boundary, master_interior, mu);
                moved = moved.max(nets[edge.slave.0 as usize][(si, sj)].distance(target));
                nets[edge.slave.0 as usize].set_point(si, sj, target);
            }
        }
        let after_soft = nets.clone();
        share_every_boundary(mesh, &mut nets);
        written.push(moved);
        reclaimed.push(nets_distance(&after_soft, &nets));
        history.push(nets.clone());
    }
    (history, written, reclaimed)
}

/// The largest distance any control point moved between two states.
fn nets_distance(first: &[RectNet], second: &[RectNet]) -> f64 {
    let mut worst = 0.0f64;
    for (was, now) in first.iter().zip(second) {
        let (degree_u, degree_v) = now.degrees();
        for i in 0..=degree_u {
            for j in 0..=degree_v {
                worst = worst.max(was.point(i, j).unwrap().distance(now.point(i, j).unwrap()));
            }
        }
    }
    worst
}

/// One soft seam that writes a contested place, and the master row end it reads to do it.
struct SeamClaim {
    slave: (u32, EdgeSlot),
    master: (u32, EdgeSlot),
    /// The master's private-interior point at the same end of its own edge.
    master_index: (usize, usize),
    mu: f64,
}

/// A place on a shared boundary that two or more soft seams both write as a row end.
struct ContestedPlace {
    edge: EdgeKey,
    position: usize,
    claims: Vec<SeamClaim>,
}

/// The shared places that two or more soft seams write as a row end.
///
/// A row end is one step in from the boundary, which puts it on the *perpendicular* edge's boundary
/// row at position one step along. So the key is not the face and the net index — two faces on
/// opposite sides of a shared edge hold the same place under different indices — it is
/// `(shared edge, position along it)`, normalised to the canonical end order so that both faces name
/// one place the same way.
///
/// Counting per face instead would find nothing, and would find nothing honestly: with one soft
/// edge per face no face can ever write a row end twice, and the conflict is not on a face at all.
fn contested_places(model: &PatchMesh) -> Vec<ContestedPlace> {
    let degree = model.degree();
    let mut found: BTreeMap<(EdgeKey, usize), Vec<SeamClaim>> = BTreeMap::new();
    for entry in model.soft_edges() {
        let corners = model.quads()[entry.slave.0 as usize];
        let slave_ends = edge_endpoints(corners, entry.slave.1);
        let master_ends = edge_endpoints(model.quads()[entry.master.0 as usize], entry.master.1);
        let mu = model
            .continuity(&entry.edge)
            .and_then(Continuity::mu)
            .unwrap_or(1.0);
        for t in [0, degree] {
            let point = interior_index(degree, entry.slave.1, t);
            for other in EdgeSlot::ALL {
                if other == entry.slave.1 {
                    continue;
                }
                let Some(position) =
                    (0..=degree).find(|p| boundary_index(degree, other, *p) == point)
                else {
                    continue;
                };
                let ends = edge_endpoints(corners, other);
                let key = canonical(ends);
                let normalised = if ends[0] == key.0 {
                    position
                } else {
                    degree - position
                };
                let master_t = mirrored_position(degree, slave_ends, master_ends, t);
                found.entry((key, normalised)).or_default().push(SeamClaim {
                    slave: entry.slave,
                    master: entry.master,
                    master_index: interior_index(degree, entry.master.1, master_t),
                    mu,
                });
            }
        }
    }
    found
        .into_iter()
        .filter(|(_, claims)| claims.len() > 1)
        .map(|((edge, position), mut claims)| {
            // The lower master id keeps its freedom, so which side is derived does not depend on the
            // order the soft edges happened to be tagged in.
            claims.sort_by_key(|claim| claim.master.0);
            ContestedPlace {
                edge,
                position,
                claims,
            }
        })
        .collect()
}

/// The shared boundary point at `position` of `edge`, read from the face that owns it.
fn shared_point(
    nets: &[RectNet],
    topology: &Topology,
    degree: usize,
    edge: &EdgeKey,
    position: usize,
) -> Option<Point3> {
    let (face, slot) = *topology.edge_faces.get(edge)?.first()?;
    let (i, j) = boundary_index(degree, slot, position);
    Some(nets[face as usize][(i, j)])
}

/// Makes every **hard** edge's boundary identical on both sides, the first face on each edge winning.
///
/// [`share_every_boundary`] copies soft edges too, which is what a whole-row soft write has to
/// survive — and what a *derived master corner* cannot. A corner is one step in from its own edge,
/// which puts it on the perpendicular edge's boundary row, so copying that edge from the far side
/// restores the value the derivation just replaced. Soft edges are therefore left alone here and the
/// soft pass writes them last, which is what [`PatchMesh::enforce`] already does.
fn share_hard_boundaries(mesh: &PatchMesh, nets: &mut [RectNet]) {
    let degree = mesh.degree();
    for (key, faces) in &mesh.topology().edge_faces {
        if mesh.continuity(key).is_some_and(Continuity::is_soft) {
            continue;
        }
        let (owner, follower) = match faces.as_slice() {
            [owner, follower] => (*owner, *follower),
            _ => continue,
        };
        let owner_ends = edge_endpoints(mesh.quads()[owner.0 as usize], owner.1);
        let follower_ends = edge_endpoints(mesh.quads()[follower.0 as usize], follower.1);
        for t in 0..=degree {
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            let (oi, oj) = boundary_index(degree, owner.1, t);
            let (fi, fj) = boundary_index(degree, follower.1, follower_t);
            let point = nets[owner.0 as usize][(oi, oj)];
            nets[follower.0 as usize].set_point(fi, fj, point);
        }
    }
}

/// How many of `model`'s soft seams are exactly tangent-plane continuous in `nets`, and how many
/// there are.
///
/// The worst gap alone cannot answer this. Resolving one contested place changes one seam, and the
/// maximum over thirty-two seams can sit on an untouched one and report the same number either way.
/// Counting the seams is what shows whether anything was fixed.
fn soft_seams_exact(model: &PatchMesh, nets: &[RectNet]) -> (usize, usize) {
    let degree = model.degree();
    let exact = model
        .soft_edges()
        .iter()
        .filter(|entry| {
            let faces = [entry.master, entry.slave];
            edge_gap_over(nets, model.quads(), degree, faces, STEP, 0..=degree) <= CONTINUOUS
        })
        .count();
    (exact, model.soft_edges().len())
}

/// The whole-row invariant with each contested place resolved by **deriving one master's corner**.
///
/// At a contested place two seams want `mu_a * (Q - A_a) == mu_b * (Q - A_b)` on one shared value,
/// so keeping `A_a` free forces `A_b = Q - (mu_a / mu_b) * (Q - A_a)`, which always has a solution.
/// The question this measures is not whether that formula exists — it does — but whether it is
/// **cheap**: what it does to the derived corner's geometry, and whether the seams come out exact
/// afterwards. That is what decides whether a corner scale is needed at all.
fn whole_row_with_derived_corners(
    model: &PatchMesh,
    places: &[ContestedPlace],
    rounds: usize,
) -> (Vec<RectNet>, Vec<(f64, f64)>) {
    let degree = model.degree();
    let topology = model.topology();
    let mut nets = model.nets().to_vec();
    let before = nets.clone();
    for _ in 0..rounds {
        share_hard_boundaries(model, &mut nets);
        for place in places {
            let keeper = &place.claims[0];
            let keeper_point = nets[keeper.master.0 as usize][keeper.master_index];
            let Some(anchor) = shared_point(&nets, topology, degree, &place.edge, place.position)
            else {
                continue;
            };
            for claim in &place.claims[1..] {
                if claim.mu.abs() < f64::EPSILON {
                    continue;
                }
                let target = anchor - (keeper_point - anchor) * (keeper.mu / claim.mu);
                nets[claim.master.0 as usize].set_point(
                    claim.master_index.0,
                    claim.master_index.1,
                    target,
                );
            }
        }
        for entry in model.soft_edges() {
            let Some(mu) = model.continuity(&entry.edge).and_then(Continuity::mu) else {
                continue;
            };
            let master_ends =
                edge_endpoints(model.quads()[entry.master.0 as usize], entry.master.1);
            let slave_ends = edge_endpoints(model.quads()[entry.slave.0 as usize], entry.slave.1);
            for t in 0..=degree {
                let slave_t = mirrored_position(degree, master_ends, slave_ends, t);
                let (mi, mj) = boundary_index(degree, entry.master.1, t);
                let (ai, aj) = interior_index(degree, entry.master.1, t);
                let (si, sj) = interior_index(degree, entry.slave.1, slave_t);
                let anchor = nets[entry.master.0 as usize][(mi, mj)];
                let master_interior = nets[entry.master.0 as usize][(ai, aj)];
                nets[entry.slave.0 as usize].set_point(
                    si,
                    sj,
                    slave_interior(anchor, master_interior, mu),
                );
            }
        }
        share_hard_boundaries(model, &mut nets);
    }
    // For each derived corner: how far it had to move, and how far the moved point sits from the seam
    // it is supposed to be one step off. The second number is the degenerate one — near zero means the
    // point collapsed onto the seam, which is smooth and a pinch.
    let mut cost = Vec::new();
    for place in places {
        let Some(anchor) = shared_point(&nets, topology, degree, &place.edge, place.position)
        else {
            continue;
        };
        for claim in place.claims.iter().skip(1) {
            let index = claim.master_index;
            let was = before[claim.master.0 as usize][index];
            let now = nets[claim.master.0 as usize][index];
            cost.push((was.distance(now), anchor.distance(now)));
        }
    }
    (nets, cost)
}

/// How far apart two points on the same edge are when read from its two faces, read from nets.
fn boundary_split_over(
    nets: &[RectNet],
    quads: &[[u32; 4]],
    degree: usize,
    faces: [(u32, EdgeSlot); 2],
    t: usize,
) -> f64 {
    let (master, slave) = (faces[0], faces[1]);
    let here = edge_endpoints(quads[master.0 as usize], master.1);
    let there = edge_endpoints(quads[slave.0 as usize], slave.1);
    let other_t = mirrored_position(degree, here, there, t);
    let (mi, mj) = boundary_index(degree, master.1, t);
    let (si, sj) = boundary_index(degree, slave.1, other_t);
    nets[master.0 as usize][(mi, mj)].distance(nets[slave.0 as usize][(si, sj)])
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
        let interior = edge_gap_over(
            mesh.nets(),
            mesh.quads(),
            mesh.degree(),
            *faces,
            STEP,
            1..DEGREE,
        );
        let ends = edge_gap_over(
            mesh.nets(),
            mesh.quads(),
            mesh.degree(),
            *faces,
            STEP,
            [0, DEGREE],
        );
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
        .map(|(_, faces)| {
            edge_gap_over(
                tall.nets(),
                tall.quads(),
                tall.degree(),
                *faces,
                STEP,
                1..DEGREE + 1,
            )
        })
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
    let gap_before = edge_gap_over(
        mesh.nets(),
        mesh.quads(),
        mesh.degree(),
        soft_faces,
        STEP,
        1..DEGREE,
    );
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
    let gap_after = edge_gap_over(
        edited.nets(),
        edited.quads(),
        edited.degree(),
        soft_faces,
        STEP,
        1..DEGREE,
    );
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

    // 10. The corner stencil's algebra, on the real mesh rather than on a diagram.

    // 10. The corner stencil's algebra, on the real mesh rather than on a diagram.
    //
    //     At a vertex with `n` faces in cyclic order, writing `q_i` for the corner-adjacent point of
    //     the `i`-th seam measured from the vertex itself, the invariant at the corner says
    //     `q_(i+1) = -m_i * q_(i-1)`. Two things follow that a diagram hides.
    //
    //     The recursion steps by **two**, so the `n` equations split into `gcd(2, n)` independent
    //     chains rather than one coupled system. Each chain closes on its own scalar condition.
    //
    //     And `m_i` is **not** `mu_i`. A seam's master is whichever of its two faces has the lower
    //     id, which does not go round a vertex consistently, so `m_i` is `mu_i` on some seams and
    //     `1 / mu_i` on others.
    let mut full = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    for (key, _) in interior_edges(&bare) {
        full.set_soft(key);
    }
    let survey = corner_survey(&full);
    if survey.chain_mismatch != 0 {
        failures.push("10  the corner system splits into gcd(2, valence) chains");
    }
    let rows: Vec<String> = survey
        .by_valence
        .iter()
        .map(|(valence, [total, feasible, needs_projection])| {
            format!(
                "   valence {valence}: {total} interior vertices, {feasible} non-degenerate at \
                 mu = 1, {needs_projection} needing projection"
            )
        })
        .collect();
    report(
        "10  the corner stencil's algebra",
        &format!(
            "vertices whose chain count disagreed with gcd(2, valence)   {}\n{}\n\
             projected mu at a valence-3 centroid: {}\n\
             the recursion steps by two, so the equations split into gcd(2, valence) chains and each\n\
             closes on its own scalar condition. odd valence needs the product of the m's to be -1;\n\
             even valence needs it to be +1. the m's are mu on some seams and 1/mu on others, because\n\
             a seam's master is the lower face id and that does not go round a vertex consistently\n\
             a vertex is never unsolvable -- when the product is wrong the only solution is q == 0,\n\
             the pinch. it is NON-degenerate solutions that the condition decides\n\
             and that is the part that matters: every mu and every 1/mu is positive when mu is\n\
             positive, so the product is positive and an odd-valence vertex can never be\n\
             non-degenerate at any positive mu. projecting it onto feasibility means driving one mu\n\
             negative -- a fold in the row -- and it then propagates to the vertex at the other end\n\
             of that seam. the {} valence-3 vertices here are Doo-Sabin face centroids, so that is one\n\
             pinch per face of the source mesh",
            survey.chain_mismatch,
            rows.join("\n"),
            if survey.centroid_mu.is_empty() {
                "no valence-3 interior vertex in this sample".to_string()
            } else {
                let all: Vec<&String> = survey.centroid_mu.iter().collect();
                format!("{} distinct outcomes, e.g. {}", all.len(), all[0])
            },
            survey.by_valence.get(&3).map_or(0, |entry| entry[0]),
        ),
    );

    // 11. Does a higher degree buy the corner its freedom, and does it lower the gap?
    //
    //     The prediction was no on the first, and a fall on the second. **The second was wrong.**
    //     The worst measurement sits at the first interior position, where the contaminating end
    //     weight is `B(degree-1, 0, 1/degree) = (1 - 1/degree)^(degree-1)`, and that tends to
    //     `e^-1` rather than to zero — 0.444 at degree 3, 0.402 at degree 6. The defect does not
    //     shrink with degree; it is asymptotically a fixed fraction of the seam.
    let mut sweep = String::new();
    let mut drifted = 0.0f64;
    let mut reference = f64::NAN;
    for degree in 3..=6 {
        let model = build_at(&derived, &soft_edges, degree);
        let gap = interior_edges(&model)
            .iter()
            .filter(|(edge, _)| soft_edges.contains(edge))
            .map(|(_, faces)| {
                edge_gap_over(
                    model.nets(),
                    model.quads(),
                    model.degree(),
                    *faces,
                    STEP,
                    1..degree,
                )
            })
            .fold(0.0f64, f64::max);
        let mut all_soft = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
        for (key, _) in interior_edges(&bare) {
            all_soft.set_soft(key);
        }
        for _ in 1..degree {
            all_soft.elevate();
        }
        let swept = corner_survey(&all_soft);
        let (non_degenerate, total) = swept.non_degenerate_at_valence_3();
        let end_weight = (1.0 - 1.0 / degree as f64).powi(degree as i32 - 1);
        if reference.is_nan() {
            reference = gap;
        } else {
            drifted = drifted.max((gap - reference).abs() / reference);
        }
        sweep.push_str(&format!(
            "   degree {degree}: worst soft gap {gap:.3e} rad   end weight at t=1 {end_weight:.3}   \
             valence-3 non-degenerate {non_degenerate} of {total}   chain mismatches {}\n",
            swept.chain_mismatch
        ));
        if swept.chain_mismatch != 0 {
            failures
                .push("11  the corner system splits into gcd(2, valence) chains at every degree");
        }
        if non_degenerate != 0 {
            failures.push("11  a higher degree would make odd valence non-degenerate");
        }
    }
    if drifted >= 0.1 {
        failures.push("11  the soft gap is flat across degree");
    }
    report(
        "11  what a higher degree buys: nothing",
        &format!(
            "{sweep}   worst relative drift of the gap across degrees 3 to 6   {drifted:.3}\n\
             the corner constraint never sees more than the row's first entry: at a corner the\n\
             derivative is degree * (A[0] - Q[0]), because B(degree-1, j, 0) is non-zero only at j = 0.\n\
             so raising the degree cannot free an odd-valence vertex, and the row of zeroes above is\n\
             the answer, not a failure to look hard enough\n\
             and the gap does NOT fall either. the worst measurement is at the first interior\n\
             position, where the contaminating end weight is (1 - 1/degree)^(degree-1): 0.444 at\n\
             degree 3, 0.402 at degree 6, heading for e^-1 and not for zero. the defect is\n\
             asymptotically a fixed fraction of the seam, so degree buys nothing here either"
        ),
    );

    // 12. Run the invariant over the whole row, ends included, and see what that costs.
    //
    //     `crate::continuity` leaves positions 0 and degree to the boundary sharing because forcing
    //     them would demand `mu == -1`, and measurement 3's 4.8e-2 gap is the price of that decision.
    //     That is an argument, and it is the only thing standing between the model and exact G1.
    //     This is the measurement: run the invariant over `0..=degree`, re-share the boundaries in
    //     between, and see whether the feedback settles or fights itself.
    //
    //     A row end is a **boundary** point of the perpendicular edge, so writing one moves shared
    //     geometry. Whether that is a contradiction is not one question but three, and the cases are
    //     separated because they have different answers:
    //
    //     - **one** soft edge has no other seam to argue with;
    //     - **two** soft edges meeting at a mesh vertex is the smallest arrangement that can argue;
    //     - the full sample is every such arrangement at once.
    //
    //     Running only the full sample says "it does not settle" without saying where, and the
    //     distinction decides whether the corner stencil is a per-vertex condition or a per-edge one.
    let shared_vertex_pair = {
        let pair = matching_edges(&bare);
        let mut found = None;
        for (index, first) in pair.iter().enumerate() {
            for second in &pair[index + 1..] {
                let touches = first.0 == second.0
                    || first.0 == second.1
                    || first.1 == second.0
                    || first.1 == second.1;
                if touches {
                    found = Some((*first, *second));
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        found
    };
    let first_edge = interior_edges(&bare)[0].0;
    let cases: Vec<(&str, PatchMesh)> = vec![
        ("one soft edge", build_with(&derived, &[first_edge])),
        (
            "two soft edges at one vertex",
            build_with(
                &derived,
                &shared_vertex_pair.map_or_else(Vec::new, |p| vec![p.0, p.1]),
            ),
        ),
        ("the full sample", mesh.clone()),
    ];
    let mut rows: Vec<String> = Vec::new();
    let mut lone_exact = false;
    let mut lone_gap = f64::NAN;
    let mut pair_exact = false;
    let mut full_exact = false;
    let mut pair_contested = 0usize;
    let mut full_contested = 0usize;
    for (name, model) in &cases {
        let (history, written, reclaimed) = whole_row_enforcement(model, 4);
        let nets = history.last().expect("four rounds were asked for");
        let settled = nets_distance(&history[history.len() - 2], nets) <= MOVED;
        let (interior, ends) = soft_gap_pair(model, nets);
        let split = worst_split_over_all(model, nets);
        let exact = interior.max(ends) <= CONTINUOUS;
        let contested = contested_places(model);
        let verdict = if exact {
            "EXACT"
        } else if settled {
            "not exact"
        } else {
            "does not settle"
        };
        rows.push(format!(
            "   {name:<24} {verdict:<15} gap {interior:.3e} / {ends:.3e} rad   split {split:.3e}   \
             {}",
            if contested.is_empty() {
                "no shared place is written by two seams".to_string()
            } else {
                format!(
                    "{} shared {} written by two seams, e.g. edge {:?} at position {}",
                    contested.len(),
                    if contested.len() == 1 {
                        "place"
                    } else {
                        "places"
                    },
                    contested[0].edge,
                    contested[0].position
                )
            }
        ));
        rows.push(format!(
            "   {:<24} the soft writes move up to {} per round; the boundary sharing takes back {}",
            "",
            written
                .iter()
                .map(|value| format!("{value:.1e}"))
                .collect::<Vec<String>>()
                .join(" "),
            reclaimed
                .iter()
                .map(|value| format!("{value:.1e}"))
                .collect::<Vec<String>>()
                .join(" "),
        ));
        match *name {
            "one soft edge" => {
                lone_exact = exact;
                lone_gap = interior;
            }
            "two soft edges at one vertex" => {
                pair_exact = exact;
                pair_contested = contested.len();
            }
            _ => {
                full_exact = exact;
                full_contested = contested.len();
            }
        }
        if split > EXACT {
            failures.push("12  whole-row enforcement cracks a boundary somewhere in the sample");
        }
        // The claim worth making is the equivalence, not the three data points: a seam is exactly
        // continuous if and only if no shared place is written by two of them.
        if exact != contested.is_empty() {
            failures.push("12  a seam is exact exactly when no shared place is written twice");
        }
    }
    if !lone_exact {
        failures.push("12  one soft edge on its own cannot be made exactly continuous");
    }
    if lone_exact && lone_gap > CONTINUOUS {
        failures.push("12  a lone soft edge cannot be made exactly continuous");
    }
    if pair_exact {
        failures.push(
            "12  two soft edges meeting at a vertex would both be exact, which is the surprise",
        );
    }
    if !full_exact {
        failures.push("12  the whole-row invariant is not exact on the full sample");
    }
    report(
        "12  running the invariant over the row ends",
        &format!(
            "{}\n\
             ONE soft edge comes out EXACT at {lone_gap:.3e} rad, with the boundary sharing giving\n\
             nothing back. the whole-row invariant is therefore not impossible, and the 4.8e-2 of\n\
             measurement 3 is not a limit of the representation: it is the price of `crate::continuity`\n\
             choosing not to enforce the two ends, and it is being paid for nothing. the module's\n\
             argument that forcing them demands `mu == -1` does not survive this measurement.\n\
             TWO soft edges meeting at a vertex is the smallest arrangement that fails, and it has\n\
             {} shared {} that two seams write as a row end. one place with two equations and one\n\
             value: the sharing hands it to whichever seam wrote last and the other seam loses its row\n\
             end. that is the obstruction, counted rather than argued.\n\
             the full sample has {} such places, and its soft writes are taken back almost exactly as\n\
             far as they reach — the sharing and the invariant are cancelling, which is why the gap\n\
             stays open rather than merely being large.\n\
             the three cases line up with nothing left over: exact when and only when no shared place\n\
             is written by two seams. that equivalence is asserted, not just observed.\n\
             so the thing that is missing is not a parameter per edge. it is one degree of freedom per\n\
             place where two seams land on the same shared point, which is what a corner scale kappa\n\
             would buy — and on this evidence it is the only thing kappa would buy",
            rows.join("\n"),
            pair_contested,
            if pair_contested == 1 {
                "place"
            } else {
                "places"
            },
            full_contested,
        ),
    );

    // 13. Can the collision be resolved without a new parameter?
    //
    //     A corner scale kappa is the proposal on the table. Before building it, the cheaper question:
    //     at a contested place two seams want `mu_a * (Q - A_a) == mu_b * (Q - A_b)` on one shared
    //     value, so **keeping one master's corner free forces the other's**:
    //     `A_b = Q - (mu_a / mu_b) * (Q - A_a)`. That always has a solution and adds no parameter.
    //
    //     What it costs is the only thing worth measuring. Three numbers:
    //       - does the seam come out exact afterwards;
    //       - how far the derived corner had to move from where the user had it;
    //       - how far the moved corner sits from the seam, because a corner that collapses **onto**
    //         the seam is smooth and a pinch, which is the failure the whole design keeps refusing.
    let places = contested_places(&mesh);
    let (resolved, cost) = whole_row_with_derived_corners(&mesh, &places, 4);
    let (resolved_interior, resolved_ends) = soft_gap_pair(&mesh, &resolved);
    let resolved_split = worst_split_over_all(&mesh, &resolved);
    let (shipped_exact, seam_count) = soft_seams_exact(&mesh, mesh.nets());
    let (whole_row_only, _, _) = whole_row_enforcement(&mesh, 4);
    let (whole_row_exact, _) = soft_seams_exact(&mesh, whole_row_only.last().expect("four rounds"));
    let (resolved_exact_count, _) = soft_seams_exact(&mesh, &resolved);
    let pinched = cost.iter().filter(|(_, gap)| *gap <= MOVED).count();
    let smallest_offset = cost
        .iter()
        .map(|(_, gap)| *gap)
        .fold(f64::INFINITY, f64::min);
    let largest_offset = cost.iter().map(|(_, gap)| *gap).fold(0.0f64, f64::max);
    let same_face = places
        .iter()
        .filter(|place| place.claims[0].slave.0 == place.claims[1].slave.0)
        .count();
    if pinched > 0 {
        failures.push("13  a derived corner collapses onto its seam");
    }
    if resolved_exact_count <= whole_row_exact {
        failures.push("13  deriving a master's corner fixes no additional seam");
    }
    if same_face > 0 {
        failures.push("13  two seams write a contested place on the SAME face");
    }
    report(
        "13  resolving the collision by deriving a corner, with no new parameter",
        &format!(
            "contested places on the full sample   {places_len}\n\
             of those, two seams on the same face   {same_face}\n\
             seams exactly continuous   shipped {shipped_exact} of {seam_count}, \
             whole row {whole_row_exact}, whole row plus derived corners {resolved_exact_count}\n\
             soft gap, interior / ends            {resolved_interior:.3e} / {resolved_ends:.3e} rad\n\
             worst boundary split                 {resolved_split:.3e}\n\
             derived-corner distance from its seam {smallest_offset:.3e} .. {largest_offset:.3e}   \
             ({pinched} of {derived_count} collapsed onto the seam)\n\
             a contested place gives two seams `mu_a * (Q - A_a) == mu_b * (Q - A_b)` on ONE shared\n\
             value, so holding the lower-id master's corner free forces the other's:\n\
             `A_b = Q - (mu_a / mu_b) * (Q - A_a)`. no new parameter, always solvable.\n\
             the two numbers to read are the third line and the last. the count says how many seams were\n\
             actually repaired, which the worst gap cannot show; the offset says whether the repaired\n\
             corners are still off their seam, because a corner ON the seam is smooth and a pinch",
            places_len = places.len(),
            derived_count = cost.len(),
        ),
    );

    if failures.is_empty() {
        println!("\nall 13 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        println!(
            "\nmeasurements 3, 12 and 13 are meant to fail: see the module documentation,\n\
             `crate::continuity`, and .agents/handoffs/. 3 is the open problem, 12 is the same\n\
             problem on the full sample, and 13 is the negative result that closes the\n\
             zero-parameter route."
        );
        std::process::exit(1);
    }
}
