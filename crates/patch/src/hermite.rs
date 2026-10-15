//! One bicubic Hermite patch: the sixteen coefficients, and the four arithmetic operations a
//! caller actually needs on them.
//!
//! # The sixteen, and why they interleave
//!
//! The coefficient array is indexed `[a][b]`, `a` along `u` and `b` along `v`, and the four indices
//! along each direction **interleave value and derivative**:
//!
//! ```text
//!   a = 0   value at the near u end        b = 0   value at the near v end
//!   a = 1   u-derivative at the near end    b = 1   v-derivative at the near end
//!   a = 2   value at the FAR u end          b = 2   value at the FAR v end
//!   a = 3   u-derivative at the FAR end     b = 3   v-derivative at the FAR end
//! ```
//!
//! So the two `u` boundary curves are `a = 0` and `a = 2`, the two `v` ones are `b = 0` and `b = 3`,
//! and the interior coefficients are `a = 1, 3` and `b = 1, 3`. Reading the far edge as `a = 3`
//! takes its **tangent** where its position belongs, and produces a surface nobody drew.
//!
//! A corner at local `(u_end, v_end)` owns the four coefficients at `2*u_end` and `2*u_end + 1`,
//! times the same in `v`, and those are exactly `[Jet]`'s four fields.
//!
//! # The basis is Hermite, not Bernstein
//!
//! The basis is `h00 = 2t^3 - 3t^2 + 1`, `h10 = t^3 - 2t^2 + t`, `h01 = -2t^3 + 3t^2`,
//! `h11 = t^3 - t^2` — **not** the degree-3 Bernstein weights, which are a different set of
//! functions of the same degree and evaluate these four numbers as a Bezier curve. Substituting one
//! for the other takes `P_0` at the near end and `d_1` at the far end instead of `P_0` and `d_0`,
//! which displaces every corner by a tangent vector and reads as a surface that does not meet.
//! `examples/hermite_continuity_probe.rs` keeps an independent Bernstein evaluator precisely so the
//! two are compared rather than assumed interchangeable.
//!
//! # Steps
//!
//! Every derivative in the coefficient array has already been multiplied by the parameter step it
//! belongs to, and [`HermitePatch::tangent_u`] divides it back out. The step therefore cancels,
//! which is what lets two cells with different `u` steps agree on the normal across an edge they
//! share. **A step that is a property of one face rather than of the shared edge is two
//! independent reparameterizations, and what comes out is G1 rather than C1.**

use crate::point::Point3;

/// The four values and derivatives that decide how a surface leaves one corner of a patch.
///
/// A displacement is the same type as a position on purpose, so a layer of the surface can be
/// stored and summed without a second type for it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Jet {
    pub point: Point3,
    /// `dS/du` in **world** units, not per unit of the local parameter.
    pub du: Point3,
    /// `dS/dv` in world units.
    pub dv: Point3,
    /// The mixed derivative, in world units.
    pub duv: Point3,
}

impl Jet {
    pub const ZERO: Self = Self {
        point: Point3::ZERO,
        du: Point3::ZERO,
        dv: Point3::ZERO,
        duv: Point3::ZERO,
    };

    pub const fn new(point: Point3, du: Point3, dv: Point3, duv: Point3) -> Self {
        Self { point, du, dv, duv }
    }

    /// Displaces this jet's position and every one of its derivatives, which is what moving a
    /// vertex on a smooth surface does: the tangent directions move with it, so the surface stays
    /// exactly C1 rather than gaining a crease at a point nobody edited.
    pub fn displaced(self, by: Point3) -> Self {
        Self {
            point: self.point + by,
            du: self.du + by,
            dv: self.dv + by,
            duv: self.duv + by,
        }
    }
}

/// A bicubic Hermite patch over `[0, 1]^2` in each direction.
///
/// Sixteen coefficients, the same count as a degree-3 Bernstein net, and the same function class:
/// a bicubic Hermite patch and a bicubic Bezier patch are a bijective linear reparameterization of
/// each other. What differs is **which of the sixteen are independent**, which is the whole content
/// of `crate::hermite_surface`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HermitePatch {
    step_u: f64,
    step_v: f64,
    coefficients: [[Point3; 4]; 4],
}

/// Which of a patch's four corners, in the winding every quad in this crate uses.
///
/// `0 = (0,0)`, `1 = (1,0)`, `2 = (1,1)`, `3 = (0,1)` — the same order as
/// [`crate::topology::Topology`]'s `edge_endpoints` and [`crate::RectNet::from_corners`].
pub type Corner = usize;

impl HermitePatch {
    /// The step a parameter direction stands for, in world units.
    pub fn step_u(&self) -> f64 {
        self.step_u
    }

