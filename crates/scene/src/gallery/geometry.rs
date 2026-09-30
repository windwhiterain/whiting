//! Geometry for the gallery, generated rather than authored.
//!
//! Every mesh is a run of points against a point schema, so what belongs here is the
//! arithmetic of a shape and what belongs in [`crate::gallery::kit`] is the document it is
//! wrapped in. A heightfield, a sphere, a cone: each is a few lines of maths, and a scene
//! is a few lines of placement.
//!
//! # Invariants
//!
//! - A generator fills the fields of the point schema it was given, in the order the
//!   schema sorts them, and no others. A point carrying a field the schema does not
//!   declare is a compile error rather than a buffer that is silently short.
//! - Normals are derived from the surface rather than supplied, so a heightfield's normal
//!   is perpendicular to the surface it actually has. A normal that disagrees with the
//!   geometry is the single most common way a lit scene stops reading as lit.

use definition::{FieldMap, Value};

/// Builds one shaded point from a position, a normal and a texture coordinate.
/// A three component point field, as a document value.
fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Builds one shaded point from a position, a normal and a texture coordinate.
pub fn shaded_point(position: [f64; 3], normal: [f64; 3], texture: [f64; 2]) -> Value {
    let point: FieldMap = [
        ("normal".to_owned(), triple(normal)),
        ("position".to_owned(), triple(position)),
        (
            "texture".to_owned(),
            Value::List(texture.into_iter().map(Value::Float).collect()),
        ),
    ]
    .into_iter()
    .collect();
    Value::structure(point)
}

/// Builds one point carrying a position alone.
pub fn positioned_point(position: [f64; 3]) -> Value {
    Value::structure([("position".to_owned(), triple(position))])
}

/// Subtracts one vector from another.
pub fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// Adds two vectors.
pub fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// Scales a vector.
pub fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
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

