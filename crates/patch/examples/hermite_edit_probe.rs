//! Can an agent edit this surface without being able to break it, and can it take a change back?
//!
//! Run with `cargo run -p whiting-patch --example hermite_edit_probe`. It exits non-zero if any
//! property fails.
//!
//! The claim being checked is not that the edits work. It is that **there is no edit here that can
//! put the surface into a state the next read has to repair** — which is what the old model required
//! a caller to know, and what `Edit::Overconstrained` and the write-order arbitration existed for.
//!
//! 1. A batch of edits — lift, tilt, fold — leaves every soft seam **exactly** C1. If any write could
//!    break continuity, this is where it shows.
//! 2. A batch of edits leaves every shared boundary **exactly** C0. No edit opens a crack.
//! 3. **`fold` is one call** and produces the fold it reports. The old model made this a tag plus two
//!    careful writes in the right order.
//! 4. **`unfold` is one call** and removes it, which is the control for 3: a crease that appears and
//!    cannot be taken away is not an edit, it is a one-way door.
//! 5. **Undo restores bit-exactly**, measured as a whole-surface comparison rather than as "the
//!    control points look right". A float that comes back one ulpe out is a surface an agent has
//!    drifted from, and it compounds.
//! 6. **A region edit leaves no lip.** The falloff reaches zero *with zero slope* at the edge of its
//!    support, so lifting a region does not leave a step where it ends — a step in the positions is
//!    exactly the kind of thing that cannot be seen until it draws.

use whiting_patch::edit::Editor;
use whiting_patch::topology::EdgeKey;
use whiting_patch::{FaceId, HermiteSurface, Join, Point3};

/// Two unit vectors that close have a dot product `f64` cannot resolve below `1 - 1e-16`, and `acos`
/// turns that into about `1.4e-8`. A normal reads a small multiple of it.
const C1_CEILING: f64 = 1.0e-7;
const EXACT: f64 = 1.0e-12;

fn parameters() -> [f64; 5] {
    [0.0, 0.25, 0.5, 0.75, 1.0]
}

/// Reads every interior edge and returns the worst split and the worst normal angle.
fn measure(surface: &HermiteSurface, folds: &[EdgeKey]) -> ((f64, f64), (f64, f64)) {
    let mut soft = (0.0f64, 0.0f64);
    let mut hard = (0.0f64, 0.0f64);
    for (edge, faces) in &surface.topology().edge_faces {
        if faces.len() != 2 {
            continue;
        }
        let (first, first_slot) = faces[0];
        let (second, _) = faces[1];
        let across_u = first_slot.separates_u();
        let (mut worst_split, mut worst_normal) = (0.0f64, 0.0f64);
        for along in parameters() {
            let (a, b) = if across_u {
                ((1.0, along), (0.0, along))
            } else {
                ((along, 1.0), (along, 0.0))
            };
            let one = surface.patch(first);
            let two = surface.patch(second);
            worst_split = worst_split.max(one.evaluate(a.0, a.1).distance(two.evaluate(b.0, b.1)));
            let here = one.normal(a.0, a.1);
            let there = two.normal(b.0, b.1);
            let angle = here.angle(there);
            worst_normal = worst_normal.max(if angle.is_finite() {
                angle
            } else {
                f64::INFINITY
            });
        }
        if folds.contains(edge) {
            hard = (hard.0.max(worst_split), hard.1.max(worst_normal));
        } else {
            soft = (soft.0.max(worst_split), soft.1.max(worst_normal));
        }
    }
    (soft, hard)
}

/// Every point of a surface, flattened, so two surfaces can be compared in full.
fn fingerprint(surface: &HermiteSurface) -> Vec<f64> {
    let mut points = surface.positions().to_vec();
    for vertex in 0..surface.positions().len() {
        let (du, dv) = surface.derivative(vertex as u32);
        points.push(du);
        points.push(dv);
    }
    for face in 0..surface.face_count() as FaceId {
        for corner in 0..4 {
            points.push(surface.twist(face, corner));
        }
    }
    let mut out = Vec::with_capacity(points.len() * 3);
    for point in points {
        out.extend_from_slice(&[point.x, point.y, point.z]);
    }
    out
}

fn worst_difference(before: &[f64], after: &[f64]) -> f64 {
    assert_eq!(
        before.len(),
        after.len(),
        "two fingerprints must hold the same data"
    );
    before
        .iter()
        .zip(after.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max)
}

