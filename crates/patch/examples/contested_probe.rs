//! Whether a face with two or more soft edges can satisfy all of them, and whether the answer depends
//! on anything but the mesh.
//!
//! Run with `cargo run -p whiting-patch --example contested_probe`. A diagnostic rather than a test.
//!
//! `crate::continuity` accepts a gap between a soft seam and a crease, and
//! `examples/quad_continuity_probe.rs` guards it. This is about a different question: a face whose
//! **two adjacent edges** are both soft, where `interior_index` puts both edges' rows on the same
//! interior point and the two demand different values there.
//!
//! 1. **A lone seam, as the control.** Two patches sharing one soft edge, with nothing to contest. It
//!    reads a mirror residual of exactly zero and still folds by the gap `continuity` documents, so
//!    every number below it is that accepted cost plus whatever the contest adds.
//! 2. **A crowded face.** Grids where every face carries two or more soft edges, with the per-edge
//!    split printed. A partial survivor is visible here in a way a median would hide.
//! 3. **Whether the tag order changes the surface.** The same edges, tagged forwards and backwards.
//!    This is the measurement that matters: `interior_index` claims contested points by write order,
//!    so before the arbitration in `crate::enforce` was keyed on the slot, this read **9 of 256**
//!    control points different by `6.142e-2` and took the worst fold from `2.325e-1` to `3.330e-1`.
//! 4. **That a second pass is a fixed point**, which is what makes the three above single-valued.

use std::sync::atomic::{AtomicUsize, Ordering};

use whiting_patch::topology::{EdgeKey, EdgeSlot, interior_index};
use whiting_patch::{Continuity, PatchMesh, Point3, VertexId};

/// The degree every grid here is built at.
const DEGREE: usize = 3;

/// The most a soft seam may fold by, in radians.
///
/// A regression ceiling, not a target. The lone seam reads `5.846e-2` and the crowded grids read up
/// to `2.3e-1`; the ceiling sits above both so that a change which makes the contest worse fails,
/// while leaving room for the shape to be less curved than this probe's.
const CEILING: f64 = 4.0e-1;

/// The largest difference two surfaces may differ by for the tag order to count as irrelevant.
///
/// **Exactly zero, not a tolerance.** Both orders run the same arithmetic on the same mesh, so there
/// is nothing for rounding to disagree about: the only thing that can differ is which edge's demand
/// was written at a contested point, and a rule keyed on the slot settles that before it is written.
const ORDER_INDEPENDENCE: f64 = 0.0;

static MEASUREMENTS: AtomicUsize = AtomicUsize::new(0);

fn report(name: &str, detail: &str) {
    MEASUREMENTS.fetch_add(1, Ordering::Relaxed);
    println!("\n== {name} ==\n{detail}");
}

/// A grid of `columns` by `rows` patches over a surface curved enough that its seams are not
/// trivially satisfied before enforcement.
fn grid(columns: usize, rows: usize) -> PatchMesh {
    let mut positions = Vec::new();
    for j in 0..=rows {
        for i in 0..=columns {
            let (x, z) = (i as f64 / columns as f64, j as f64 / rows as f64);
            let height = 0.2 * ((x * 3.1).sin() * (z * 2.7).cos() + 0.3 * (x * 5.3).sin());
            positions.push(Point3::new(x, height, z));
        }
    }
    let stride = columns + 1;
    let mut quads = Vec::new();
    for j in 0..rows {
        for i in 0..columns {
            let near = (j * stride + i) as VertexId;
            let along_u = near + 1;
            let along_v = near + stride as VertexId;
            quads.push([near, along_u, along_v + 1, along_v]);
        }
    }
    let mut mesh = PatchMesh::build(positions, quads);
    for _ in 1..DEGREE {
        mesh.elevate();
    }
    mesh
}

/// The angle between the two faces of `edge` at the middle of it.
///
/// The slot names the parameter the edge **fixes**, so the other is what `along` moves. Reading them
/// the other way round asks each face for a normal off its *perpendicular* edge and returns a
/// plausible fold that is not there — which is how an earlier draft of this probe reported the soft
/// folds at `1.7e-1` against a crease at `3.4e-1` and was measuring nothing.
fn fold_where(mesh: &PatchMesh, edge: &EdgeKey, along: f64) -> Option<f64> {
    let faces = mesh.topology().edge_faces.get(edge)?;
    if faces.len() != 2 {
        return None;
    }
    let normal_of = |(face, slot): (whiting_patch::FaceId, EdgeSlot)| {
        let (fixed, moves_u) = match slot {
            EdgeSlot::MinV => (0.0, true),
            EdgeSlot::MaxV => (1.0, true),
            EdgeSlot::MinU => (0.0, false),
            EdgeSlot::MaxU => (1.0, false),
        };
        mesh.composed().face(face as usize).map(|net| {
            if moves_u {
                net.normal(along, fixed)
            } else {
                net.normal(fixed, along)
            }
        })
    };
    Some(normal_of(faces[0])?.angle(normal_of(faces[1])?))
}

