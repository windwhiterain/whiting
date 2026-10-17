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

    /// Returns `around` points around the loop, point `i` at arc-length fraction `i / around`.
    ///
    /// # Invariants
    ///
    /// **`v` is a fraction of arc length**, the convention [`crate::curve`]'s `t` uses along a spine,
    /// so `v = 0` is the same named place for every section and a sweep's quads do not bunch where the
    /// section happens to be curved. It is `points[0]` for an [`Outline`] and `(+x, 0)` for an
    /// [`Section::Ellipse`], and `v` increases from there counter-clockwise in `(x, y)`.
    ///
    /// The last point is at `(around - 1) / around` and not at `1`: the loop is closed by the caller's
    /// wrap-around rather than by a vertex written twice. `around` is asserted at three or more — a
    /// ring of two points has no area and no inside.
    ///
    /// An ellipse's arc length has no closed form, so the [`Section::Ellipse`] arm builds a
    /// [`ELLIPSE_STEPS`]-step midpoint table of it per call and inverts each target by binary search
    /// and one interpolation inside the step: `O(ELLIPSE_STEPS + around · log ELLIPSE_STEPS)`. An
    /// [`Outline`]'s table is exact from its points and was measured once in [`Outline::new`], so the
    /// same inversion is `O(around · log points)` and allocates nothing.
    pub fn ring(&self, around: usize) -> Vec<Vec2> {
        assert!(
            around >= 3,
            "a ring of {around} points has no area and no inside"
        );
        match self {
            Self::Ellipse { x, y } => {
                assert!(
                    x.is_finite() && *x > 0.0 && y.is_finite() && *y >= 0.0,
                    "an ellipse needs a positive half axis along the normal, a non-negative one along \
                     the binormal, and both finite"
                );
                ellipse_ring(*x, *y, around)
            }
            Self::Outline(outline) => (0..around)
                .map(|index| outline.at(index as f32 / around as f32))
                .collect(),
        }
    }
}

/// The closed loop of an ellipse, sampled at `around` arc-length fractions.
///
/// # Invariants
///
/// `|dP/dθ|` is at least `min(x, y)`, which [`Section::ring`] asserts is positive, so every step of
/// the table carries a positive length and the inversion divides without a guard — including at
/// `y = 0`, where the speed vanishes only at `θ = 0` itself and the table samples step midpoints.
fn ellipse_ring(x: f32, y: f32, around: usize) -> Vec<Vec2> {
    let width = std::f32::consts::TAU / ELLIPSE_STEPS as f32;
    let mut cumulative = Vec::with_capacity(ELLIPSE_STEPS + 1);
    let mut total = 0.0;
    cumulative.push(0.0);
    for step in 0..ELLIPSE_STEPS {
        total += ellipse_speed(x, y, (step as f32 + 0.5) * width) * width;
        cumulative.push(total);
    }
    (0..around)
        .map(|index| {
            let target = index as f32 / around as f32 * total;
            let step = cumulative
                .partition_point(|&length| length < target)
                .saturating_sub(1)
                .min(ELLIPSE_STEPS - 1);
            let span = cumulative[step + 1] - cumulative[step];
            let theta = (step as f32 + (target - cumulative[step]) / span) * width;
            [x * theta.cos(), y * theta.sin()]
        })
        .collect()
}

/// `|dP/dθ|` for the ellipse `P(θ) = (x cos θ, y sin θ)`.
fn ellipse_speed(x: f32, y: f32, theta: f32) -> f32 {
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
