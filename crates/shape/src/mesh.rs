//! The predicates over a triangle list, shared by every crate that builds a surface.
//!
//! # Invariants
//!
//! An **undirected edge** is an unordered pair of vertex indices, and a **directed edge** is the
//! ordered pair one triangle's winding traverses: `(c0, c1)`, `(c1, c2)`, `(c2, c0)`. Every
//! predicate here is one pass over the corners plus one table, and all but [`signed_volume`] read
//! the indices alone — so each topological answer is a property of the corner list, and
//! re-indexing a surface cannot move one.
//!
//! The set and, just as load-bearing, what it does not promise are
//! `.agents/proposals/shape-primitives.md` D5.

use std::collections::HashMap;

use crate::math::{Vec3, cross3, dot3};

/// A triangle list: the vertices, and three corner indices into them per triangle.
#[derive(Clone, Copy)]
pub struct Triangles<'a> {
    /// The vertices, which several triangles may share.
    pub positions: &'a [Vec3],
    /// The triangles, each wound in the order its three corners are given.
    pub corners: &'a [[u32; 3]],
}

/// Returns how many separate boundary components the surface has.
///
/// # Invariants
///
/// A boundary edge is an undirected edge used by exactly one triangle, two of them are connected
/// when they share a vertex, and the answer is the number of connected components of that set — so
/// a closed surface reads `0` because no edge of it is used once.
///
/// Nothing is special-cased. A boundary that has collapsed onto a vertex other triangles also
/// reach is the one component it is rather than a ring of its own, and a caller that wants a ring
/// rather than a count has to walk the edges itself.
pub fn boundary_loops(surface: Triangles<'_>) -> usize {
    survey(surface).boundary_loops
}

/// Returns whether every edge of the surface is shared by one or two triangles.
///
/// # Invariants
///
/// It says **nothing about orientation**, so a cube with one triangle wound backwards is still
/// manifold here and that fault is [`winding_is_consistent`] instead. Nor does it say anything
/// about two disjoint fans meeting at one vertex, about triangles that intersect in space, or
/// about where the positions are: a bowtie reads here exactly as a sphere does, because the two
/// differ only in the positions this predicate never reads.
///
/// One pass over the corners and one table is its whole cost.
pub fn is_manifold(surface: Triangles<'_>) -> bool {
    survey(surface).manifold
}

/// Returns `V - E + F`: referenced vertices, distinct undirected edges, and triangles.
///
/// # Invariants
///
/// `V` counts the vertex **indices at least one triangle names**, so a position no corner refers to
/// does not move the answer — which is what makes the number a property of the surface rather than
/// of the buffer it happens to sit in. A closed genus-zero shell reads `2`, that shell with one
/// triangle removed reads `1`, and a tube reads `0`.
pub fn euler_characteristic(surface: Triangles<'_>) -> i64 {
    let surveyed = survey(surface);
    surveyed.referenced_vertices as i64 - surveyed.distinct_edges as i64
        + surface.corners.len() as i64
}

/// Returns the volume the surface encloses, signed by its winding.
///
/// # Invariants
///
/// **It means nothing for an open surface.** The sum is a divergence-theorem integral over a
/// closed boundary, and a shell with a hole in it has no interior to measure.
///
/// For a **closed** surface it is positive when the triangles are wound counter-clockwise as seen
/// from outside and negative when they are wound inside out, and it is **translation invariant** —
/// which is what the closure buys. Translating every position by `t` adds the signed volume of
/// whatever the surface fails to enclose, which is nothing when it is closed and is the whole
/// reading when it is not.
///
/// A corner naming an index `positions` does not have is a caller error and panics.
pub fn signed_volume(surface: Triangles<'_>) -> f32 {
    let mut sum = 0.0f32;
    for triangle in surface.corners {
        let [first, second, third] = *triangle;
        sum += dot3(
            position(surface, first),
            cross3(position(surface, second), position(surface, third)),
        );
    }
    sum * (1.0 / 6.0)
}

/// Returns whether every shared edge is traversed the two ways by the two triangles on it.
///
/// # Invariants
///
/// It is [`same_direction_edges`] being zero, read through the same pass, so the two cannot
/// disagree. A surface can be consistent here and still not be a manifold.
pub fn winding_is_consistent(surface: Triangles<'_>) -> bool {
    same_direction_edges(surface) == 0
}

/// Returns how many undirected edges more than one triangle traverses in the **same** direction.
///
/// # Invariants
///
/// Each undirected edge is counted once however many times it is repeated, so three triangles wound
/// the same way across one edge read `1` and not `3`. A closed shell with one triangle wound
/// backwards reads `3`: the fault is orientation, and the shell is still manifold and still has no
/// boundary.
pub fn same_direction_edges(surface: Triangles<'_>) -> usize {
    survey(surface).same_direction_edges
}

/// The position a corner names, where a corner index outside `positions` is a caller error.
fn position(surface: Triangles<'_>, index: u32) -> Vec3 {
    surface.positions[index as usize]
}