/// A gently bowed sheet, so an edit has something to change that is not a flat plane.
fn sample() -> HermiteSurface {
    let (columns, rows) = (7usize, 5usize);
    let mut positions = Vec::new();
    for row in 0..=rows {
        for column in 0..=columns {
            let x = column as f64 / columns as f64;
            let y = row as f64 / rows as f64;
            positions.push(Point3::new(
                4.0 * x - 2.0,
                3.0 * y - 1.5,
                0.7 * (x * x + y * y),
            ));
        }
    }
    let mut quads = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let base = (row * (columns + 1) + column) as u32;
            quads.push([
                base,
                base + 1,
                base + columns as u32 + 2,
                base + columns as u32 + 1,
            ]);
        }
    }
    HermiteSurface::build(positions, quads).expect("a grid mesh is representable")
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let surface = sample();
    let before = fingerprint(&surface);
    let folds: Vec<EdgeKey> = Vec::new();

    let (soft_before, _) = measure(&surface, &folds);
    let mut editor = Editor::new(surface);
    let centre = Point3::new(0.2, -0.1, 0.0);

    // -- 1 and 2: a batch of edits, then measure. ---------------------------------------------------
    let lifted = editor.lift(centre, 2.0, Point3::new(0.0, 0.0, 0.35));
    let tilted = editor.tilt(centre, 1.5, Point3::new(0.0, 0.0, 0.2), 1.0);
    let fold_edge = editor
        .surface()
        .topology()
        .edge_faces
        .keys()
        .find(|edge| editor.surface().topology().edge_faces[*edge].len() == 2)
        .copied()
        .expect("a grid has an interior edge");
    let angle = editor.fold(fold_edge, 6.0);
    let (soft_after, hard_after) = measure(editor.surface(), &[fold_edge]);

    println!(
        "\nedits: lift moved {lifted} vertices, tilt touched {tilted}, fold read {angle:.3e} rad"
    );
    println!(
        "  soft seams before   split {:.3e}   normal {:.3e} rad",
        soft_before.0, soft_before.1
    );
    println!(
        "  soft seams after    split {:.3e}   normal {:.3e} rad",
        soft_after.0, soft_after.1
    );
    println!(
        "  the fold itself     split {:.3e}   normal {:.3e} rad",
        hard_after.0, hard_after.1
    );

    if lifted == 0 || tilted == 0 {
        failures.push("1  a region edit reached the surface at all");
    }
    if soft_after.1 > C1_CEILING {
        failures.push("1  a batch of edits leaves every soft seam exactly C1");
    }
    if soft_after.0 > EXACT {
        failures.push("2  a batch of edits opens no crack");
    }
    if hard_after.0 > EXACT {
        failures.push("2  a fold is C0, never a crack");
    }

    // -- 3 and 4: fold and unfold, one call each. --------------------------------------------------
    if angle < 0.5 {
        failures.push("3  fold produces the fold it reports");
    }
    editor.unfold(fold_edge);
    let (soft_unfolded, hard_unfolded) = measure(editor.surface(), &[fold_edge]);
    println!(
        "  after unfold        split {:.3e}   normal {:.3e} rad  (the crease is gone)",
        hard_unfolded.0, hard_unfolded.1
    );
    if hard_unfolded.1 > C1_CEILING {
        failures.push("4  unfold removes the crease it made");
    }
    if soft_unfolded.1 > C1_CEILING || soft_unfolded.0 > EXACT {
        failures.push("4  unfolding leaves the surface exactly C1 and closed");
    }

    // -- 5: undo restores bit-exactly. ---------------------------------------------------------------
    //
    // **Known to fail, and the cause is known.** A twist write's reach depends on the tags at the
    // moment of the write — `set_twist` carries along the soft edges, so the same call on a soft
    // edge touches one slot and on a hard one touches two. Undo replays the carry with the tags as
    // they are *now*, and the batch above folded, unfolded and folded again, so the rings recorded on
    // the way out do not line up with the rings replayed on the way back. Measured `3.220e-1`.
    //
    // The fix is to record the ring itself rather than recompute it, which means an edit carries a
    // list rather than three numbers. Not built. The rest of the editing claim does not depend on
    // it — nothing below is about undo — so this stays red rather than being dropped, and the number
    // is written down so the next person does not rediscover it.
    editor.fold(fold_edge, 6.0);
    let depth = editor.depth();
    while editor.undo() {}
    let after_undo = fingerprint(editor.surface());
    let drift = worst_difference(&before, &after_undo);
    println!(
        "\nundo: {depth} edits popped, whole-surface drift {drift:.3e}  (known gap, see the comment)"
    );
    if drift != 0.0 {
        failures.push(
            "5  undo restores the surface bit-exactly (known gap: twist reach is recomputed)",
        );
    }

    // -- 6: a region edit leaves no lip. ------------------------------------------------------------
    let original = sample();
    let start: Vec<Point3> = original.positions().to_vec();
    let mut editor = Editor::new(original);
    let radius = 1.5;
    editor.lift(centre, radius, Point3::new(0.0, 0.0, 1.0));
    // **The displacement, not the height.** Measuring `positions()[v].z` measures the dome as well,
    // and the dome is up at `0.7` everywhere, so a probe that reads the height reports a lift of the
    // whole sheet by its own shape -- which is the second `0.000e+00` this session that could not
    // fail. A region edit's claim is about what it changed.
    let mut inner_moved = 0.0f64;
    let mut outer_moved = 0.0f64;
    for vertex in 0..editor.surface().positions().len() {
        let distance = editor.surface().positions()[vertex].distance(centre);
        let moved = editor.surface().positions()[vertex].distance(start[vertex]);
        if distance < radius * 0.9 {
            inner_moved = inner_moved.max(moved);
        } else if distance > radius * 1.1 {
            outer_moved = outer_moved.max(moved);
        }
    }
    println!(
        "\nregion lift of radius {radius}: the support's interior moved up to {inner_moved:.3e}, \
         outside it up to {outer_moved:.3e}"
    );
    if inner_moved <= 0.0 {
        failures.push("6  the region edit did anything at all");
    }
    if outer_moved != 0.0 {
        failures.push("6  the region edit left a lip outside its support");
    }

    // A tag is still a tag: an edge nobody touched keeps its default.
    let default = editor.surface().join(&fold_edge);
    println!(
        "an untouched edge is still {}",
        match default {
            Join::Soft => "soft",
            Join::Hard => "hard",
        }
    );

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
