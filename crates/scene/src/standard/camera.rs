//! Where a scene is drawn from, and the projection that decides how it looks.
//!
//! # The decision
//!
//! A scene that writes no camera is drawn by the convention: parallel, dead ahead, framing
//! everything that is drawn. That convention is a stated one rather than an author's
//! decision, and it is what every scene drew before a camera existed.
//!
//! A scene that writes a camera is a scene that chose its composition — where it looks
//! from, what it looks at, how wide the lens is, and whether the projection divides at
//! all. Nothing about the pipeline changes: the camera is a projection on the pass, and the
//! pass is what a shader reads.
//!
//! # Invariants
//!
//! - The `position` a pass carries is the eye the projection was **actually** made from.
//!   A perspective projection pulls the eye back until the scene fits the field of view, so
//!   the eye the author asked for sets the direction of the view and not its distance —
//!   and a fragment shader that derived its view vector from the eye the author wrote
//!   would be looking from somewhere the camera is not.
//! - The horizontal field of view follows the aspect of the frame being drawn into, so a
//!   wide frame is wider rather than cropped. A scene cannot set the aspect itself, because
//!   the frame is not the document's to choose.
//! - A field of view is a full angle in radians, because a half angle is a number an
//!   author has to halve in their head and get wrong.
//!
//! # What a reader gets from a projection
//!
//! A matrix is enough to draw through and not enough to reason about: how deep something is,
//! how far away a depth is, and how wide the frame is at a distance are the three questions a
//! clustered grid asks, and all three are answered here rather than by a reader working them
//! out of four matrix entries.
//!
//! ```
//! use definition::{Path, Value};
//! use whiting_scene::standard::math::Bounds;
//! use whiting_scene::standard::{Camera, ProjectionKind};
//!
//! let bounds = Bounds::around([0.0, 0.0, 0.0], 4.0);
//! let triple = |v: [f64; 3]| Value::List(v.into_iter().map(Value::Float).collect());
//!
//! // A parallel camera: the near plane is at depth zero, the far plane at depth one, and
//! // the frame is the same width at every distance.
//! let parallel = Camera::read(None, &Path::root(), &bounds, 1.0)
//!     .expect("the convention camera reads")
//!     .projection();
//! assert_eq!(parallel.kind, ProjectionKind::Orthographic);
//! assert!(parallel.depth_at(parallel.near).abs() < 1e-12);
//! assert!((parallel.depth_at(parallel.far) - 1.0).abs() < 1e-12);
//! assert_eq!(
//!     parallel.half_width_at(parallel.near),
//!     parallel.half_width_at(parallel.far),
//!     "a parallel view does not widen"
//! );
//!
//! // A perspective camera: the same two planes at the same two depths, and a frame that
//! // widens with distance, four times as wide at the far plane as at the near one here.
//! let perspective = Camera::read(
//!     Some(&Value::structure([
//!         ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
//!         ("target".to_owned(), triple([0.0, 0.0, 0.0])),
//!         ("projection".to_owned(), Value::variant("Perspective", [])),
//!         ("near".to_owned(), Value::Float(1.0)),
//!         ("far".to_owned(), Value::Float(21.0)),
//!     ])),
//!     &Path::root(),
//!     &bounds,
//!     1.0,
//! )
//! .expect("a perspective camera reads")
//! .projection();
//! assert_eq!(perspective.kind, ProjectionKind::Perspective);
//! assert!(perspective.depth_at(1.0).abs() < 1e-12);
//! assert!((perspective.depth_at(21.0) - 1.0).abs() < 1e-12);
//! let (near_half, far_half) = (
//!     perspective.half_width_at(1.0)[0],
//!     perspective.half_width_at(21.0)[0],
//! );
//! assert!(
//! (far_half / near_half - 21.0).abs() < 1e-9,
//!     "the frame is twenty one times as wide at twenty one times the distance"
//! );
//!
//! // The two mappings invert, for both kinds, which is what lets a depth slice be found by
//! // its depth and then measured as a distance.
//! for projection in [parallel, perspective] {
//!     for step in 0..=8 {
//!         let depth = step as f64 / 8.0;
//!         let distance = projection.distance_at(depth);
//!         // The near plane comes back a hair under itself, so the mapping is exact to
//!         // floating point rather than bit for bit. Anything that indexed a slice boundary
//!         // on equality would be at the mercy of the last bit.
//!         assert!(
//!             (distance - projection.near).min(projection.far - distance) > -1e-9,
//!             "depth {depth} gave {distance}, outside {} to {}",
//!             projection.near,
//!             projection.far
//!         );
//!         assert!(
//!             (projection.depth_at(distance) - depth).abs() < 1e-9,
//!             "depth {depth} came back from distance {distance}"
//!         );
//!     }
//! }
//! ```

