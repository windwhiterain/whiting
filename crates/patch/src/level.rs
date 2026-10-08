//! Every control point of one level, laid out face after face in a single allocation.
//!
//! A `Vec<RectNet>` is one heap allocation per face, so elevating a mesh allocates and frees once
//! per face, and a hierarchy multiplies that by the level count. The layout here is flat, for the
//! same reason elevation is two one-dimensional passes and not a closed form: a face's points are
//! **contiguous**, so gathering them into the next degree is a copy and not a reallocation.
//!
//! One level is *one buffer*, whether it holds absolute positions or a displacement against the root.
//! What makes it safe to sum levels is not this module — it is that the continuity invariant is
//! affine in its inputs, so enforcing it level by level and adding gives the same surface as enforcing
//! it once on the total. [`crate::continuity`] carries that argument and `layers_probe` checks it.
//!
//! Nothing here knows about edges, seams or topology. This is where control points live; what they
//! mean is [`crate::patch_mesh`]'s business.

use crate::point::Point3;
use crate::rect::{RectNet, count, evaluate_slice};

/// Where one face's points live inside a [`Level`], and what shape they are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceLayout {
    degree_u: usize,
    degree_v: usize,
    start: usize,
}

impl FaceLayout {
    /// The degrees of the face whose points begin at `start`.
    pub const fn new(degree_u: usize, degree_v: usize, start: usize) -> Self {
        Self {
            degree_u,
            degree_v,
            start,
        }
    }

    pub const fn degrees(&self) -> (usize, usize) {
        (self.degree_u, self.degree_v)
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    /// How many points a face of these degrees holds.
    const fn length(&self) -> usize {
        (self.degree_u + 1) * (self.degree_v + 1)
    }
}

/// One face's control points, borrowed.
///
/// This is the hot-path view and it allocates nothing. A caller that wants to evaluate patch 7 does
/// not get the other 99,999 faces materialised first, which is the whole reason a mesh keeps its
/// points flat.
#[derive(Clone, Copy, Debug)]
pub struct Face<'a> {
    degree_u: usize,
    degree_v: usize,
    points: &'a [Point3],
}

impl<'a> Face<'a> {
    pub const fn degrees(&self) -> (usize, usize) {
        (self.degree_u, self.degree_v)
    }

    pub const fn points(&self) -> &'a [Point3] {
        self.points
    }

    /// The point at `(i, j)`, or `None` when the index is outside this face.
    pub fn point(&self, i: usize, j: usize) -> Option<Point3> {
        if i > self.degree_u || j > self.degree_v {
            return None;
        }
        self.points.get(i * (self.degree_v + 1) + j).copied()
    }

    /// Evaluates this face at `(u, v)`. See [`RectNet::evaluate`] for the no-clamping contract.
    pub fn evaluate(&self, u: f64, v: f64) -> Point3 {
        evaluate_slice(self.degree_u, self.degree_v, self.points, u, v)
    }

    /// Copies this face into a [`RectNet`], which is one allocation. For callers that want a patch
    /// value; [`Face::evaluate`] is the allocation-free route.
    pub fn to_net(&self) -> RectNet {
        RectNet::new(self.degree_u, self.degree_v, self.points.to_vec())
            .expect("a face's own points always have its own count")
    }

    /// The point at `(i, j)`, or zero when the index is outside this face.
    ///
    /// Out-of-range reads are zero rather than a panic so that elevation can run one expression over
    /// the edge rows, where a new row's end point has only one real neighbour.
    fn at_or_zero(&self, i: usize, j: usize) -> Point3 {
        self.point(i, j).unwrap_or(Point3::ZERO)
    }
}

/// Indexing panics on an index outside the face, which is the standard `Index` contract and the same
/// one [`RectNet`] keeps.
impl core::ops::Index<(usize, usize)> for Face<'_> {
    type Output = Point3;

    fn index(&self, (i, j): (usize, usize)) -> &Point3 {
        &self.points[i * (self.degree_v + 1) + j]
    }
}

/// All the faces of one level, sharing one buffer of control points.
///
/// Faces may differ in degree. A level normally has every face at one degree, but nothing here
/// requires it and [`Level::elevate_into`] raises each face on its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Level {
    points: Vec<Point3>,
    faces: Vec<FaceLayout>,
}

impl Level {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lays out `nets` face after face.
    pub fn from_nets(nets: &[RectNet]) -> Self {
        let mut points = Vec::with_capacity(nets.iter().map(RectNet::len).sum());
        let mut faces = Vec::with_capacity(nets.len());
        for net in nets {
            faces.push(FaceLayout::new(
                net.degrees().0,
                net.degrees().1,
                points.len(),
            ));
            points.extend_from_slice(net.points());
        }
        Self { points, faces }
    }

    /// An all-zero level over `nets`' shapes. The zero level: summing it changes nothing.
    pub fn zeros_like(nets: &[RectNet]) -> Self {
        let mut points = Vec::with_capacity(nets.iter().map(RectNet::len).sum());
        let mut faces = Vec::with_capacity(nets.len());
        for net in nets {
            faces.push(FaceLayout::new(
                net.degrees().0,
                net.degrees().1,
                points.len(),
            ));
            points.resize(points.len() + net.len(), Point3::ZERO);
        }
        Self { points, faces }
    }

    /// How many faces.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Every control point, face after face. The only way to see a level whole, and one borrow.
    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    /// Mutable access to every control point, face after face.
    pub fn points_mut(&mut self) -> &mut [Point3] {
        &mut self.points
    }

