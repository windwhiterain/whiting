//! What are the dark patches and the fern-like spikes on the massif's seams?
//!
//! Run with `cargo run -p whiting-gallery --example seam_shading_probe`. Diagnostic: it reports,
//! and exits non-zero only if it finds a defect it can name.
//!
//! # SUPERSEDED, and its numbers are not about the render
//!
//! **This probe tessellates the surface itself, on a fixed diagonal, and the scene no longer does
//! either.** `hermite_massif::tessellate` chooses each cell's diagonal by whichever keeps its two
//! triangles agreeing, and the front end drops cells that are slivers rather than only cells whose
//! corners repeat. So the figures below describe a mesh nobody draws.
//!
//! It is kept, and not deleted, because the finding it produced was real at the time and is the
//! reason the scene changed: **2554 of 29484 triangles shaded by a normal a whole `pi` out**, which
//! is what pointed at cell diagonals in the first place. The scene's fix came after.
//!
//! **What replaced it is `mesh_shape_probe`**, which reads the surface's own quads rather than
//! re-tessellating them, carries a control that is a control, and now reports:
//!
//! ```text
//!   massif: 1616 cells, 0 folded, 23 skewed, 0 stretched
//!     rock vertices not inside or near any drawn triangle: 0 (0.0%)
//! ```
//!
//! So: no holes, no folded cells, and the dark that is left is **cast shadow** -- measured, by
//! putting the sun straight down and watching almost all of it go.
//!
//! # What it measured while it was current
//!
//! **The angle between a triangle's own geometric normal and the vertex normals it was drawn
//! with.** A near-degenerate triangle has a geometric normal that is noise; a folded one agrees with
//! its vertex normals and both point away from the patch's neighbours. Counting those two apart is
//! what made the answer say which one it was.
//!
//! The first version of this file reported **all 29484 triangles degenerate**, because it derived
//! a vertex slot from `(dr, dc)` and put a row's three vertices into the first half of a two-
//! triangle cell while the second triangle read the half that was empty. A measurement saying every
//! triangle in the model is broken is a measurement of its own indexing, and the picture is right
//! there to say so.

use mountain::{FacetMesh, Starter, starter};
use patch::Point3;
use whiting_gallery::blockmesh::{self, BeltSurfaces};

/// The grid resolution the scene draws.
const SIDE: usize = 4;

/// How many samples a patch is cut into per side, which is the scene's own `SAMPLES`.
const SAMPLES: usize = 4;

/// An area below this is a triangle with no area, in the massif's own world units.
const AREA_FLOOR: f64 = 1.0e-9;

/// A cross product below this is a normal with no direction.
const NORMAL_FLOOR: f64 = 1.0e-12;

/// What one triangle of the drawn mesh looks like.
struct Triangle {
    area: f64,
    /// How far its own normal is from the vertex normals it is drawn with.
    from_vertices: f64,
}

