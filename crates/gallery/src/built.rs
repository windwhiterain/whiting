//! Built forms: the solid things a scene is put together from.
//!
//! [`geometry`](crate::geometry) holds the primitives — a cuboid, a cone, a sphere, a
//! lathe — and each of those is a shape a formula can be written for. What is here is the
//! step up from a primitive: the forms that are **assembled** out of faces, a wall run, a
//! ring of them, a swept pole, a roof profile, and the turned profiles of the vessels that
//! stand against a wall.
//!
//! # Invariants
//!
//! - Every form here is a generator, and a generator **returns** a
//!   [`PackedList`](definition::PackedList) rather than appending into a caller's run. Which
//!   forms share one mesh is a fact about a scene and not about the forms, so a scene that
//!   wants a wall and its coping in a single buffer composes them itself.
//! - Every face is wound to the normal it is **told**, through
//!   [`quad_normal`](crate::geometry::quad_normal), and never to one derived from the order
//!   its corners happen to be listed in. The faces of a composed form are the faces of a
//!   box, and a box's top face looks up and its outer face looks outward whatever order its
//!   corners are given: a generator that took the normal from the corner order would be a
//!   generator with one way to be wrong on every face it has.
//! - Nothing here names a place, a period or a plant. A wall run is two ends, a height band
//!   and a thickness, and what those numbers describe — and what stands against the wall —
//!   is the caller's business. What is worth keeping when a scene's numbers change is the
//!   reasoning underneath them, so the reasoning is what this file is mostly made of.

use definition::PackedList;

use crate::geometry::{
    Shaded, add, cross, dot, normalized, quad_normal, scale, smoothstep, subtract,
};

/// The number of faces a wall run is made of: the outer, the inner, the top and the two ends.
const WALL_FACES: usize = 5;

/// The way the outer face of the run from `from` to `to` looks.
///
/// # Invariants
///
/// The answer is `along` turned a quarter turn, so it is a **horizontal** normal that
/// ignores the height of either end: a run's faces stand up whatever the ground does under
/// them. Which way it points is fixed by the order the ring is walked in, not by anything
/// about this pair of points, and the ring is walked so that the answer is away from the
/// middle — anticlockwise on plan, seen from above.
///
/// It is turned rather than tested against a reference point because a side test needs a
/// point to test against, and a wall run's middle is not a point a ring with chamfered
/// corners has. A ring's own centre would do, and then a ring that is not convex, or a
/// single run standing on its own, would be decided by a centre that has nothing to do with
/// it.
///
/// The direction alone never decides the face. Two runs either side of a gap in the same
/// wall both face the same way and one of them runs the other way along it, so a caller
/// that derived the face from the direction would put one of them inside out — which is not
/// an error but a wall the rasterizer culls.
pub fn edge_normal(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    let along = normalized([to[0] - from[0], 0.0, to[2] - from[2]]);
    [along[2], 0.0, -along[0]]
}