use definition::{Path, Value};

use crate::CompileError;
use crate::standard::emit::FRAMING_PADDING;
use crate::standard::math::{
    Bounds, Matrix, add, apply, look_at, multiply, normalize, orthographic, perspective, scale,
    subtract,
};
use crate::standard::scene_schema::{
    EYE, FAR, FIELD_OF_VIEW, NEAR, PADDING as PADDING_FIELD, PROJECTION, TARGET, UP,
};

/// The angle a camera that names no field of view sees through, in radians.
pub const DEFAULT_FIELD_OF_VIEW: f64 = std::f64::consts::PI / 3.0;

/// Which projection a camera draws through.
///
/// It is part of the projection rather than a private detail of building one, because a reader
/// of a [`Projection`] needs it to know how the frame widens with distance: a parallel view is
/// the same width everywhere and a perspective view grows with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionKind {
    /// Distances measure the same wherever they are, which is what a diagram is made of.
    Orthographic,
    /// Distances shorten with range, which is what a photograph is made of.
    Perspective,
}

/// A camera, read out of a scene document.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    eye: [f64; 3],
    pub(crate) target: [f64; 3],
    pub(crate) up: [f64; 3],
    pub(crate) field_of_view: f64,
    pub(crate) kind: ProjectionKind,
    pub(crate) padding: f64,
    /// The near plane the scene asked for, when it named one.
    pub(crate) near: Option<f64>,
    /// The far plane the scene asked for, when it named one.
    pub(crate) far: Option<f64>,
    pub(crate) aspect: f64,
    pub(crate) bounds: Bounds,
}

/// A projection, and the eye it was made from.
///
/// # Invariants
///
/// The basis is carried alongside the matrix because a stage that draws over the frame
/// has to reconstruct a ray through each pixel, and reconstructing one from a matrix needs
/// an inverse that a shader cannot compute — `WGSL` has no matrix inverse. Carrying the
/// three axes the view was built from turns that into arithmetic: a ray is the forward axis
/// plus so much of the right and up axes as the pixel is off centre.
pub struct Projection {
    /// The matrix world space is drawn through.
    pub matrix: Matrix,
    /// Where the projection was made from, which may not be where the camera asked to be.
    pub eye: [f64; 3],
    /// The direction the camera looks along.
    pub forward: [f64; 3],
    /// The axis the camera's right runs along.
    pub right: [f64; 3],
    /// The axis the camera's up runs along.
    pub up: [f64; 3],
    /// How far off centre the edge of the frame is, horizontally and then vertically.
    ///
    /// A ray through a pixel is `forward + right * x * half_extent.0 + up * y *
    /// half_extent.1`, for a pixel whose centre is at `x` and `y` in minus one to one.
    pub half_extent: [f64; 2],
    /// Which projection this was, which is what says whether the frame widens with distance.
    pub kind: ProjectionKind,
    /// How far from the eye the near plane is.
    pub near: f64,
    /// How far from the eye the far plane is.
    pub far: f64,
}

