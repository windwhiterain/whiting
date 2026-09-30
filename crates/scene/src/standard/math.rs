//! The arithmetic a projection is made of.
//!
//! A projection is data: the pass carries it, and the shader reads it. What a compiler
//! has to decide is which box the projection covers and where it looks from, and those two
//! decisions are stated where they are used rather than hidden here.
//!
//! Matrices are column major, sixteen floats, which is the order `wgpu` and `WGSL` both
//! read, and the depth range is zero to one.
//!
//! # Example
//!
//! ```
//! use whiting_scene::standard::math::{Bounds, apply, look_at, projection_of};
//!
//! // A point at the half width lands on the edge of the frame, on the side it is on, and
//! // the near and far planes land at depth zero and depth one.
//! let projection = whiting_scene::standard::math::orthographic(2.0, 1.0, 10.0);
//! let near = apply(&projection, [2.0, 2.0, -1.0]);
//! let far = apply(&projection, [0.0, 0.0, -10.0]);
//! assert!((near[0] - 1.0).abs() < 1e-9 && (near[1] - 1.0).abs() < 1e-9);
//! assert!((near[2]).abs() < 1e-9, "the near plane is at depth zero");
//! assert!((far[2] - 1.0).abs() < 1e-9, "the far plane is at depth one");
//!
//! // A view from above the origin sees the origin in the middle of the frame, at a
//! // depth inside the range, which is what makes the projection cover the box.
//! let view = projection_of(
//!     &Bounds::around([0.0, 0.0, 0.0], 4.0),
//!     [0.0, 0.0, 10.0],
//!     [0.0, 0.0, 0.0],
//!     1.1,
//! );
//! let centre = apply(&view, [0.0, 0.0, 0.0]);
//! assert!(centre[0].abs() < 1e-9 && centre[1].abs() < 1e-9);
//! assert!((0.0..=1.0).contains(&centre[2]), "the centre is inside the depth range");
//!
//! // A view is not mirrored: positive x in view space stays positive in clip space.
//! let corner = look_at([0.0, 0.0, 10.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
//! assert!(apply(&corner, [1.0, 0.0, 0.0])[0] > 0.0);
//!
//! // A perspective projection puts the near plane at depth zero and the far plane at depth
//! // one, and a point at the half width of the near plane lands on the edge of the frame.
//! let projection = whiting_scene::standard::math::perspective(
//!     std::f64::consts::FRAC_PI_2, 1.0, 1.0, 11.0,
//! );
//! let near = apply(&projection, [1.0, 1.0, -1.0]);
//! let far = apply(&projection, [0.0, 0.0, -11.0]);
//! assert!((near[2] / near[3]).abs() < 1e-9, "the near plane is at depth zero");
//! assert!(((far[2] / far[3]) - 1.0).abs() < 1e-9, "the far plane is at depth one");
//! assert!((near[0] / near[3] - 1.0).abs() < 1e-9, "the half width is the edge of the frame");
//! ```
//!
//! A perspective projection is a divide, so `apply` returns clip space: the depth is `z` over `w`,
//! and `w` is how far the point is in front of the camera. That is what makes a nearer point larger
//! than a further one of the same size, and it is the whole of what separates a perspective scene
//! from a parallel one.

/// A four by four matrix, column major.
pub type Matrix = [f64; 16];

/// An axis aligned box, in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The smallest corner.
    pub minimum: [f64; 3],
    /// The largest corner.
    pub maximum: [f64; 3],
}

impl Bounds {
    /// Returns the centre of the box.
    pub fn centre(&self) -> [f64; 3] {
        [
            (self.minimum[0] + self.maximum[0]) / 2.0,
            (self.minimum[1] + self.maximum[1]) / 2.0,
            (self.minimum[2] + self.maximum[2]) / 2.0,
        ]
    }

    /// Returns the half width of the box along an axis.
    pub fn half_width(&self, axis: usize) -> f64 {
        ((self.maximum[axis] - self.minimum[axis]) / 2.0).abs()
    }

    /// Returns the half width of the box, which is the largest of the three.
    pub fn largest_half_width(&self) -> f64 {
        (0..3)
            .map(|axis| self.half_width(axis))
            .fold(0.0f64, f64::max)
    }

    /// Returns a box around `centre` with the given half width, never degenerate.
    pub fn around(centre: [f64; 3], half_width: f64) -> Self {
        let half_width = if half_width > 0.0 { half_width } else { 1.0 };
        Self {
            minimum: [
                centre[0] - half_width,
                centre[1] - half_width,
                centre[2] - half_width,
            ],
            maximum: [
                centre[0] + half_width,
                centre[1] + half_width,
                centre[2] + half_width,
            ],
        }
    }