/// One wall segment, as five faces: the outer, the inner, the top and the two ends. It is
/// open at the bottom, because a wall stands on something and the ground does not show the
/// underside of a run.
///
/// # Invariants
///
/// - The **top face** is the whole reason a wall reads as a wall. A run with no top is a
///   folded sheet, and from any camera above the coping you look straight through it into
///   the inside of the run — which, in an enclosed yard, is three of the six views. Its
///   corners are the run's base cycle raised by the rise, so it is a horizontal strip
///   spanning the run, and the whole five-face claim rests on that face being the top and
///   not the near end, which is the mistake a five-face box is easiest to make.
/// - The two **ends** are there so a run that stops in the middle of a wall — either side of
///   a gate — shows a face rather than a hole. They are the reason a gap is a pair of
///   finished runs rather than two stumps, and they are the only faces in this form that
///   depend on the run's direction rather than on the face it belongs to.
/// - `outward` is which way this run's **outer** face looks, written by the caller rather
///   than worked out from the run's direction, for the reason [`edge_normal`] gives. A run
///   whose outer face points the wrong way is not a broken wall but a wall that is not
///   drawn, and nothing in the frame says which of the two happened.
/// - `thickness` is a parameter rather than a fixed number because one wall is built from
///   three bands of it — the wall itself, the wider coping laid above it, and a thinner face
///   behind it — and a run that could only be one width would make the coping a second wall
///   instead of a cap on the first.
/// - The corners are lifted so that the run stands between `low` and `high` in the world's
///   own height, whatever height its two end points were written at. The ends decide where
///   the run is on the ground; the band decides how tall it is; neither decides for the
///   other, so a caller can lay a ring's corners out on sloping ground and still get one
///   level band of wall.
pub fn wall_run(
    from: [f64; 3],
    to: [f64; 3],
    outward: [f64; 3],
    low: f64,
    high: f64,
    thickness: f64,
) -> PackedList {
    let mut points = Shaded::with_capacity(WALL_FACES * 6);
    push_wall_run(&mut points, from, to, outward, low, high, thickness);
    points.finish()
}

/// Appends one wall run into a run of shaded points, so a form built out of several bands of
/// wall can be one mesh rather than three.
fn push_wall_run(
    points: &mut Shaded,
    from: [f64; 3],
    to: [f64; 3],
    outward: [f64; 3],
    low: f64,
    high: f64,
    thickness: f64,
) {
    let half = thickness * 0.5;
    let over = scale(outward, half);
    let under = scale(outward, -half);
    // The two heights this run spans, as an offset from its own base corners.
    let rise = [0.0, high - low, 0.0];
    let lift = |corner: [f64; 3]| add(corner, [0.0, low - from[1], 0.0]);

    // Four corners at the bottom of the band, named so that the outer face is `a` and `b`
    // and the inner face is `d` and `c`, which is what makes the inner face the same cycle
    // as the outer one read from the other side rather than a second pair of corners.
    let (a, b) = (add(from, over), add(to, over));
    let (c, d) = (add(to, under), add(from, under));
    let (a, b, c, d) = (lift(a), lift(b), lift(c), lift(d));

    // The outer face, then the inner one wound the other way round, then the top. Each
    // corner order is the one that makes the normal it is told: the outer face and the top
    // are both the base cycle read from a different corner, and the inner face is that
    // cycle reversed.
    quad_normal(points, [a, add(a, rise), add(b, rise), b], outward);
    quad_normal(
        points,
        [d, c, add(c, rise), add(d, rise)],
        scale(outward, -1.0),
    );
    // The top, which is the base cycle raised: `a d c b`, not the near end's `a d d' a'`.
    quad_normal(
        points,
        [add(a, rise), add(d, rise), add(c, rise), add(b, rise)],
        [0.0, 1.0, 0.0],
    );
    // The two ends, so a run that stops in the middle of the wall shows a face rather than
    // a hole.
    let along = normalized([to[0] - from[0], 0.0, to[2] - from[2]]);
    quad_normal(
        points,
        [a, d, add(d, rise), add(a, rise)],
        scale(along, -1.0),
    );
    quad_normal(points, [c, b, add(b, rise), add(c, rise)], along);
}

/// One interruption in one run of a ring, written as the fraction of the run along which it
/// falls.
///
/// The fractions rather than two points because a run of a ring is a **stretch** of a side
/// of the outline, not a side in itself, and a gap is a proportion of that stretch. A caller
/// that measured a gate in metres would otherwise have to work out which side of the outline
/// the metres fell on before it could say anything at all.
pub struct WallGap {
    /// The index of the corner whose run is interrupted. The run runs from this corner to
    /// the next one, and the last corner's run closes back to the first.
    pub edge: usize,
    /// The fraction along that run where the gap begins, from zero at the first corner to
    /// one at the next.
    pub from: f64,
    /// The fraction along that run where the gap ends. A gap whose end is not past its
    /// start is not a gap, and is ignored rather than reversed, because a caller that wrote
    /// the two the wrong way round has a mistake worth seeing rather than one worth having
    /// guessed at.
    pub to: f64,
}

