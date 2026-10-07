//! A control net over a triangle, indexed by barycentric coordinates.
//!
//! The net is a Bernstein tensor product on the 2-simplex: degree `n`, one control point per
//! triple `(i, j, k)` with `i + j + k == n`, evaluated as
//!
//! ```text
//! S(a, b, c) = sum over i + j + k = n of  P[i][j] * n!/(i! j! k!) * a^i b^j c^k
//! ```
//!
//! Two properties of this basis carry the whole design, and both are measured by
//! `examples/elevation_probe.rs` rather than assumed:
//!
//! - **Partition of unity.** The basis values sum to `(a + b + c)^n == 1`. Moving every control
//!   point in a point's support by the same displacement moves that point by exactly that
//!   displacement, with no solve.
//! - **Linear, shape-preserving elevation.** [`SimplexNet::elevate`] adds control points without
//!   moving the surface, and is linear in the control points. That is what lets a refinement
//!   layer hold a displacement rather than an absolute net, so an empty layer contributes
//!   nothing and stays contributing nothing through any number of further refinements.

use core::fmt;

use crate::bernstein::multinomial;

/// The number of control points in a degree-`degree` net over a triangle.
pub const fn count(degree: usize) -> usize {
    (degree + 1) * (degree + 2) / 2
}

use crate::point::Point3;

/// Why a net could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimplexNetError {
    /// A degree-`degree` net needs exactly `expected` control points and `found` were supplied.
    PointCount {
        degree: usize,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for SimplexNetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::PointCount {
                degree,
                expected,
                found,
            } => write!(
                formatter,
                "a degree-{degree} net needs {expected} control points, {found} were supplied"
            ),
        }
    }
}

impl std::error::Error for SimplexNetError {}

/// Row `i` starts here and holds `degree + 1 - i` points, `j` running from `0`.
const fn row_start(degree: usize, i: usize) -> usize {
    i * (degree + 1) - i * i.saturating_sub(1) / 2
}

/// A Bernstein control net over a triangle.
///
/// Points are addressed by the pair `(i, j)`; the third barycentric index is implied as
/// `degree - i - j`. A degree-`n` net holds `(n+1)(n+2)/2` points.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplexNet {
    degree: usize,
    points: Vec<Point3>,
}

impl SimplexNet {
    /// Builds a net of `degree` from exactly [`count`]`(degree)` points.
    pub fn new(degree: usize, points: Vec<Point3>) -> Result<Self, SimplexNetError> {
        let expected = count(degree);
        if points.len() != expected {
            return Err(SimplexNetError::PointCount {
                degree,
                expected,
                found: points.len(),
            });
        }
        Ok(Self { degree, points })
    }

    /// The linear net over the three corners of a triangle — the mesh itself.
    ///
    /// This is **degree 1**, not degree 0. A degree-0 simplex net holds [`count`]`(0)` = 1 point
    /// and is a constant surface; the ladder has to start at degree 1 for the patch to be the
    /// triangle. Level 0 of every patch in the design is this net, so a model reproduces its mesh
    /// exactly and has no construction error to absorb.
    ///
    /// The arguments are in **barycentric vertex order**: `a` is the point the surface reaches at
    /// `[1, 0, 0]`, `b` at `[0, 1, 0]`, `c` at `[0, 0, 1]`. Storage order is not that order — a
    /// degree-1 net addresses `(i, j)` with `k = 1 - i - j`, so the `c` vertex is the `k = 1` one
    /// and comes first.
    pub fn from_corners(a: Point3, b: Point3, c: Point3) -> Self {
        let mut points = vec![Point3::ZERO; count(1)];
        points[0] = c;
        points[1] = b;
        points[2] = a;
        Self { degree: 1, points }
    }

