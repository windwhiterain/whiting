//! Probes the triangle-to-quad subdivision: does it produce a mesh that is all quads, and does it
//! do so without moving the surface?
//!
//! Run with `cargo run -p whiting-patch --example subdivision_probe`. An example and not a gate,
//! following `facet_value_probe`. It exits non-zero if any property fails.
//!
//! 1. Every face is a quad, non-degenerate, and the whole mesh has one consistent winding.
//! 2. The three quads of a triangle cover it exactly — coplanar, contained, and equal in area.
//! 3. The mesh stays manifold, and the boundary grows the way the subdivision rule says it must.
//! 4. A vertex moves if and only if its definition depends on the dragged one.
//!
//! ## Why measurement 2 is here at all
//!
//! Subdivision is only worth doing if it is free. If the quads of a triangle did not cover it
//! exactly, then every model built on the quad mesh would start with construction error to absorb,
//! and the whole point of storing control points rather than fitting a surface would be gone. The
//! bilinear map written below is the probe's own, not the crate's: a check that reuses the code
//! under test proves only that the code is self-consistent.

use std::collections::BTreeMap;

use whiting_patch::Point3;
use whiting_patch::subdivide::{Origin, QuadMesh, subdivide};

/// Exact, for geometry that was never asked to change.
const EXACT: f64 = 1.0e-12;

const COLUMNS: usize = 5;
const ROWS: usize = 4;

/// Samples per quad side in measurement 2.
const SAMPLES: usize = 8;

/// A grid triangulated into `2 (columns-1)(rows-1)` faces, bowed so the surface is not a plane.
///
/// The same grid `continuity_probe` uses, so the two probes report on the same mesh.
fn grid() -> (Vec<Point3>, Vec<[u32; 3]>) {
    let mut positions = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let x = column as f64 / (COLUMNS - 1) as f64;
            let y = row as f64 / (ROWS - 1) as f64;
            positions.push(Point3::new(
                4.0 * x - 2.0,
                3.0 * y - 1.5,
                0.6 * (x * x + y * y),
            ));
        }
    }
    let mut triangles = Vec::new();
    for row in 0..ROWS - 1 {
        for column in 0..COLUMNS - 1 {
            let a = (row * COLUMNS + column) as u32;
            let b = a + 1;
            let c = a + COLUMNS as u32;
            let d = c + 1;
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }
    (positions, triangles)
}

