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
use shape::loft::Loft;
use shape::math::Vec3;

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

/// One corner of a heightfield cell, as the four numbers a cell carries: where the corner stands in
/// plan, and where it sits in the grid the texture coordinate names.
///
/// # Invariants
///
/// **The texture coordinate travels with the station rather than being looked up again**, so a
/// corner the grid put there keeps the exact value [`heightfield`] would have written. A cell no
/// cutter touches has to come out byte for byte what it was, or every ground in the gallery moves
/// for the sake of a mesh that has no hole in it.
pub type Station = [f64; 4];

/// Returns the plan position a station stands at.
pub fn plan_of(station: Station) -> [f64; 2] {
    [station[0], station[1]]
}

/// Returns the station `amount` of the way from one to another, in all four of its numbers.
pub fn between(from: Station, to: Station, amount: f64) -> Station {
    [
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
        from[2] + (to[2] - from[2]) * amount,
        from[3] + (to[3] - from[3]) * amount,
    ]
}

/// How long an accumulated normal below which it is thrown away.
///
/// A vertex whose every incident triangle has no area — the ring at a station the scale law has
/// collapsed to a point — accumulates nothing, and a normal of zero length is a normal the shader
/// would read as `NaN` on the one vertex where it is asked about. The floor is far above the
/// rounding of a sum of a few thousand face normals and far below any real one, so it catches the
/// genuinely degenerate case and nothing else.
const NORMAL_FLOOR: f64 = 1.0e-12;

/// How long a section edge below which it carries no direction to measure a turn in.
///
/// A vertex whose two section edges both collapse is a ring the scale law has flattened to a point,
/// and there is no angle at a point: the crease test reads it as smooth and the corner falls back to
/// the accumulated average. The floor is the same floor [`NORMAL_FLOOR`] uses, for the same reason
/// — far above the rounding of a subtraction and far below any edge a plant is drawn with.
const EDGE_FLOOR: f64 = 1.0e-12;

/// One triangle's corner: where it stands, which way it faces, and where on the surface it is.
///
/// # Invariants
///
/// It is the whole of a shaded point with nothing packed, so a scene that adds a field to its own
/// point schema — [`crate::building_identity`]'s `authored_id` is the one that exists — reads a
/// loft through this and packs the corners itself rather than unpacking them again.
pub struct Corner {
    /// Where the corner stands, in the space the loft was sampled in.
    pub position: [f64; 3],
    /// The normal this corner is shaded with: its vertex's accumulated face normal, or — at a section
    /// corner sharp enough to crease, in [`loft_corners_creased_at`] — the face normal of the triangle
    /// this corner is one of.
    pub normal: [f64; 3],
    /// The `(u, v)` the loft gave this vertex.
    pub texture: [f64; 2],
}

/// Returns every triangle of a loft as three corners, in the order the loft wound them.
///
/// # Invariants
///
/// - **Every triangle is emitted in the order the loft wound it, and every corner carries the
///   `(u, v)` the loft gave that vertex.** Nothing here reorders, re-winds or re-samples: a scene
///   is the evidence of what `Sweep::sample` wrote, and a normal computed from the same triangles
///   is the only normal it can be right about.
/// - **A normal is accumulated per loft vertex and normalised once**, not per triangle, so a tube
///   shades as a tube however many rings it is given. A facet normal per corner would make a
///   thirty two sided cylinder read as a prism, which is the one thing the section's own
///   arc-length sampling exists to avoid.
/// - **A vertex that accumulated nothing falls back to the direction away from the shape's own
///   centre.** That is the ring at a station the scale law collapsed to nothing, where every
///   incident triangle has no area; it is outward for every solid drawn here and it is better than
///   the zero-length vector a shader would read as `NaN`.
///
/// A smooth average is the right reading of a rounded tube and the wrong one of a fluted bole,
/// where the two faces meeting at a corner are then lit as one. This function stays the whole of the
/// smooth reading, and [`loft_corners_creased_at`] is the sibling that reads a sharp section corner
/// flat, so a scene that wants the flat one asks for the flat one.
pub fn loft_corners(loft: &Loft) -> Vec<Corner> {
    corners_within(loft, None)
}