impl Camera {
    /// Reads one camera of a scene, falling back to the convention field by field.
    ///
    /// # Invariants
    ///
    /// A scene with no camera, or a compile that names one the scene does not have, is not an
    /// error. Every field is read against the convention rather than against a default, so a
    /// camera that names only an eye is a camera that looks at the middle of what is drawn.
    pub fn read(
        camera: Option<&Value>,
        path: &Path,
        bounds: &Bounds,
        aspect: f64,
    ) -> Result<Self, CompileError> {
        let convention = Self::convention(bounds, aspect);
        let Some(camera) = camera else {
            return Ok(convention);
        };
        let read = |name: &str| field(camera, name);
        let kind = match read(PROJECTION).map(Value::variant_tag) {
            Some(Some("Perspective")) => ProjectionKind::Perspective,
            Some(Some("Orthographic")) => ProjectionKind::Orthographic,
            None => convention.kind,
            Some(Some(other)) => {
                return Err(CompileError::Emission {
                    reason: format!(
                        "at {}: `{other}` is not a projection a camera draws through",
                        path.key(PROJECTION)
                    ),
                });
            }
            Some(None) => {
                return Err(CompileError::Emission {
                    reason: format!(
                        "at {}: a camera's projection is Perspective or Orthographic",
                        path.key(PROJECTION)
                    ),
                });
            }
        };
        Ok(Self {
            eye: optional_triple(read(EYE), path.key(EYE))?.unwrap_or(convention.eye),
            target: optional_triple(read(TARGET), path.key(TARGET))?.unwrap_or(convention.target),
            up: optional_triple(read(UP), path.key(UP))?.unwrap_or(convention.up),
            field_of_view: read(FIELD_OF_VIEW)
                .and_then(Value::as_float)
                .unwrap_or(DEFAULT_FIELD_OF_VIEW),
            kind,
            padding: read(PADDING_FIELD)
                .and_then(Value::as_float)
                .unwrap_or(FRAMING_PADDING),
            near: read(NEAR).and_then(Value::as_float),
            far: read(FAR).and_then(Value::as_float),
            aspect,
            bounds: *bounds,
        })
    }

    /// Returns the camera a scene with no camera field is drawn through.
    fn convention(bounds: &Bounds, aspect: f64) -> Self {
        let centre = bounds.centre();
        Self {
            eye: [
                centre[0],
                centre[1],
                centre[2] + 3.0 * bounds.largest_half_width(),
            ],
            target: centre,
            up: [0.0, 1.0, 0.0],
            field_of_view: DEFAULT_FIELD_OF_VIEW,
            kind: ProjectionKind::Orthographic,
            padding: FRAMING_PADDING,
            near: None,
            far: None,
            aspect,
            bounds: *bounds,
        }
    }

    /// Returns the two depth planes the scene asked for, or `None` when it asked for
    /// neither.
    ///
    /// # Invariants
    ///
    /// The two are read as a pair because one plane without the other cannot place a
    /// camera: what a projection needs is where the depth range starts and ends, and a
    /// near plane on its own is a number with nothing to be measured against.
    fn written_planes(&self) -> Option<(f64, f64)> {
        match (self.near, self.far) {
            (Some(near), Some(far)) => Some((near, far)),
            _ => None,
        }
    }

    /// Returns the nearest distance the eye can sit at and still hold the whole box.
    ///
    /// # Invariants
    ///
    /// A corner is inside the frame when its offset from the axis is no larger than the
    /// frame is wide **at that corner's own depth**, and depth is measured from the eye.
    /// Rearranging gives the bound below: the eye has to be at least as far as the
    /// corner's own distance along the view plus the distance the frame's width covers.
    /// The bound is taken over all eight corners because the one that needs the most room
    /// is whichever is nearest the eye and furthest off axis, and which corner that is
    /// depends on where the eye is.
    fn fitting_distance(
        &self,
        forward: [f64; 3],
        right: [f64; 3],
        up: [f64; 3],
        vertical_half: f64,
    ) -> f64 {
        let horizontal_half = (vertical_half * self.aspect).max(1e-6);
        let mut needed: f64 = 0.0;
        for corner in corners(&self.bounds) {
            let offset = subtract(corner, self.target);
            let along = dot(offset, forward);
            needed = needed.max(dot(offset, right).abs() / horizontal_half + along);
            needed = needed.max(dot(offset, up).abs() / vertical_half + along);
        }
        needed.max(1e-3)
    }

