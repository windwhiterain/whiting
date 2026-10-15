//! Measures the Hermite gallery scene's surface, not the document it compiles to.
//!
//! Run with `cargo run -p whiting-gallery --example hermite_probe`. It exits non-zero if any
//! property fails.
//!
//! The scene's claim is narrow and this is it: **the crease is the sharpest thing on the sheet, and
//! every other seam is exactly C1.** Those are two different measurements and a scene can satisfy
//! one without the other, so both are read, and the ratio between them is the number that says the
//! sheet is a sheet with a fold in it rather than a grid of facets that happens to include a fold.
//!
//! 1. Every seam that is **not** on the crease reads exactly C1 — at the `acos` floor, not at a
//!    tolerance.
//! 2. The crease reads a large fold. Written down rather than asserted as "large", so a scene that
//!    quietly lost its crease fails.
//! 3. The crease is the separation. A sheet whose creases are softer than its seams would pass both
//!    of the above and still be the wrong picture.
//! 4. **The control.** Tagging the crease soft and re-twisting removes the fold, which is what makes
//!    measurement 2 a measurement of the tag rather than of the geometry. Without it, a dome with a
//!    ridge baked into its positions would pass everything.

use patch::hermite_surface::Join;
use patch::topology::EdgeKey;
use patch::{FaceId, HermiteSurface, Point3};
use whiting_gallery::hermite::{fold_edges, sheet};

/// Two unit vectors that close have a dot product `f64` cannot resolve below `1 - 1e-16`, and `acos`
/// turns that into about `1.4e-8`. A normal reads a small multiple of it, so the ceiling sits a
/// decade above and no tighter.
const C1_CEILING: f64 = 1.0e-7;
const MINIMUM_CREASE: f64 = 0.5;

fn parameters() -> [f64; 5] {
    [0.0, 0.25, 0.5, 0.75, 1.0]
}

/// Reads one shared edge: how far the two faces put the point, and how far apart their normals are.
fn read_edge(surface: &HermiteSurface, edge: EdgeKey) -> (f64, f64) {
    let faces = &surface.topology().edge_faces[&edge];
    if faces.len() != 2 {
        return (0.0, 0.0);
    }
    let (first_face, first_slot) = faces[0];
    let (second_face, second_slot) = faces[1];

    // Which parameter the edge fixes, from the two faces' slots. Both are `Min*U`/`Max*U` or both
    // are `Min*V`/`Max*V`, which `HermiteSurface::build` guarantees.
    let across_u = first_slot.separates_u();

    // **The two faces run along a shared edge in opposite directions**, so the parameter that names
    // the same place on one is `1 - t` on the other. Comparing both at the same number measures a
    // distance between two different points on one curve — and it reads as a small split rather
    // than an obvious one, because the curve is nearly straight and its two ends are nearly
    // interchangeable. Measured that way it came out `2.9e-3` on a seam whose two faces are the
    // same curve. `topology::mirrored_position` is this same question for the Bernstein model.
    let first_ends =
        patch::topology::edge_endpoints(surface.quads()[first_face as usize], first_slot);
    let second_ends =
        patch::topology::edge_endpoints(surface.quads()[second_face as usize], second_slot);
    let mirrored = first_ends != second_ends;

    let mut worst_split = 0.0f64;
    let mut worst_normal = 0.0f64;
    for along in parameters() {
        let across = if mirrored { 1.0 - along } else { along };
        let (first_uv, second_uv) = if across_u {
            ((1.0, along), (0.0, across))
        } else {
            ((along, 1.0), (across, 0.0))
        };
        let first = surface.patch(first_face);
        let second = surface.patch(second_face);
        worst_split = worst_split.max(
            first
                .evaluate(first_uv.0, first_uv.1)
                .distance(second.evaluate(second_uv.0, second_uv.1)),
        );
        let here = first.normal(first_uv.0, first_uv.1);
        let there = second.normal(second_uv.0, second_uv.1);
        let angle = here.angle(there);
        worst_normal = worst_normal.max(if angle.is_finite() {
            angle
        } else {
            f64::INFINITY
        });
    }
    (worst_split, worst_normal)
}

fn seam_split_both(surface: &HermiteSurface, first: FaceId, second: FaceId) -> (f64, f64) {
    let a = surface.patch(first);
    let b = surface.patch(second);
    let as_u = a.evaluate(1.0, 0.0).distance(b.evaluate(0.0, 0.0));
    let as_v = a.evaluate(0.0, 1.0).distance(b.evaluate(0.0, 0.0));
    (as_u, as_v)
}
fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let surface = sheet();
    let fold = fold_edges();

    let mut soft_split = 0.0f64;
    let mut soft_normal = 0.0f64;
    let mut crease_split = 0.0f64;
    let mut crease_normal = 0.0f64;
    for (edge, faces) in &surface.topology().edge_faces {
        if faces.len() != 2 {
            continue;
        }
        let (split, normal) = read_edge(&surface, *edge);
        if fold.contains(edge) {
            crease_split = crease_split.max(split);
            crease_normal = crease_normal.max(normal);
        } else {
            soft_split = soft_split.max(split);
            soft_normal = soft_normal.max(normal);
        }
    }

    println!(
        "\n{} interior edges, {} of them on the crease",
        surface
            .topology()
            .edge_faces
            .values()
            .filter(|faces| faces.len() == 2)
            .count(),
        fold.len()
    );
    println!("soft seams   split {soft_split:.3e}   normal {soft_normal:.3e} rad");
    println!("crease       split {crease_split:.3e}   normal {crease_normal:.3e} rad");

    if soft_split > 1.0e-12 || soft_normal > C1_CEILING {
        failures.push("1  every seam off the crease is exactly C1");
    }
    if crease_split > 1.0e-12 {
        failures.push("2  the crease is C0, never a crack");
    }
    if crease_normal < MINIMUM_CREASE {
        failures.push("2  the crease is a visible fold");
    }
    if crease_normal / soft_normal.max(f64::MIN_POSITIVE) < 1.0e6 {
        failures.push("3  the crease is far sharper than the seams beside it");
    }

    // 4. The control: untag the crease and re-twist, and the fold goes with it. Untagging alone is
    // not enough and would report a null result -- the two faces already hold different twists
    // from `crease`, and what the tag governs is whether a write is *carried*, not whether the
    // values that are already there match. This is the mistake a control that cannot fail makes.
    let mut softened = surface.clone();
    for edge in &fold {
        softened.set_join(*edge, Join::Soft);
    }
    for row in 0..whiting_gallery::hermite::fold_edges().len() {
        let _ = row;
    }
    let crease_faces = softened.topology().edge_faces[&fold[0]].clone();
    softened.set_twist(crease_faces[0].0, 1, Point3::ZERO);
    softened.set_twist(crease_faces[1].0, 3, Point3::ZERO);
    let (softened_split, softened_normal) = read_edge(&softened, fold[0]);
    println!(
        "\ncontrol: the same edge retagged soft and both sides re-twisted\n  split {softened_split:.3e}   normal {softened_normal:.3e} rad"
    );
    if softened_normal > C1_CEILING {
        failures.push("4  the control fires: retagging the crease soft removes the fold");
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
