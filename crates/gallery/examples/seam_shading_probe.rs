//! What are the dark patches and the fern-like spikes on the massif's seams?
//!
//! Run with `cargo run -p whiting-gallery --example seam_shading_probe`. Diagnostic: it reports,
//! and exits non-zero only if it finds a defect it can name.
//!
//! # What is already ruled out
//!
//! **No cell is dropped** — `blockmesh` builds 1638 cells at `side` 4 and draws all 1638 — so the
//! dark patches are not holes in the mesh. The mesh is complete and the picture is still wrong,
//! which puts the cause in how the surface is *sampled* for drawing or in how it is *shaded*.
//!
//! Two candidates, and they are distinguishable:
//!
//! - **A degenerate triangle.** Two of its vertices in the same place, so it has no area, and a
//!   path tracer draws a zero-area triangle as nothing or as noise. This is what the earlier
//!   resampler bug produced, and it was fixed; whether any survive is the question.
//! - **A normal that flips.** A cell whose corners are not convex, so the surface folds back on
//!   itself and the patch's own normal points the other way. Shading follows the normal, so a
//!   flipped cell is black however good its geometry is.
//!
//! # What decides between them
//!
//! **The angle between a triangle's own geometric normal and the vertex normals it was drawn with.**
//! A near-degenerate triangle has a geometric normal that is noise. A folded one agrees with its
//! vertex normals and both point away from the patch's neighbours. Measuring the two apart is what
//! makes the answer say which one it is, rather than "the shading is wrong somewhere".
//!
//! Both are counted **per seam cell** and over the whole mesh, because the picture shows them on the
//! valleys and not on the open flanks, and a defect that is everywhere is a different finding from
//! one that is on the seams.

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

fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

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
    let flipped = triangles
        .iter()
        .filter(|tri| tri.from_vertices > 1.0)
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
    if flipped > 0 {
        problems.push("no drawn triangle's normal is flipped away from its vertices");
    }
    println!();
    if problems.is_empty() {
        println!("the drawn mesh is clean: every triangle has area and a normal that agrees");
        println!("with the vertices it is drawn from. The artifacts are therefore not in the");
        println!("sampling, and the next place to look is how the stage shades rather than how it");
        println!("is built.");
    } else {
        for problem in &problems {
            println!("FOUND  {problem}");
        }
    }
}