    /// Returns the projection the camera draws the scene through.
    ///
    /// # Invariants
    ///
    /// The basis returned is the one `look_at` built the view from and the half extent is
    /// that same view's field of view, so a ray reconstructed from the two is the ray the
    /// matrix would have produced. That pairing is the only test that matters: a basis from
    /// one camera and a matrix from another is a sky drawn in the wrong place.
    pub fn projection(&self) -> Projection {
        let direction = normalize(subtract(self.eye, self.target));
        // The first three columns of a view matrix are its right, up and back axes, in
        // that order, and the back axis is the direction back towards the eye.
        let basis = |view: &Matrix| {
            let column = |index: usize| [view[index], view[4 + index], view[8 + index]];
            (column(0), column(1), column(2))
        };
        let forward = scale(direction, -1.0);
        match self.kind {
            // A parallel projection has no distance to fit: the box lands on the frame
            // wherever the eye is, so the eye sets the direction and nothing else. The
            // extent is twice the scene's half width on each side, which is the margin the
            // convention has always left.
            ProjectionKind::Orthographic => {
                let distance = dot(
                    subtract(self.eye, self.target),
                    subtract(self.eye, self.target),
                )
                .sqrt();
                let half_extent = (2.0 * self.bounds.largest_half_width() * self.padding).max(1e-3);
                let depth = distance + 2.0 * self.bounds.largest_half_width();
                let view = look_at(self.eye, self.target, self.up);
                let (right, up, _) = basis(&view);
                Projection {
                    matrix: multiply(&orthographic(half_extent, depth / 4.0, depth), &view),
                    eye: self.eye,
                    forward,
                    right,
                    up,
                    half_extent: [half_extent, half_extent],
                    kind: ProjectionKind::Orthographic,
                    near: depth / 4.0,
                    far: depth,
                }
            }
            ProjectionKind::Perspective => {
                // A perspective projection maps depth with a divide, so how far back the
                // eye has to be is not one number but a bound per corner of the box: a
                // corner is inside the frame when it is no further off the axis than the
                // frame is wide at that corner's own depth.
                //
                // Fitting a sphere around the box instead — the obvious thing — puts the
                // eye much further out than any corner needs, and for a wide flat scene
                // much further out than the scene is deep. A heightfield fills a box whose
                // height is a tenth of its width; a sphere around that box is as wide as
                // the box and as tall as its half width, and fitting it puts the camera
                // far enough away that the field's own edges fall inside the frame.
                let vertical_half = (self.field_of_view / 2.0)
                    .clamp(1e-3, std::f64::consts::FRAC_PI_2 - 1e-3)
                    .tan();
                let forward = scale(direction, -1.0);
                // Where the eye ends up is the scene's decision when the scene wrote both
                // depth planes, and the frame's otherwise. Fitting never moves the eye off
                // the ray it was given, so the direction above is the view either way.
                //
                // A camera that composes itself — one on a hillside looking down a valley —
                // cannot have its viewpoint decided by how large the scene is. The extent
                // of the ground says nothing about where a photographer stood.
                let written = self.written_planes();
                let eye = match written {
                    Some(_) => self.eye,
                    None => {
                        let view = look_at(self.eye, self.target, self.up);
                        let (right, up, _) = basis(&view);
                        let distance = (self.fitting_distance(forward, right, up, vertical_half)
                            * self.padding)
                            .max(1e-3);
                        add(self.target, scale(direction, distance))
                    }
                };
                let view = look_at(eye, self.target, self.up);
                let (right, up, _) = basis(&view);
                // The depth planes bracket the whole scene rather than the part of it the
                // frame holds. A camera that crops in — a `padding` below one — is closer
                // to the scene than the scene is deep, and planes drawn at the frame's own
                // depth would cut the near half of it away.
                //
                // The bracket is the span of the corners' own depths along the view, which
                // is the deepest the box reaches in each direction. Bracketing by the half
                // diagonal instead — `distance` minus one diagonal to `distance` plus one —
                // holds a wide flat scene's near plane an entire diagonal in front of
                // anything drawn, and depth mapped from a near plane at zero is depth that
                // cannot tell two close surfaces apart.
                let (near, far) = written.unwrap_or_else(|| {
                    let mut nearest = f64::INFINITY;
                    let mut farthest = 0.0f64;
                    for corner in corners(&self.bounds) {
                        let depth = dot(subtract(corner, eye), forward);
                        nearest = nearest.min(depth);
                        farthest = farthest.max(depth);
                    }
                    // A small margin, so a point exactly on the bound is not clipped by a
                    // rounding error in the matrix.
                    let margin = (farthest - nearest).max(1.0) * 0.01;
                    // The floor under the near plane is a fraction of the eye's own distance
                    // rather than a hair above zero. A camera that crops in stands inside the
                    // box, no near plane brackets a box the eye is inside, and one at zero
                    // maps every drawn depth into the last few float values below one: the
                    // depth buffer becomes unable to tell any two surfaces apart. A five
                    // hundredth of the distance loses nothing the frame can show, because
                    // nothing that close to the eye was going to be in it.
                    let reach = dot(subtract(eye, self.target), subtract(eye, self.target)).sqrt();
                    ((nearest - margin).max(reach * 0.002), farthest + margin)
                });
                Projection {
                    matrix: multiply(
                        &perspective(self.field_of_view, self.aspect, near, far),
                        &view,
                    ),
                    eye,
                    forward,
                    right,
                    up,
                    // The vertical half angle of the view, and the horizontal one widened
                    // by the aspect: a wide frame is wider rather than cropped.
                    half_extent: [vertical_half * self.aspect, vertical_half],
                    kind: ProjectionKind::Perspective,
                    near,
                    far,
                }
            }
        }
    }
}

