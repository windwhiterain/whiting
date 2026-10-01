//! Geometry for the gallery, generated rather than authored.
//!
//! Every mesh is a run of points against a point schema, so what belongs here is the
//! arithmetic of a shape and what belongs in [`crate::kit`] is the document it is
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

use definition::{FieldMap, PackedBuilder, PackedLayout, PackedList, Value};

/// Field a shaded point's normal is written under.
const NORMAL: &str = "normal";
/// Field a shaded point's position is written under.
const POSITION: &str = "position";
/// Field a shaded point's texture coordinate is written under.
const TEXTURE: &str = "texture";

/// The layout a shaded mesh's points are written with, which is the run the generators below
/// build.
///
/// # Invariants
///
/// The fields are in the order [`shaded_point_schema`] sorts them into, which is the order a
/// buffer lays them out in, so a point written through this builder is already the bytes a vertex
/// buffer wants and packing is a copy rather than a walk.
fn shaded_layout() -> PackedLayout {
    PackedLayout::new([
        (NORMAL.to_owned(), 3),
        (POSITION.to_owned(), 3),
        (TEXTURE.to_owned(), 2),
    ])
}

/// A run of shaded points, built without ever building a point as a value.
struct Shaded {
    points: PackedBuilder,
}

impl Shaded {
    /// Creates a run of shaded points.
    fn with_capacity(count: usize) -> Self {
        let mut points = PackedBuilder::new(shaded_layout());
        points.reserve(count);
        Self { points }
    }

    /// Appends one shaded point.
    ///
    /// # Invariants
    ///
    /// The fields are written in layout order with the literal widths the layout declares, so
    /// this cannot fail: a refusal here is a bug in the generator rather than a document to
    /// report, and it is reported as one rather than swallowed.
    fn push(&mut self, position: [f64; 3], normal: [f64; 3], texture: [f64; 2]) {
        let written = self
            .points
            .field(NORMAL, &normal)
            .and_then(|points| points.field(POSITION, &position))
            .and_then(|points| points.field(TEXTURE, &texture));
        written.expect("a generator writes whole elements of its own layout");
    }

    /// Ends the run.
    fn finish(self) -> PackedList {
        self.points
            .finish()
            .expect("every generator writes whole elements of its own layout")
    }
}

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

/// Returns creased noise summed over several octaves, in roughly zero to one.
///
/// Each octave is folded over its midpoint before it is summed, so the result is crests
/// and gullies rather than swells: smooth noise makes domes whatever its amplitude, and
/// a folded noise makes ridge lines, which is what a mountain range's silhouette is.
pub fn ridged_noise(point: [f64; 2], octaves: usize, seed: i64) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for octave in 0..octaves {
        let noise = value_noise(
            [point[0] * frequency, point[1] * frequency],
            seed + octave as i64,
        );
        value += amplitude * (1.0 - (2.0 * noise - 1.0).abs());
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

/// Builds a heightfield as a grid of shaded points, packed.
///
/// # Invariants
///
/// The normal of every point is taken from the heightfield's own gradient at that point,
/// so it is perpendicular to the surface rather than to the cell it happens to be drawn
/// in. A normal per cell shades a smooth hillside as tiled facets, and a terrace riser —
/// a slope a cell wide — as a stripe of alternating triangles, which is the single most
/// common way a lit scene stops reading as lit.
///
/// `height` is called for each grid point and for the four neighbours of that point, so
/// a caller whose height depends on anything but its argument produces a surface whose
/// normals disagree with its geometry.
pub fn heightfield(divisions: usize, extent: f64, height: impl Fn(f64, f64) -> f64) -> PackedList {
    let step = 2.0 * extent / divisions as f64;
    let origin = -extent;
    let mut points = Shaded::with_capacity(divisions * divisions * 6);

    // The position, normal and texture coordinate of one grid point. The gradient is
    // the difference either side along each axis; a heightfield sampled on a grid is
    // smooth enough that the difference over one step is its slope.
    let at = |column: usize, row: usize| {
        let (x, z) = (origin + column as f64 * step, origin + row as f64 * step);
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
        (position, normal, texture)
    };

    for row in 0..divisions {
        for column in 0..divisions {
            let (p00, n00, t00) = at(column, row);
            let (p10, n10, t10) = at(column + 1, row);
            let (p01, n01, t01) = at(column, row + 1);
            let (p11, n11, t11) = at(column + 1, row + 1);
            // Two triangles per cell, wound counter clockwise seen from above, so the
            // surface faces up and the winding the rasterizer wants is the winding that
            // makes it visible.
            points.push(p00, n00, t00);
            points.push(p10, n10, t10);
            points.push(p01, n01, t01);
            points.push(p10, n10, t10);
            points.push(p11, n11, t11);
            points.push(p01, n01, t01);
        }
    }
    points.finish()
}

/// Builds a sphere as a stack of triangles, as packed shaded points.
///
/// # Invariants
///
/// The normal of a point is the direction it sits in from the centre, so a sphere lit by
/// its own normals is round rather than faceted whatever the tessellation. The winding is
/// counter clockwise seen from outside, so the surface faces the viewer.
pub fn sphere(divisions: usize, radius: f64) -> PackedList {
    let mut points = Shaded::with_capacity(divisions * divisions * 6);
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
                points.push(position, normalized(position), texture);
            }
        }
    }
    points.finish()
}

