//! Filling the region between two plan loops: **the ring a void makes**.
//!
//! # Why this is its own module
//!
//! `clip` answers *how much* two plans share; this answers *which triangles* fill the space between an outline and a
//! hole in it. They are different questions with different failure modes, and the ring's failure mode is nasty: a
//! fill can have the right number of triangles and still overlap itself, which measured as 140 m² where the annulus
//! was 84. Keeping the two apart makes the ring's rule readable on its own.
//!
//! # The rule
//!
//! Two loops, one strictly inside the other, both **convex**. A two-pointer walk starts at the closest pair of
//! vertices and closes one ear a step. The ear it takes is the one whose triangle **holds no other vertex of either
//! ring** — the ear definition itself, not "the smaller triangle", which is a heuristic for monotone pieces and
//! produced the overlapping fill above.
//!
//! # Invariants
//!
//! - **Both loops convex**, or no triangles at all. A bridged non-convex ring is not a simple polygon, and ear
//!   clipping one leaks.
//! - **The winding is normalised here**: the inner loop is reversed when it runs opposite the outer, because the walk
//!   advances both in the same rotational direction.
//! - **The hole's edge is preserved exactly** — every edge of the inner loop lands in exactly one triangle, which is
//!   what lets a wall stand on it.
//! - **The acceptance is arithmetic**: the triangles' areas sum to the outer area minus the inner area.

use crate::math::Vec2;

/// Returns the triangles that fill the region between an outer loop and a loop strictly inside it.
pub fn ring_fill(outer: &[Vec2], inner: &[Vec2]) -> Vec<[Vec2; 3]> {
    if outer.len() < 3 || inner.len() < 3 {
        return Vec::new();
    }
    if !convex(outer) || !convex(inner) {
        return Vec::new();
    }
    let mut ring = inner.to_vec();
    if signed_area(outer) * signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    let (count, holes) = (outer.len(), ring.len());
    let centre = [
        ring.iter().map(|corner| corner[0]).sum::<f32>() / holes as f32,
        ring.iter().map(|corner| corner[1]).sum::<f32>() / holes as f32,
    ];
    let mut every: Vec<Vec2> = outer.to_vec();
    every.extend_from_slice(&ring);
    let (mut a, mut b) = closest_pair(outer, &ring);
    let mut triangles = Vec::with_capacity(count + holes);
    let (mut walked, mut walked_holes) = (0usize, 0usize);
    while walked < count && walked_holes < holes {
        let next_a = (a + 1) % count;
        let next_b = (b + 1) % holes;
        let outer_ear = [outer[a], outer[next_a], ring[b]];
        let hole_ear = [ring[b], ring[next_b], outer[a]];
        let take_outer = match (
            !unusable(outer_ear, &every, [outer, &ring], centre),
            !unusable(hole_ear, &every, [outer, &ring], centre),
        ) {
            (true, false) => true,
            (false, true) => false,
            // Both clean happens when the loops are nearly aligned, and the smaller ear is then the safe
            // choice. Both holding a vertex cannot happen for two convex loops.
            _ => area(outer_ear) <= area(hole_ear),
        };
        if take_outer {
            triangles.push(outer_ear);
            a = next_a;
            walked += 1;
        } else {
            triangles.push(hole_ear);
            b = next_b;
            walked_holes += 1;
        }
    }
    // One loop ran out: the rest is a fan around the vertex the other stopped at.
    while walked < count {
        let next_a = (a + 1) % count;
        triangles.push([ring[b], outer[a], outer[next_a]]);
        a = next_a;
        walked += 1;
    }
    while walked_holes < holes {
        let next_b = (b + 1) % holes;
        triangles.push([outer[a], ring[b], ring[next_b]]);
        b = next_b;
        walked_holes += 1;
    }
    triangles
}

/// Returns whether a candidate ear is unusable: it holds a ring vertex, or one of its edges crosses a ring's edge.
///
/// # Invariants
///
/// **Both halves are needed.** The vertex test alone lets a triangle whose corners all sit on the hole **cover the
/// hole's interior** — its three edges then cross the hole's boundary, which is the leak that measured 168 m² against
/// an 84 m² annulus. The crossing test alone lets a triangle swallow a vertex without touching an edge, which is the
/// other leak.
fn unusable(triangle: [Vec2; 3], every: &[Vec2], loops: [&[Vec2]; 2], centre: Vec2) -> bool {
    // **A triangle that swallows the hole's middle is leaking**, however it entered: a segment through the hole's
    // own corners crosses nothing *properly*, and that is exactly the symmetric case — a centred void whose
    // diagonals line up with the walk's chords.
    if triangle_contains(centre, triangle) {
        return true;
    }
    if every
        .iter()
        .any(|point| !triangle.contains(point) && strictly_inside_triangle(*point, triangle))
    {
        return true;
    }
    (0..3).any(|index| {
        let from = triangle[index];
        let to = triangle[(index + 1) % 3];
        loops.iter().any(|polygon| {
            (0..polygon.len()).any(|at| {
                let a = polygon[at];
                let b = polygon[(at + 1) % polygon.len()];
                properly_crosses(from, to, a, b)
            })
        })
    })
}