/// Returns a unit vector, falling back to the unit y axis for one with no length.
pub fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let length = dot(vector, vector).sqrt();
    if length > f64::EPSILON {
        scale(vector, 1.0 / length)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Returns a vector's length.
pub fn length(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}

/// Returns a deterministic value in zero to one from two integers and a seed.
///
/// # Invariants
///
/// The same arguments always give the same value. A scene that places a thousand things
/// has to give the same thousand things the same values every time it is compiled, or an
/// unchanged scene would produce a different frame and the delta between the two outputs
/// would be the whole document.
pub fn hash01(x: i64, y: i64, seed: i64) -> f64 {
    // A large odd multiplier and a bit mix, which is enough to spread lattice points
    // apart. The result is folded into zero to one, where it is used.
    let mut value = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (seed as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    (value >> 11) as f64 / (1u64 << 53) as f64
}

/// Returns a smooth two dimensional value noise, in zero to one.
///
/// # Invariants
///
/// The value is zero at every lattice point and one at the centre of every cell, and it
/// is smooth across a cell boundary, so a surface sampled from it has no creases where
/// two cells meet. A hash used directly would put a discontinuity at every lattice point,
/// and a heightfield built from one is a field of spikes.
pub fn value_noise(point: [f64; 2], seed: i64) -> f64 {
    let cell = [point[0].floor(), point[1].floor()];
    let within = [point[0] - cell[0], point[1] - cell[1]];
    // A smoothstep on each axis, so the value is flat at a lattice point and flat at the
    // centre of a cell and only turns in between.
    let fade = |t: f64| t * t * (3.0 - 2.0 * t);
    let (u, v) = (fade(within[0]), fade(within[1]));
    let corner = |dx: i64, dy: i64| hash01(cell[0] as i64 + dx, cell[1] as i64 + dy, seed);
    let near = lerp(corner(0, 0), corner(1, 0), u);
    let far = lerp(corner(0, 1), corner(1, 1), u);
    lerp(near, far, v)
}

/// Returns noise summed over several octaves, in roughly zero to one.
///
/// Each octave is twice the frequency and a third again the amplitude of the one below,
/// which is the ratio a surface stays recognisable at: one octave is a shape, four is a
/// shape with weather on it.
pub fn fractal_noise(point: [f64; 2], octaves: usize, seed: i64) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for octave in 0..octaves {
        value = value
            + amplitude
                * value_noise(
                    [point[0] * frequency, point[1] * frequency],
                    seed + octave as i64,
                );
        total += amplitude;
        amplitude /= 3.0;
        frequency *= 2.0;
    }
    value / total
}

/// Interpolates between two numbers.
pub fn lerp(from: f64, to: f64, amount: f64) -> f64 {
    from + (to - from) * amount
}

/// Returns a number wrapped into zero to one.
pub fn fract(value: f64) -> f64 {
    value - value.floor()
}

/// Builds a heightfield as a grid of shaded points.
///
/// # Invariants
///
/// The normal of each point is taken from the heightfield's own gradient, so it is
/// perpendicular to the surface rather than to the ground. A terrain lit by the slope of
/// the ground rather than the slope of the terrain has no form at all.
///
/// `height` is called once per grid point, and the four neighbours of that point are read
/// from the same function, so a caller whose height depends on anything but its argument
/// produces a surface whose normals disagree with its geometry.
pub fn heightfield(divisions: usize, extent: f64, height: impl Fn(f64, f64) -> f64) -> Vec<Value> {
    let step = 2.0 * extent / divisions as f64;
    let origin = -extent;
    let mut points = Vec::with_capacity(divisions * divisions * 6);

    for row in 0..divisions {
        for column in 0..divisions {
            let (x, z) = (origin + column as f64 * step, origin + row as f64 * step);
            // The gradient, by the difference either side along each axis. A heightfield
            // that is sampled on a grid is smooth enough that the difference over one
            // step is its slope.
            let (dx, dz) = (step * 0.5, step * 0.5);
            let slope_x = (height(x + dx, z) - height(x - dx, z)) / (2.0 * dx);
            let slope_z = (height(x, z + dz) - height(x, z - dz)) / (2.0 * dz);
            // The normal of a surface whose height rises by one per unit of x and z.
            let normal = normalized([-slope_x, 1.0, -slope_z]);
            let position = [x, height(x, z), z];
            let texture = [
                column as f64 / divisions as f64,
                row as f64 / divisions as f64,
            ];

            // Two triangles per cell, wound counter clockwise seen from above, so the
            // surface faces up and the winding the rasterizer wants is the winding that
            // makes it visible.
            let right = [x + step, 0.0, z];
            let up = [x, 0.0, z + step];
            let corner = [x + step, 0.0, z + step];
            let lift = |corner: [f64; 3]| [corner[0], height(corner[0], corner[2]), corner[2]];
            points.push(shaded_point(position, normal, texture));
            points.push(shaded_point(
                lift(right),
                normal,
                [texture[0] + 0.5, texture[1]],
            ));
            points.push(shaded_point(
                lift(up),
                normal,
                [texture[0], texture[1] + 0.5],
            ));
            points.push(shaded_point(
                lift(right),
                normal,
                [texture[0] + 0.5, texture[1]],
            ));
            points.push(shaded_point(
                lift(corner),
                normal,
                [texture[0] + 0.5, texture[1] + 0.5],
            ));
            points.push(shaded_point(
                lift(up),
                normal,
                [texture[0], texture[1] + 0.5],
            ));
        }
    }
    points
}

/// Builds a sphere as a stack of triangles, as shaded points.
///
/// # Invariants
///
/// The normal of a point is the direction it sits in from the centre, so a sphere lit by
/// its own normals is round rather than faceted whatever the tessellation. The winding is
/// counter clockwise seen from outside, so the surface faces the viewer.
pub fn sphere(divisions: usize, radius: f64) -> Vec<Value> {
    let mut points = Vec::with_capacity(divisions * divisions * 6);
    let vertex = |theta: f64, phi: f64| -> ([f64; 3], [f64; 2]) {
        let normal = [theta.sin() * phi.cos(), phi.sin(), theta.cos() * phi.cos()];
        (
            scale(normal, radius),
            [theta / std::f64::consts::TAU, phi / std::f64::consts::PI],
        )
    };
    for row in 0..divisions {
        for column in 0..divisions {
            let (t0, t1) = (
                column as f64 / divisions as f64 * std::f64::consts::TAU,
                (column + 1) as f64 / divisions as f64 * std::f64::consts::TAU,
            );
            let (p0, p1) = (
                row as f64 / divisions as f64 * std::f64::consts::PI,
                (row + 1) as f64 / divisions as f64 * std::f64::consts::PI,
            );
            let corners = [
                vertex(t0, p0),
                vertex(t1, p0),
                vertex(t1, p1),
                vertex(t0, p1),
            ];
            for index in [0usize, 1, 2, 0, 2, 3] {
                let (position, texture) = corners[index];
                points.push(shaded_point(position, normalized(position), texture));
            }
        }
    }
    points
}

/// Builds a cone standing on the ground, tapering to a point above it.
///
/// The base is a regular polygon of `sides` faces and the side is a fan from the apex, so
/// a cone costs `sides` quads rather than a tessellated surface.
pub fn cone(sides: usize, radius: f64, height: f64) -> Vec<Value> {
    let mut points = Vec::with_capacity(sides * 6);
    let apex = [0.0, height, 0.0];
    let rim = |index: usize| {
        let angle = index as f64 / sides as f64 * std::f64::consts::TAU;
        [angle.cos() * radius, 0.0, angle.sin() * radius]
    };
    // The side of a cone leans outwards, so its normal is perpendicular to the slant and
    // halfway between the two rim directions it joins.
    for index in 0..sides {
        let (a, b) = (rim(index), rim(index + 1));
        let slope = [a[0] + b[0], height * 2.0, a[2] + b[2]];
        let normal = normalized(cross(slope, subtract(b, a)));
        for corner in [a, b, apex] {
            points.push(shaded_point(
                corner,
                normal,
                [corner[0] * 0.5 + 0.5, corner[2] * 0.5 + 0.5],
            ));
        }
    }
    points
}

/// Builds a run of line segments, as positions alone.
pub fn segments(pairs: impl IntoIterator<Item = ([f64; 3], [f64; 3])>) -> Vec<Value> {
    let mut points = Vec::new();
    for (from, to) in pairs {
        points.push(positioned_point(from));
        points.push(positioned_point(to));
    }
    points
}