/// Returns a loft's corners with a **hard** normal at every section corner sharper than `crease`,
/// with their positions and normals moved by a plant's placement.
///
/// # Which of the two faces a creased corner takes
///
/// **Its own triangle's.** The two faces meeting at a sharp corner read as two faces only if they
/// carry two different normals, so a corner that takes one face's normal *and hands it to the other
/// face as well* has not creased anything — it has drawn the averaged normal again and renamed it.
/// The normal is therefore taken per corner, from the triangle that corner is a corner of, and this
/// is possible because the wood mesh is **unindexed**: a corner is free to disagree with the same
/// vertex's other corners, which is exactly what a crease spends.
///
/// # Invariants
///
/// - **A section whose sharpest corner is below `crease` draws exactly the corners it drew before,
///   bit for bit.** Every corner then takes the accumulated average down the same arithmetic
///   [`loft_corners`] uses — `creased` is every `false` and the two paths are one path — so the
///   change is an identity on every round-section plant rather than a small change to all of them.
/// - **`crease` is an angle of turn, not an interior angle**, so a section is argued in the same
///   numbers its tessellation is quoted in: a regular `n`-gon turns `360 / n` degrees at each of its
///   corners, and it is the turn that says whether it creases. An `f32` because a section is
///   authored in `f32` and a crease is a property of one.
/// - **A creased corner whose own triangle has no area falls back exactly as a smooth one does**,
///   to the direction away from the shape's centre, because there is no face there to take a normal
///   from. The case is the same degenerate ring [`loft_corners`] names and is equally unreachable in
///   practice; it is answered rather than asserted.
/// - **Nothing else moves.** Positions, texture coordinates, corner order and corner count are
///   [`loft_corners`]'s, so a normal is the only thing a scene can measure as changed.
pub fn loft_corners_creased_at(
    loft: &Loft,
    placement: &vegetation::plant::Placement,
    crease: f32,
) -> Vec<Corner> {
    placed(corners_within(loft, Some(crease.into())), placement)
}

/// Returns a loft's corners with their positions and normals moved by a plant's placement.
///
/// # Invariants
///
/// **The normals are rotated and never translated**, because a placement moves a frame and not a direction; and the
/// placement never scales, which is why a normal stays unit length and the texture coordinates are untouched. A wood
/// mesh baked in the world and a wood mesh baked at the origin therefore shade alike.
///
/// This is [`loft_corners`] moved and nothing else — a crease is decided before a placement sees a
/// corner, so the two builders share the one walk rather than each having a corner of their own.
pub fn loft_corners_at(loft: &Loft, placement: &vegetation::plant::Placement) -> Vec<Corner> {
    placed(loft_corners(loft), placement)
}

/// The one corner walk behind every builder above: [`loft_corners`] with no crease,
/// [`loft_corners_creased_at`] with one.
///
/// # Invariants
///
/// **With `crease` of `None` this is [`loft_corners`] arithmetically, not merely in its result.** The
/// accumulated normals, the centre and the floor are computed by the same statements and in the same
/// order, and the emit loop takes its normal from `accumulated` down the same expressions, so the
/// two builders cannot drift apart in a corner case that only one of them has. The face normal is
/// computed for every triangle whether or not anything wants it, because a branch here would be a
/// second rule to keep in step with the first.
fn corners_within(loft: &Loft, crease: Option<f64>) -> Vec<Corner> {
    let positions = loft.positions();
    let mut accumulated = vec![[0.0f64; 3]; positions.len()];
    for triangle in loft.triangles() {
        let [first, second, third] = (*triangle).map(|index| at(positions, index));
        let face = cross(subtract(second, first), subtract(third, first));
        for index in *triangle {
            let slot = &mut accumulated[index as usize];
            for axis in 0..3 {
                slot[axis] += face[axis];
            }
        }
    }
    let centre = {
        let mut sum = [0.0f64; 3];
        for point in positions {
            for axis in 0..3 {
                sum[axis] += f64::from(point[axis]);
            }
        }
        let count = positions.len().max(1) as f64;
        [sum[0] / count, sum[1] / count, sum[2] / count]
    };
    let creased: Vec<bool> = match crease {
        None => vec![false; positions.len()],
        Some(crease) => (0..positions.len() as u32)
            .map(|index| is_creased(loft, index, crease))
            .collect(),
    };

    let mut corners = Vec::with_capacity(loft.triangles().len() * 3);
    for triangle in loft.triangles() {
        let [first, second, third] = (*triangle).map(|index| at(positions, index));
        let face = cross(subtract(second, first), subtract(third, first));
        for &index in triangle {
            let position = positions[index as usize];
            let parameter = loft.uvs()[index as usize];
            let mut normal = if creased[index as usize] {
                face
            } else {
                accumulated[index as usize]
            };
            let outward = [
                f64::from(position[0]) - centre[0],
                f64::from(position[1]) - centre[1],
                f64::from(position[2]) - centre[2],
            ];
            let squared = dot(normal, normal);
            if squared <= NORMAL_FLOOR * NORMAL_FLOOR {
                normal = outward;
            }
            corners.push(Corner {
                position: [
                    f64::from(position[0]),
                    f64::from(position[1]),
                    f64::from(position[2]),
                ],
                normal: normalized(normal),
                texture: [f64::from(parameter[0]), f64::from(parameter[1])],
            });
        }
    }
    corners
}

