//! Height functions for terrain, lifted out of the scenes that first wrote them.
//!
//! Every landscape is the same arithmetic — a ramp, a creased noise, a floor cut into a
//! slope — and a scene that keeps that arithmetic to itself keeps a copy that differs from
//! the one beside it. What follows is the shared form, together with the reasoning that
//! came with it, because the reasoning is why the arithmetic is what it is.
//!
//! # Invariants
//!
//! - Every ramp here names its two bounds **in ascending order**, which is the contract
//!   [`document::geometry::smoothstep`] documents. A landscape read off a call with its
//!   bounds the other way round is one that cannot be read.
//! - Every function here is a pure function of its arguments. A heightfield takes its
//!   normal from the height at four neighbouring points, so a height that depends on
//!   anything else is a surface whose normal belongs to a different surface.

use document::geometry::{fract, fractal_noise, normalized, ridged_noise, smoothstep};

/// The second coordinate the crest profile along a ridge's axis is sampled at, held
/// constant so that the profile is one dimensional.
const PROFILE_PHASE: f64 = 0.5;

// ---------------------------------------------------------------------------
// The skyline
// ---------------------------------------------------------------------------

/// The profile of a crest line, in roughly 0.45 to 1.20.
///
/// # Invariants
///
/// A skyline is a line of **crests and saddles**, not a smooth wander, and the shape of
/// this function is what puts one there. Each term is a sawtooth folded at its midpoint,
/// and the product of two of them rises and falls at two different scales at once, so the
/// line crests and drops through a saddle rather than rolling.
///
/// The sawtooth is **smoothed before it is folded**, and that is the difference between a
/// crest line and a zigzag: a bare fold is a triangle wave, and a triangle wave along a
/// skyline reads as a row of saw teeth rather than as rock.
///
/// The two periods differ by a little under a factor of two, so the crests and the saddles
/// between them are not all the same size.
///
/// # When this is the profile and when a fractal is
///
/// There are two ways to make a line wander along a ridge's axis, and they are not the same
/// shape. This one is a **closed form of two periods**: it crests and drops through a saddle
/// at two fixed scales, forever, and it takes no seed because there is nothing in it to
/// randomise. A multi-octave fractal is the other, and it is what [`Ridge`] uses, because a
/// modelled mass wants detail at a dozen scales down to the size of a heightfield cell.
///
/// So: this is the profile for a crest line that has to read as **crests and saddles** —
/// a skyline drawn in the post stage, a horizon, anything the eye reads as a line — and a
/// fractal is the profile for a mass the eye reads as a **body**. A range whose along-axis
/// profile is a fractal still wants a crease across it, and one whose profile is this still
/// wants the cross creasing; neither substitutes for the other.
pub fn crest_profile(value: f64) -> f64 {
    let folded = 1.0 - (smoothstep(0.0, 1.0, fract(value * 0.055)) * 2.0 - 1.0).abs();
    let second = 1.0 - (smoothstep(0.0, 1.0, fract(value * 0.107 + 0.31)) * 2.0 - 1.0).abs();
    0.45 + 0.75 * folded * second
}

// ---------------------------------------------------------------------------
// The mass
// ---------------------------------------------------------------------------