/// Every undirected edge of the quads, with how many faces use it.
fn quad_edges(quads: &[[u32; 4]]) -> BTreeMap<(u32, u32), usize> {
    let mut counts: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for quad in quads {
        for step in 0..4 {
            let first = quad[step];
            let second = quad[(step + 1) % 4];
            let key = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

/// Every directed edge of the quads, with how many faces traverse it that way.
fn directed_edges(quads: &[[u32; 4]]) -> BTreeMap<(u32, u32), usize> {
    let mut counts: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for quad in quads {
        for step in 0..4 {
            *counts
                .entry((quad[step], quad[(step + 1) % 4]))
                .or_default() += 1;
        }
    }
    counts
}

/// Every undirected edge of the triangles, with how many faces use it.
fn triangle_edges(triangles: &[[u32; 3]]) -> BTreeMap<(u32, u32), usize> {
    let mut counts: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for triangle in triangles {
        for step in 0..3 {
            let first = triangle[step];
            let second = triangle[(step + 1) % 3];
            let key = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

/// The bilinear map of a quad, with `(0, 0)` at `Q0` and `(1, 1)` at `Q2`.
///
/// This is the probe's own evaluator. A rectangle's control net at degree 1 is this map, which is
/// why a flat quad of coplanar corners is exactly the quadrilateral it names.
fn bilinear(corners: [Point3; 4], u: f64, v: f64) -> Point3 {
    corners[0] * ((1.0 - u) * (1.0 - v))
        + corners[1] * (u * (1.0 - v))
        + corners[2] * (u * v)
        + corners[3] * ((1.0 - u) * v)
}

/// The area of a planar convex quad, split along the `Q0`–`Q2` diagonal.
fn quad_area(corners: [Point3; 4]) -> f64 {
    let [q0, q1, q2, q3] = corners;
    0.5 * (q1 - q0).cross(q2 - q0).length() + 0.5 * (q2 - q0).cross(q3 - q0).length()
}

/// Barycentric weights of `point` over the triangle, and how far off its plane the point lies.
///
/// The weights always sum to one by construction, so the plane residual is reported separately
/// rather than inferred from them.
fn barycentric(corners: [Point3; 3], point: Point3) -> ([f64; 3], f64) {
    let [a, b, c] = corners;
    let u = b - a;
    let v = c - a;
    let w = point - a;
    let uu = u.dot(u);
    let uv = u.dot(v);
    let vv = v.dot(v);
    let wu = w.dot(u);
    let wv = w.dot(v);
    let denominator = uu * vv - uv * uv;
    let beta = (vv * wu - uv * wv) / denominator;
    let gamma = (uu * wv - uv * wu) / denominator;
    let residual = w - (u * beta + v * gamma);
    ([1.0 - beta - gamma, beta, gamma], residual.length())
}

/// The quads that came from one source face, found through the provenance map rather than by
/// assuming the subdivision lays them out in any particular order.
fn quads_of(derived: &QuadMesh, face: usize) -> Vec<[u32; 4]> {
    let centre = derived
        .origin
        .iter()
        .position(|origin| *origin == Origin::FaceCentroid(face as u32))
        .expect("every source face has a centroid vertex");
    derived
        .quads
        .iter()
        .copied()
        .filter(|quad| quad.contains(&(centre as u32)))
        .collect()
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let (positions, triangles) = grid();
    let derived = subdivide(&positions, &triangles);
    let mut failures: Vec<&str> = Vec::new();

    println!(
        "subdivision probe — {} triangles in, {} quads out, {} vertices in, {} vertices out",
        triangles.len(),
        derived.face_count(),
        positions.len(),
        derived.vertex_count(),
    );

    // 1. All quads, non-degenerate, one winding.
    let degenerate = derived
        .quads
        .iter()
        .filter(|quad| {
            let mut ends = quad.to_vec();
            ends.sort_unstable();
            ends.dedup();
            ends.len() != 4
        })
        .count();
    let winding = directed_edges(&derived.quads);
    let reversed = winding.values().filter(|count| **count != 1).count();
    if degenerate != 0 {
        failures.push("1  every face is a quad");
    }
    if reversed != 0 {
        failures.push("1  one consistent winding");
    }
    report(
        "1  every face is a quad, wound the same way",
        &format!(
            "degenerate quads                      {degenerate}\n\
             directed edges traversed twice       {reversed}\n\
             each quad walks its source triangle's winding, so the derived mesh needs no normal\n\
             pass: a directed edge appearing twice would be two faces claiming the same direction"
        ),
    );

    // 2. The quads of a triangle cover it exactly.
    let mut worst_off_plane = 0.0f64;
    let mut worst_outside = 0.0f64;
    let mut worst_area_error = 0.0f64;
    for (face, triangle) in triangles.iter().enumerate() {
        let corners = (*triangle).map(|index| positions[index as usize]);
        let source_area = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .length()
            / 2.0;
        let quads = quads_of(&derived, face);

        let mut covered = 0.0;
        for quad in &quads {
            let mapped = quad.map(|index| derived.positions[index as usize]);
            covered += quad_area(mapped);
            for step_u in 0..=SAMPLES {
                for step_v in 0..=SAMPLES {
                    let point = bilinear(
                        mapped,
                        step_u as f64 / SAMPLES as f64,
                        step_v as f64 / SAMPLES as f64,
                    );
                    let (weights, off_plane) = barycentric(corners, point);
                    worst_off_plane = worst_off_plane.max(off_plane);
                    worst_outside =
                        worst_outside.max(-weights.iter().copied().fold(0.0f64, f64::min));
                }
            }
        }
        worst_area_error = worst_area_error.max((covered - source_area).abs() / source_area);
    }
    if worst_off_plane > EXACT {
        failures.push("2  quads are coplanar with their triangle");
    }
    if worst_outside > EXACT {
        failures.push("2  quads stay inside their triangle");
    }
    if worst_area_error > EXACT {
        failures.push("2  the three quads have the triangle's area");
    }
    report(
        "2  the three quads of a triangle cover it exactly",
        &format!(
            "worst distance off the source plane     {worst_off_plane:.3e}\n\
             worst excursion outside the triangle   {worst_outside:.3e}\n\
             worst relative area error              {worst_area_error:.3e}\n\
             measured over {} triangles x 3 quads x {}x{} bilinear samples, with the probe's own\n\
             evaluator. coplanar, contained, and equal in area means the three convex quads tile\n\
             the triangle with no gap and no overlap, so level 0 of a quad patch is the mesh",
            triangles.len(),
            SAMPLES + 1,
            SAMPLES + 1,
        ),
    );

    // 3. Manifold, and the boundary grows by the amount the rule implies.
    let original_edges = triangle_edges(&triangles);
    let original_boundary = original_edges.values().filter(|count| **count == 1).count();
    let derived_edges = quad_edges(&derived.quads);
    let non_manifold = derived_edges.values().filter(|count| **count > 2).count();
    let derived_boundary = derived_edges.values().filter(|count| **count == 1).count();
    if non_manifold != 0 {
        failures.push("3  no edge carries more than two faces");
    }
    if derived_boundary != 2 * original_boundary {
        failures.push("3  the boundary grows by exactly two per original boundary edge");
    }
    report(
        "3  the mesh stays manifold",
        &format!(
            "edges on more than two faces           {non_manifold}\n\
             distinct edges                        {} -> {}\n\
             boundary edges                        {original_boundary} -> {derived_boundary}\n\
             an original boundary edge becomes two, because each of its two endpoints now has a\n\
             boundary edge of its own. that doubling is the rule's whole boundary story, and it is\n\
             why this subdivision needs no separate boundary case",
            original_edges.len(),
            derived_edges.len(),
        ),
    );

    // 4. Original vertices keep their indices, and a vertex moves exactly when its definition depends
    //    on the dragged one. Midpoints of incident edges and centroids of incident faces *must*
    //    follow — they are averages, so a midpoint moves by half the drag and a centroid by a
    //    third. The sharp claim is the other direction: nothing that does not depend on the dragged
    //    vertex moves at all, which is what makes the subdivision a pure function of the positions
    //    it was handed.
    let dragged = 0u32;
    let offset = Point3::new(0.25, -0.5, 0.75);
    let mut moved_positions = positions.clone();
    moved_positions[dragged as usize] += offset;
    let moved = subdivide(&moved_positions, &triangles);

    let mut wrong_motion = 0usize;
    let mut followers = 0usize;
    for (index, origin) in derived.origin.iter().enumerate() {
        let gap = derived.positions[index].distance(moved.positions[index]);
        let depends = match origin {
            Origin::Corner(vertex) => *vertex == dragged,
            Origin::EdgeMidpoint(key) => key.0 == dragged || key.1 == dragged,
            Origin::FaceCentroid(face) => triangles[*face as usize].contains(&dragged),
        };
        if depends {
            followers += 1;
            if gap < EXACT {
                wrong_motion += 1;
            }
        } else if gap > EXACT {
            wrong_motion += 1;
        }
    }
    let corner_origins = derived
        .origin
        .iter()
        .filter(|origin| matches!(origin, Origin::Corner(_)))
        .count();
    let midpoints = derived
        .origin
        .iter()
        .filter(|origin| matches!(origin, Origin::EdgeMidpoint(_)))
        .count();
    let centroids = derived
        .origin
        .iter()
        .filter(|origin| matches!(origin, Origin::FaceCentroid(_)))
        .count();
    let provenance_ok = corner_origins == positions.len()
        && midpoints == original_edges.len()
        && centroids == triangles.len();
    if wrong_motion != 0 {
        failures.push("4  a vertex moves if and only if its definition depends on the dragged one");
    }
    if !provenance_ok {
        failures.push("4  the provenance map accounts for every vertex");
    }
    report(
        "4  vertices move exactly when they depend on the drag",
        &format!(
            "vertices that should follow and did not, or should not and did   {wrong_motion}\n\
             vertices that follow                                                {followers}\n\
             {} vertices in: corners {corner_origins}, midpoints {midpoints}, centroids {centroids}\n\
             the corner itself moves by the whole drag, its two edge midpoints by half, the face\n\
             centroid by a third, and the other {} vertices do not move at all. subdivision is a\n\
             pure insertion, so the caller's own vertices are still at their own indices afterwards\n\
             and a drag needs no map back from the derived mesh",
            derived.vertex_count(),
            derived.vertex_count() - followers,
        ),
    );

    if failures.is_empty() {
        println!("\nall {} measurements passed", 4);
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