    /// Returns the smallest box holding both.
    pub fn union(self, other: Self) -> Self {
        Self {
            minimum: [
                self.minimum[0].min(other.minimum[0]),
                self.minimum[1].min(other.minimum[1]),
                self.minimum[2].min(other.minimum[2]),
            ],
            maximum: [
                self.maximum[0].max(other.maximum[0]),
                self.maximum[1].max(other.maximum[1]),
                self.maximum[2].max(other.maximum[2]),
            ],
        }
    }
}

/// Returns an orthographic projection of `box` looked at from `eye` towards `target`.
///
/// The box is fitted to the view, so whatever the scene's extent is, the projection
/// covers all of it. `padding` widens the box, which keeps geometry exactly on the edge
/// from being clipped by a rounding error. The near and far planes are derived from how
/// far the camera is from the box, so the whole box is between them.
pub fn projection_of(box_of: &Bounds, eye: [f64; 3], target: [f64; 3], padding: f64) -> Matrix {
    let distance = dot(subtract(eye, target), subtract(eye, target)).sqrt();
    let depth = distance + 2.0 * box_of.largest_half_width();
    let view = look_at(eye, target, [0.0, 1.0, 0.0]);
    let ortho = orthographic(
        2.0 * box_of.largest_half_width() * padding,
        depth / 4.0,
        depth,
    );
    multiply(&ortho, &view)
}

/// Returns a perspective projection of `box` looked at from `eye` towards `target`.
///
/// # Invariants
///
/// - The eye is pushed back along its own direction until the box fits inside the field
///   of view, because a box that overflows the frame is a scene that is cropped rather
///   than a scene that is drawn. The eye the caller asked for therefore sets the
///   *direction* of the view, and the distance is the smallest that frames the box.
/// - The near and far planes are placed one box radius outside it. A perspective
///   projection maps depth with a divide, so a near plane inside the box clips geometry
///   the box says is visible and a far plane short of the back clips the far wall.
/// - `vertical_field_of_view` is the full angle at the vertical, in radians, and the
///   horizontal follows from `aspect`, so a wide frame is wider rather than cropped.
pub fn perspective_of(
    box_of: &Bounds,
    eye: [f64; 3],
    target: [f64; 3],
    vertical_field_of_view: f64,
    aspect: f64,
    padding: f64,
) -> Matrix {
    let radius = box_of.largest_half_width() * padding;
    let direction = normalize(subtract(eye, target));
    // The distance at which a sphere of `radius` exactly fills the vertical field.
    let fitting = radius / (vertical_field_of_view / 2.0).tan().max(1e-3);
    let fitted = fitting.max(1e-3);
    multiply(
        &perspective(
            vertical_field_of_view,
            aspect,
            (fitted - radius).max(1e-4),
            fitted + radius,
        ),
        &look_at(
            add(target, scale(direction, fitted)),
            target,
            [0.0, 1.0, 0.0],
        ),
    )
}

/// Returns the matrix that transforms world space into the space the camera is in.
pub fn look_at(eye: [f64; 3], target: [f64; 3], up: [f64; 3]) -> Matrix {
    // The camera looks down its own negative z axis, so the basis is built from the
    // direction back towards the eye.
    let back = normalize(subtract(eye, target));
    let right = normalize(cross(up, back));
    let up = cross(back, right);

    [
        right[0],
        up[0],
        back[0],
        0.0,
        right[1],
        up[1],
        back[1],
        0.0,
        right[2],
        up[2],
        back[2],
        0.0,
        -dot(right, eye),
        -dot(up, eye),
        -dot(back, eye),
        1.0,
    ]
}

/// Returns an orthographic projection of half width `half_width`, from `near` to `far`.
///
/// # Invariants
///
/// The camera looks down its own negative z, so a point `d` in front of it sits at
/// `z == -d`. This puts `d == near` at depth zero and `d == far` at depth one, and maps a
/// point at `half_width` to the edge of the frame on the same side it is on. The doctest
/// in the module is what holds all three true.
pub fn orthographic(half_width: f64, near: f64, far: f64) -> Matrix {
    let half_width = if half_width > 0.0 { half_width } else { 1.0 };
    let (near, far) = (near.max(1e-6), far.max(near + 1e-6));
    let a = 1.0 / (near - far);
    [
        1.0 / half_width,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / half_width,
        0.0,
        0.0,
        0.0,
        0.0,
        a,
        0.0,
        0.0,
        0.0,
        near * a,
        1.0,
    ]
}

