//! A control net over a rectangle, indexed by two integers.
//!
//! This is the net the design needs and the triangle's net could not provide. A rectangle has a
//! **linear order across each edge**, so the row of control points immediately inside an edge has
//! exactly as many entries as the edge itself and lines up with it index by index. A triangle has
//! no such order: one side of an edge carries `degree` interior positions and the other `degree -
//! 1`, and no assignment makes them one to one. That mismatch is why the G1 invariant below is
//! exact here and had nothing to say on a triangle.
//!
//! The degree may differ per direction. Nothing in the representation needs it to — every patch in
//! this crate is bi-degree — but a net that could only be bi-degree would make "elevate one side"
//! unrepresentable, and the storage cost of allowing it is zero.
//!
//! Two properties carry the layered design, and both are measured by `examples/rect_elevation_probe.rs`
//! rather than assumed:
//!
//! - **Partition of unity.** The basis weights sum to one, so moving every control point in a
//!   point's support by the same displacement moves that point by exactly that displacement, with
//!   no solve. A drag is an assignment, not a system.
//! - **Linear, shape-preserving elevation.** [`RectNet::elevate_u`] and [`RectNet::elevate_v`] add
//!   control points without moving the surface and are linear in the control points. That is what
//!   lets a refinement layer hold a *displacement* rather than an absolute net, so an empty layer
//!   contributes nothing and keeps contributing nothing through further refinements.

use core::fmt;

use crate::bernstein::basis;
use crate::point::Point3;

/// The number of control points in a net of the given degrees.
pub const fn count(degree_u: usize, degree_v: usize) -> usize {
    (degree_u + 1) * (degree_v + 1)
}

/// Why a net could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectNetError {
    /// The degrees need exactly `expected` control points and `found` were supplied.
    PointCount {
        degree_u: usize,
        degree_v: usize,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for RectNetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::PointCount {
                degree_u,
                degree_v,
                expected,
                found,
            } => write!(
                formatter,
                "a net of degrees ({degree_u}, {degree_v}) needs {expected} control points, \
                 {found} were supplied"
            ),
        }
    }
}

impl std::error::Error for RectNetError {}

/// A Bernstein control net over a rectangle.
///
/// Points are addressed by `(i, j)` and stored with `i` running slowest, so the point at `(i, j)`
/// lives at `i * (degree_v + 1) + j`. Row-major in `u` rather than `v` only matters to code that
/// walks the storage directly; every other route goes through [`RectNet::point`].
#[derive(Clone, Debug, PartialEq)]
pub struct RectNet {
    degree_u: usize,
    degree_v: usize,
    points: Vec<Point3>,
}

impl RectNet {
    /// Builds a net of the given degrees from exactly [`count`]`(degree_u, degree_v)` points.
    pub fn new(
        degree_u: usize,
        degree_v: usize,
        points: Vec<Point3>,
    ) -> Result<Self, RectNetError> {
        let expected = count(degree_u, degree_v);
        if points.len() != expected {
            return Err(RectNetError::PointCount {
                degree_u,
                degree_v,
                expected,
                found: points.len(),
            });
        }
        Ok(Self {
            degree_u,
            degree_v,
            points,
        })
    }

    /// The bilinear net through the four corners of a rectangle.
    ///
    /// This is the shape a freshly built patch starts at, exactly as [`crate::simplex::SimplexNet`]
    /// starts at its own linear net: the assembled surface *is* the input face, so a model has no
    /// construction error to absorb later.
    ///
    /// The arguments are in `(u, v)` corner order and the storage is `u`-major, so the two are not
    /// the same sequence: the net holds `c00, c01, c10, c11`.
    pub fn from_corners(c00: Point3, c10: Point3, c01: Point3, c11: Point3) -> Self {
        Self {
            degree_u: 1,
            degree_v: 1,
            points: vec![c00, c01, c10, c11],
        }
    }

