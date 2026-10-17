//! Plan-polygon clipping: **the area two simple polygons share**, computed rather than guessed at.
//!
//! # Why this exists
//!
//! `validate` has to say whether two masses' plans share area. It used to ask four predicates — one polygon
//! contains a vertex of the other, or its edges properly cross — and that test missed the shape two rectangles
//! sharing a wall have: **collinear** shared edges and a corner touching an edge are neither containment nor a
//! proper crossing, and a 0.15 m strip of two buildings' walls went unreported.
//!
//! An edge-midpoint predicate closed the case the crate draws, and its own docs said it was a detector rather than
//! a proof. This module is the proof: two polygons are ear-clipped into triangles, every pair of triangles is
//! clipped against each other, and the areas are summed. A flush pair shares **exactly** zero, not "no vertex
//! inside".
//!
//! # Invariants
//!
//! - **Both polygons are simple** (non-self-intersecting). `validate` already reports a reversed or too-short
//!   outline, so this is the crate's existing assumption rather than a new one.
//! - **The result is a sum of exact triangle intersections**, so it is a lower bound only for degenerate input: a
//!   polygon with collinear corners loses area-free triangles, never area.
//! - Ear clipping **stops rather than loops** when no ear can be clipped (degenerate outline), and what it has
//!   clipped is returned: a partial triangulation can only under-report an overlap.

use crate::math::Vec2;

/// Returns the area two simple plan polygons share, in plan units squared.
///
/// # Invariants
///
/// **A flush pair answers exactly zero**: two masses standing against each other share a boundary and no area, and
/// computing rather than testing is what makes that "no area" rather than "no predicate fired".
pub fn shared_area(first: &[Vec2], second: &[Vec2]) -> f32 {
    if first.len() < 3 || second.len() < 3 {
        return 0.0;
    }
    let mut total = 0.0;
    for one in &triangles(first) {
        for other in &triangles(second) {
            total += triangle_overlap(*one, *other);
        }
    }
    total.max(0.0)
}

/// Returns a simple polygon's triangles, by ear clipping.
///
/// # Invariants
///
/// The polygon is wound counter-clockwise first, so a corner is convex exactly when its cross product is
/// positive — the same sign convention every other plan test in this crate reads.
fn triangles(polygon: &[Vec2]) -> Vec<[Vec2; 3]> {
    let mut remaining: Vec<Vec2> = polygon.to_vec();
    if signed_area(&remaining) < 0.0 {
        remaining.reverse();
    }
    let mut ears = Vec::new();
    while remaining.len() > 3 {
        let mut clipped = false;
        for index in 0..remaining.len() {
            let count = remaining.len();
            let previous = remaining[(index + count - 1) % count];
            let corner = remaining[index];
            let next = remaining[(index + 1) % count];
            if cross(subtract2(corner, previous), subtract2(next, corner)) <= 0.0 {
                continue;
            }
            let holds_a_vertex = remaining.iter().enumerate().any(|(at, point)| {
                at != index
                    && at != (index + count - 1) % count
                    && at != (index + 1) % count
                    && inside_triangle(*point, previous, corner, next)
            });
            if holds_a_vertex {
                continue;
            }
            ears.push([previous, corner, next]);
            remaining.remove(index);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if remaining.len() == 3 {
        ears.push([remaining[0], remaining[1], remaining[2]]);
    }
    ears
}

/// Returns how much of two triangles is shared.
///
/// # Invariants
///
/// The first triangle is clipped by each of the second's three edges in turn, which is a convex clip because a
/// triangle is convex — so the result is the exact intersection polygon and its area is exact.
fn triangle_overlap(first: [Vec2; 3], second: [Vec2; 3]) -> f32 {
    let mut held: Vec<Vec2> = first.to_vec();
    for index in 0..3 {
        let from = second[index];
        let to = second[(index + 1) % 3];
        held = clip_to_left(held, from, to);
        if held.len() < 3 {
            return 0.0;
        }
    }
    signed_area(&held).abs()
}

/// Returns a polygon clipped to the left of the directed line `from → to`.
///
/// # Invariants
///
/// A segment that crosses the line is cut at the crossing; a segment wholly on the kept side is kept whole; a
/// segment wholly on the cut side is dropped. A point **on** the line is kept, so a shared edge survives the clip
/// and contributes zero area rather than dropping a face.
fn clip_to_left(polygon: Vec<Vec2>, from: Vec2, to: Vec2) -> Vec<Vec2> {
    if polygon.is_empty() {
        return polygon;
    }
    let direction = subtract2(to, from);
    let side = |point: Vec2| cross(direction, subtract2(point, from));
    let mut kept = Vec::new();
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let current_side = side(current);
        let next_side = side(next);
        if current_side >= 0.0 {
            kept.push(current);
        }
        if (current_side > 0.0 && next_side < 0.0) || (current_side < 0.0 && next_side > 0.0) {
            let ratio = current_side / (current_side - next_side);
            kept.push([
                current[0] + (next[0] - current[0]) * ratio,
                current[1] + (next[1] - current[1]) * ratio,
            ]);
        }
    }
    kept
}

/// Returns a polygon's signed area, positive when wound counter-clockwise.
fn signed_area(polygon: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        twice += current[0] * next[1] - next[0] * current[1];
    }
    twice * 0.5
}

/// Returns whether a point stands inside a triangle, corners included.
fn inside_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let one = cross(subtract2(b, a), subtract2(point, a));
    let two = cross(subtract2(c, b), subtract2(point, b));
    let three = cross(subtract2(a, c), subtract2(point, c));
    (one >= 0.0 && two >= 0.0 && three >= 0.0) || (one <= 0.0 && two <= 0.0 && three <= 0.0)
}

/// Returns the two-dimensional cross product, which is the sign of a corner's turn.
fn cross(first: Vec2, second: Vec2) -> f32 {
    first[0] * second[1] - first[1] * second[0]
}

/// Returns the plan difference of two points.
fn subtract2(first: Vec2, second: Vec2) -> Vec2 {
    [first[0] - second[0], first[1] - second[1]]
}
