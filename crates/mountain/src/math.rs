//! Plan and space arithmetic.
//!
//! # Invariants
//!
//! A [plan point](Vec2) is `(x, z)` and a [space point](Vec3) is `(x, y, z)`, so the plan is
//! the space with `y` dropped. Every function that reads a plan point out of a space point
//! reads those two axes and no others, which is what keeps "the mesh is a graph over the
//! plan" true at every call site rather than at most of them.

/// A point or direction in the plan: `x` east, `z` south.
pub type Vec2 = [f32; 2];

/// A point or direction in space: `x` east, `y` up, `z` south.
pub type Vec3 = [f32; 3];

/// Returns the plan part of a space point.
pub fn plan(point: Vec3) -> Vec2 {
    [point[0], point[2]]
}

/// Returns a plan point lifted to a space point at an elevation.
pub fn lift(point: Vec2, elevation: f32) -> Vec3 {
    [point[0], elevation, point[1]]
}

/// Returns `to - from`, in the plan.
pub fn difference(to: Vec2, from: Vec2) -> Vec2 {
    [to[0] - from[0], to[1] - from[1]]
}

/// Returns `from` moved by `delta`.
pub fn offset(from: Vec2, delta: Vec2) -> Vec2 {
    [from[0] + delta[0], from[1] + delta[1]]
}

/// Returns `from` to `to` interpolated by `amount`.
pub fn lerp(from: Vec2, to: Vec2, amount: f32) -> Vec2 {
    [
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
    ]
}

/// Returns the cross product of two plan vectors, which is a scalar: positive when the turn
/// from `left` to `right` is the one the upward face is wound with.
pub fn cross(left: Vec2, right: Vec2) -> f32 {
    left[0] * right[1] - left[1] * right[0]
}

/// Returns the dot product of two plan vectors.
pub fn dot(left: Vec2, right: Vec2) -> f32 {
    left[0] * right[0] + left[1] * right[1]
}

/// Returns the length of a plan vector.
pub fn length(vector: Vec2) -> f32 {
    dot(vector, vector).sqrt()
}

/// Returns the distance between two plan points.
pub fn distance(from: Vec2, to: Vec2) -> f32 {
    length(difference(to, from))
}

/// Returns the distance between two space points.
///
/// # Invariants
///
/// **In three dimensions, not the plan distance of the same two points.** A crest is a line
/// that climbs, and "62% of the way along it" is a fraction of the line's real length; a plan
/// distance would flatten it, and a steep flank would read as shorter than a shallow one of the
/// same footprint. It is a separate function rather than a generalisation of [`distance`]
/// because every existing caller wanted the plan one, and a silent switch would move every
/// tessellation step in the crate.
pub fn span(from: Vec3, to: Vec3) -> f32 {
    let squared = (0..3)
        .map(|axis| to[axis] - from[axis])
        .map(|d| d * d)
        .sum::<f32>();
    squared.sqrt()
}

/// Returns a unit plan vector, falling back to east for one with no length.
///
/// # Invariants
///
/// Two coincident points have no direction between them, and returning a direction for them
/// would put a sign on a quantity that has none. East is the fallback rather than zero
/// because every caller here multiplies a direction by a length, and a zero direction would
/// silently delete an edge rather than place it somewhere arbitrary and visible.
pub fn normalized(vector: Vec2) -> Vec2 {
    let magnitude = length(vector);
    if magnitude > f32::EPSILON {
        [vector[0] / magnitude, vector[1] / magnitude]
    } else {
        [1.0, 0.0]
    }
}

/// Returns the signed plan area of a triangle, positive for the winding an upward face uses.
pub fn signed_area(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    cross(difference(b, a), difference(c, a)) * 0.5
}

/// Returns `to - from`, in space.
pub fn subtract(to: Vec3, from: Vec3) -> Vec3 {
    [to[0] - from[0], to[1] - from[1], to[2] - from[2]]
}

/// Returns the cross product of two space vectors.
pub fn cross3(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Returns the unit normal of a triangle wound the way its corners are given.
///
/// # Invariants
///
/// A degenerate triangle — three collinear corners, which a belt produces wherever a rib
/// collapses to a point — has no normal to return, and east is the fallback for the same
/// reason [`normalized`] falls back: a caller multiplying by it gets a direction rather than
/// a zero that silently deletes a term.
pub fn normal(left: Vec3, right: Vec3) -> Vec3 {
    let product = cross3(left, right);
    let magnitude =
        (product[0] * product[0] + product[1] * product[1] + product[2] * product[2]).sqrt();
    if magnitude > f32::EPSILON {
        [
            product[0] / magnitude,
            product[1] / magnitude,
            product[2] / magnitude,
        ]
    } else {
        [1.0, 0.0, 0.0]
    }
}

/// Returns the upward normal of a triangle whose corners are wound so its plan area is positive.
///
/// # Invariants
///
/// The corners are taken in the **other** order from [`normal`], and that is not a slip. `y` is
/// up, the plan is `(x, z)`, and in a right-handed frame `x` cross `z` is *minus* `y`: so a
/// triangle whose signed plan area is positive — the winding the gallery's own generators write,
/// and the one [`signed_area`] measures — has a right-handed normal pointing **down**. What this
/// crate builds is a height field, and the side of it that is looked at is the one above, so the
/// normal returned is the negation of the right-handed one.
///
/// Getting this backwards is not a subtle error and it does not look like a wrong normal: it is
/// every face shaded by a vector pointing away from the light, which reads as a massif in
/// permanent shadow with a few facets lit where the two mistakes very nearly cancel.
pub fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    normal(subtract(c, a), subtract(b, a))
}

/// Returns a value pinned into a range.
pub fn clamp(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}