/// The mappings a reader of a [`Projection`] needs and a matrix alone does not give.
///
/// They are methods rather than free functions because every one of them is read off the
/// projection itself, and a mapping that took the matrix and the near and far planes as three
/// separate arguments is a mapping that can be handed the wrong one of the three.
impl Projection {
    /// Returns the depth the projection gives a point a distance in front of the eye, along the
    /// view's own axis, in zero to one.
    ///
    /// # Invariants
    ///
    /// Depth is a function of distance alone, and a fragment carries the depth it was drawn
    /// at, so this is the number a fragment knows and the one a grid divided by depth is
    /// indexed by.
    ///
    /// The point is a **world** point a distance along the view's own axis, because the
    /// matrix is the projection times the view and a point given in view space would be
    /// measured from the wrong place. Depth is a linear fractional function of the distance —
    /// a divide by a quantity that grows with it — and the two clip points of the axis below
    /// are what fix it, so the pair here inverts exactly for either kind of camera.
    ///
    /// The distance must be in front of the eye: a perspective projection divides by the
    /// distance, and the distance of the eye itself is the one that divides by zero.
    pub fn depth_at(&self, distance: f64) -> f64 {
        self.depth_map().depth_at(distance)
    }

    /// Returns how far in front of the eye a point at a given depth is, which inverts
    /// [`Projection::depth_at`].
    ///
    /// # Invariants
    ///
    /// The inverse is the same algebra read backwards, rather than a matrix inverse, which
    /// `WGSL` has no way to write. The doctest in the module is what holds that it inverts,
    /// for both kinds of camera.
    ///
    /// A depth outside zero to one has no distance in front of the eye for a parallel view and
    /// has one behind it for a perspective one, so only zero to one is meaningful here.
    pub fn distance_at(&self, depth: f64) -> f64 {
        self.depth_map().distance_at(depth)
    }

    /// Returns the depth map of the view: what the projection's depth runs with.
    fn depth_map(&self) -> DepthMap {
        let at_eye = apply(&self.matrix, self.eye);
        let ahead = apply(&self.matrix, add(self.eye, self.forward));
        DepthMap {
            depth_at_eye: at_eye[2],
            depth_rise: ahead[2] - at_eye[2],
            weight_at_eye: at_eye[3],
            weight_rise: ahead[3] - at_eye[3],
        }
    }

    /// Returns how far off the centre of the frame the edge is at a distance from the eye,
    /// horizontally and then vertically.
    ///
    /// # Invariants
    ///
    /// A parallel view is the same width everywhere and a perspective view's widens with the
    /// distance it is at, so the half extent is one of the two readings and the kind is what
    /// says which. `half_extent` is the width itself for the first and the slope of the second,
    /// and multiplying by the distance is what turns a slope into a width.
    pub fn half_width_at(&self, distance: f64) -> [f64; 2] {
        match self.kind {
            ProjectionKind::Orthographic => self.half_extent,
            ProjectionKind::Perspective => [
                self.half_extent[0] * distance,
                self.half_extent[1] * distance,
            ],
        }
    }
}

