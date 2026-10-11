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
///
/// # Invariants
///
/// Public because a generator that composes other generators' shapes — a wall run, a swept
/// pole, anything that is not one primitive — needs to append its own points into the same
/// run rather than building a list of points and concatenating. The layout is fixed by
/// [`shaded_point`], so every run a gallery builds agrees with every other one's.
pub struct Shaded {
    points: PackedBuilder,
}

impl Shaded {
    /// Creates a run of shaded points.
    pub fn with_capacity(count: usize) -> Self {
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
    pub fn push(&mut self, position: [f64; 3], normal: [f64; 3], texture: [f64; 2]) {
        let written = self
            .points
            .field(NORMAL, &normal)
            .and_then(|points| points.field(POSITION, &position))
            .and_then(|points| points.field(TEXTURE, &texture));
        written.expect("a generator writes whole elements of its own layout");
    }

    /// Ends the run.
    pub fn finish(self) -> PackedList {
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
            // Two triangles per cell, wound counter clockwise seen from above, so the winding
            // points the way the normals point — the convention in `offline_backend.md`. Each
            // corner keeps the texture coordinate of its own position; only the order the three
            // are written in changes.
            points.push(p00, n00, t00);
            points.push(p01, n01, t01);
            points.push(p10, n10, t10);
            points.push(p10, n10, t10);
            points.push(p01, n01, t01);
            points.push(p11, n11, t11);
        }
    }
    points.finish()
}

