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
    pub(crate) radius: f64,
}

/// A projection, and the eye it was made from.
pub struct Projection {
    /// The matrix world space is drawn through.
    pub matrix: Matrix,
    /// Where the projection was made from, which may not be where the camera asked to be.
    pub eye: [f64; 3],
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
            radius: bounds.largest_half_width(),
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
            radius: bounds.largest_half_width(),
        }
    }

    /// Returns the projection the camera draws the scene through.
    pub fn projection(&self) -> Projection {
        let direction = normalize(subtract(self.eye, self.target));
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
                let half_extent = 2.0 * self.radius * self.padding;
                let depth = distance + 2.0 * self.radius;
                Projection {
                    matrix: multiply(
                        &orthographic(half_extent.max(1e-3), depth / 4.0, depth),
                        &look_at(self.eye, self.target, self.up),
                    ),
                    eye: self.eye,
                }
            }
            Kind::Perspective => {
                // The eye is pushed back until a sphere around the scene fills the field of
                // view, because a scene that overflows the frame is a scene that is
                // cropped. The eye the author wrote therefore sets the direction of the
                // view and not its distance, and the eye the pass carries is the one the
                // projection was actually made from.
                let radius = (self.radius * self.padding).max(1e-3);
                let distance = (radius / (self.field_of_view / 2.0).tan().max(1e-3)).max(1e-3);
                let eye = add(self.target, scale(direction, distance));
                Projection {
                    matrix: multiply(
                        &perspective(
                            self.field_of_view,
                            self.aspect,
                            (distance - radius).max(1e-4),
                            distance + radius,
                        ),
                        &look_at(eye, self.target, self.up),
                    ),
                    eye,
                }
            }
        }
    }
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