    pub fn step_v(&self) -> f64 {
        self.step_v
    }

    /// Builds a patch from the jets at its four corners, in `(0,0) (1,0) (1,1) (0,1)` order.
    ///
    /// `step_u` and `step_v` are the parameter distances each direction spans. They are stored so
    /// the derivatives above can be written once, in world units, and converted on the way out.
    pub fn new(step_u: f64, step_v: f64, corners: [Jet; 4]) -> Self {
        debug_assert!(
            step_u > 0.0 && step_v > 0.0,
            "a patch's steps are distances"
        );
        let mut coefficients = [[Point3::ZERO; 4]; 4];
        for (index, corner) in corners.iter().enumerate() {
            let (u_end, v_end) = Self::corner_ends(index);
            let value = (2 * u_end, 2 * v_end);
            let along_u = (2 * u_end + 1, 2 * v_end);
            let along_v = (2 * u_end, 2 * v_end + 1);
            let mixed = (2 * u_end + 1, 2 * v_end + 1);
            coefficients[value.0][value.1] = corner.point;
            coefficients[along_u.0][along_u.1] = corner.du * step_u;
            coefficients[along_v.0][along_v.1] = corner.dv * step_v;
            coefficients[mixed.0][mixed.1] = corner.duv * (step_u * step_v);
        }
        Self {
            step_u,
            step_v,
            coefficients,
        }
    }

    /// A patch over a unit square in each direction.
    ///
    /// The shape a freshly built model starts at when all it has is four corner positions: the
    /// Hermite image of the bilinear map, which reproduces a coplanar quad exactly. The
    /// cross-boundary derivatives are taken from the quad's own diagonals rather than set to zero,
    /// so the starting surface is not a ruled fan.
    pub fn from_corners(corners: [Point3; 4]) -> Self {
        let [c00, c10, c11, c01] = corners;
        // Two triangles of the quad, each contributing a diagonal-aligned tangent. The choice is
        // not unique and is not load-bearing: any pair of directions makes a valid patch, and every
        // edit after this replaces them.
        let du0 = (c10 - c00) * 0.5 + (c01 - c11) * 0.5;
        let du1 = (c11 - c01) * 0.5 + (c10 - c00) * 0.5;
        let dv0 = (c01 - c00) * 0.5 + (c10 - c11) * 0.5;
        let dv1 = (c11 - c10) * 0.5 + (c01 - c00) * 0.5;
        Self::new(
            1.0,
            1.0,
            [
                Jet::new(c00, du0, dv0, Point3::ZERO),
                Jet::new(c10, du1, dv0, Point3::ZERO),
                Jet::new(c11, du1, dv1, Point3::ZERO),
                Jet::new(c01, du0, dv1, Point3::ZERO),
            ],
        )
    }

    /// Where a corner sits on the unit square, in the winding every quad in this crate uses.
    ///
    /// `0 = (0,0)`, `1 = (1,0)`, `2 = (1,1)`, `3 = (0,1)`. The `_ =>` arm is `(0,1)` rather than
    /// `(1,1)`: folding the last two corners together looks harmless and is not, because corner 3
    /// is a different vertex of the quad from corner 2 and placing it on the same one makes a face
    /// disagree with its neighbour about where the edge runs.
    pub const fn corner_ends(index: Corner) -> (usize, usize) {
        match index {
            0 => (0, 0),
            1 => (1, 0),
            2 => (1, 1),
            _ => (0, 1),
        }
    }

    /// The value at `(s, t)`. No clamping and no renormalizing: a parameter outside `[0, 1]` is
    /// evaluated as given, so a caller bug shows up as a wrong answer rather than a plausible one.
    pub fn evaluate(&self, s: f64, t: f64) -> Point3 {
        let along_u = hermite_basis(s);
        let along_v = hermite_basis(t);
        self.blend(&along_u, &along_v)
    }

    /// `dS/du` in **world** units.
    ///
    /// The weights along `u` are the basis slopes **in the order they are stored**, with no
    /// rearrangement: `S = sum over a, b of h_a(s) h_b(t) coefficient[a][b]`, so `dS/ds` weights
    /// index `a` by `h_a'(s)` and leaves index `b` alone. What falls out is that `S_s` at an end
    /// reads only the coefficients whose `a` is the **derivative** slot at that end — which is why
    /// the cross-boundary derivative reaches into the far corner and a probe reading only the near
    /// edge cannot see it.
    pub fn tangent_u(&self, s: f64, t: f64) -> Point3 {
        let along_u = hermite_basis_slope(s);
        let along_v = hermite_basis(t);
        self.blend(&along_u, &along_v) / self.step_u
    }