/// Builds a surface of revolution about the **y** axis: a profile, turned.
///
/// # Invariants
///
/// - The profile is a polyline of `(radius, height)` pairs read in order, and the surface
///   between two consecutive entries is one band of quads. A profile that begins and ends on
///   the axis — a radius of zero — closes the shape, and that is how a hollow object is one
///   mesh: a bowl is a profile that goes out along its underside, up its outside, over its
///   rim, back down its inside and in along the inside of its base. Its two surfaces are one
///   run of points and there is no second mesh to keep in step with the first.
/// - The normal is taken from the **profile's own tangent** and not from the triangles the
///   surface is drawn as, so a coarse division shades as a smooth solid. Normals per facet
///   make a turned shape read as a polygon however many segments it is given, which is the
///   one thing a turned shape cannot look like.
/// - The texture coordinate runs one turn across the profile and its own arc length up it, so
///   a pattern sampled from it has no seam at the join and no stretch where the profile turns.
///
/// A profile with fewer than two entries, or with no segments, is an empty run: a shape with
/// no surface is drawn as nothing rather than refused, because a scene that generated one
/// generated it from its own numbers.
pub fn lathe(profile: &[[f64; 2]], segments: usize) -> PackedList {
    let mut points = Shaded::with_capacity(profile.len().saturating_mul(segments) * 6);
    if profile.len() < 2 || segments == 0 {
        return points.finish();
    }

    // The arc length up the profile, so a texture does not slide where the profile turns a
    // corner, and the tangent either side of each entry, so a vertex on the join of two bands
    // is shaded as the average of them rather than as one of them.
    let mut arc = Vec::with_capacity(profile.len());
    let mut total = 0.0;
    arc.push(0.0);
    for pair in profile.windows(2) {
        let (dr, dy) = (pair[1][0] - pair[0][0], pair[1][1] - pair[0][1]);
        total += (dr * dr + dy * dy).sqrt();
        arc.push(total);
    }
    let height_at = |index: usize| arc[index] / total.max(1e-9);

    // The outward normal of a revolve at one entry of the profile.
    //
    // The surface is `(r cosθ, y, r sinθ)`, so its tangent along the profile is
    // `(dr cosθ, dy, dr sinθ)` and its tangent around the axis is `(-sinθ, 0, cosθ)`. The
    // cross product of the second into the first is `(-dy cosθ, dr, -dy sinθ)`, which points
    // **in**, so what is returned is its negation. A profile that rises as it widens — the
    // outside of a bowl — then has a normal that leans out and down, which is where the
    // surface is.
    let normal_at = |index: usize, angle: f64| -> [f64; 3] {
        let (mut dr, mut dy) = (0.0, 0.0);
        if index + 1 < profile.len() {
            dr += profile[index + 1][0] - profile[index][0];
            dy += profile[index + 1][1] - profile[index][1];
        }
        if index > 0 {
            dr += profile[index][0] - profile[index - 1][0];
            dy += profile[index][1] - profile[index - 1][1];
        }
        normalized([dy * angle.cos(), -dr, dy * angle.sin()])
    };
    let at = |index: usize, angle: f64| -> ([f64; 3], [f64; 3], [f64; 2]) {
        let (radius, height) = (profile[index][0], profile[index][1]);
        (
            [radius * angle.cos(), height, radius * angle.sin()],
            normal_at(index, angle),
            [angle / std::f64::consts::TAU, height_at(index)],
        )
    };

    for segment in 0..segments {
        let (from, to) = (
            segment as f64 / segments as f64 * std::f64::consts::TAU,
            (segment + 1) as f64 / segments as f64 * std::f64::consts::TAU,
        );
        for band in 0..profile.len() - 1 {
            let corners = [
                at(band, from),
                at(band + 1, from),
                at(band + 1, to),
                at(band, to),
            ];
            // Wound counter clockwise seen from outside the shape, so the surface faces away
            // from the axis it was turned about.
            for index in [0usize, 1, 2, 0, 2, 3] {
                let (position, normal, texture) = corners[index];
                points.push(position, normal, texture);
            }
        }
    }
    points.finish()
}