/// How the projection's depth runs with the distance from the eye, as a linear fractional
/// function of it.
///
/// # Invariants
///
/// A projection divides by a quantity that grows with the distance from the eye, so the
/// depth is `(a + b * d) / (c + e * d)` and the four numbers come from the two clip points of
/// the view's own axis. The point has to be a *world* point: the matrix is the projection
/// times the view, so a point given in view space would be measured from the wrong place, and
/// a projection read as though it were the view alone gives a depth of a third of one at the
/// near plane rather than nothing at all.
struct DepthMap {
    /// The clip depth at the eye itself.
    depth_at_eye: f64,
    /// How the clip depth rises per unit of distance.
    depth_rise: f64,
    /// The clip `w` at the eye itself, which is one for a parallel view and zero for a
    /// perspective one.
    weight_at_eye: f64,
    /// How the clip `w` rises per unit of distance, which is the divide itself.
    weight_rise: f64,
}

impl DepthMap {
    /// Returns the depth a point a distance in front of the eye has.
    fn depth_at(&self, distance: f64) -> f64 {
        (self.depth_at_eye + distance * self.depth_rise)
            / (self.weight_at_eye + distance * self.weight_rise)
    }

    /// Returns the distance a point at a depth is in front of the eye.
    fn distance_at(&self, depth: f64) -> f64 {
        (depth * self.weight_at_eye - self.depth_at_eye)
            / (self.depth_rise - depth * self.weight_rise)
    }
}

/// Returns the eight corners of a box.
fn corners(box_of: &Bounds) -> [[f64; 3]; 8] {
    let mut found = [[0.0; 3]; 8];
    for (index, corner) in found.iter_mut().enumerate() {
        *corner = [
            if index & 1 == 0 {
                box_of.minimum[0]
            } else {
                box_of.maximum[0]
            },
            if index & 2 == 0 {
                box_of.minimum[1]
            } else {
                box_of.maximum[1]
            },
            if index & 4 == 0 {
                box_of.minimum[2]
            } else {
                box_of.maximum[2]
            },
        ];
    }
    found
}

/// Wraps a matrix and the eye it was made from as the value a pass carries.
///
/// This is what a pass that is not drawn through a camera needs: a light's projection is
/// a matrix and an eye, and has no basis because nothing reconstructs rays through it.
pub fn matrix_projection(matrix: Matrix, eye: [f64; 3]) -> Value {
    Value::structure([
        (
            "view_projection".to_owned(),
            Value::List(matrix.into_iter().map(Value::Float).collect()),
        ),
        (
            "position".to_owned(),
            Value::List(eye.into_iter().map(Value::Float).collect()),
        ),
    ])
}

/// Wraps a projection as the value a pass carries.
pub fn projection_value(projection: &Projection) -> Value {
    Value::structure([
        (
            "view_projection".to_owned(),
            Value::List(
                projection
                    .matrix
                    .iter()
                    .copied()
                    .map(Value::Float)
                    .collect(),
            ),
        ),
        (
            "position".to_owned(),
            Value::List(projection.eye.into_iter().map(Value::Float).collect()),
        ),
    ])
}

/// Returns a named field of a struct or the fields of an enum variant.
fn field<'value>(value: &'value Value, name: &str) -> Option<&'value Value> {
    value
        .as_field_map()
        .or_else(|| value.variant_fields())
        .and_then(|fields| fields.get(name))
}

/// Reads an optional three component vector, reporting a field that is not one.
fn optional_triple(value: Option<&Value>, path: Path) -> Result<Option<[f64; 3]>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(components) = value.as_list() else {
        return Err(CompileError::Emission {
            reason: format!("at {path}: expected a list of three components"),
        });
    };
    let numbers: Result<Vec<f64>, CompileError> = components
        .iter()
        .map(|component| {
            component
                .as_float()
                .or_else(|| component.as_int().map(|number| number as f64))
                .ok_or_else(|| CompileError::Emission {
                    reason: format!(
                        "at {path}: expected a number, found {}",
                        component.kind_name()
                    ),
                })
        })
        .collect();
    let numbers = numbers?;
    numbers
        .try_into()
        .map(Some)
        .map_err(|_| CompileError::Emission {
            reason: format!("at {path}: expected three components"),
        })
}

/// Returns the dot product of two vectors.
fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