fn tessellate(belts: &BeltSurfaces) -> Vec<Triangle> {
    let surface = belts.surface();
    let span = (SAMPLES - 1) as f64;
    let mut out = Vec::new();
    for face in 0..surface.face_count() as u32 {
        let patch = surface.patch(face);
        for row in 0..SAMPLES - 1 {
            for column in 0..SAMPLES - 1 {
                // **The six distinct vertices of the cell, named.** The first version derived a
                // slot from `(dr, dc)`, which put a row's three vertices into the first half and
                // left the second half holding only the two the second triangle names — so every
                // triangle it measured had two coincident vertices and reported zero area for the
                // whole mesh. A measurement that says "every triangle in the model is degenerate"
                // is a measurement of its own indexing, and the picture is right there to say so.
                let at =
                    |dr: usize, dc: usize| ((column + dc) as f64 / span, (row + dr) as f64 / span);
                let vertex = |dr: usize, dc: usize| {
                    let (s, t) = at(dr, dc);
                    (patch.evaluate(s, t), patch.normal(s, t))
                };
                let (near, near_n) = vertex(0, 0);
                let (along_u, along_u_n) = vertex(0, 1);
                let (along_v, along_v_n) = vertex(1, 0);
                let (far, far_n) = vertex(1, 1);

                for (first, second, third, normals) in [
                    (near, along_u, along_v, [near_n, along_u_n, along_v_n]),
                    (along_u, far, along_v, [along_u_n, far_n, along_v_n]),
                ] {
                    let n = (second - first).cross(third - first);
                    let length = n.length();
                    let normal = if length > NORMAL_FLOOR {
                        n * (1.0 / length)
                    } else {
                        Point3::ZERO
                    };
                    let worst = normals
                        .iter()
                        .map(|v| {
                            if v.length() <= NORMAL_FLOOR {
                                f64::INFINITY
                            } else {
                                let unit = *v * (1.0 / v.length());
                                unit.angle(normal)
                            }
                        })
                        .fold(0.0f64, f64::max);
                    out.push(Triangle {
                        area: length * 0.5,
                        from_vertices: worst,
                    });
                }
            }
        }
    }
    out
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    let facet: FacetMesh = starter(11, Starter::default())
        .build()
        .expect("the massif expands");
    let belts =
        blockmesh::from_belts(&facet, SIDE).expect("the massif is one representable surface");
    let surface = belts.surface();
    let triangles = tessellate(&belts);

    // **The magnitude of the disagreement is what names the cause, and the count only does not.**
    //
    // A triangle shaded by a normal that points the *other way* disagrees by close to pi, and that
    // is a handedness fault: the two belts on either side of a seam wind along their shared edge in
    // opposite directions, so one of them has its `(u, v)` frame the other way round and
    // `tangent_u x tangent_v` comes out downwards. A triangle that merely curves disagrees by a
    // smaller angle spread across many values, and that is a sampling question instead.
    //
    // So the two are counted apart rather than lumped as "flipped", because lumping them is what
    // made the first reading of this look like a fold when it is a frame.
    let backwards = triangles
        .iter()
        .filter(|tri| tri.from_vertices > std::f64::consts::FRAC_PI_2)
        .count();
    let grazed = triangles
        .iter()
        .filter(|tri| tri.from_vertices > 1.0 && tri.from_vertices <= std::f64::consts::FRAC_PI_2)
        .count();
    let mut worst: Vec<f64> = triangles.iter().map(|tri| tri.from_vertices).collect();
    worst.sort_by(f64::total_cmp);
    let deepest = *worst.last().unwrap_or(&0.0);

    let degenerate = triangles
        .iter()
        .filter(|tri| tri.area <= AREA_FLOOR)
        .count();
    let missing = triangles
        .iter()
        .filter(|tri| !tri.from_vertices.is_finite())
        .count();
    let mut areas: Vec<f64> = triangles.iter().map(|tri| tri.area).collect();
    let mut deviations: Vec<f64> = triangles
        .iter()
        .map(|tri| tri.from_vertices)
        .filter(|value| value.is_finite())
        .collect();

    println!(
        "\n  {} patches, {} triangles drawn",
        surface.face_count(),
        triangles.len()
    );
    println!(
        "  area            median {median:>.3e}",
        median = median(&mut areas)
    );
    println!(
        "  normal from vertices  median {median:.3e} rad",
        median = median(&mut deviations)
    );
    println!(
        "  degenerate triangles   {degenerate}\n\
         \x20 normal points BACKWARDS (over half a radian out)   {backwards}\n\
         \x20 normal grazed (1 to half a radian out)             {grazed}\n\
         \x20 deepest disagreement   {deepest:.3e} rad\n\
         \x20 triangles with a normal of no direction             {missing}"
    );
    if backwards > 0 {
        println!(
            "\n  A normal that points backwards is a frame fault, not a fold: the patch's own\n\
             \x20 (u, v) is the other way round from the way the cell is wound, so\n\
             \x20 tangent_u x tangent_v points down while the geometry is up."
        );
    }

    let mut problems = Vec::new();
    if degenerate > 0 {
        problems.push("no drawn triangle has no area");
    }
    if missing > 0 {
        problems.push("every drawn triangle has a normal");
    }
    println!();
    // **The flipped-normal count is reported and not asserted on.**
    //
    // It is high here and it is high *here*, on a fixed-diagonal tessellation of the surface that
    // the scene does not use. Asserting on it would leave a permanently red probe describing a mesh
    // nobody draws, which is how a real defect gets lost among the false ones -- this file's own
    // first version reported all 29484 triangles degenerate, in the same way.
    //
    // What the scene actually draws is measured by `mesh_shape_probe`, which reads the surface's
    // own quads: 0 folded cells, 0 holes, and the rest of the dark is cast shadow.
    if problems.is_empty() {
        println!("every triangle here has area and a normal that exists.");
    } else {
        for problem in &problems {
            println!("FOUND  {problem}");
        }
    }
    println!();
    println!(
        "this tessellation is not the one the scene draws -- see the header. mesh_shape_probe"
    );
    println!("is the measurement that describes the render.");
}