/// Builds a cone standing on the ground, tapering to a point above it.
///
/// The base is a regular polygon of `sides` faces and the side is a fan from the apex, so
/// a cone costs `sides` quads rather than a tessellated surface.
pub fn cone(sides: usize, radius: f64, height: f64) -> PackedList {
    let mut points = Shaded::with_capacity(sides * 6);
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
            points.push(
                corner,
                normal,
                [corner[0] * 0.5 + 0.5, corner[2] * 0.5 + 0.5],
            );
        }
    }
    points.finish()
}

/// Builds a run of line segments, as packed positions alone.
pub fn segments(pairs: impl IntoIterator<Item = ([f64; 3], [f64; 3])>) -> PackedList {
    let mut builder = PackedBuilder::new(PackedLayout::new([(POSITION.to_owned(), 3)]));
    for (from, to) in pairs {
        builder
            .field(POSITION, &from)
            .and_then(|builder| builder.field(POSITION, &to))
            .expect("a segment writes one whole position");
    }
    builder
        .finish()
        .expect("every segment writes one whole position")
}

/// Appends one quad to a run of shaded points, as two triangles.
///
/// # Invariants
///
/// The quad is wound counter clockwise seen from the side `u` cross `v` points to, so the
/// normal is derived from the edges rather than supplied: a face whose corners are given
/// in the wrong order is a face whose normal is computed from the wrong order, not a
/// silent inward surface.
fn quad(points: &mut Shaded, corner: [f64; 3], u: [f64; 3], v: [f64; 3]) {
    let normal = normalized(cross(u, v));
    let far = add(add(corner, u), v);
    // The texture coordinate runs along the two edges, so a face is a unit square of
    // texture whatever its shape: nothing samples it yet, but a point schema carries one
    // and a zero is a lie about where on the face the point is.
    points.push(corner, normal, [0.0, 0.0]);
    points.push(add(corner, u), normal, [1.0, 0.0]);
    points.push(far, normal, [1.0, 1.0]);
    points.push(corner, normal, [0.0, 0.0]);
    points.push(far, normal, [1.0, 1.0]);
    points.push(add(corner, v), normal, [0.0, 1.0]);
}

/// Builds a box standing on the ground, centred on its footprint.
///
/// The base is left open, as the cone's is: a box is stood on terrain and the face
/// underneath is never seen. A box that floats is placed by its instance, and the open
/// base is still inside whatever it meets.
pub fn cuboid(width: f64, height: f64, depth: f64) -> PackedList {
    let mut points = Shaded::with_capacity(30);
    let (w, d) = (width * 0.5, depth * 0.5);
    // The four walls, each a quad whose two edges cross to its outward normal.
    quad(
        &mut points,
        [w, 0.0, -d],
        [0.0, height, 0.0],
        [0.0, 0.0, 2.0 * d],
    );
    quad(
        &mut points,
        [-w, 0.0, d],
        [0.0, height, 0.0],
        [0.0, 0.0, -2.0 * d],
    );
    quad(
        &mut points,
        [-w, 0.0, d],
        [2.0 * w, 0.0, 0.0],
        [0.0, height, 0.0],
    );
    quad(
        &mut points,
        [w, 0.0, -d],
        [-2.0 * w, 0.0, 0.0],
        [0.0, height, 0.0],
    );
    quad(
        &mut points,
        [-w, height, -d],
        [0.0, 0.0, 2.0 * d],
        [2.0 * w, 0.0, 0.0],
    );
    points.finish()
}

/// Builds a gable roof: a triangular prism with its ridge along the depth axis.
///
/// The eaves sit on the ground at the two sides and the ridge rises to `height` along
/// the centre line, which is the shape a tiled roof has over the walls a [`cuboid`]
/// makes. The underside is open, so the roof can sit a touch into the walls it caps.
pub fn prism(width: f64, height: f64, depth: f64) -> PackedList {
    let mut points = Shaded::with_capacity(18);
    let (w, d) = (width * 0.5, depth * 0.5);
    // The two slopes, each a quad from eave to ridge.
    quad(
        &mut points,
        [w, 0.0, -d],
        [-w, height, 0.0],
        [0.0, 0.0, 2.0 * d],
    );
    quad(
        &mut points,
        [-w, 0.0, d],
        [w, height, 0.0],
        [0.0, 0.0, -2.0 * d],
    );
    // The two gable ends, each one triangle with the apex up.
    for (a, b, c) in [
        ([-w, 0.0, d], [w, 0.0, d], [0.0, height, d]),
        ([w, 0.0, -d], [-w, 0.0, -d], [0.0, height, -d]),
    ] {
        let normal = normalized(cross(subtract(b, a), subtract(c, a)));
        points.push(a, normal, [0.0, 0.0]);
        points.push(b, normal, [1.0, 0.0]);
        points.push(c, normal, [0.5, 1.0]);
    }
    points.finish()
}