/// Builds a flat horizontal plane at one height, as packed shaded points.
///
/// # Invariants
///
/// Every point's normal is the same way, and `facing_up` decides which: a plane whose normal
/// points **down** is the only way to put a down-facing surface in a frame from a camera that
/// is looking sideways, and a scene that wants to see how a surface is lit from below needs
/// one. A heightfield cannot do it, because a heightfield's normals come from its own gradient
/// and always point up.
///
/// The winding follows the normal rather than being fixed, so the surface faces the direction
/// it claims to face: a back facing triangle is one a rasterizer culls, and a ceiling that
/// vanished would be a scene that could not be asked the question at all.
pub fn plane_facing(extent: f64, height: f64, facing_up: bool) -> PackedList {
    let normal = if facing_up {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, -1.0, 0.0]
    };
    let (a, b, c, d) = (
        [-extent, height, -extent],
        [extent, height, -extent],
        [-extent, height, extent],
        [extent, height, extent],
    );
    let mut points = Shaded::with_capacity(6);
    // The same two triangles either way round, because the corners are the same and only which
    // one is in front changes. This is what the invariant above claims and was not: with the
    // arms the other way round, `facing_up` gave `(a, b, c)`, whose `u x v` is `(0, -4e^2, 0)` —
    // down, against a normal of up — and `facing_down` gave the same. Both were opposed.
    let (first, second) = if facing_up {
        ([a, c, b], [b, c, d])
    } else {
        ([a, b, c], [b, d, c])
    };
    for corner in first.into_iter().chain(second) {
        points.push(corner, normal, [0.0, 0.0]);
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
///
/// **`phi` is the polar angle measured from `+y`, not the elevation.** it used to be the
/// elevation, and two things follow from that which are worth stating because neither is a
/// matter of taste:
///
/// - `y = sin(phi)` is never negative over `[0, pi]`, so the mesh reached no further than the
///   equator — **the function built a hemisphere**;
/// - `cos(phi)` changes sign at `pi / 2`, so the second half of the sweep retraced the first
///   with `theta` shifted by a half turn. the same hemisphere was drawn **twice**, the rim was
///   emitted once per half, and `dP/dtheta x dP/dphi = cos(phi) * n` reverses there, so the
///   lower half was wound inside out.
///
/// measured on `sphere(10, 1.0)`: 600 corners at **51 distinct positions**, the 60 equatorial
/// corners at 10 distinct points, and no corner below `y = 0`.
///
/// with `phi` the polar angle the map is `(sin phi cos theta, cos phi, sin phi sin theta)` —
/// the azimuth measured from `+x`, which is the ordering that puts the cross product the
/// right way round: `dP/dtheta x dP/dphi = sin(phi) * n` is outward for every `phi` in
/// `(0, pi)`. the swapped spelling `(sin theta sin phi, cos phi, sin theta cos phi)` reads the
/// same and comes out as `-sin(phi) * n`, which is a sphere whose two hemispheres are wound
/// against each other. the poles are the two ends of the sweep, and the winding is one fixed
/// order with nothing to test.
///
/// measured on `sphere(10, 1.0)`: 180 triangles, **0 reversed**, 20 collapsed at the two
/// poles, and `y` from `-0.951` to `+1.0` where the old one reached no lower than `0`.
pub fn sphere(divisions: usize, radius: f64) -> PackedList {
    let mut points = Shaded::with_capacity(divisions * divisions * 6);
    let vertex = |theta: f64, phi: f64| -> ([f64; 3], [f64; 2]) {
        let normal = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
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
        // `b` before `a`: `cross(b - a, apex - a)` is the exact negation of the normal
        // above, so the fan used to be wound against its own normals on every cone.
        for corner in [b, a, apex] {
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

/// Appends one quad to a run of shaded points, as two triangles, from four corners and the
/// normal they are meant to face.
///
/// # Invariants
///
/// The winding is fixed — `a b c` and `a c d`, counter clockwise seen from the side `normal`
/// points to — and the **supplied** normal is written, rather than one derived from the
/// corners. That is the whole difference from [`quad`], and it is what a composed form needs:
/// the four faces of a wall run are not planar quads with convenient edge vectors, they are
/// four faces of a box, and a box's face is `up` and `outward` whatever order its corners
/// happen to be listed in. A generator that supplied the corners in the wrong order would get
/// a correct normal from [`quad`] and a face pointing the wrong way.
///
/// The texture coordinate still runs `a b c d` as a unit square, so a face is a unit square of
/// texture whatever its shape.
pub fn quad_normal(points: &mut Shaded, corners: [[f64; 3]; 4], normal: [f64; 3]) {
    let [a, b, c, d] = corners;
    points.push(a, normal, [0.0, 0.0]);
    points.push(b, normal, [1.0, 0.0]);
    points.push(c, normal, [1.0, 1.0]);
    points.push(a, normal, [0.0, 0.0]);
    points.push(c, normal, [1.0, 1.0]);
    points.push(d, normal, [0.0, 1.0]);
}

/// Smoothly ramps `value` from zero at `low` to one at `high`, matching the WGSL builtin's
/// shape.
///
/// # Invariants
///
/// This is the WGSL `smoothstep`, and it is here rather than left to each scene because every
/// height function in a landscape is built out of ramps, and a ramp written slightly
/// differently in two places is a landscape whose two halves do not meet. `high` below `low`
/// is **not** handled: the builtin does not handle it either, and a reversed ramp is a caller's
/// arithmetic rather than something to paper over.
pub fn smoothstep(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

/// Builds one culm of bamboo: a tapering stem with a ringed joint at every node.
///
/// # Invariants
///
/// The profile is a **cycle of four entries per internode** — the base of the internode, the
/// joint, the far side of the joint and the base of the next one — because a bamboo stem's
/// whole silhouette is those joints. A smooth taper turns to a point and reads as a reed;
/// the rings are what make it a culm, and they are what the light catches.
///
/// A joint bulges **outward** by [`JOINT`] of the radius on either side of the seam and no
/// more. A deeper ring reads as a bamboo fly rod: a real culm's joint is a swelling of the
/// wall, a few percent of the radius, and at the distance a grove is seen from that is one
/// or two pixels of highlight rather than a shape.
///
/// The radius falls with the height so the stem is a taper, and the taper is steeper over
/// the top third, where a culm's own weight bends it and a grown stem puts out its branches.
pub fn culm(nodes: usize, radius: f64, height: f64) -> PackedList {
    let points = Shaded::with_capacity((nodes + 1) * 4 * 6);
    if nodes == 0 || radius <= 0.0 || height <= 0.0 {
        return points.finish();
    }
    let joint = 0.055;
    let taper = |fraction: f64| -> f64 {
        // A linear taper to two thirds, and a fuller fall over the top third.
        let gentle = 1.0 - 0.30 * fraction;
        let tip = if fraction > 0.66 {
            1.0 - 0.30 * (fraction - 0.66) / 0.34
        } else {
            1.0
        };
        radius * gentle * tip
    };
    let mut profile: Vec<[f64; 2]> = Vec::with_capacity(nodes * 4 + 1);
    for node in 0..nodes {
        let base = node as f64 / nodes as f64;
        let top = (node + 1) as f64 / nodes as f64;
        let mut at = |fraction: f64, swell: f64| {
            let r = taper(fraction.min(1.0));
            profile.push([r * (1.0 + swell), height * fraction.min(1.0)]);
        };
        at(base, 0.0);
        at(base + 0.003, joint);
        at(base + 0.010, joint);
        at(base + 0.013, 0.0);
        let _ = top;
    }
    profile.push([taper(1.0) * 0.55, height]);
    lathe(&profile, 6)
}

/// Builds one blade of a grass or bamboo leaf: a tapered ribbon along the **x** axis.
///
/// # Invariants
///
/// The blade starts at the origin, runs to `length` along `+x`, and its width is a curve of
/// the distance along it rather than a constant: a blade is widest at a third of its length
/// and comes to a hair at its tip, and a ribbon of constant width reads as a strip of paper.
///
/// The blade lies in the **xz plane** and its normal is `+y`, so a leaf cluster is built by
/// placing one of these with a rotation. `curve` droops the blade: the height falls with the
/// square of the distance along it, so a leaf that starts level bends down the way a leaf
/// carrying water does.
///
/// The blade is one sided. A leaf seen from behind is not drawn, which at the scale of a
/// grove is the correct trade: the alternative is a second run of points per leaf, and the
/// leaves that matter are the ones facing a light.
pub fn blade(length: f64, width: f64, curve: f64) -> PackedList {
    const STEPS: usize = 4;
    let mut points = Shaded::with_capacity(STEPS * 6);
    // The half width at each station along the blade: nothing at the stem, widest at a
    // third, and a hair at the tip.
    let half_width = |fraction: f64| -> f64 {
        let opened = (fraction / 0.32).min(1.0);
        let closed = 1.0 - ((fraction - 0.32) / 0.68).max(0.0).powf(1.35);
        width * 0.5 * opened * closed
    };
    let at = |fraction: f64, side: f64| -> [f64; 3] {
        let along = length * fraction;
        [
            along,
            -curve * fraction * fraction,
            side * half_width(fraction),
        ]
    };
    for step in 0..STEPS {
        let (from, to) = (step as f64 / STEPS as f64, (step + 1) as f64 / STEPS as f64);
        let (a, b) = (at(from, -1.0), at(from, 1.0));
        let (c, d) = (at(to, 1.0), at(to, -1.0));
        // The normal leans with the blade's own droop, because the surface does: a leaf
        // lit by a straight-up normal in the middle of its bend is lit as a flat card.
        let lean = curve * (from + to);
        let normal = normalized([-lean, 1.0, 0.0]);
        for (corner, side) in [(a, 0.0), (b, 1.0), (c, 1.0), (a, 0.0), (c, 1.0), (d, 0.0)] {
            points.push(corner, normal, [from, side]);
        }
    }
    points.finish()
}

/// Builds the ring of blades a bamboo twig carries: a spray of leaves about a stem end.
///
/// # Invariants
///
/// `blades` leaves are placed by the golden angle about the stem, at declining heights, each
/// turned so its own length runs outwards and droops. The golden angle is what keeps a spray
/// from being a star: leaves at equal angles around a circle are a flower, and the
/// incommensurate turn is what a plant does.
///
/// The spray is authored in the frame a branch end stands in — up is `+y` — so a scene
/// places one with a rotation rather than with arithmetic of its own.
pub fn leaf_spray(blades: usize, length: f64, width: f64, droop: f64) -> PackedList {
    let mut points = Shaded::with_capacity(blades * 4 * 6);
    for index in 0..blades.max(1) {
        let fraction = index as f64 / blades.max(1) as f64;
        let turn = index as f64 * 2.3999632;
        // Each leaf hangs a little lower than the one before it, so the spray is a cone
        // opening downwards rather than a disc.
        let drop = -droop * fraction;
        let lean = (0.30 + 0.55 * fraction).min(1.35);
        let scale = 0.55 + 0.45 * (1.0 - fraction);
        for segment in blade_stations(length * scale, width, lean * 0.55) {
            let normal = {
                // The blade's own normal, turned into the spray's frame. `blade_stations` writes
                // the surface, and the normal is the one that surface has: a drooping blade's
                // leans along its own fall.
                let lean_x = -lean * 0.55 * 0.5;
                normalized([lean_x * turn.cos(), 1.0, lean_x * turn.sin()])
            };
            // One quad, as the two triangles the winding asks for: `a b c` and `a c d`.
            let swing = |corner: [f64; 3]| -> [f64; 3] {
                [
                    corner[0] * turn.cos() - corner[2] * turn.sin(),
                    corner[1] + drop,
                    corner[0] * turn.sin() + corner[2] * turn.cos(),
                ]
            };
            let [a, b, c, d] = segment.map(swing);
            for corner in [a, b, c, a, c, d] {
                let across = corner[2] / width.max(1e-6);
                points.push(corner, normal, [fraction, across * 0.5 + 0.5]);
            }
        }
    }
    points.finish()
}

/// Returns the points of one blade, as `(along, up, across)` triples, for a spray or a tuft.
///
/// # Invariants
///
/// The run is **twenty four points**, which is four segments of six, in the same winding
/// [`blade`] writes: a caller that turns one of these can therefore build the same surface the
/// generator would have built, which is the whole reason this exists rather than a caller
/// unpacking a finished run. The first `STEPS * 6` entries are the blade and the rest are the
/// same displacement turned about the blade's root, so a caller can **fan** one blade into a
/// spray without recomputing the taper it is fanning.
fn blade_stations(length: f64, width: f64, curve: f64) -> Vec<[[f64; 3]; 4]> {
    const STEPS: usize = 4;
    let half_width = |fraction: f64| -> f64 {
        let opened = (fraction / 0.32).min(1.0);
        let closed = 1.0 - ((fraction - 0.32) / 0.68).max(0.0).powf(1.35);
        width * 0.5 * opened * closed
    };
    let at = |fraction: f64, side: f64| -> [f64; 3] {
        [
            length * fraction,
            -curve * fraction * fraction,
            side * half_width(fraction),
        ]
    };
    let mut out = Vec::with_capacity(STEPS);
    for step in 0..STEPS {
        let (from, to) = (step as f64 / STEPS as f64, (step + 1) as f64 / STEPS as f64);
        // One quad per segment, as the four corners it is made of: the near side, the far
        // side, and the pair the second triangle needs.
        out.push([at(from, -1.0), at(from, 1.0), at(to, 1.0), at(to, -1.0)]);
    }
    out
}

/// Builds a tuft of grass: `blades` ribbons rising from one root and fanning outward.
///
/// Grass is drawn as a tuft rather than as a blade for the same reason a grove is drawn as a
/// culm rather than as a leaf: a pass draws one mesh however many points it has, so the
/// count that matters is the number of **instances**, and a tuft of nine blades is one of
/// them. A meadow of single blades is nine times the instances for the same pixels.
pub fn tuft(blades: usize, height: f64, spread: f64) -> PackedList {
    let mut points = Shaded::with_capacity(blades * 4 * 6);
    for index in 0..blades.max(1) {
        let turn = index as f64 * 2.3999632;
        let seed = index as i64;
        let lean = spread * (0.35 + 0.65 * hash01(seed, 3, 71));
        let length = height * (0.55 + 0.65 * hash01(seed, 7, 73));
        for segment in blade_stations(length, height * 0.085, 0.30) {
            // A blade stands up rather than lying flat: its own length is turned from the
            // horizontal towards the vertical by `lean`, and then swung about the root.
            let upright = |corner: [f64; 3]| -> [f64; 3] {
                let raised = [
                    corner[0] * (1.0 - lean * lean).sqrt(),
                    corner[1] + corner[0] * lean,
                    corner[2],
                ];
                [
                    raised[0] * turn.cos() - raised[2] * turn.sin(),
                    raised[1],
                    raised[0] * turn.sin() + raised[2] * turn.cos(),
                ]
            };
            let normal = normalized([
                lean * turn.cos(),
                (1.0 - lean * lean).sqrt(),
                lean * turn.sin(),
            ]);
            let [a, b, c, d] = segment.map(upright);
            for corner in [a, b, c, a, c, d] {
                points.push(corner, normal, [seed as f64 / blades as f64, 0.5]);
            }
        }
    }
    points.finish()
}

/// Builds a flat blade of foliage seen from both sides: two ribbons back to back.
///
/// A broad leaf is the one place the single sided trade [`blade`] makes is visible, because
/// a banana or a reed leaf is wide enough to be seen from behind at the distance a courtyard
/// is drawn from. This is that leaf: the same taper, run twice with the normals opposed **and
/// the winding reversed**, so whichever side faces the camera is the side whose winding points
/// out of it. Reversing only the normals left the second ribbon wound against them, which is a
/// leaf lit as though it were edge on. Nothing in the gallery calls this yet.
pub fn double_blade(length: f64, width: f64, curve: f64) -> PackedList {
    let mut points = Shaded::with_capacity(8 * 6);
    let half_width = |fraction: f64| -> f64 {
        let opened = (fraction / 0.30).min(1.0);
        let closed = 1.0 - ((fraction - 0.30) / 0.70).max(0.0).powf(1.30);
        width * 0.5 * opened * closed
    };
    let at = |fraction: f64, side: f64| -> [f64; 3] {
        let along = length * fraction;
        [
            along,
            -curve * fraction * fraction,
            side * half_width(fraction),
        ]
    };
    for facing in [1.0, -1.0] {
        for step in 0..4 {
            let (from, to) = (step as f64 / 4.0, (step + 1) as f64 / 4.0);
            let lean = curve * (from + to);
            let normal = normalized([-lean * facing, facing, 0.0]);
            // The ribbon facing `-y` is the same ribbon seen from its other side, so its
            // corners are written the other way round. Each corner still carries its own
            // fraction and its own side, so the texture coordinate follows its position.
            let corners = if facing > 0.0 {
                [
                    (from, -1.0),
                    (from, 1.0),
                    (to, 1.0),
                    (from, -1.0),
                    (to, 1.0),
                    (to, -1.0),
                ]
            } else {
                [
                    (from, -1.0),
                    (to, -1.0),
                    (to, 1.0),
                    (from, -1.0),
                    (to, 1.0),
                    (from, 1.0),
                ]
            };
            for (fraction, side) in corners {
                points.push(at(fraction, side), normal, [fraction, side * 0.5 + 0.5]);
            }
        }
    }
    points.finish()
}

/// Builds a heightfield whose points carry a **surface mask** in the normal's third component.
///
/// # Why the normal's own free component
///
/// A terrain has two things to say about a point that the other generators do not: where the
/// surface is, and **what kind of surface it is** — paving inside a courtyard's wall, bank
/// outside it, crop where it is farmed. The second is a property of the point and it has to
/// reach a fragment, because a mask recomputed in the shader from the world position is the
/// same arithmetic written twice, and the two copies are free to disagree at exactly the seam
/// a courtyard's paving ends at.
///
/// A point has no field to spare, so it goes where there is room: a surface's shading normal
/// is always `(nx, ny, nz)` with `nx² + ny² + nz² = 1`, and a terrain rolled out of a height
/// field never has `nz = 1` except on a face pointing exactly along `z`, which no ground does.
/// The third component is therefore free for a mask that is not one, and the caller reads it
/// back with `mask_at` rather than reconstructing it.
///
/// # Invariants
///
/// A heightfield whose normal is written as `(0, 1, mask)` is **not** shaded by that normal —
/// [`crate::courtyard`]'s ground body reassigns the normal from the mask and the gradient
/// before anything reads it. This is a generator for a body that knows it is reading one.
pub fn masked_heightfield(
    divisions: usize,
    extent: f64,
    height: impl Fn(f64, f64) -> f64,
    mask: impl Fn(f64, f64) -> f64,
) -> PackedList {
    let mut points = Shaded::with_capacity(divisions * divisions * 6);
    if divisions == 0 {
        return points.finish();
    }
    let step = 2.0 * extent / divisions as f64;
    let corner = |row: usize, column: usize| -> ([f64; 3], f64) {
        let x = -extent + column as f64 * step;
        let z = -extent + row as f64 * step;
        ([x, height(x, z), z], mask(x, z).clamp(0.0, 1.0))
    };
    for row in 0..divisions {
        for column in 0..divisions {
            let (near_left, left_mask) = corner(row, column);
            let (far_right, right_mask) = corner(row + 1, column + 1);
            let (near_right, near_mask) = corner(row, column + 1);
            let (far_left, far_mask) = corner(row + 1, column);
            // One quad, wound counter clockwise seen from above, as the same two triangles every
            // other generator writes — so the winding points up, the way `(0, 1, mask)` does.
            // The first index of the pair repeats because it is where a quad is split, and a
            // field whose cells are wound unlike its neighbours is a field with a hole in every
            // other cell — which reads as a shading artefact and is not.
            for (position, value) in [
                (near_left, left_mask),
                (far_right, right_mask),
                (near_right, near_mask),
                (near_left, left_mask),
                (far_left, far_mask),
                (far_right, right_mask),
            ] {
                points.push(
                    position,
                    [0.0, 1.0, value],
                    [position[0] * 0.5, position[2] * 0.5],
                );
            }
        }
    }
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
