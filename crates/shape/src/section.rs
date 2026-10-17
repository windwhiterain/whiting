//! A cross-section: the closed loop a sweep draws in the plane of a spine frame.
//!
//! # The one idea
//!
//! **A section is a shape, and the sweep owns its size.** There is no scale, no taper and no curve of
//! `t` on a [`Section`]: a branch's taper, a culm's internodes and a crown's reach are one scalar law
//! applied along the spine, and a section that narrows keeps its aspect ratio by construction. The two
//! arms below are therefore the whole vocabulary — a circle and a blade are one ellipse at different
//! aspect ratios, and every other section is an authored outline. (D4 in
//! `.agents/proposals/shape-primitives.md` sketches the same sections with a size curve attached to
//! each; the size is the sweep's, which is the next piece of this crate.)
//!
//! # The plane, and what `v` means
//!
//! A section's `x` is the frame's `normal` and its `y` the frame's `binormal`, and
//! [`crate::curve::Frame`] defines `binormal = tangent × normal`, so `(x, y, tangent)` is
//! right-handed and a loop wound counter-clockwise in `(x, y)` faces along `+tangent`.
//! **`v` increases counter-clockwise for every section**, and [`Section::ring`] is where that is
//! enforced: an [`Outline`] normalises its winding on the way in, because the winding of an authored
//! list is a representation choice and a section is a shape.
//!
//! # What is not checked
//!
//! Self-intersection, exactly as the mesh predicates in this crate's plan do not check it: they are
//! counts over an edge table, and an answer here costs an ear-clip and a pairwise clip rather than a
//! predicate one calls in a loop. The checks that *are* here are the ones whose failure would make
//! the arc-length table a division by zero rather than a difference of taste.

use crate::math::{Vec2, clamp, cross, distance, lerp};

/// How many uniform parameter steps an ellipse's arc-length table holds.
///
/// # Invariants
///
/// 1024 is a resolution, not a promise of accuracy: the table's error falls with the step, and the
/// probe measures the fraction error it produces against an independent 200 000-step integration.
const ELLIPSE_STEPS: usize = 1024;

/// A closed loop of authored points, with its arc length measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    /// The points in order, wound counter-clockwise, the closing edge implied.
    points: Vec<Vec2>,
    /// Cumulative arc length at each point plus the closing value; `len == points.len() + 1`.
    cumulative: Vec<f32>,
    /// The total length of the closed loop.
    perimeter: f32,
}

/// Why a section could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionError {
    /// Fewer than three points: there is no area inside the loop.
    TooFewPoints,
    /// A point holds a coordinate that is not finite.
    NotFinite,
    /// Two consecutive points stand at the same place, so the edge between them has no direction.
    RepeatedPoint,
}

/// A cross-section: a closed loop in the plane of the spine frame's `normal` and `binormal`.
#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    /// An ellipse of the given half axes.
    ///
    /// # Invariants
    ///
    /// `x` must be positive and `y` non-negative; zero is a ribbon, a loop of no area that still has
    /// two sides, and a negative half axis is not a shape. [`Section::ring`] asserts it, and
    /// [`Section::circle`] is `Ellipse { x: 1.0, y: 1.0 }`.
    Ellipse {
        /// The half axis along the frame's `normal`.
        x: f32,
        /// The half axis along the frame's `binormal`.
        y: f32,
    },
    /// A closed authored outline, wound counter-clockwise.
    Outline(Outline),
}

