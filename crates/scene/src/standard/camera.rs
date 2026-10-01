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

use definition::{Path, Value};

use crate::CompileError;
use crate::standard::emit::FRAMING_PADDING;
use crate::standard::math::{
    Bounds, Matrix, add, look_at, multiply, normalize, orthographic, perspective, scale, subtract,
};
use crate::standard::scene_schema::{
    CAMERA, EYE, FIELD_OF_VIEW, PADDING as PADDING_FIELD, PROJECTION, TARGET, UP,
};

/// The angle a camera that names no field of view sees through, in radians.
pub const DEFAULT_FIELD_OF_VIEW: f64 = std::f64::consts::PI / 3.0;

/// Which projection a camera draws through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
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
    pub(crate) kind: Kind,
    pub(crate) padding: f64,
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
}

impl Camera {
    /// Reads the camera of a scene, falling back to the convention field by field.
    ///
    /// # Invariants
    ///
    /// A scene with no camera is not an error. Every field is read against the convention
    /// rather than against a default, so a camera that names only an eye is a camera that
    /// looks at the middle of what is drawn.
    pub fn read(scene: &Value, bounds: &Bounds, aspect: f64) -> Result<Self, CompileError> {
        let path = Path::root().key(CAMERA);
        let convention = Self::convention(bounds, aspect);
        let Some(camera) = field(scene, CAMERA) else {
            return Ok(convention);
        };
        let read = |name: &str| field(camera, name);
        let kind = match read(PROJECTION).map(Value::variant_tag) {
            Some(Some("Perspective")) => Kind::Perspective,
            Some(Some("Orthographic")) => Kind::Orthographic,
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
            kind: Kind::Orthographic,
            padding: FRAMING_PADDING,
            aspect,
            bounds: *bounds,
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
            Kind::Orthographic => {
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
                }
            }
            Kind::Perspective => {
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
                let view = look_at(self.eye, self.target, self.up);
                let (right, up, _) = basis(&view);
                let forward = scale(direction, -1.0);
                let distance = (self.fitting_distance(forward, right, up, vertical_half)
                    * self.padding)
                    .max(1e-3);
                let eye = add(self.target, scale(direction, distance));
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
                let nearest = (nearest - margin).max(distance * 0.002);
                Projection {
                    matrix: multiply(
                        &perspective(self.field_of_view, self.aspect, nearest, farthest + margin),
                        &view,
                    ),
                    eye,
                    forward,
                    right,
                    up,
                    // The vertical half angle of the view, and the horizontal one widened
                    // by the aspect: a wide frame is wider rather than cropped.
                    half_extent: [vertical_half * self.aspect, vertical_half],
                }
            }
        }
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