/// Every count the predicates read out of one pass over the corners, so that no two of them can
/// disagree about the same edge.
struct Survey {
    referenced_vertices: usize,
    distinct_edges: usize,
    boundary_loops: usize,
    same_direction_edges: usize,
    manifold: bool,
}

/// Counts what every predicate needs in one pass over the corners, then one walk over the edge
/// table. The walk is over the table rather than over the corners because an edge's use count is
/// only final once every triangle that touches it has been seen.
fn survey(surface: Triangles<'_>) -> Survey {
    let mut vertices: HashMap<u32, VertexSlot> = HashMap::new();
    let mut edges: HashMap<[u32; 2], EdgeUse> = HashMap::new();
    let mut repeated_corner = false;

    for triangle in surface.corners {
        let [first, second, third] = *triangle;
        repeated_corner |= first == second || second == third || third == first;
        for directed in [[first, second], [second, third], [third, first]] {
            vertices
                .entry(directed[0])
                .or_insert_with(|| VertexSlot::new(directed[0]));
            let (edge, ascending) = undirected(directed[0], directed[1]);
            let use_count = edges.entry(edge).or_default();
            if ascending {
                use_count.forward += 1;
            } else {
                use_count.backward += 1;
            }
        }
    }

    let mut boundary_loops = 0usize;
    let mut same_direction_edges = 0usize;
    let mut manifold = !repeated_corner;
    for (edge, use_count) in &edges {
        let uses = use_count.forward + use_count.backward;
        if uses > 2 {
            manifold = false;
        }
        if use_count.forward > 1 || use_count.backward > 1 {
            same_direction_edges += 1;
        }
        if uses == 1 {
            // A vertex no boundary edge reaches is not a boundary component of its own, so a
            // component is opened only by the first edge that meets the vertex and closed when two
            // already-counted vertices turn out to be one set.
            if mark_boundary(&mut vertices, edge[0]) {
                boundary_loops += 1;
            }
            if mark_boundary(&mut vertices, edge[1]) {
                boundary_loops += 1;
            }
            if unite(&mut vertices, edge[0], edge[1]) {
                boundary_loops -= 1;
            }
        }
    }

    Survey {
        referenced_vertices: vertices.len(),
        distinct_edges: edges.len(),
        boundary_loops,
        same_direction_edges,
        manifold,
    }
}

/// How many triangles traverse one undirected edge, and in which of its two directions.
///
/// # Invariants
///
/// Every directed edge adds exactly one to exactly one of the two counts, so their sum is the
/// number of triangles on the edge whatever the winding is.
#[derive(Default)]
struct EdgeUse {
    forward: u32,
    backward: u32,
}

/// One vertex of the surface's union-find, which the boundary components are counted through.
struct VertexSlot {
    parent: u32,
    size: u32,
    touches_boundary: bool,
}

impl VertexSlot {
    fn new(vertex: u32) -> Self {
        VertexSlot {
            parent: vertex,
            size: 1,
            touches_boundary: false,
        }
    }
}

/// The undirected edge a directed edge runs along, and whether it runs from the lower index to the
/// higher, so that the two directions of one edge are counted apart inside one entry. A directed
/// edge whose ends are the same index is its own reverse and is counted once, as ascending.
fn undirected(start: u32, end: u32) -> ([u32; 2], bool) {
    if start <= end {
        ([start, end], true)
    } else {
        ([end, start], false)
    }
}

/// Marks a vertex as reached by a boundary edge, and reports whether this is the first such edge,
/// which is what opens a component.
fn mark_boundary(vertices: &mut HashMap<u32, VertexSlot>, vertex: u32) -> bool {
    let slot = vertices
        .get_mut(&vertex)
        .expect("every corner index was inserted");
    let first = !slot.touches_boundary;
    slot.touches_boundary = true;
    first
}

/// Merges the sets of two boundary vertices, and reports whether they were apart — which is what
/// closes a component. The smaller set is attached to the larger one, so a representative walk
/// stays logarithmic without a second table to compress paths into.
fn unite(vertices: &mut HashMap<u32, VertexSlot>, left: u32, right: u32) -> bool {
    let left_root = representative(vertices, left);
    let right_root = representative(vertices, right);
    if left_root == right_root {
        return false;
    }
    let (smaller, larger) = if vertices[&left_root].size <= vertices[&right_root].size {
        (left_root, right_root)
    } else {
        (right_root, left_root)
    };
    let size = vertices[&smaller].size + vertices[&larger].size;
    vertices
        .get_mut(&smaller)
        .expect("every corner index was inserted")
        .parent = larger;
    vertices
        .get_mut(&larger)
        .expect("every corner index was inserted")
        .size = size;
    true
}

/// The vertex a set is represented by, following the parents the unions left behind.
fn representative(vertices: &HashMap<u32, VertexSlot>, vertex: u32) -> u32 {
    let mut current = vertex;
    while vertices[&current].parent != current {
        current = vertices[&current].parent;
    }
    current
}