    pub fn degree(&self) -> usize {
        self.degree
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

    /// The control point at barycentric index `(i, j)`, with the third index implied.
    pub fn point(&self, i: usize, j: usize) -> Option<Point3> {
        if i > self.degree || j > self.degree - i {
            None
        } else {
            Some(self.points[row_start(self.degree, i) + j])
        }
    }

    pub fn point_mut(&mut self, i: usize, j: usize) -> Option<&mut Point3> {
        if i > self.degree || j > self.degree - i {
            None
        } else {
            Some(&mut self.points[row_start(self.degree, i) + j])
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

    /// Evaluates the surface at the barycentric coordinates `[a, b, c]`.
    ///
    /// The caller supplies all three because the basis needs `a + b + c == 1` to hold; a triple
    /// that does not sum to one is evaluated as given rather than silently renormalised, so a
    /// caller bug shows up as a wrong answer instead of as a plausible one.
    pub fn evaluate(&self, barycentric: [f64; 3]) -> Point3 {
        let [a, b, c] = barycentric;
        let n = self.degree;
        let mut sum = Point3::ZERO;
        for i in 0..=n {
            let a_power = a.powi(i as i32);
            for j in 0..=(n - i) {
                let k = n - i - j;
                let weight = multinomial(n, i, j) * a_power * b.powi(j as i32) * c.powi(k as i32);
                sum += self.points[row_start(n, i) + j] * weight;
            }
        }
        sum
    }

    /// Evaluates the surface on the triangle edge `k == 0`, which is the curve two patches
    /// sharing that edge both interpolate.
    pub fn evaluate_boundary(&self, a: f64, b: f64) -> Point3 {
        self.evaluate([a, b, 0.0])
    }

    /// Raises the degree by one without moving the surface.
    ///
    /// The identity this rests on: on the simplex `a + b + c == 1`, so
    /// `a^i b^j c^k == a^(i+1) b^j c^k + a^i b^(j+1) c^k + a^i b^j c^(k+1)`. Converting each term
    /// into the normalised degree-`n+1` basis turns it into a neighbour of the same net scaled by
    /// its index over `n + 1`.
    ///
    /// Linear in the control points and shape preserving, which is the property the layered
    /// representation is built on.
    pub fn elevate(&self) -> Self {
        let n = self.degree;
        let raised = n + 1;
        let scale = 1.0 / raised as f64;
        let mut points = Vec::with_capacity(count(raised));

        for i in 0..=raised {
            for j in 0..=(raised - i) {
                let k = raised - i - j;
                let mut point = Point3::ZERO;
                // Sources are this net's (i-1, j), (i, j-1) and (i, j); the third barycentric
                // index of each is implied, and lands on k, k and k-1 respectively.
                if i >= 1 {
                    point += self.at_or_zero(i - 1, j) * (i as f64 * scale);
                }
                if j >= 1 {
                    point += self.at_or_zero(i, j - 1) * (j as f64 * scale);
                }
                if k >= 1 {
                    point += self.at_or_zero(i, j) * (k as f64 * scale);
                }
                points.push(point);
            }
        }

        Self {
            degree: raised,
            points,
        }
    }

    /// Applies [`SimplexNet::elevate`] `steps` times.
    pub fn elevate_by(&self, steps: usize) -> Self {
        let mut net = self.clone();
        for _ in 0..steps {
            net = net.elevate();
        }
        net
    }

    /// The point at barycentric index `(i, j)`, or zero when the index is out of range.
    fn at_or_zero(&self, i: usize, j: usize) -> Point3 {
        self.point(i, j).unwrap_or(Point3::ZERO)
    }
}

/// Indexing panics on an index outside the net, which is the standard `Index` contract. Use
/// [`SimplexNet::point`] when the index is not known to be in range.
impl core::ops::Index<(usize, usize)> for SimplexNet {
    type Output = Point3;

    fn index(&self, (i, j): (usize, usize)) -> &Point3 {
        &self.points[row_start(self.degree, i) + j]
    }
}

impl core::ops::IndexMut<(usize, usize)> for SimplexNet {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut Point3 {
        &mut self.points[row_start(self.degree, i) + j]
    }
}