impl Outline {
    /// Builds a closed loop from its points, with the closing edge implied.
    ///
    /// # Invariants
    ///
    /// The points are **distinct consecutive points**, and the edge from the last back to the first is
    /// one the list does not spell out. Fewer than three points is [`SectionError::TooFewPoints`], a
    /// coordinate that is not finite is [`SectionError::NotFinite`], and two equal consecutive points
    /// — the closing pair included — are [`SectionError::RepeatedPoint`]. A zero-length edge is data
    /// error rather than a shape, and refusing it here is what lets the arc-length inversion divide by
    /// every edge length without a guard.
    ///
    /// The winding is **normalised to counter-clockwise, keeping `points[0]` first**: a clockwise list
    /// is stored as `[p0, pₙ₋₁, …, p1]`. The winding an author typed is a representation choice, and a
    /// section is a shape, so `v` has to increase the same way around every loop or a sweep's faces
    /// turn inside out on a clockwise one.
    pub fn new(points: &[Vec2]) -> Result<Self, SectionError> {
        if points.len() < 3 {
            return Err(SectionError::TooFewPoints);
        }
        if points.iter().flatten().any(|value| !value.is_finite()) {
            return Err(SectionError::NotFinite);
        }
        let count = points.len();
        if (0..count).any(|index| points[index] == points[(index + 1) % count]) {
            return Err(SectionError::RepeatedPoint);
        }
        let mut points = points.to_vec();
        if shoelace(&points) < 0.0 {
            points[1..].reverse();
        }
        let mut cumulative = Vec::with_capacity(count + 1);
        let mut perimeter = 0.0;
        cumulative.push(0.0);
        for index in 0..count {
            perimeter += distance(points[index], points[(index + 1) % count]);
            cumulative.push(perimeter);
        }
        Ok(Self {
            points,
            cumulative,
            perimeter,
        })
    }

    /// The loop's points, wound counter-clockwise, with the closing edge implied.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// The closed loop's total length.
    pub fn perimeter(&self) -> f32 {
        self.perimeter
    }

    /// The point at an arc-length fraction around the loop.
    ///
    /// # Invariants
    ///
    /// Every edge has positive length — [`Outline::new`] refuses the ones that do not — so the
    /// division by the edge length needs no guard, and the perimeter is positive for the same reason.
    fn at(&self, v: f32) -> Vec2 {
        let count = self.points.len();
        let target = clamp(v, 0.0, 1.0) * self.perimeter;
        let index = self
            .cumulative
            .partition_point(|&length| length < target)
            .saturating_sub(1)
            .min(count - 1);
        let start = self.points[index];
        let end = self.points[(index + 1) % count];
        let span = self.cumulative[index + 1] - self.cumulative[index];
        let within = (target - self.cumulative[index]) / span;
        if within >= 1.0 {
            // The seam: `v = 1` lands on the first point and has to land on it bit-identically rather
            // than through a lerp that rounds (shape-primitives.md, open question 3).
            return end;
        }
        lerp(start, end, within)
    }
}

impl Section {
    /// A circle of radius one.
    pub fn circle() -> Self {
        Self::ellipse(1.0, 1.0)
    }

    /// An ellipse of the given half axes, in the frame's `normal` and `binormal`.
    ///
    /// # Invariants
    ///
    /// `x` must be positive and `y` non-negative, and both finite; [`Section::ring`] asserts it rather
    /// than the constructor, because a ring is the only query that would have to divide by the shape.
    pub fn ellipse(x: f32, y: f32) -> Self {
        Self::Ellipse { x, y }
    }

    /// A lens of the given half-width and thickness.
    ///
    /// # Invariants
    ///
    /// **A flat section, not a zero-thickness sheet**: a blade drawn as a loop has to be a closed
    /// solid or it has no inside for a sweep to fill, so the section is an ellipse of half axes
    /// `half_width` and `thickness`. A `thickness` of `0.0` is the ribbon — a double-sided sheet, lit
    /// from either side — and `half_width` must be positive, as for any [`Section::ellipse`].
    pub fn flat(half_width: f32, thickness: f32) -> Self {
        Self::ellipse(half_width, thickness)
    }

    /// A closed loop from authored points, wound counter-clockwise whatever order they were given in.
    ///
    /// See [`Outline::new`] for what it refuses.
    pub fn outline(points: &[Vec2]) -> Result<Self, SectionError> {
        Outline::new(points).map(Self::Outline)
    }