/// A whole run of corners as **one** wall: every run between two corners, interrupted where
/// the caller says it is interrupted, written into a single mesh.
///
/// # Invariants
///
/// The ring is one mesh rather than one mesh per side because the sides are **one** wall as
/// far as the picture is concerned. A seam at a chamfer would be a seam in the only
/// unbroken horizontal line in the frame, and the pale band above that line is what the
/// dark verticals of whatever stands in front of it are read against. The same argument
/// says the corners are walked anticlockwise on plan, so that [`edge_normal`] can decide
/// which way each run faces by a formula, and so that a second band built by walking the
/// same ring — a coping, a colonnade, a lean-to — lands on the chamfers instead of stopping
/// short of them.
///
/// A gate is a **gap in the run rather than a boolean cut**, so there is no interior face to
/// get the winding of wrong. A cut would have to build the two reveals inside the opening,
/// and a reveal is four more faces whose corners are all new; two runs either side of a gap
/// are the same five faces twice, and they cannot be wrong in a way the ring is not already
/// right in. The two runs' end faces are the reveal — they are the faces a viewer standing
/// in the opening actually sees — so the gate is drawn by the same rule that draws the wall
/// and not by a second one.
///
/// Two composition rules decide what is built **around** a ring, and both are about where
/// one box stops against another:
///
/// - A lintel must run slightly **past** its posts, so that its own end faces are buried
///   inside them. Two boxes that stop on the same plane leave two coincident faces, and a
///   wall's gate pier shows that as a bright vertical line up the corner of the opening,
///   which is the one thing in the whole form that cannot be explained as a shadow.
/// - A coping should be **wider** than the wall it caps, not flush with it, because a cap
///   flush with its wall is a lid, and a lid throws no shadow line of its own along the top
///   of the band it caps. The coping is the dark line along the top of the wall, and the
///   overhang is where that line's shadow falls.
///
/// Every run of the ring is written with both of its ends, including the two ends that meet
/// at a corner. At a chamfer the pair is what closes the corner, and leaving them off would
/// open a notch on the outside of it that neither run's faces cover; at a square corner the
/// pair is coincident, so a caller who wants a clean corner should chamfer the ring rather
/// than rely on the rasterizer to choose between two faces on the same plane.
///
/// A ring of fewer than two corners is an empty run, and a run with no length is skipped
/// rather than built, because a degenerate run's outward normal is not a direction.
pub fn wall_ring(
    corners: &[[f64; 3]],
    low: f64,
    high: f64,
    thickness: f64,
    gaps: &[WallGap],
) -> PackedList {
    let capacity = corners
        .len()
        .saturating_add(gaps.len())
        .saturating_mul(WALL_FACES * 6);
    let mut points = Shaded::with_capacity(capacity);
    if corners.len() < 2 {
        return points.finish();
    }

    for (index, corner) in corners.iter().enumerate() {
        let from = *corner;
        let to = corners[(index + 1) % corners.len()];
        let outward = edge_normal(from, to);
        // Where this run is interrupted, as fractions of it, in order. The stops come in
        // pairs — the start of a gap and the end of it — so what is left between one pair
        // and the next is wall, and a pair whose two fractions are the same lengthens
        // nothing. Two gaps that meet leave a stretch of no length between them, and the
        // loop below skips a stretch that has none rather than building a run with no
        // direction in it.
        let mut stops: Vec<f64> = gaps
            .iter()
            .filter(|gap| gap.edge == index && gap.to > gap.from)
            .flat_map(|gap| [gap.from.clamp(0.0, 1.0), gap.to.clamp(0.0, 1.0)])
            .collect();
        stops.sort_by(f64::total_cmp);

        let mut start = 0.0;
        let mut read = 0;
        while read < stops.len() {
            let end = stops[read];
            if end > start {
                push_wall_run(
                    &mut points,
                    between(from, to, start),
                    between(from, to, end),
                    outward,
                    low,
                    high,
                    thickness,
                );
            }
            // Past the gap: the next run starts where this one ended, not where it began.
            if read + 1 < stops.len() {
                start = stops[read + 1];
            }
            read += 2;
        }
        if start < 1.0 {
            push_wall_run(
                &mut points,
                between(from, to, start),
                between(from, to, 1.0),
                outward,
                low,
                high,
                thickness,
            );
        }
    }
    points.finish()
}

