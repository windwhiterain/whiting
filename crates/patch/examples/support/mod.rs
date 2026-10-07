//! Pieces the probes share: one sample mesh, one planar reference map, one set of tolerances.
//!
//! A probe that reuses the crate's own evaluator to check the crate proves only that the code is
//! self-consistent. The evaluator in here is written out separately for exactly that reason: the
//! bilinear map below is the *reference*, and the crate's `RectNet` is what gets compared against
//! it. What may be shared is the sample — one mesh, so two probes' numbers are about the same
//! thing.

#![allow(dead_code)]

use whiting_patch::Point3;

/// Exact, for geometry that was never asked to change.
pub const EXACT: f64 = 1.0e-12;

/// A measured angle floors at about this, because two unit vectors that close have a dot product
/// `f64` cannot resolve below `1 - 1e-16` and `acos` turns that into `sqrt(2e-16)`.
pub const ANGLE_FLOOR: f64 = 1.4e-8;

pub const COLUMNS: usize = 5;
pub const ROWS: usize = 4;

/// A grid triangulated into `2 (columns-1)(rows-1)` faces, bowed so the surface is not a plane.
///
/// The same sample every probe in this crate uses, so their numbers are comparable.
pub fn grid() -> (Vec<Point3>, Vec<[u32; 3]>) {
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

/// The bilinear map of a quad, with `(0, 0)` at `q0` and `(1, 1)` at `q2`.
///
/// A rectangle's control net at degree 1 *is* this map, which is why a flat quad of coplanar corners
/// is exactly the quadrilateral it names. Written by hand rather than called, so it can serve as the
/// reference a `RectNet` is measured against.
pub fn bilinear(corners: [Point3; 4], u: f64, v: f64) -> Point3 {
    corners[0] * ((1.0 - u) * (1.0 - v))
        + corners[1] * (u * (1.0 - v))
        + corners[2] * (u * v)
        + corners[3] * ((1.0 - u) * v)
}

/// Barycentric weights of `point` over the triangle, and how far off its plane the point lies.
///
/// The weights always sum to one by construction, so the plane residual is reported separately
/// rather than inferred from them.
pub fn barycentric(corners: [Point3; 3], point: Point3) -> ([f64; 3], f64) {
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