    pub fn degrees(&self) -> (usize, usize) {
        (self.degree_u, self.degree_v)
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    /// The control point at `(i, j)`, or `None` when the index is outside the net.
    pub fn point(&self, i: usize, j: usize) -> Option<Point3> {
        if i > self.degree_u || j > self.degree_v {
            None
        } else {
            Some(self.points[i * (self.degree_v + 1) + j])
        }
    }

    pub fn point_mut(&mut self, i: usize, j: usize) -> Option<&mut Point3> {
        if i > self.degree_u || j > self.degree_v {
            None
        } else {
            Some(&mut self.points[i * (self.degree_v + 1) + j])
        }
    }

    /// Overwrites the control point at `(i, j)`. Returns whether the index was in range.
    ///
    /// A single-point move is the whole editing primitive; nothing else in the crate writes a
    /// control point.
    pub fn set_point(&mut self, i: usize, j: usize, point: Point3) -> bool {
        match self.point_mut(i, j) {
            Some(slot) => {
                *slot = point;
                true
            }
            None => false,
        }
    }

    /// Evaluates the surface at the parameters `(u, v)`.
    ///
    /// No clamping and no renormalising: a parameter outside `[0, 1]` is evaluated as given, so a
    /// caller bug shows up as a wrong answer instead of as a plausible one.
    pub fn evaluate(&self, u: f64, v: f64) -> Point3 {
        evaluate_slice(self.degree_u, self.degree_v, &self.points, u, v)
    }

    /// The surface normal at `(u, v)`. See [`normal_slice`], which carries the contract: a normal is
    /// not a displacement and does not sum across levels.
    pub fn normal(&self, u: f64, v: f64) -> Point3 {
        normal_slice(self.degree_u, self.degree_v, &self.points, u, v)
    }

    /// Raises the `u` degree by one without moving the surface.
    ///
    /// The one-dimensional identity is `B(n+1, k) = k/(n+1) B(n, k-1) + (n+1-k)/(n+1) B(n, k)`,
    /// which turns each old row into a new row of `degree + 2` points by a convex combination of its
    /// own neighbours. The two weights sum to one, so every new point is a blend of old ones and the
    /// surface they span is unchanged.
    ///
    /// Elevating the two directions separately is not an approximation of a single step: elevation
    /// is separable over a tensor product, and the two orders give the same net.
    pub fn elevate_u(&self) -> Self {
        let raised = self.degree_u + 1;
        let scale = 1.0 / raised as f64;
        let mut points = Vec::with_capacity(count(raised, self.degree_v));
        for i in 0..=raised {
            let before = i as f64 * scale;
            let after = 1.0 - before;
            for j in 0..=self.degree_v {
                points.push(
                    self.at_or_zero(i.saturating_sub(1), j) * before
                        + self.at_or_zero(i, j) * after,
                );
            }
        }
        debug_assert_eq!(points.len(), count(raised, self.degree_v));
        Self {
            degree_u: raised,
            degree_v: self.degree_v,
            points,
        }
    }

    /// Raises the `v` degree by one without moving the surface. See [`RectNet::elevate_u`].
    pub fn elevate_v(&self) -> Self {
        let raised = self.degree_v + 1;
        let scale = 1.0 / raised as f64;
        let new_stride = raised + 1;
        let mut points = vec![Point3::ZERO; count(self.degree_u, raised)];
        for i in 0..=self.degree_u {
            for j in 0..=raised {
                let before = j as f64 * scale;
                let after = 1.0 - before;
                points[i * new_stride + j] = self.at_or_zero(i, j.saturating_sub(1)) * before
                    + self.at_or_zero(i, j) * after;
            }
        }
        Self {
            degree_u: self.degree_u,
            degree_v: raised,
            points,
        }
    }

    /// Raises both degrees by one without moving the surface.
    pub fn elevate(&self) -> Self {
        self.elevate_u().elevate_v()
    }

    /// Applies [`RectNet::elevate`] `steps` times.
    pub fn elevate_by(&self, steps: usize) -> Self {
        let mut net = self.clone();
        for _ in 0..steps {
            net = net.elevate();
        }
        net
    }

    /// The point at `(i, j)`, or zero when the index is outside the net.
    ///
    /// The out-of-range neighbours are the reason elevation works in one line each: a new row's end
    /// points have only one old neighbour, and a missing one contributes nothing.
    fn at_or_zero(&self, i: usize, j: usize) -> Point3 {
        self.point(i, j).unwrap_or(Point3::ZERO)
    }
}

/// Evaluates the Bernstein surface whose control points are `points`, laid out with `i` slowest.
///
/// Free rather than a method because a mesh keeps its control points in one flat buffer and lends
/// out one face's slice of it — see [`crate::level::Level`]. A method on [`RectNet`] alone would
/// force every such caller to materialise a net first, which is one allocation per face read.
pub fn evaluate_slice(
    degree_u: usize,
    degree_v: usize,
    points: &[Point3],
    u: f64,
    v: f64,
) -> Point3 {
    let weights_u = basis(degree_u, u);
    let weights_v = basis(degree_v, v);
    let stride = degree_v + 1;
    let mut sum = Point3::ZERO;
    for (i, weight_u) in weights_u.iter().enumerate() {
        for (j, weight_v) in weights_v.iter().enumerate() {
            sum += points[i * stride + j] * (weight_u * weight_v);
        }
    }
    sum
}

/// The surface normal of the Bernstein surface whose control points are `points`, laid out with `i`
/// slowest.
///
/// The two partial derivatives, and the cross product between them:
///
/// ```text
/// S_u = sum over i,j of  i * B(i, u) * P(i, j)
/// S_v = sum over i,j of  j * B(j, v) * P(i, j)
/// ```
///
/// The second basis drops out of each derivative because the weights sum to one, so each tangent is
/// a weighted sum of the whole net rather than a weighted sum along a row.
///
/// # A normal is not a displacement
///
/// **Normals of levels do not add.** A displacement level contributes `dS`, and the normal of the
/// sum is `normal(base + offset)`, which is not `normal(base) + normal(offset)` and not the length-
/// weighted blend of the two either. So this is read from the **composed** level — see
/// [`crate::PatchMesh::composed`] — and never summed per level the way a position is.
///
/// Returns zero where the surface has no tangent: a degree-0 direction, or a patch whose two
/// tangents happen to be parallel there. A zero normal is a fact about the patch rather than a
/// failure, and a caller shading from it has to decide what that means.
pub fn normal_slice(degree_u: usize, degree_v: usize, points: &[Point3], u: f64, v: f64) -> Point3 {
    let weights_u = basis(degree_u, u);
    let weights_v = basis(degree_v, v);
    let stride = degree_v + 1;
    let mut tangent_u = Point3::ZERO;
    let mut tangent_v = Point3::ZERO;
    for i in 0..=degree_u {
        for j in 0..=degree_v {
            let point = points[i * stride + j];
            tangent_u += point * (weights_u[i] * i as f64);
            tangent_v += point * (weights_v[j] * j as f64);
        }
    }
    tangent_u.cross(tangent_v)
}

/// Indexing panics on an index outside the net, which is the standard `Index` contract. Use
/// [`RectNet::point`] when the index is not known to be in range.
impl core::ops::Index<(usize, usize)> for RectNet {
    type Output = Point3;

    fn index(&self, (i, j): (usize, usize)) -> &Point3 {
        &self.points[i * (self.degree_v + 1) + j]
    }
}

impl core::ops::IndexMut<(usize, usize)> for RectNet {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut Point3 {
        let stride = self.degree_v + 1;
        &mut self.points[i * stride + j]
    }
}