    /// The point at an arc-length fraction around the loop.
    ///
    /// # Invariants
    ///
    /// **`v` is a fraction of arc length**, the convention [`crate::curve`]'s `t` uses along a spine, so
    /// `v = 0` is the same named place for every section and a sweep's quads do not bunch where the
    /// section happens to be curved. It is `points[0]` for an [`Outline`] and `(+x, 0)` for an
    /// [`Section::Ellipse`], and `v` increases from there counter-clockwise in `(x, y)`.
    ///
    /// **`v` wraps**, because a loop is periodic: `v = 1` is `v = 0` bit for bit, `v = -0.1` is
    /// `v = 0.9`, and a caller holding an angle does not normalise it first. That is also what makes a
    /// loop's seam a place rather than an approximation.
    pub fn at(&self, v: f32) -> Vec2 {
        assert!(v.is_finite(), "an arc-length fraction is a number");
        self.track().at(v.rem_euclid(1.0))
    }

    /// How far the section reaches from its own origin in a direction.
    ///
    /// # Invariants
    ///
    /// - **The section's own plane is the frame it is swept in**, so a direction is read in `(x, y)`: the
    ///   section's `+x` half axis is on `+x`, as [`Section::at`]'s first point is.
    /// - **An [`Section::Ellipse`] answers exactly**: the ray `t · d` is on the ellipse where
    ///   `(t·d.x / x)² + (t·d.y / y)² = 1`, so the reach is `1 / √((d.x/x)² + (d.y/y)²)` — no sampling, and the
    ///   same formula covers a circle, a flat lens and any aspect ratio.
    /// - **An [`Section::Outline`] answers where the ray first crosses an edge**, walked in order, so a
    ///   refined envelope costs one pass over the authored points and not a table.
    /// - **A direction with no length answers `0.0`**, because there is no direction to answer about.
    pub fn reach(&self, direction: Vec2) -> f32 {
        let size = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        if !size.is_finite() || size < 1.0e-9 {
            return 0.0;
        }
        let unit = [direction[0] / size, direction[1] / size];
        match self {
            Section::Ellipse { x, y } => {
                let one = unit[0] / x;
                let two = unit[1] / y;
                1.0 / (one * one + two * two).sqrt()
            }
            Section::Outline(outline) => {
                let points = outline.points();
                let mut nearest = f32::MAX;
                for index in 0..points.len() {
                    let start = points[index];
                    let end = points[(index + 1) % points.len()];
                    let edge = [end[0] - start[0], end[1] - start[1]];
                    let base = cross(unit, edge);
                    if base.abs() < 1.0e-12 {
                        continue;
                    }
                    let reach = cross(start, edge) / base;
                    let along = cross(start, unit) / base;
                    if reach > 0.0 && (0.0..=1.0).contains(&along) {
                        nearest = nearest.min(reach);
                    }
                }
                if nearest == f32::MAX { 0.0 } else { nearest }
            }
        }
    }

    /// `around` points around the loop, point `i` at `v = i / around`.    ///
    /// # Invariants
    ///
    /// The loop is closed by the caller's wrap-around, so the last point is at
    /// `(around - 1) / around` and there is no vertex written twice at the seam. `around` is asserted at
    /// three or more: a ring of two points has no area and no inside.
    ///
    /// This is [`Section::at`] at uniform fractions and not a second sampling rule, so the two cannot
    /// disagree, and an ellipse's table is measured **once per call** either way:
    /// `O(ELLIPSE_STEPS + around · log ELLIPSE_STEPS)`.
    pub fn ring(&self, around: usize) -> Vec<Vec2> {
        assert!(
            around >= 3,
            "a ring of {around} points has no area and no inside"
        );
        let track = self.track();
        (0..around)
            .map(|index| track.at(index as f32 / around as f32))
            .collect()
    }