/// How far each soft edge's slave row sits from the mirror the invariant asks for.
///
/// Zero on a lone seam, and the size of the contest's share of the disagreement on a crowded one.
fn mirror_residual(mesh: &PatchMesh) -> f64 {
    let mut worst: f64 = 0.0;
    for edge in mesh.soft_edges() {
        let master = edge.master.0 as usize;
        let slave = edge.slave.0 as usize;
        for t in 1..DEGREE {
            let (mi, mj) = whiting_patch::topology::boundary_index(DEGREE, edge.master.1, t);
            let (ai, aj) = interior_index(DEGREE, edge.master.1, t);
            let (si, sj) = interior_index(DEGREE, edge.slave.1, t);
            let boundary = mesh.level().face(master).unwrap()[(mi, mj)];
            let master_interior = mesh.level().face(master).unwrap()[(ai, aj)];
            let wanted = whiting_patch::slave_interior(boundary, master_interior, 1.0);
            let have = mesh.level().face(slave).unwrap()[(si, sj)];
            worst = worst.max((wanted - have).length());
        }
    }
    worst
}

/// Every soft edge's fold, worst first.
fn soft_folds(mesh: &PatchMesh) -> Vec<(EdgeKey, f64)> {
    let mut folds: Vec<(EdgeKey, f64)> = mesh
        .topology()
        .edge_faces
        .iter()
        .filter(|(edge, faces)| {
            faces.len() == 2 && mesh.continuity(edge).is_some_and(Continuity::is_soft)
        })
        .filter_map(|(edge, _)| fold_where(mesh, edge, 0.5).map(|fold| (*edge, fold)))
        .collect();
    folds.sort_by(|a, b| b.1.total_cmp(&a.1));
    folds
}

/// How many faces carry more than one soft edge, which is what puts a point in contention.
fn crowded_faces(mesh: &PatchMesh) -> usize {
    let mut per_face = vec![0usize; mesh.quads().len()];
    for edge in mesh.soft_edges() {
        per_face[edge.slave.0 as usize] += 1;
        per_face[edge.master.0 as usize] += 1;
    }
    per_face.iter().filter(|count| **count > 1).count()
}

fn interior_edges(mesh: &PatchMesh) -> Vec<EdgeKey> {
    mesh.topology()
        .edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() == 2)
        .map(|(edge, _)| *edge)
        .collect()
}

fn describe(label: &str, mesh: &PatchMesh, failures: &mut Vec<String>) {
    let folds = soft_folds(mesh);
    let worst = folds.first().map(|(_, fold)| *fold).unwrap_or(f64::NAN);
    let contested = crowded_faces(mesh);
    report(
        label,
        &format!(
            "soft edges {:>3}   faces carrying 2+ soft edges {:>3}\n\
             mirror residual {:.3e}   worst fold {:.3e} rad ({:.3} deg)\n\
             per edge, worst first: {}",
            mesh.soft_edges().len(),
            contested,
            mirror_residual(mesh),
            worst,
            worst.to_degrees(),
            folds
                .iter()
                .map(|(edge, fold)| format!("{edge:?}={fold:.3e}"))
                .collect::<Vec<_>>()
                .join("  "),
        ),
    );
    if worst > CEILING {
        failures.push(format!(
            "{label}: the worst soft fold is {worst:.3e} against a ceiling of {CEILING:.3e}"
        ));
    }
    if !worst.is_finite() {
        failures.push(format!("{label}: the worst fold could be read"));
    }
}