/// Whether one of a loft's vertices is a section corner sharper than `crease` radians.
///
/// # Invariants
///
/// **The two edges measured are the two polygon edges of the vertex's own ring.** A sweep lays every
/// ring out as `ring * around + slot` ([`shape::loft::Loft`]'s own layout), the wrap around is the
/// loop's seam, and so a corner is measured from the two segments that meet there and nothing else —
/// which is what makes the answer a property of the *section* rather than of one tessellated cell.
///
/// **A vertex either of whose two edges has no length creases nothing.** That is the ring the scale
/// law collapsed to a point: there is no direction there to measure a turn in, so it falls back to
/// the smooth average, which is the same reading [`NORMAL_FLOOR`] gives a vertex that accumulated
/// nothing. A corner is not creased at one ring of a collapsed solid and creased at another.
///
/// **The angle compared is the turn — `pi` less the interior angle — so it reads as `360 / n` for a
/// regular `n`-gon.** Quoting the interior angle instead would put a twelve-gon's corner at 150
/// degrees, which is not the number its tessellation is counted in and not the number a crease is
/// chosen by.
fn is_creased(loft: &Loft, index: u32, crease: f64) -> bool {
    let around = loft.around() as u32;
    if around < 3 || index as usize >= loft.rings() * loft.around() {
        return false;
    }
    let positions = loft.positions();
    let base = index / around * around;
    let slot = index - base;
    let vertex = at(positions, index);
    let incoming = subtract(at(positions, base + (slot + around - 1) % around), vertex);
    let outgoing = subtract(at(positions, base + (slot + 1) % around), vertex);
    let (incoming_length, outgoing_length) = (length(incoming), length(outgoing));
    if incoming_length <= EDGE_FLOOR || outgoing_length <= EDGE_FLOOR {
        return false;
    }
    let cosine = (dot(incoming, outgoing) / (incoming_length * outgoing_length)).clamp(-1.0, 1.0);
    std::f64::consts::PI - cosine.acos() > crease
}

/// Moves corners by a plant's placement, in both their positions and their normals.
fn placed(corners: Vec<Corner>, placement: &vegetation::plant::Placement) -> Vec<Corner> {
    corners
        .into_iter()
        .map(|corner| {
            let position = placement.place([
                corner.position[0] as f32,
                corner.position[1] as f32,
                corner.position[2] as f32,
            ]);
            let normal = placement.rotate([
                corner.normal[0] as f32,
                corner.normal[1] as f32,
                corner.normal[2] as f32,
            ]);
            Corner {
                position: [
                    f64::from(position[0]),
                    f64::from(position[1]),
                    f64::from(position[2]),
                ],
                normal: [
                    f64::from(normal[0]),
                    f64::from(normal[1]),
                    f64::from(normal[2]),
                ],
                texture: corner.texture,
            }
        })
        .collect()
}

