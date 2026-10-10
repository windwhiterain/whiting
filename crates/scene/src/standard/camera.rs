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
use crate::lighting::ViewProjection;
use crate::standard::math::{
    Bounds, Matrix, add, apply, cross, dot, look_at, multiply, normalize, orthographic,
    perspective, scale, subtract,
};
use crate::standard::scene_schema::{
    EYE, FAR, FIELD_OF_VIEW, FOCAL_LENGTH, NEAR, PROJECTION, ROLL, SENSOR_WIDTH, TARGET, UP,
};

/// The margin the convention's parallel view leaves on each side of the scene.
///
/// # Invariants
///
/// This is the only framing number left in this reader, and it belongs to a camera the
/// document never wrote: a scene that declares no camera is drawn by the convention, and the
/// convention has to choose an extent for itself. A camera that **is** written says how wide
/// its frame is and is not touched by it — see [`Camera::projection`].
const PARALLEL_MARGIN: f64 = 1.1;

/// The angle a camera that names no field of view sees through, in radians.
pub const DEFAULT_FIELD_OF_VIEW: f64 = std::f64::consts::PI / 3.0;

/// The width of the sensor a focal length is measured against when a scene names none, in
/// millimetres.
///
/// Thirty six is full frame, and it is the default because it is the number a focal length
/// is quoted against everywhere a lens is named. A scene with a smaller negative says so
/// and gets a wider field out of the same lens.
pub const DEFAULT_SENSOR_WIDTH: f64 = 36.0;

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
    /// The near plane the scene asked for, when it named one.
    pub(crate) near: Option<f64>,
    /// The far plane the scene asked for, when it named one.
    pub(crate) far: Option<f64>,
    /// How far the camera is turned about its own axis, in radians.
    pub(crate) roll: f64,
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
            // An angle is what the projection is built from, so a document that carries
            // both an angle and a focal length is a document whose one of the two is being
            // overridden. Taking the angle is that override, stated rather than inferred:
            // the alternative is a scene that changes picture when a field it never read
            // is added to it. The precedence itself is [`vertical_angle`], which a backend
            // that never builds a projection asks instead of this reader.
            field_of_view: vertical_angle(Some(camera), aspect),
            kind,
            near: read(NEAR).and_then(Value::as_float),
            far: read(FAR).and_then(Value::as_float),
            roll: read(ROLL).and_then(Value::as_float).unwrap_or(0.0),
            aspect,
            bounds: *bounds,
        })
    }

    /// Returns the `up` axis the view is built from, turned about the view by the roll.
    ///
    /// # Invariants
    ///
    /// The roll turns the **frame**, not the world: the eye and the target are untouched,
    /// so a rolled camera still looks at exactly what it looked at and a horizon in the
    /// frame is a horizon no longer level. That is the whole difference between this and
    /// `up`, which is a statement about which way is up in the world — and which is why
    /// `up` cannot express a roll at all: a camera whose up is not world-up has a tilted
    /// frame *or* a horizon that is not the world's, and a photograph wants the first.
    ///
    /// The axis is the view's own direction, so the turn is about the optical axis rather
    /// than about the world's y, and a camera already looking sideways rolls the same way
    /// whichever way it happens to be pointed.
    ///
    /// The result is made perpendicular to the view again afterwards, because a roll that
    /// carries the up axis towards the view direction would otherwise reach a `look_at`
    /// whose cross product has nothing left to normalise.
    fn rolled_up(&self, up: [f64; 3]) -> [f64; 3] {
        if self.roll == 0.0 {
            return up;
        }
        let axis = normalize(subtract(self.target, self.eye));
        let (sin, cos) = self.roll.sin_cos();
        let turned = add(
            add(scale(up, cos), scale(cross(axis, up), sin)),
            scale(axis, dot(axis, up) * (1.0 - cos)),
        );
        let perpendicular = subtract(turned, scale(axis, dot(axis, turned)));
        let length = dot(perpendicular, perpendicular).sqrt();
        if length < 1e-6 {
            // The roll has carried the up axis onto the view direction, where every way of
            // choosing a frame is as good as another and none of them is a frame at all.
            // The unturned axis is the one answer that is still a frame.
            return up;
        }
        scale(perpendicular, 1.0 / length)
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
            near: None,
            far: None,
            roll: 0.0,
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

    /// Returns the projection the camera draws the scene through.
    ///
    /// # Invariants
    ///
    /// The basis returned is the one `look_at` built the view from and the half extent is
    /// that same view's field of view, so a ray reconstructed from the two is the ray the
    /// matrix would have produced. That pairing is the only test that matters: a basis from
    /// one camera and a matrix from another is a sky drawn in the wrong place.
    ///
    /// **The eye is the one the document wrote, and nothing here moves it.** Deciding where a
    /// photographer stood is a decision about the scene, and a scene document carries
    /// decisions rather than the arithmetic that made them: a reader that fits the eye to the
    /// bounds would draw a different picture from every other backend reading the same
    /// document, and none of them could say why. A scene that wants its camera framed against
    /// its own geometry computes that eye when it builds the document, and writes the result
    /// down. See [`vertical_angle`] for the other half of the same rule: what a document says
    /// is drawn from is what is drawn.
    pub fn projection(&self) -> Projection {
        let direction = normalize(subtract(self.eye, self.target));
        // The roll is applied to the up axis once, here, and every `look_at` below is given
        // the turned axis. Turning it inside `look_at` instead would give two view matrices
        // for one camera — one per call site — and a scene whose matrix and whose ray basis
        // disagree is a sky drawn in the wrong place.
        let up = self.rolled_up(self.up);
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
                let half_extent =
                    (2.0 * self.bounds.largest_half_width() * PARALLEL_MARGIN).max(1e-3);
                let depth = distance + 2.0 * self.bounds.largest_half_width();
                let view = look_at(self.eye, self.target, up);
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
                // The eye is what the document wrote. See `projection`: fitting it here would
                // make this reader the only thing in the pipeline that decides where a camera
                // stands, and no other backend could reproduce the answer.
                let eye = self.eye;
                let view = look_at(eye, self.target, up);
                let (right, up, _) = basis(&view);
                let written = self.written_planes();
                // The depth planes bracket the whole scene rather than the part of it the
                // frame holds. A camera standing close to the scene is closer to it than the
                // scene is deep, and planes drawn at the frame's own depth would cut the near
                // half of it away.
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

    /// Returns how far in front of the eye a world point is, along the view's own axis.
    ///
    /// # Invariants
    ///
    /// The measure is **along the axis** rather than straight-line, because that is the axis
    /// the near plane stands perpendicular to: a point off to one side of the frame is
    /// clipped by its depth and not by how far away it happens to be. A parallel view has no
    /// divide and measures the same thing.
    ///
    /// A point behind the eye is negative rather than refused, so a caller asking where
    /// something is can see that it is behind rather than having to compare it against a
    /// distance it never had.
    ///
    /// ```
    /// use definition::{Path, Value};
    /// use whiting_scene::standard::math::Bounds;
    /// use whiting_scene::standard::Camera;
    ///
    /// let triple = |v: [f64; 3]| Value::List(v.into_iter().map(Value::Float).collect());
    /// let projection = Camera::read(
    ///     Some(&Value::structure([
    ///         ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
    ///         ("target".to_owned(), triple([0.0, 0.0, 0.0])),
    ///         ("projection".to_owned(), Value::variant("Perspective", [])),
    ///         ("near".to_owned(), Value::Float(1.0)),
    ///         ("far".to_owned(), Value::Float(21.0)),
    ///     ])),
    ///     &Path::root(),
    ///     &Bounds::around([0.0; 3], 4.0),
    ///     1.0,
    /// )
    /// .expect("a perspective camera reads")
    /// .projection();
    ///
    /// // The target is ten units in front of the eye, and so is a point five units to one
    /// // side of it: the frame's width is not the distance a near plane is measured by.
    /// assert!((projection.distance_to([0.0, 0.0, 0.0]) - 10.0).abs() < 1e-9);
    /// assert!((projection.distance_to([5.0, 0.0, 0.0]) - 10.0).abs() < 1e-9);
    /// // A point behind the eye is negative, which is the answer a caller comparing it
    /// // against the near plane needs.
    /// assert!(projection.distance_to([0.0, 0.0, 20.0]) < 0.0);
    /// ```
    pub fn distance_to(&self, point: [f64; 3]) -> f64 {
        dot(subtract(point, self.eye), self.forward)
    }

    /// Returns how much of the frame a box covers, as a fraction between zero and one, or
    /// `None` when the box reaches the eye and there is nothing to measure.
    ///
    /// # Invariants
    ///
    /// **This is a bound and not a measurement of what is drawn.** The eight corners of the
    /// box are projected and the rectangle they reach is what is compared with the frame, so
    /// a box turned away from the axes is counted by the axis aligned box around it and a
    /// long thin one is counted by the square it spans. Both read high, which is the
    /// acceptable direction: a note that says a box covers more of the frame than it does
    /// sends an author to look at a box that was worth looking at.
    ///
    /// A box the eye is inside, or one with a corner at or behind the eye, has **no** answer
    /// rather than a wrong one. A perspective projection divides by the distance in front of
    /// the eye, so the near face of such a box flips over the whole frame and any number
    /// computed from it is a statement about the divide. A box the eye is inside is a box
    /// the notes about a frame name separately and better, by label and by depth.
    ///
    /// The frame is minus one to one on both axes, so its area is four, and the fraction is
    /// the part of that rectangle the box's own reaches. A box entirely outside the frame
    /// covers nothing of it and returns zero rather than a negative share of it.
    ///
    /// ```
    /// use definition::{Path, Value};
    /// use whiting_scene::standard::math::Bounds;
    /// use whiting_scene::standard::Camera;
    ///
    /// let triple = |v: [f64; 3]| Value::List(v.into_iter().map(Value::Float).collect());
    /// let projection = Camera::read(
    ///     Some(&Value::structure([
    ///         ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
    ///         ("target".to_owned(), triple([0.0, 0.0, 0.0])),
    ///         ("projection".to_owned(), Value::variant("Perspective", [])),
    ///         ("near".to_owned(), Value::Float(1.0)),
    ///         ("far".to_owned(), Value::Float(21.0)),
    ///     ])),
    ///     &Path::root(),
    ///     &Bounds::around([0.0; 3], 4.0),
    ///     1.0,
    /// )
    /// .expect("a perspective camera reads")
    /// .projection();
    ///
    /// // A box two units across, five units in front of the eye, through a sixty degree
    /// // lens: the frame is 5.8 units wide at the box's near face, so a sixth of it by area.
    /// // The same box twice as far off covers a fifth as much, and the fall is a little
    /// // steeper than the square of the range because the box's own near face is nearer
    /// // than its centre.
    /// let near = projection
    ///     .frame_coverage(&Bounds::around([0.0, 0.0, 5.0], 1.0))
    ///     .expect("a box in front of the eye has a coverage");
    /// let far = projection
    ///     .frame_coverage(&Bounds::around([0.0, 0.0, 0.0], 1.0))
    ///     .expect("so does a box twice as far away");
    /// assert!((near - 0.1875).abs() < 1e-6, "a two unit box at five units covers {near}");
    /// assert!((far - 0.037).abs() < 1e-3, "and the same box at ten units covers {far}");
    ///
    /// // A wall twenty units across and two thick, four units in front of the eye, holds
    /// // all of the frame and none of the eye. One the eye is inside is not measured at
    /// // all.
    /// assert_eq!(
    ///     projection.frame_coverage(&Bounds {
    ///         minimum: [-10.0, -10.0, 4.0],
    ///         maximum: [10.0, 10.0, 6.0],
    ///     }),
    ///     Some(1.0)
    /// );
    /// assert_eq!(
    ///     projection.frame_coverage(&Bounds::around([0.0, 0.0, 9.0], 2.0)),
    ///     None,
    ///     "the eye is inside this box, so its share of the frame is not a thing to measure"
    /// );
    /// assert_eq!(
    ///     projection.frame_coverage(&Bounds::around([0.0, 0.0, 20.0], 0.5)),
    ///     None,
    ///     "and a box behind the eye is behind the frame"
    /// );
    /// ```
    pub fn frame_coverage(&self, box_of: &Bounds) -> Option<f64> {
        if box_of.depth_of(self.eye).is_some() {
            return None;
        }
        // The reach of the box in the frame's own units, as minus one to one on each axis.
        // Starting outside the frame's range is a seed rather than a box: the first corner
        // then moves both ends of both axes to where it actually is.
        let mut reach = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for corner in corners(box_of) {
            let clip = apply(&self.matrix, corner);
            // The clip `w` is the distance in front of the eye, and a parallel view carries
            // one whatever is where. A divide by it is the whole of what a perspective
            // projection is, so a corner at or behind the eye leaves nothing to divide.
            if clip[3] <= 1e-9 {
                return None;
            }
            let point = [clip[0] / clip[3], clip[1] / clip[3]];
            if !point[0].is_finite() || !point[1].is_finite() {
                return None;
            }
            reach[0] = reach[0].min(point[0]);
            reach[1] = reach[1].min(point[1]);
            reach[2] = reach[2].max(point[0]);
            reach[3] = reach[3].max(point[1]);
        }
        let share = |from: f64, to: f64| (to.min(1.0) - from.max(-1.0)).max(0.0);
        Some(share(reach[0], reach[2]) * share(reach[1], reach[3]) / 4.0)
    }
}

/// The standard camera seen as the readings a clustered grid measures a view by.
///
/// # Invariants
///
/// Every one of these forwards to the projection's own field or method, so the grid is
/// measured against the same numbers the pass is drawn through. A forwarding impl rather than
/// a second computation is the whole point: two answers to "how deep is this distance" is how
/// a light ends up in a cluster no fragment is in.
impl ViewProjection for Projection {
    fn eye(&self) -> [f64; 3] {
        self.eye
    }

    fn forward(&self) -> [f64; 3] {
        self.forward
    }

    fn right(&self) -> [f64; 3] {
        self.right
    }

    fn up(&self) -> [f64; 3] {
        self.up
    }

    fn near(&self) -> f64 {
        self.near
    }

    fn far(&self) -> f64 {
        self.far
    }

    fn depth_at(&self, distance: f64) -> f64 {
        Projection::depth_at(self, distance)
    }

    fn distance_at(&self, depth: f64) -> f64 {
        Projection::distance_at(self, depth)
    }

    fn half_width_at(&self, distance: f64) -> [f64; 2] {
        Projection::half_width_at(self, distance)
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

/// Returns the full **vertical** angle a camera sees through, in radians.
///
/// # Invariants
///
/// The precedence is one function of its own, and a backend that never builds a projection
/// asks it rather than reading the camera's fields a second time: **an angle beats a lens**, a
/// lens beats the convention, and the convention is [`DEFAULT_FIELD_OF_VIEW`]. A second reader
/// would be a second answer to the same question, and the two would drift exactly where the
/// drift is invisible — a scene that names a lens.
///
/// `aspect` is the frame's own, because a lens is named against a sensor and a sensor is a
/// width: see [`vertical_field_of_view`], which is the lens half on its own.
///
/// `camera` is `None` for a scene that declares no camera at all, which is the convention.
pub fn vertical_angle(camera: Option<&Value>, aspect: f64) -> f64 {
    let Some(camera) = camera else {
        return DEFAULT_FIELD_OF_VIEW;
    };
    let number = |name: &str| field(camera, name).and_then(Value::as_float);
    number(FIELD_OF_VIEW)
        .or_else(|| {
            number(FOCAL_LENGTH).map(|focal_length| {
                vertical_field_of_view(
                    focal_length,
                    number(SENSOR_WIDTH).unwrap_or(DEFAULT_SENSOR_WIDTH),
                    aspect,
                )
            })
        })
        .unwrap_or(DEFAULT_FIELD_OF_VIEW)
}

/// Returns the full vertical angle a lens of a given focal length sees through.
///
/// # Invariants
///
/// - **The angle a focal length buys is horizontal, and the projection wants it vertical.**
///   A lens is named against a film size, and a thirty six millimetre sensor is thirty six
///   millimetres across; a scene's `field_of_view` is the angle down the frame, which is a
///   different number as soon as the frame is not square. Dividing the horizontal tangent
///   by the aspect is what turns one into the other, and doing it the other way round
///   quietly widens every frame that is not square — which is every frame a scene asks for.
/// - **A frame taller than the sensor is wide is measured against the other side.** The
///   number the scene writes is a width, so a portrait frame on a full frame negative wants
///   the height, and the scene says which by writing the other number. There is no way to
///   tell the two apart from the focal length alone, and guessing would be a lens the
///   author did not name.
/// - **A degenerate pair is the convention.** A focal length of zero, or a sensor narrower
///   than nothing, is not a lens; it falls back to the angle a camera that named no lens
///   has always had, rather than to an infinite field of view.
fn vertical_field_of_view(focal_length: f64, sensor_width: f64, aspect: f64) -> f64 {
    let (focal_length, sensor_width, aspect) = (
        focal_length.abs().max(1e-3),
        sensor_width.abs().max(1e-3),
        aspect.max(1e-3),
    );
    let horizontal = 2.0 * (sensor_width / (2.0 * focal_length)).atan();
    let vertical = 2.0 * ((horizontal / 2.0).tan() / aspect).atan();
    if vertical.is_finite() && vertical > 1e-6 {
        vertical
    } else {
        DEFAULT_FIELD_OF_VIEW
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