    /// How many control points in total, which is what memory this level actually costs.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// The layout of every face, which is what a caller needs to walk the level without borrowing a
    /// face per step.
    pub fn layouts(&self) -> &[FaceLayout] {
        &self.faces
    }

    /// The face at `index`, or `None` when there is no such face.
    pub fn face(&self, index: usize) -> Option<Face<'_>> {
        let layout = *self.faces.get(index)?;
        self.points
            .get(layout.start..layout.start + layout.length())
            .map(|points| Face {
                degree_u: layout.degree_u,
                degree_v: layout.degree_v,
                points,
            })
    }

    /// The degrees of the first face, which is where a mesh-wide degree is read from. Every face of a
    /// level moves together, so the first one answers for all of them.
    pub fn degrees(&self) -> (usize, usize) {
        self.faces
            .first()
            .map(FaceLayout::degrees)
            .unwrap_or((0, 0))
    }

    /// The point at `(i, j)` of `face`, or `None` when the face or the index is out of range.
    pub fn point(&self, face: usize, i: usize, j: usize) -> Option<Point3> {
        self.face(face)?.point(i, j)
    }

    /// Overwrites the point at `(i, j)` of `face`. Returns whether the index was in range, matching
    /// [`RectNet::set_point`].
    pub fn set_point(&mut self, face: usize, i: usize, j: usize, point: Point3) -> bool {
        let Some(layout) = self.faces.get(face).copied() else {
            return false;
        };
        if i > layout.degree_u || j > layout.degree_v {
            return false;
        }
        let stride = layout.degree_v + 1;
        match self.points.get_mut(layout.start + i * stride + j) {
            Some(slot) => {
                *slot = point;
                true
            }
            None => false,
        }
    }

    /// Adds `other` into this level, point for point.
    ///
    /// **The two levels must describe the same mesh.** A face of `other` whose points start past the
    /// end of this level's buffer is a different mesh, not a smaller one, and is left alone rather
    /// than silently dropped into the wrong face.
    pub fn add_assign(&mut self, other: &Level) {
        for layout in &other.faces {
            let Some(slice) = other
                .points
                .get(layout.start..layout.start + layout.length())
            else {
                continue;
            };
            for (offset, point) in slice.iter().enumerate() {
                if let Some(slot) = self.points.get_mut(layout.start + offset) {
                    *slot += *point;
                }
            }
        }
    }

    /// Raises every face one degree in both directions, into `out`.
    ///
    /// `scratch` is reused for every face, so elevating a whole level costs **one** allocation rather
    /// than one per face — which is the reason this type exists at all.
    pub fn elevate_into(&self, out: &mut Level, scratch: &mut Vec<Point3>) {
        let mut points = Vec::with_capacity(self.points.len() * 2);
        let mut faces = Vec::with_capacity(self.faces.len());
        for index in 0..self.faces.len() {
            let face = self
                .face(index)
                .expect("a layout index always names a face");
            faces.push(FaceLayout::new(
                face.degrees().0 + 1,
                face.degrees().1 + 1,
                points.len(),
            ));
            elevated_points(face, scratch);
            points.extend_from_slice(scratch);
        }
        out.points = points;
        out.faces = faces;
    }

    /// Raises every face one degree, in a fresh level. Convenient; [`Level::elevate_into`] is the one
    /// to reach for in a loop.
    pub fn elevated(&self) -> Self {
        let mut out = Level::new();
        self.elevate_into(&mut out, &mut Vec::new());
        out
    }

    /// Every face as a [`RectNet`], one allocation each. For callers that want patches as values.
    pub fn to_nets(&self) -> Vec<RectNet> {
        (0..self.len())
            .map(|face| {
                self.face(face)
                    .expect("a face index below the length always resolves")
                    .to_net()
            })
            .collect()
    }
}

/// Gathers `face`'s points one degree up in each direction, into `scratch`.
///
/// Two one-dimensional passes, because elevation is separable over a tensor product and the two
/// orders give the same net. The out-of-range reads in [`Face::at_or_zero`] are load-bearing: a new
/// row's end point has only one real neighbour and the missing one contributes nothing.
fn elevated_points(face: Face<'_>, scratch: &mut Vec<Point3>) {
    let (degree_u, degree_v) = face.degrees();
    let raised_u = degree_u + 1;
    let raised_v = degree_v + 1;

    let mut in_u = Vec::with_capacity(count(raised_u, degree_v));
    let scale_u = 1.0 / raised_u as f64;
    for i in 0..=raised_u {
        let before = i as f64 * scale_u;
        let after = 1.0 - before;
        for j in 0..=degree_v {
            in_u.push(
                face.at_or_zero(i.saturating_sub(1), j) * before + face.at_or_zero(i, j) * after,
            );
        }
    }

    scratch.clear();
    scratch.reserve(count(raised_u, raised_v));
    let scale_v = 1.0 / raised_v as f64;
    for i in 0..=raised_u {
        let at = |j: usize| -> Point3 {
            in_u.get(i * (degree_v + 1) + j)
                .copied()
                .unwrap_or(Point3::ZERO)
        };
        for j in 0..=raised_v {
            let before = j as f64 * scale_v;
            let after = 1.0 - before;
            scratch.push(at(j.saturating_sub(1)) * before + at(j) * after);
        }
    }
}