/// A tent roof profile: a ridge at the local origin, a straight fall to either side, and the
/// last stretch of each slope turned up into an eave. It is the height function of a
/// [`heightfield`](crate::geometry::heightfield), so the ridge runs along the x axis and the
/// roof falls away along z.
///
/// # Invariants
///
/// The profile is a **tent**: the ridge at the local origin and a straight fall to either
/// side, with the last tenth of each slope turned **up** into an eave. That turn is the one
/// line that says which tradition a roof is built in — a straight eave is a European roof —
/// and the turn is what a photograph of one actually shows, so it is modelled rather than
/// left to a shader. It is a proportion of the half run rather than a fixed length because
/// the eave of a roof is the size of its tiles, and a roof built for its own size rather
/// than for this one's.
///
/// The tile channels are **not** geometry. A tile is a quarter of a metre and a roof's own
/// cell is a twentieth of the run, so a ripple modelled into this surface would be sampled
/// twice a cycle exactly at the distances a roof is looked at from, and would alias into a
/// moiré. The channels belong to the surface body instead, drawn from the world position,
/// where they can be faded out as a tile falls below a pixel — the same argument a woven
/// cloth is drawn from rather than modelled at.
///
/// The last two terms are the roof's age. The crest is the fraction of tiles at the ridge
/// that sit a little proud, the saddle is the slight drop a roof takes as it reaches the
/// eave, and the settle is a slow sag along the ridge, one cycle across the whole run,
/// because a roof that has stood for fifty years has one and a roof that has stood for none
/// is a drawing.
///
/// `length` is the half run, ridge to eave, and it must be positive: it is a divisor, and a
/// profile that divides by zero returns no surface at all rather than a flat one. Beyond
/// the eave the fall stops at `rise`, so the profile can be swept past its own edge without
/// folding over.
pub fn roof_ridge(length: f64, rise: f64) -> impl Fn(f64, f64) -> f64 {
    move |x: f64, z: f64| {
        let run = z.abs();
        // The fall is measured **down** from the ridge, so the ridge is the high point and
        // the eaves are the low ones. A profile whose height grows away from the ridge is
        // a valley dressed as a roof, and no placement can rescue it: the mesh is placed
        // by its own origin, which is the ridge, so a rising profile puts the eaves in the
        // sky and the ridge under the tiles.
        let fall = rise * (1.0 - (run / length).min(1.0));
        // The eave, turned up over the last tenth of the run.
        let turn = smoothstep(length * EAVE_SPAN, length, run) * rise * EAVE_TURN;
        // The crest, and the saddle as the slope reaches the eave.
        let crest = rise * CREST_LIFT * (1.0 - smoothstep(0.0, length * CREST_SPAN, run));
        let saddle = -rise * SADDLE * smoothstep(length * SADDLE_START, length, run);
        // The sag along the ridge, one cycle across the whole run.
        let settle = rise
            * SETTLE
            * (1.0 - smoothstep(0.0, length * SETTLE_SPAN, run))
            * ((x * std::f64::consts::TAU / length).cos() * 0.5 + 0.5);
        fall + turn + crest + saddle - settle
    }
}