fn main() {
    let mut failures: Vec<String> = Vec::new();

    // 1 -- the control. Nothing to contest, so this is the accepted gap and nothing else.
    let mut lone = grid(2, 1);
    let seam = interior_edges(&lone)[0];
    lone.set_soft(seam);
    let lone_residual = mirror_residual(&lone);
    let lone_worst = soft_folds(&lone)[0].1;
    report(
        "1  a lone seam, which is the control for the three below",
        &format!(
            "soft edges 1   faces carrying 2+ soft edges 0\n\
             mirror residual {lone_residual:.3e}   worst fold {lone_worst:.3e} rad ({:.3} deg)\n\
             \n\
             The mirror is satisfied to the last bit and the seam still folds. That residue is the\n\
             gap `crate::continuity` documents and accepts -- the ends of a slave's row are left\n\
             un-derived -- so every number below is that cost plus whatever the contest adds.",
            lone_worst.to_degrees()
        ),
    );
    if lone_residual != 0.0 {
        failures.push(format!(
            "1  a lone seam satisfies the mirror exactly (residual {lone_residual:.3e})"
        ));
    }
    if lone_worst > CEILING {
        failures.push(format!(
            "1  the lone seam is within the ceiling ({lone_worst:.3e} against {CEILING:.3e})"
        ));
    }

    // 2 -- crowded faces, with the per-edge split so a partial survivor stays visible.
    let mut crowded: Vec<(usize, usize)> = Vec::new();
    for (columns, rows) in [(2usize, 2usize), (3, 2), (4, 2), (4, 4)] {
        let mut mesh = grid(columns, rows);
        for edge in interior_edges(&mesh) {
            mesh.set_soft(edge);
        }
        describe(
            &format!("2  {columns} by {rows}, every interior edge soft"),
            &mesh,
            &mut failures,
        );
        crowded.push((columns, rows));
    }

    // 3 -- the order the edges were tagged in. This is the measurement the change was made for.
    let forward_edges = interior_edges(&grid(4, 4));
    let mut forward = grid(4, 4);
    for edge in &forward_edges {
        forward.set_soft(*edge);
    }
    let mut backward = grid(4, 4);
    for edge in forward_edges.iter().rev() {
        backward.set_soft(*edge);
    }
    let differing = forward
        .level()
        .points()
        .iter()
        .zip(backward.level().points())
        .filter(|(a, b)| (**a - **b).length() > 0.0)
        .count();
    let furthest = forward
        .level()
        .points()
        .iter()
        .zip(backward.level().points())
        .map(|(a, b)| (*a - *b).length())
        .fold(0.0f64, f64::max);
    let forward_worst = soft_folds(&forward)[0].1;
    let backward_worst = soft_folds(&backward)[0].1;
    report(
        "3  the tag order does not change the surface",
        &format!(
            "control points that differ   {differing} of {}\n\
             largest difference          {furthest:.3e}\n\
             worst fold, tagged forwards {forward_worst:.3e} rad ({:.3} deg)\n\
             worst fold, tagged backwards {backward_worst:.3e} rad ({:.3} deg)\n\
             \n\
             Two edges claiming one interior point want different values there, and only one can\n\
             hold. Which one held used to be whichever came last in the order the caller tagged them,\n\
             so the same mesh and the same tags gave a different surface depending on how it was\n\
             walked. The arbitration is now keyed on the slot, which is a property of the geometry.",
            forward.level().points().len(),
            forward_worst.to_degrees(),
            backward_worst.to_degrees(),
        ),
    );
    if furthest > ORDER_INDEPENDENCE {
        failures.push(format!(
            "3  the two tag orders give the same surface ({differing} control points differ, furthest \
             {furthest:.3e} against a required {ORDER_INDEPENDENCE:.3e})"
        ));
    }
    if (forward_worst - backward_worst).abs() > ORDER_INDEPENDENCE {
        failures.push(format!(
            "3  the two tag orders give the same worst fold ({forward_worst:.3e} against \
             {backward_worst:.3e})"
        ));
    }

    // 4 -- a fixed point, without which the three above are not single-valued.
    let mut settled = grid(4, 4);
    for edge in interior_edges(&settled) {
        settled.set_soft(edge);
    }
    let before = settled.level().points().to_vec();
    settled.enforce();
    let moved = before
        .iter()
        .zip(settled.level().points())
        .filter(|(a, b)| (**a - **b).length() > 0.0)
        .count();
    report(
        "4  a second pass moves nothing",
        &format!(
            "control points a second pass moves   {moved} of {}\n\
             \n\
             `enforce` writes only affine combinations of points already in the level, so it is a\n\
             fixed point of itself rather than an iteration that has to be run to convergence. The\n\
             earlier version ran to a fixed point by accident of the write order; this one does so\n\
             because there is nothing left to arbitrate.",
            before.len()
        ),
    );
    if moved != 0 {
        failures.push(format!(
            "4  a second enforcement pass moves nothing ({moved} moved)"
        ));
    }

    if failures.is_empty() {
        println!(
            "\nall {} measurements passed",
            MEASUREMENTS.load(Ordering::Relaxed)
        );
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