/// Returns whether a point is inside a triangle or on its boundary.
fn triangle_contains(point: Vec2, triangle: [Vec2; 3]) -> bool {
    let one = cross(
        subtract2(triangle[1], triangle[0]),
        subtract2(point, triangle[0]),
    );
    let two = cross(
        subtract2(triangle[2], triangle[1]),
        subtract2(point, triangle[1]),
    );
    let three = cross(
        subtract2(triangle[0], triangle[2]),
        subtract2(point, triangle[2]),
    );
    (one >= 0.0 && two >= 0.0 && three >= 0.0) || (one <= 0.0 && two <= 0.0 && three <= 0.0)
}

/// Returns whether two segments properly cross: each has the other's endpoints strictly on opposite sides.
fn properly_crosses(from_a: Vec2, to_a: Vec2, from_b: Vec2, to_b: Vec2) -> bool {
    let along = subtract2(to_a, from_a);
    let first = cross(along, subtract2(from_b, from_a));
    let second = cross(along, subtract2(to_b, from_a));
    let other = subtract2(to_b, from_b);
    let third = cross(other, subtract2(from_a, from_b));
    let fourth = cross(other, subtract2(to_a, from_b));
    let straddles = (first > 0.0 && second < 0.0) || (first < 0.0 && second > 0.0);
    let straddles_other = (third > 0.0 && fourth < 0.0) || (third < 0.0 && fourth > 0.0);
    straddles && straddles_other
}

/// Returns whether a point is inside a triangle and not on its boundary.
///
/// **Strictness is load-bearing.** The boundary-inclusive test would veto every ear whose leg runs along another
/// loop's edge, and those are most of the ears a two-convex-loop walk closes.
fn strictly_inside_triangle(point: Vec2, triangle: [Vec2; 3]) -> bool {
    let one = cross(
        subtract2(triangle[1], triangle[0]),
        subtract2(point, triangle[0]),
    );
    let two = cross(
        subtract2(triangle[2], triangle[1]),
        subtract2(point, triangle[1]),
    );
    let three = cross(
        subtract2(triangle[0], triangle[2]),
        subtract2(point, triangle[2]),
    );
    (one > 1.0e-9 && two > 1.0e-9 && three > 1.0e-9)
        || (one < -1.0e-9 && two < -1.0e-9 && three < -1.0e-9)
}

/// Returns a triangle's unsigned area.
fn area(triangle: [Vec2; 3]) -> f32 {
    cross(
        subtract2(triangle[1], triangle[0]),
        subtract2(triangle[2], triangle[0]),
    )
    .abs()
        * 0.5
}

/// Returns whether a loop turns the same way at every corner.
fn convex(polygon: &[Vec2]) -> bool {
    let mut sign = 0.0f32;
    for index in 0..polygon.len() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let corner = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let turn = cross(subtract2(corner, previous), subtract2(next, corner));
        if turn.abs() <= 1.0e-9 {
            continue;
        }
        if sign == 0.0 {
            sign = turn;
        } else if turn * sign < 0.0 {
            return false;
        }
    }
    true
}

/// Returns a polygon's signed area, positive when wound counter-clockwise.
fn signed_area(polygon: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..polygon.len() {
        let here = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        twice += here[0] * next[1] - next[0] * here[1];
    }
    twice * 0.5
}

/// Returns the closest pair of vertices, one from each loop.
fn closest_pair(outer: &[Vec2], inner: &[Vec2]) -> (usize, usize) {
    let mut best = f32::INFINITY;
    let mut found = (0, 0);
    for (one, first) in outer.iter().enumerate() {
        for (other, second) in inner.iter().enumerate() {
            let gap = subtract2(*first, *second);
            let distance = gap[0] * gap[0] + gap[1] * gap[1];
            if distance < best {
                best = distance;
                found = (one, other);
            }
        }
    }
    found
}

/// Returns the plan difference of two points.
fn subtract2(first: Vec2, second: Vec2) -> Vec2 {
    [first[0] - second[0], first[1] - second[1]]
}

/// Returns the two-dimensional cross product, which is the sign of a corner's turn.
fn cross(first: Vec2, second: Vec2) -> f32 {
    first[0] * second[1] - first[1] * second[0]
}