/// The fraction of the half run over which the eave turns up.
const EAVE_SPAN: f64 = 0.9;
/// How far the eave turns up, as a fraction of the rise.
const EAVE_TURN: f64 = 0.2;
/// How much the tiles at the ridge stand proud, as a fraction of the rise.
const CREST_LIFT: f64 = 0.11;
/// The fraction of the half run over which the crest settles back into the slope.
const CREST_SPAN: f64 = 0.07;
/// How far the roof drops as it reaches the eave, as a fraction of the rise.
const SADDLE: f64 = 0.03;
/// The fraction of the half run at which the saddle begins.
const SADDLE_START: f64 = 0.94;
/// How far the ridge settles, as a fraction of the rise.
const SETTLE: f64 = 0.03;
/// The fraction of the half run over which the ridge settles out of the crest.
const SETTLE_SPAN: f64 = 0.17;

/// A pole between two points: a tapering tube, cut square at both ends.
///
/// # Invariants
///
/// The ring basis is built from the axis and one reference vector, and the reference is the
/// one of the two that is least in line with the axis, so the cross product that makes the
/// basis is never taken between two parallel vectors. A pole that happens to stand up
/// straight is the one that would otherwise have no basis at all, and it is the shape most
/// likely to be standing straight. The basis is right handed from the axis outwards, so the
/// winding of the band follows the axis from `from` to `to`.
///
/// The radius falls **linearly** from `radius_from` to `radius_to`, so one band of quads is
/// the whole surface and the taper cannot be a chord of something else. A taper that curves
/// needs a sweep per step, and a pole whose thickness changes faster than it is read is
/// being modelled at a resolution nothing will sample it at.
///
/// Each face's normal is taken at the **middle** of the two edges it joins rather than at
/// either. A hexagonal tube shaded by its corner normals is a hexagon with six lights in it,
/// and a pole that reads as a faceted crystal is worse than one that reads as smooth. The
/// ring's own sides are what the silhouette shows, so the count is the only knob: a dozen
/// sides for a post seen close, six for one seen across a yard.
///
/// The two ends are fanned to the axis, and a fan is only written where the radius there is
/// greater than nothing. They are two or three pixels each at any distance a pole is seen
/// from, and a pole with a hole in the top of it is a pipe; a fan at a radius of zero is not
/// a fan, and writing one would be six triangles of no area where the point already is.
///
/// A sweep with fewer than three sides is an empty run, and one whose two ends coincide is
/// a surface of no area. Both are drawn as nothing rather than refused, because a scene that
/// generated one generated it from its own numbers.
pub fn sweep(
    from: [f64; 3],
    to: [f64; 3],
    radius_from: f64,
    radius_to: f64,
    sides: usize,
) -> PackedList {
    let mut points = Shaded::with_capacity(sides * 12);
    if sides < 3 {
        return points.finish();
    }
    let span = subtract(to, from);
    let axis = normalized(span);
    // A ring basis about the axis. The reference is whichever of the two is least in line
    // with it, so a pole standing up straight — the common case — still has a basis.
    let reference = if dot(axis, [0.0, 1.0, 0.0]).abs() > 0.99 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let side = normalized(cross(axis, reference));
    let up = normalized(cross(side, axis));
    let at = |step: f64, corner: usize| -> [f64; 3] {
        let angle = corner as f64 / sides as f64 * std::f64::consts::TAU;
        let radius = radius_from + (radius_to - radius_from) * step;
        add(
            add(from, scale(span, step)),
            add(
                scale(side, radius * angle.cos()),
                scale(up, radius * angle.sin()),
            ),
        )
    };
    for corner in 0..sides {
        let next = (corner + 1) % sides;
        let middle = (corner as f64 + 0.5) / sides as f64 * std::f64::consts::TAU;
        let normal = normalized(add(scale(side, middle.cos()), scale(up, middle.sin())));
        quad_normal(
            &mut points,
            [
                at(0.0, corner),
                at(1.0, corner),
                at(1.0, next),
                at(0.0, next),
            ],
            normal,
        );
    }
    // The two cut ends, fanned to the axis.
    for (step, radius, normal) in [
        (0.0, radius_from, scale(axis, -1.0)),
        (1.0, radius_to, axis),
    ] {
        if radius <= 0.0 {
            continue;
        }
        let centre = add(from, scale(span, step));
        for corner in 0..sides {
            let next = (corner + 1) % sides;
            let corners = if step == 1.0 {
                [at(step, corner), at(step, next), centre]
            } else {
                [at(step, next), at(step, corner), centre]
            };
            for vertex in corners {
                points.push(vertex, normal, [0.0, 0.0]);
            }
        }
    }
    points.finish()
}