    /// `dS/dv` in world units. See [`HermitePatch::tangent_u`].
    pub fn tangent_v(&self, s: f64, t: f64) -> Point3 {
        let along_u = hermite_basis(s);
        let along_v = hermite_basis_slope(t);
        self.blend(&along_u, &along_v) / self.step_v
    }

    /// `dS/du x dS/dv`, which is zero where the two tangents are parallel.
    ///
    /// **A normal is not a displacement and does not add across layers** — the normal of a sum is
    /// not the sum of the normals. Read it from the composed surface, never per layer.
    pub fn normal(&self, s: f64, t: f64) -> Point3 {
        self.tangent_u(s, t).cross(self.tangent_v(s, t))
    }

    /// One coefficient, in the indexed sense the module documentation describes.
    pub fn coefficient(&self, a: usize, b: usize) -> Point3 {
        self.coefficients[a][b]
    }

    /// The jet at one corner, with the steps divided back out.
    pub fn corner(&self, index: Corner) -> Jet {
        let (u_end, v_end) = Self::corner_ends(index);
        let at = |a: usize, b: usize| self.coefficients[a][b];
        Jet {
            point: at(2 * u_end, 2 * v_end),
            du: at(2 * u_end + 1, 2 * v_end) / self.step_u,
            dv: at(2 * u_end, 2 * v_end + 1) / self.step_v,
            duv: at(2 * u_end + 1, 2 * v_end + 1) / (self.step_u * self.step_v),
        }
    }

    /// Overwrites the jet at one corner, leaving the other fifteen alone.
    pub fn set_corner(&mut self, index: Corner, jet: Jet) {
        *self = Self::new(self.step_u, self.step_v, {
            let mut corners = [
                self.corner(0),
                self.corner(1),
                self.corner(2),
                self.corner(3),
            ];
            corners[index] = jet;
            corners
        });
    }

    /// A boundary curve, as the four Hermite jets that draw it: `(P, dP/dt, P, dP/dt)`.
    ///
    /// `far` picks which of the pair. Which boundary a shared edge is depends on the two faces'
    /// parameter directions rather than on the edge, so it is asked for rather than assumed — and
    /// reading the near boundary when the edge is the far one compares two curves that share
    /// nothing and reports a split on a surface that is perfect.
    pub fn boundary(&self, along_u: bool, far: bool) -> [Point3; 4] {
        let line = if far { 2 } else { 0 };
        if along_u {
            [
                self.coefficients[line][0],
                self.coefficients[line][1],
                self.coefficients[line][2],
                self.coefficients[line][3],
            ]
        } else {
            [
                self.coefficients[0][line],
                self.coefficients[1][line],
                self.coefficients[2][line],
                self.coefficients[3][line],
            ]
        }
    }

    /// The four Hermite jets of a boundary as the four Bezier control points that draw the same
    /// curve, `(P_0, P_0 + d_0/3, P_1 - d_1/3, P_1)`.
    ///
    /// This identity is why a Hermite boundary **is** the shared Bernstein row it replaces rather
    /// than a different curve.
    pub fn boundary_as_bezier(boundary: [Point3; 4]) -> [Point3; 4] {
        [
            boundary[0],
            boundary[0] + boundary[1] / 3.0,
            boundary[2] - boundary[3] / 3.0,
            boundary[2],
        ]
    }

    /// The tensor product, `sum over a, b of coefficient[a][b] * along_u[a] * along_v[b]`.
    fn blend(&self, along_u: &[f64; 4], along_v: &[f64; 4]) -> Point3 {
        let mut sum = Point3::ZERO;
        for (a, row) in self.coefficients.iter().enumerate() {
            for (b, coefficient) in row.iter().enumerate() {
                sum += *coefficient * (along_u[a] * along_v[b]);
            }
        }
        sum
    }
}

/// `h00, h10, h01, h11` at `t`, in the order the coefficient array stores them.
pub fn hermite_basis(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        2.0 * t3 - 3.0 * t2 + 1.0,
        t3 - 2.0 * t2 + t,
        -2.0 * t3 + 3.0 * t2,
        t3 - t2,
    ]
}

/// `d/dt` of [`hermite_basis`].
///
/// Differentiated here rather than by finite difference: a tangent plane measured through a
/// difference is a tangent plane one order wrong, and the two derivative functions that vanish at an
/// end are the reason the cross-boundary derivative reaches into the far corner.
pub fn hermite_basis_slope(t: f64) -> [f64; 4] {
    let t2 = t * t;
    [
        6.0 * t2 - 6.0 * t,
        3.0 * t2 - 4.0 * t + 1.0,
        -6.0 * t2 + 6.0 * t,
        3.0 * t2 - 2.0 * t,
    ]
}
