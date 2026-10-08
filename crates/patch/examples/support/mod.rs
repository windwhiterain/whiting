//! Pieces the probes share: one sample mesh, one planar reference map, one set of tolerances.
//!
//! A probe that reuses the crate's own evaluator to check the crate proves only that the code is
//! self-consistent. The evaluator in here is written out separately for exactly that reason: the
//! bilinear map below is the *reference*, and the crate's `RectNet` is what gets compared against
//! it. What may be shared is the sample — one mesh, so two probes' numbers are about the same
//! thing.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt;

use whiting_patch::topology::{EdgeKey, boundary_index, edge_endpoints, mirrored_position};
use whiting_patch::{Continuity, Level, PatchMesh, Point3};

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

/// The largest distance any control point moved between two states of a level.
///
/// Compares whole buffers, so it is the right question whenever the claim is about a level having
/// changed or two routes to a level agreeing — not about one patch. Mismatched lengths are a probe
/// bug and stop the run: zipping them would report a smaller maximum than either level contains,
/// which reads exactly like a null result.
pub fn level_distance(first: &Level, second: &Level) -> f64 {
    assert_eq!(
        first.point_count(),
        second.point_count(),
        "two levels compared by distance must hold the same number of points"
    );
    first
        .points()
        .iter()
        .zip(second.points())
        .map(|(was, now)| was.distance(*now))
        .fold(0.0f64, f64::max)
}

/// The largest distance any component of any control point in `level` is from the origin.
pub fn level_magnitude(level: &Level) -> f64 {
    level
        .points()
        .iter()
        .map(|point| point.length())
        .fold(0.0f64, f64::max)
}

/// Overwrites every point with a seeded ramp, which violates every tag in the level at once:
/// neighbours disagree, so the shared boundaries are not shared and no soft edge's slave row is
/// derived.
///
/// Deliberately **not** a rigid translation. A uniform offset already satisfies both invariants —
/// sharing compares two equally shifted points, and `C + mu (C - A)` is unchanged when `C` and `A`
/// move together — so a translated level would sail through enforcement untouched and every
/// measurement that used it would pass without testing anything.
pub fn scramble(level: &mut Level, seed: f64) {
    let mut next = seed;
    for face in 0..level.len() {
        let (degree_u, degree_v) = level.layouts()[face].degrees();
        for i in 0..=degree_u {
            for j in 0..=degree_v {
                next += 0.004;
                level.set_point(
                    face,
                    i,
                    j,
                    Point3::new(next, -next * 0.5, seed * 0.5 + next * 0.25),
                );
            }
        }
    }
}

/// How much two faces of a shared hard edge disagree, in one line for a report.
pub struct Split {
    pub worst: f64,
    pub positions: usize,
}

impl fmt::Display for Split {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:.3e} over {} positions",
            self.worst, self.positions
        )
    }
}

/// The worst disagreement between the two faces of a shared hard edge, and how many positions
/// disagree at all.
///
/// The count matters as much as the worst: one bad edge and one bad edge per vertex are different
/// faults, and only the count tells them apart. Measured at the control points rather than at the
/// surface, because a surface can look right while the two sides disagree about where the edge is.
pub fn split_over_shared_edges(mesh: &PatchMesh, level: &Level) -> Split {
    let degree = level.degrees().0;
    let rules = mesh.rules();
    let mut worst = 0.0f64;
    let mut positions = 0usize;
    for (key, faces) in &mesh.topology().edge_faces {
        if faces.len() != 2 || rules.continuity(key).is_some_and(Continuity::is_soft) {
            continue;
        }
        let (owner, follower) = (faces[0], faces[1]);
        let owner_ends = edge_endpoints(mesh.quads()[owner.0 as usize], owner.1);
        let follower_ends = edge_endpoints(mesh.quads()[follower.0 as usize], follower.1);
        for t in 0..=degree {
            let follower_t = mirrored_position(degree, owner_ends, follower_ends, t);
            let (oi, oj) = boundary_index(degree, owner.1, t);
            let (fi, fj) = boundary_index(degree, follower.1, follower_t);
            let held = level.face(owner.0 as usize).expect("a face resolves")[(oi, oj)];
            let other = level.face(follower.0 as usize).expect("a face resolves")[(fi, fj)];
            let gap = held.distance(other);
            worst = worst.max(gap);
            if gap > 0.0 {
                positions += 1;
            }
        }
    }
    Split { worst, positions }
}

/// Every distinct edge of a triangle mesh, and how many triangles share it.
///
/// The counts are the whole answer to "is this mesh something a per-edge tag can be attached to": an
/// edge with three triangles on it is not a crease, it is a fold, and nothing downstream can tag it.
pub fn triangle_edges(triangles: &[[u32; 3]]) -> BTreeMap<EdgeKey, usize> {
    let mut counts: BTreeMap<EdgeKey, usize> = BTreeMap::new();
    for triangle in triangles {
        for ends in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            *counts.entry(canonical(ends)).or_insert(0) += 1;
        }
    }
    counts
}

/// Every distinct edge of a quad mesh, and how many quads share it.
pub fn quad_edges(quads: &[[u32; 4]]) -> BTreeMap<EdgeKey, usize> {
    let mut counts: BTreeMap<EdgeKey, usize> = BTreeMap::new();
    for quad in quads {
        for slot in 0..4 {
            *counts
                .entry(canonical([quad[slot], quad[(slot + 1) % 4]]))
                .or_insert(0) += 1;
        }
    }
    counts
}

/// The unordered pair in ascending order, which is what makes two faces name the same edge.
fn canonical(ends: [u32; 2]) -> EdgeKey {
    if ends[0] <= ends[1] {
        (ends[0], ends[1])
    } else {
        (ends[1], ends[0])
    }
}

/// How a mesh's edges divide up: the boundary, the interior, and everything a per-edge tag cannot
/// describe.
#[derive(Clone, Copy, Debug, Default)]
pub struct Incidence {
    pub boundary: usize,
    pub interior: usize,
    /// Edges shared by three or more faces. `enforce` shares an edge when exactly two faces meet on
    /// it, so these keep their own copies and crack silently.
    pub non_manifold: usize,
    pub worst: usize,
}

impl Incidence {
    pub fn of(counts: &BTreeMap<EdgeKey, usize>) -> Self {
        let mut found = Incidence::default();
        for shared in counts.values() {
            found.worst = found.worst.max(*shared);
            match shared {
                1 => found.boundary += 1,
                2 => found.interior += 1,
                _ => found.non_manifold += 1,
            }
        }
        found
    }

    pub fn total(&self) -> usize {
        self.boundary + self.interior + self.non_manifold
    }
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