/// A ridge mass: a crest line along an axis, creased across it, standing on a floor.
///
/// # Invariants
///
/// The crest line is sampled from noise **along the axis and along nothing else** — the
/// second coordinate is held constant, which is what makes the profile one dimensional.
/// Sampled across the ridge as well, the same noise becomes a field of swells whose high
/// points are wherever two coordinates happened to agree, and a range built from that is a
/// field rather than a line of mountains.
///
/// The two noises carry **different seeds, on purpose**. Two attempts to build this same
/// landscape independently both arrived at seed 131 for the profile along the axis and 139
/// for the crease across it, neither having seen the other's code. A shape two people reach
/// separately with the same constants is a shape the eye already has a name for, and
/// keeping the two seeds apart is what stops a range's crest line and its cross creasing
/// from agreeing with each other by accident, which is what a single seed gives.
///
/// The crease is floored so a gully running across the ridge is a **saddle** in the mass
/// rather than a hole clean through it, and the whole mass is floored so it has a base to
/// stand on instead of coming to nothing in the middle.
///
/// The profile along the axis is a multi-octave **fractal** and not [`crest_profile`],
/// because this is a mass the eye reads as a body and a body wants detail at a dozen scales
/// down to the size of a heightfield cell, where a closed form of two periods repeats. See
/// that function for when the closed form is the right one instead.
pub struct Ridge {
    /// The direction the crest line runs, in the ground plane.
    pub axis: [f64; 2],
    /// How fast the crest line wanders along the axis, per unit of ground.
    pub along_scale: f64,
    /// How fast the creasing runs across the ridge, on each ground axis.
    pub across_scale: [f64; 2],
    /// The height the mass stands at where both its profile and its crease are full.
    pub height: f64,
    /// The height the mass never falls below, so it has a base.
    pub floor: f64,
    /// How much of a full crease a gully across the ridge still stands at.
    pub crease_floor: f64,
    /// The seed of the profile along the axis.
    pub profile_seed: i64,
    /// The seed of the crease across the ridge, and never the profile's own seed.
    pub crease_seed: i64,
    /// How many octaves each of the two noises is summed over.
    pub octaves: usize,
}

impl Ridge {
    /// The height of this ridge's mass at a point, ungated.
    ///
    /// The gate that decides where the mass stands — how far out it gathers, which quarter
    /// it fills, how far it tapers before the field's edge — belongs to the caller, so that
    /// one mass can be used as a range, as a wall or as a far body.
    pub fn height_at(&self, point: [f64; 2]) -> f64 {
        let axis = normalized([self.axis[0], self.axis[1], 0.0]);
        let along = point[0] * axis[0] + point[1] * axis[1];
        let profile = fractal_noise(
            [along * self.along_scale, PROFILE_PHASE],
            self.octaves,
            self.profile_seed,
        );
        let crease = ridged_noise(
            [
                point[0] * self.across_scale[0],
                point[1] * self.across_scale[1],
            ],
            self.octaves,
            self.crease_seed,
        );
        let creasing = self.crease_floor + (1.0 - self.crease_floor) * crease;
        self.floor + self.height * profile * creasing
    }
}

// ---------------------------------------------------------------------------
// The cut into it
// ---------------------------------------------------------------------------