/// The profile of a turned vessel: a wide belly, a narrow neck and a rolled lip.
///
/// # Invariants
///
/// The inside is part of the same **closed** profile, so the interior is a real surface and
/// the rim is a lip rather than an edge. That is what a bowl is: a profile that goes out
/// along its underside, up its outside, over its rim, back down its inside and in along the
/// inside of its base. An open profile turned into a vessel gives an inside made of the
/// outside's back faces, and a seam at the rim where the two were joined by hand.
///
/// This is a profile and not a mesh. [`lathe`](crate::geometry::lathe) turns one, and a
/// vessel of a size nobody asked for is the profile scaled rather than a second list of
/// numbers, so nothing here wraps it: pass this to `lathe` with the segment count the
/// distance wants, which is the same argument a cube makes for a box.
pub const JAR: [[f64; 2]; 15] = [
    [0.000, 0.000],
    [0.115, 0.000],
    [0.205, 0.035],
    [0.270, 0.115],
    [0.298, 0.230],
    [0.290, 0.350],
    [0.255, 0.455],
    [0.205, 0.530],
    [0.185, 0.575],
    [0.200, 0.610],
    [0.196, 0.628],
    [0.172, 0.600],
    [0.165, 0.520],
    [0.100, 0.420],
    [0.000, 0.360],
];

/// The profile of a carrying basket: an open cone with a heavier foot.
///
/// # Invariants
///
/// It is closed on the same terms as [`JAR`] — a profile that begins and ends on the axis —
/// so the basket has an inside, its rim has thickness, and its floor is a surface rather
/// than a hole. The foot is heavier than the wall it stands on because a basket is
/// carried, and a basket that tapers away to nothing at the ground is a shape that has been
/// described rather than one that could be lifted.
pub const BASKET: [[f64; 2]; 15] = [
    [0.000, 0.000],
    [0.105, 0.000],
    [0.135, 0.018],
    [0.150, 0.050],
    [0.190, 0.150],
    [0.235, 0.270],
    [0.268, 0.372],
    [0.282, 0.400],
    [0.268, 0.392],
    [0.250, 0.360],
    [0.240, 0.330],
    [0.196, 0.215],
    [0.150, 0.115],
    [0.090, 0.050],
    [0.000, 0.035],
];

/// The profile of a stone mill: a thick disc with a raised rim.
///
/// # Invariants
///
/// It is closed on the same terms as [`JAR`], so the disc has a top and a bottom rather
/// than a rim and an edge. The raised rim is what the thing is for — a mill is a disc that
/// is held to keep what is ground on it from leaving — so the profile puts the last of its
/// height on the outside rather than rounding the whole disc off.
pub const MILL: [[f64; 2]; 9] = [
    [0.000, 0.000],
    [0.360, 0.000],
    [0.410, 0.030],
    [0.420, 0.120],
    [0.408, 0.150],
    [0.360, 0.140],
    [0.250, 0.132],
    [0.120, 0.118],
    [0.000, 0.108],
];

/// Returns the point that fraction of the way along the run from one point to another.
///
/// A fraction rather than a length, so that a gap written as a proportion of a run lands in
/// the same place whether the run is two metres or twenty.
fn between(from: [f64; 3], to: [f64; 3], fraction: f64) -> [f64; 3] {
    add(from, scale(subtract(to, from), fraction))
}