    /// This section's sampling rule, with its arc-length table measured once.
    ///
    /// # Invariants
    ///
    /// An ellipse's half axes are asserted here rather than in its constructor, because a ring is the
    /// only query that has to divide by the shape and this is the one place both queries pass through.
    fn track(&self) -> Track<'_> {
        match self {
            Self::Ellipse { x, y } => {
                assert!(
                    x.is_finite() && *x > 0.0 && y.is_finite() && *y >= 0.0,
                    "an ellipse needs a positive half axis along the normal, a non-negative one along \
                     the binormal, and both finite"
                );
                Track::Ellipse(EllipseTrack::new(*x, *y))
            }
            Self::Outline(outline) => Track::Outline(outline),
        }
    }
}

/// One section's sampling rule, with its table measured once per call.
enum Track<'a> {
    Ellipse(EllipseTrack),
    Outline(&'a Outline),
}

impl Track<'_> {
    /// The point at an arc-length fraction in `[0, 1)`.
    fn at(&self, v: f32) -> Vec2 {
        match self {
            Self::Ellipse(ellipse) => ellipse.at(v),
            Self::Outline(outline) => outline.at(v),
        }
    }
}

/// An ellipse's arc-length table and the inversion that samples it.
///
/// # Invariants
///
/// `|dP/dθ|` is at least `min(x, y)`, which [`Section::track`] asserts is positive, so every step of the
/// table carries a positive length and the inversion divides without a guard — including at `y = 0`,
/// where the speed vanishes only at `θ = 0` itself and the table samples step midpoints.
///
/// **The table is accumulated in `f64` and only the sampled coordinates are cast back.** A step of the
/// table is a difference of two numbers near the perimeter, so storing it in `f32` rounds every step's
/// length to an ulp of the whole loop — about `4e-5` of it, which the probe sees as a circle whose chords
/// differ in the fifth decimal.
struct EllipseTrack {
    x: f64,
    y: f64,
    /// The uniform parameter step the table was measured at.
    width: f64,
    /// Cumulative arc length at each step: `ELLIPSE_STEPS + 1` entries, starting at zero.
    cumulative: Vec<f64>,
}

impl EllipseTrack {
    fn new(x: f32, y: f32) -> Self {
        let (x, y) = (x as f64, y as f64);
        let width = std::f64::consts::TAU / ELLIPSE_STEPS as f64;
        let mut cumulative = Vec::with_capacity(ELLIPSE_STEPS + 1);
        let mut total = 0.0;
        cumulative.push(0.0);
        for step in 0..ELLIPSE_STEPS {
            let midpoint = (step as f64 + 0.5) * width;
            total += ellipse_speed(x, y, midpoint) * width;
            cumulative.push(total);
        }
        Self {
            x,
            y,
            width,
            cumulative,
        }
    }

    /// The point at an arc-length fraction in `[0, 1)`.
    fn at(&self, v: f32) -> Vec2 {
        let target = f64::from(v) * self.cumulative[ELLIPSE_STEPS];
        let step = self
            .cumulative
            .partition_point(|&length| length < target)
            .saturating_sub(1)
            .min(ELLIPSE_STEPS - 1);
        let span = self.cumulative[step + 1] - self.cumulative[step];
        let within = if span > 0.0 {
            (target - self.cumulative[step]) / span
        } else {
            0.0
        };
        let theta = (step as f64 + within) * self.width;
        [(self.x * theta.cos()) as f32, (self.y * theta.sin()) as f32]
    }
}

/// `|dP/dθ|` for the ellipse `P(θ) = (x cos θ, y sin θ)`, in the table's own `f64`.
fn ellipse_speed(x: f64, y: f64, theta: f64) -> f64 {
    (x * theta.sin()).hypot(y * theta.cos())
}

/// The signed area of a closed polygon, positive for the counter-clockwise winding.
fn shoelace(points: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        twice += cross(points[index], points[next]);
    }
    0.5 * twice
}