/// The signed distance to an axis-aligned rectangle centred on the origin: negative inside.
///
/// # Invariants
///
/// Inside the rectangle the distance is the distance to the nearest **edge**, and outside
/// it is the distance to the nearest point of the rectangle's own outline, and the two
/// agree on the boundary. A caller easing over this distance therefore changes its mind
/// at the rectangle's edge rather than somewhere inside or outside it, which is the only
/// place a blend can change its mind without being visible.
pub fn box_distance(point: [f64; 2], half: [f64; 2]) -> f64 {
    let dx = point[0].abs() - half[0];
    let dz = point[1].abs() - half[1];
    let outside = (dx.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt();
    outside + dx.max(dz).min(0.0)
}

/// Cuts a level platform into a slope, easing from the level out to `radius`.
///
/// # Invariants
///
/// The blend is a smoothstep of the **distance** to the platform's edge, and not a mask on
/// the rectangle, and that is the whole of why this works. A mask multiplies the land down
/// toward the level inside the rectangle and simply stops at the edge, so the bank begins
/// as a discontinuity — and a heightfield takes its normal from its own height function,
/// so a discontinuity has no normal to give: the bank shades as a hard line where the
/// paving meets the ground, and the ground's own normal there disagrees with the surface
/// it belongs to. Easing over a distance instead makes the bank a surface the normal is
/// true to, rising out of the paving as one continuous thing.
pub fn bench(land: f64, level: f64, distance: f64, radius: f64) -> f64 {
    land + (level - land) * (1.0 - smoothstep(0.0, radius, distance))
}

/// Quantises a height into steps, the riser a smooth turn over the top of each step.
///
/// # Invariants
///
/// A bare `floor` **tears** the surface at every level, and the heightfield's normals are
/// taken from the height function itself, so a torn riser shades as a stripe of noise
/// rather than as a bank. Holding the tread flat for `flat` of each step and turning over
/// the whole of the rest gives the riser a width in the height function, which is the only
/// place a riser can have a width at all.
pub fn terrace(value: f64, step: f64, flat: f64) -> f64 {
    let quantised = value / step;
    let whole = quantised.floor();
    let part = quantised - whole;
    (whole + smoothstep(flat, 1.0, part)) * step
}

/// The slope of a height function at a point, by central difference.
///
/// # Invariants
///
/// `step` must be the **heightfield's own cell size**, and for the same function: shading
/// and geometry measured over different distances disagree, and the one that disagrees is
/// the one the eye finds first as a hillside that is not lit like the slope it is.
///
/// The surface is the function itself, sampled through the argument, so a caller that keeps
/// a second copy of the height for its shading has to keep it identical — the argument is
/// the only definition of the surface there is.
pub fn slope(heights: impl Fn(f64, f64) -> f64, point: [f64; 2], step: f64) -> f64 {
    let (x, z) = (point[0], point[1]);
    let across = heights(x + step, z) - heights(x - step, z);
    let along = heights(x, z + step) - heights(x, z - step);
    (across * across + along * along).sqrt() / (2.0 * step)
}

// ---------------------------------------------------------------------------
// The valley
// ---------------------------------------------------------------------------

/// A valley: a long fall, a range at its head, two lower walls, a far body beyond it, and
/// an undulation under the lot.
///
/// The fall runs along the ground `z`, rising from the open end at positive `z` to the head
/// at negative `z`; the walls close the two sides of that fall, and the ground between them
/// is the valley's floor. Every ramp is measured as a distance from a boundary rather than
/// as a signed coordinate, so a caller reads the shape of the place off the numbers and
/// not off their signs.
///
/// # Invariants
///
/// The parts divide the work between them rather than piling onto one height. The fall is
/// the long one, and the range is gathered out of the head of it. The walls are **lower**
/// than the range, so the valley closes on a range and a rise rather than on a crater rim.
/// The far body is lower than both, because a body for the mist to lie against has to be
/// something the mist can lie against rather than another mountain to be scaled by.
///
/// The undulation is gated by the height already reached, and that gate is the detail the
/// rest of the structure depends on: five metres of wrinkle is nothing on a hundred metre
/// face, and leaving it in is what makes a mountain read as noise.
pub struct Valley {
    /// Half the width of the ground this valley is built on.
    pub extent: f64,
    /// The height the fall is at the open end of the valley.
    pub fall_low: f64,
    /// The height the fall reaches where it gathers into the range.
    pub fall_high: f64,
    /// How much the fall steepens as it rises, above linear.
    pub fall_steepening: f64,
    /// The range closing the head of the valley.
    pub range: Ridge,
    /// The distances from the open end at which the range begins and finishes gathering.
    pub range_ramp: [f64; 2],
    /// The two walls closing the sides of the fall: the first on the negative ground `x`,
    /// the second on the positive ground `x`.
    pub walls: [Ridge; 2],
    /// The distances across the valley at which the walls begin and finish gathering.
    pub wall_ramp: [f64; 2],
    /// The low body beyond the open end, for weather to lie against.
    pub far_body: Ridge,
    /// The distances along the fall at which the far body begins and finishes rising.
    pub far_ramp: [f64; 2],
    /// The height of the undulation on open ground.
    pub undulation: f64,
    /// How fast that undulation runs, on each ground axis.
    pub undulation_scale: f64,
    /// The heights already reached at which the undulation fades out entirely.
    pub undulation_gate: [f64; 2],
    /// How many octaves the undulation is summed over.
    pub undulation_octaves: usize,
    /// The seed of the undulation, and of nothing else in this valley.
    pub undulation_seed: i64,
}

impl Valley {
    /// The height of the ground at a point, before anything is cut into it.
    pub fn height_at(&self, point: [f64; 2]) -> f64 {
        let (x, z) = (point[0], point[1]);
        // The long fall, up the valley from its open end and steepening as it rises.
        let up = (self.extent - z) / (2.0 * self.extent);
        let fall = self.fall_low + (self.fall_high - self.fall_low) * up.powf(self.fall_steepening);

        // The range at the head of the valley, gathered by a ramp measured from the open
        // end, so the mass is nowhere near the floor and the fall carries the eye to it.
        let mass = smoothstep(self.range_ramp[0], self.range_ramp[1], self.extent - z)
            * self.range.height_at(point);

        // The two side walls, each gathered on **its own** side of the centre line.
        //
        // Gating both on the same `x.abs()` and adding them would put a point on the west
        // side of the valley under the east wall as well, so the two would differ only in the
        // shape of their noise and the valley would carry twice the wall it was given on
        // every side. Each wall reads the distance from the centre line **outward into its
        // own half**, which is zero on the other side and therefore gathers nothing there.
        let wall = |across: f64| smoothstep(self.wall_ramp[0], self.wall_ramp[1], across);
        let walls = wall(-x.min(0.0)) * self.walls[0].height_at(point)
            + wall(x.max(0.0)) * self.walls[1].height_at(point);

        // The low body beyond the open end, so the far horizon has something behind the
        // weather rather than the weather standing on nothing.
        let far =
            smoothstep(self.far_ramp[0], self.far_ramp[1], z) * self.far_body.height_at(point);

        // The undulation that keeps open ground from being a ramp, faded out where the
        // mass is already high enough for five metres of wrinkle to be nothing at all.
        let ground = fall + mass;
        let undulation = (1.0
            - smoothstep(self.undulation_gate[0], self.undulation_gate[1], ground))
            * self.undulation
            * fractal_noise(
                [x * self.undulation_scale, z * self.undulation_scale],
                self.undulation_octaves,
                self.undulation_seed,
            );

        ground + walls + far + undulation
    }
}

// ---------------------------------------------------------------------------
// The gates
// ---------------------------------------------------------------------------

/// How much of a point's ground survives, falling to nothing before the field's own edge.
///
/// # Invariants
///
/// A heightfield's silhouette has to fall away to nothing **before** the field's own edge,
/// or the far ground is a cliff of cut earth standing against the sky, and the only thing
/// that can hide a cliff is what is drawn beyond it. So any scene that draws its far ranges
/// as weather — mist, ink, a painted band of the air's own colour — needs the modelled
/// ground to die down to a crest before the mesh stops, and the two then meet without
/// either being visible as the edge of a mesh. This is the most reusable thing here: every
/// scene that ends in weather needs it, and a scene that does not have it has a stage set
/// standing on a table.
///
/// `start` and `end` are the distances from the origin at which the taper begins and
/// finishes, taken on whichever ground axis is further out, and `end` belongs inside the
/// field.
pub fn edge_taper(point: [f64; 2], start: f64, end: f64) -> f64 {
    1.0 - smoothstep(start, end, point[0].abs().max(point[1].abs()))
}

/// How much of a point's ground is in the quarter `corner` names, and in no other quarter.
///
/// # Invariants
///
/// High ground confined to a **quarter** is a valley. High ground all the way round is a
/// **bowl**, and a bowl puts its own wall between a camera and its subject on every side —
/// which is a hole in the world rather than a valley, and a hole that swallows any camera
/// set on the open side. The mass belongs in the quarter the scene is looked from, and
/// nowhere else, so the place has a floor, a wall and a way out.
///
/// `fade` is the distance over which the gate falls away outside the corner, on each ground
/// axis at once, so the ground thins to nothing diagonally out of the quarter rather than
/// along its edges alone.
pub fn quarter(point: [f64; 2], corner: [f64; 2], fade: f64) -> f64 {
    (1.0 - smoothstep(corner[0] - fade, corner[0], point[0]))
        * (1.0 - smoothstep(corner[1] - fade, corner[1], point[1]))
}