/// Returns the product of two column major matrices.
pub fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    let mut product = [0.0f64; 16];
    for column in 0..4 {
        for row in 0..4 {
            product[column * 4 + row] = (0..4)
                .map(|inner| left[inner * 4 + row] * right[column * 4 + inner])
                .sum();
        }
    }
    product
}

/// Returns the model matrix of a transform: scale, then turn, then move.
///
/// # Example
///
/// ```
/// use whiting_scene::standard::{math::apply, transform_matrix};
///
/// // A translation moves the origin and leaves a direction alone.
/// let moved = transform_matrix([1.0, 2.0, 3.0], [0.0; 3], [1.0; 3]);
/// assert_eq!(apply(&moved, [0.0; 3]), [1.0, 2.0, 3.0, 1.0]);
///
/// // A quarter turn about z takes x to y.
/// let turned = transform_matrix([0.0; 3], [0.0, 0.0, std::f64::consts::FRAC_PI_2], [1.0; 3]);
/// let point = apply(&turned, [1.0, 0.0, 0.0]);
/// assert!((point[0]).abs() < 1e-12 && (point[1] - 1.0).abs() < 1e-12);
///
/// // Scale is applied first, so it does not scale the translation.
/// let scaled = transform_matrix([1.0, 0.0, 0.0], [0.0; 3], [2.0; 3]);
/// assert_eq!(apply(&scaled, [1.0, 0.0, 0.0]), [3.0, 0.0, 0.0, 1.0]);
/// ```
pub fn transform_matrix(translation: [f64; 3], rotation: [f64; 3], scale: [f64; 3]) -> Matrix {
    let (sin_x, cos_x) = rotation[0].sin_cos();
    let (sin_y, cos_y) = rotation[1].sin_cos();
    let (sin_z, cos_z) = rotation[2].sin_cos();

    // Column major, so the first four entries are the image of the x axis.
    let x = [
        cos_y * cos_z,
        cos_x * sin_z + sin_x * sin_y * cos_z,
        sin_x * sin_z - cos_x * sin_y * cos_z,
        0.0,
    ];
    let y = [
        -cos_y * sin_z,
        cos_x * cos_z - sin_x * sin_y * sin_z,
        sin_x * cos_z + cos_x * sin_y * sin_z,
        0.0,
    ];
    let z = [sin_y, -sin_x * cos_y, cos_x * cos_y, 0.0];
    [
        x[0] * scale[0],
        x[1] * scale[0],
        x[2] * scale[0],
        0.0,
        y[0] * scale[1],
        y[1] * scale[1],
        y[2] * scale[1],
        0.0,
        z[0] * scale[2],
        z[1] * scale[2],
        z[2] * scale[2],
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

/// Applies a matrix to a point, which is the point in clip space it maps to.
pub fn apply(matrix: &Matrix, point: [f64; 3]) -> [f64; 4] {
    [
        matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12],
        matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13],
        matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14],
        matrix[3] * point[0] + matrix[7] * point[1] + matrix[11] * point[2] + matrix[15],
    ]
}

/// Returns a perspective projection of a `vertical_field_of_view`, from `near` to `far`.
///
/// # Invariants
///
/// The camera looks down its own negative z, so a point `d` in front of it sits at
/// `z == -d`. This puts `d == near` at depth zero and `d == far` at depth one, and maps a
/// point at the half width of the near plane to the edge of the frame on the same side it
/// is on. The doctest in the module is what holds all three true.
pub fn perspective(vertical_field_of_view: f64, aspect: f64, near: f64, far: f64) -> Matrix {
    // A field of view at or beyond half a turn makes the divide singular and projects
    // everything to infinity, so the half angle is clamped to a range a lens can produce.
    let half_angle = (vertical_field_of_view / 2.0).clamp(1e-3, std::f64::consts::FRAC_PI_2 - 1e-3);
    let (aspect, near, far) = (aspect.max(1e-3), near.max(1e-4), far.max(near + 1e-4));
    let focal = 1.0 / half_angle.tan();
    [
        focal / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        focal,
        0.0,
        0.0,
        0.0,
        0.0,
        far / (near - far),
        -1.0,
        0.0,
        0.0,
        (near * far) / (near - far),
        0.0,
    ]
}

/// Adds two vectors.
pub fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// Scales a vector.
pub fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

/// Subtracts one vector from another.
pub fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// Returns the cross product of two vectors.
pub fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Returns the dot product of two vectors.
pub fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// Returns a unit vector, falling back to a unit x vector for a zero length one.
pub fn normalize(vector: [f64; 3]) -> [f64; 3] {
    let length = dot(vector, vector).sqrt();
    if length == 0.0 {
        return [1.0, 0.0, 0.0];
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}