/// One of a loft's positions, read as the `f64` the corner arithmetic is done in.
fn at(positions: &[Vec3], index: u32) -> [f64; 3] {
    let point = positions[index as usize];
    [
        f64::from(point[0]),
        f64::from(point[1]),
        f64::from(point[2]),
    ]
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
        value += amplitude
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
    // The one walk is [`heightfield_over`]'s; a heightfield is that walk with a cutter that cuts
    // nothing, so the grid a hole changes and the grid it does not are the same arithmetic.
    heightfield_over(divisions, extent, height, |cell| vec![cell.to_vec()])
}

/// Builds a heightfield whose cells are cut by `over`, which returns the pieces of one triangle.
///
/// # Invariants
///
/// - **`over` is handed one triangle at a time and returns whole convex pieces in the winding that
///   triangle had**, and each piece is then **fanned from its own first corner**. A piece with four
///   corners is two triangles and not one triangle with four points: a mesh is a stream of corners
///   three at a time, so writing a quadrilateral as four corners shifts every triangle after it by
///   one and the surface stops being the surface the generator described.
/// - A corner a cutter invented is **shaded from the same surface as every other corner**: its
///   position and normal come from `height` at its own plan position and its texture coordinate
///   from the grid it was interpolated in, so a cut edge is lit like the ground either side of it
///   rather than as a facet with no law behind it.
/// - Everything about the grid is [`heightfield`]'s, down to the order the two triangles of a cell
///   are written in.
pub fn heightfield_over(
    divisions: usize,
    extent: f64,
    height: impl Fn(f64, f64) -> f64,
    over: impl Fn(&[Station]) -> Vec<Vec<Station>>,
) -> PackedList {
    let step = 2.0 * extent / divisions as f64;
    let origin = -extent;
    let mut points = Shaded::with_capacity(divisions * divisions * 6);

    // One grid corner as a station: where it stands in plan, and where it sits in the grid.
    let corner = |column: usize, row: usize| {
        [
            origin + column as f64 * step,
            origin + row as f64 * step,
            column as f64 / divisions as f64,
            row as f64 / divisions as f64,
        ]
    };
    for row in 0..divisions {
        for column in 0..divisions {
            let (near_left, near_right) = (corner(column, row), corner(column + 1, row));
            let (far_right, far_left) = (corner(column + 1, row + 1), corner(column, row + 1));
            // Two triangles per cell, wound counter clockwise seen from above, so the winding
            // points the way the normals point — the convention in `offline_backend.md`. Each
            // corner keeps the texture coordinate of its own position; only the order the three
            // are written in changes.
            for cell in [
                [near_left, far_left, near_right],
                [near_right, far_left, far_right],
            ] {
                for piece in over(&cell) {
                    for corner in 1..piece.len().saturating_sub(1) {
                        for station in [piece[0], piece[corner], piece[corner + 1]] {
                            let (position, normal, texture) = shade(&height, step, station);
                            points.push(position, normal, texture);
                        }
                    }
                }
            }
        }
    }
    points.finish()
}

/// Returns the shaded point one station stands as: where it is on the surface, the normal of that
/// surface, and the texture coordinate the station carried.
///
/// # Invariants
///
/// The normal is taken from the heightfield's own gradient at that point, so it is perpendicular to
/// the surface rather than to the cell it happens to be drawn in. A normal per cell shades a
/// smooth hillside as tiled facets, and a terrace riser — a slope a cell wide — as a stripe of
/// alternating triangles, which is the single most common way a lit scene stops reading as lit.
fn shade(
    height: &impl Fn(f64, f64) -> f64,
    step: f64,
    station: Station,
) -> ([f64; 3], [f64; 3], [f64; 2]) {
    let [x, z, u, v] = station;
    let (dx, dz) = (step * 0.5, step * 0.5);
    let slope_x = (height(x + dx, z) - height(x - dx, z)) / (2.0 * dx);
    let slope_z = (height(x, z + dz) - height(x, z - dz)) / (2.0 * dz);
    // The normal of a surface whose height rises by one per unit of x and z.
    let normal = normalized([-slope_x, 1.0, -slope_z]);
    ([x, height(x, z), z], normal, [u, v])
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
