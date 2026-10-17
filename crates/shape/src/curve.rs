//! A cubic Hermite polyline: the one curve a spine, a leaf's midrib and an envelope are.
//!
//! # The one idea
//!
//! **A shape is its control points.** A [`Jet`] is a position and a derivative, and the derivative at
//! a shared knot is data rather than something derived from its neighbours — so the chain is smooth
//! by construction and there is no reconciliation pass. This is the argument `patch::hermite` makes
//! for its surface one dimension up, and it is why this is Hermite rather than Catmull-Rom: a
//! Catmull-Rom polygon recomputes its tangents from the positions, so inserting a control point changes
//! the tangents at its neighbours and an edit is no longer local.
//!
//! # `t` is an arc-length fraction
//!
//! [`Curve::at`] takes a fraction of the curve's **length**, not of its knot index. Every authored
//! number a caller writes is a fraction of real length — the top third of a culm, the radius at 70%,
//! an envelope's reach — and `mountain::math::span` makes the same argument for a massif.
//!
//! The cost, stated rather than discovered: `t` is not affine in the knot parameter, so an edit that
//! changes a segment's length moves the physical point a given `t` names. That is accepted because the
//! alternative is worse in the direction that matters — a knot-parameter `t` moves whenever a control
//! point is inserted anywhere before it.
//!
//! # The frame is transported, and its invariants are checkable
//!
//! [`Curve::frame`] returns a tangent and two perpendicular directions carried along the curve so that
//! they do not spin. Two invariants are the contract, and neither names an algorithm:
//!
//! 1. **a straight curve's frame is constant**;
//! 2. **an inflection does not flip the frame's normal** — which is exactly what a Frenet frame does,
//!    and the reason a Frenet frame is not offered as an option.
//!
//! # What a split does to the parameterisation
//!
//! [`Curve::split`] and [`Curve::insert`] are exact: each half is the original cubic restricted to a
//! sub-interval, so the geometry is identical. The *stored* derivative magnitudes at the new knot
//! differ from each other by the interval split, because each half carries its own normalised
//! parameter. Nothing a caller can observe through this API depends on the parameterisation's speed —
//! every query is by arc length — so the difference is recorded here rather than reconciled.

use crate::hermite;
use crate::math::{Vec3, clamp, cross3, dot3, length3, normalized3, subtract};

/// One knot: where the curve is, and its derivative with respect to the knot parameter.
///
/// # Invariants
///
/// The tangent is **not** a unit vector: its magnitude is part of the shape, in the same way a cubic
/// Bezier's handle length is. Normalising it here would delete the bulge between two knots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jet {
    /// Where the knot is.
    pub position: Vec3,
    /// The derivative there, with respect to the knot parameter.
    pub tangent: Vec3,
}

impl Jet {
    /// Builds a knot from a position and a derivative.
    pub fn new(position: Vec3, tangent: Vec3) -> Self {
        Self { position, tangent }
    }
}

/// Why a curve could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveError {
    /// Fewer than two knots: there is nothing to evaluate between.
    TooFewKnots,
    /// A knot holds a value that is not finite.
    NotFinite,
    /// Two consecutive knots stand at the same place, so that segment has no direction.
    CoincidentKnots,
}

/// A frame on the curve: the direction it goes, and two directions perpendicular to it.
///
/// # Invariants
///
/// `binormal` is `tangent × normal`, so `(tangent, normal, binormal)` is right-handed and a section
/// drawn in this frame keeps its handedness along the whole curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The unit direction of travel.
    pub tangent: Vec3,
    /// The transported direction a section's `x` is measured from.
    pub normal: Vec3,
    /// `tangent × normal`.
    pub binormal: Vec3,
}

impl Frame {
    fn of(tangent: Vec3, normal: Vec3) -> Self {
        let tangent = normalized3(tangent);
        let normal = perpendicular_to(normal, tangent);
        let binormal = cross3(tangent, normal);
        Self {
            tangent,
            normal,
            binormal,
        }
    }
}

/// One station along a curve: its parameter, its position, and its frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// The arc-length fraction, `0` at the first knot and `1` at the last.
    pub t: f32,
    /// The position there.
    pub position: Vec3,
    /// The frame there.
    pub frame: Frame,
}

/// The nearest point on a curve to somewhere else: what a search is for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Closest {
    /// The arc-length fraction of the nearest point.
    pub t: f32,
    /// The nearest point itself.
    pub position: Vec3,
    /// How far the query point is from it.
    pub distance: f32,
}

/// How many positions per segment [`Curve::closest`] brackets its answer with.
pub const CLOSEST_SAMPLES: usize = 16;

/// How many golden-section steps [`Curve::closest`] narrows the bracket with.
pub const CLOSEST_STEPS: usize = 24;

/// The squared distance between two points: what a nearest-point search compares.
fn squared_distance(one: Vec3, other: Vec3) -> f32 {
    let step = subtract(one, other);
    dot3(step, step)
}

/// One segment as the cubic Bezier every exporter speaks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bezier {
    /// The start.
    pub p0: Vec3,
    /// The start's handle: `p0 + tangent / 3`.
    pub p1: Vec3,
    /// The end's handle: `p3 − tangent / 3`.
    pub p2: Vec3,
    /// The end.
    pub p3: Vec3,
}

/// One cubic Hermite segment: two ends and the derivative at each.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    p0: Vec3,
    m0: Vec3,
    p1: Vec3,
    m1: Vec3,
}

impl Segment {
    fn at(&self, u: f32) -> Vec3 {
        let [h00, h10, h01, h11] = hermite::value(u);
        add(
            add(scale(self.p0, h00), scale(self.m0, h10)),
            add(scale(self.p1, h01), scale(self.m1, h11)),
        )
    }

    fn derivative(&self, u: f32) -> Vec3 {
        let [d00, d10, d01, d11] = hermite::slope(u);
        add(
            add(scale(self.p0, d00), scale(self.m0, d10)),
            add(scale(self.p1, d01), scale(self.m1, d11)),
        )
    }

    fn second(&self, u: f32) -> Vec3 {
        let [dd00, dd10, dd01, dd11] = hermite::second(u);
        add(
            add(scale(self.p0, dd00), scale(self.m0, dd10)),
            add(scale(self.p1, dd01), scale(self.m1, dd11)),
        )
    }

    /// The length of the whole segment, by Gauss-Legendre quadrature.
    fn length(&self) -> f32 {
        self.length_to(1.0)
    }

    /// The length of the segment from `0` to `u`.
    ///
    /// # Invariants
    ///
    /// Eight Gauss points per call: a cubic's speed is the magnitude of a quadratic, and eight points
    /// integrate that to well under `1e-5` of a segment.
    fn length_to(&self, u: f32) -> f32 {
        let half = 0.5 * u;
        let mut total = 0.0;
        for (node, weight) in GAUSS_NODES.iter().zip(GAUSS_WEIGHTS.iter()) {
            let s = half * (node + 1.0);
            total += weight * length3(self.derivative(s));
        }
        total * half
    }

    /// The segment-local parameter whose arc length from the start is `target`.
    ///
    /// Bisection rather than Newton: the length is monotone in `u`, so thirty-two halvings reach
    /// `1e-9` in `u` with no derivative to divide by, and a constant number of steps is what keeps the
    /// query O(1).
    fn invert(&self, target: f32) -> f32 {
        if self.length() <= 0.0 {
            return 0.0;
        }
        let mut low = 0.0;
        let mut high = 1.0;
        for _ in 0..32 {
            let mid = 0.5 * (low + high);
            if self.length_to(mid) < target {
                low = mid;
            } else {
                high = mid;
            }
        }
        0.5 * (low + high)
    }

    fn bezier(&self) -> Bezier {
        Bezier {
            p0: self.p0,
            p1: add(self.p0, scale(self.m0, 1.0 / 3.0)),
            p2: add(self.p1, scale(self.m1, -1.0 / 3.0)),
            p3: self.p1,
        }
    }
}

/// A cubic Hermite polyline.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    segments: Vec<Segment>,
    /// Cumulative arc length at each knot; `cumulative.len() == segments.len() + 1`.
    cumulative: Vec<f32>,
    /// The transported frame at [`FRAME_SAMPLES`] steps per segment, plus the end.
    samples: Vec<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    position: Vec3,
    tangent: Vec3,
    normal: Vec3,
}

/// How many frame samples one segment holds. Eight is the resolution [`Curve::frame`] interpolates
/// over, and the probe measures what it costs rather than this constant asserting it.
const FRAME_SAMPLES: usize = 8;

impl Curve {
    /// Builds a curve from its knots.
    ///
    /// # Invariants
    ///
    /// Consecutive knots share their derivative, so the curve is smooth at every interior knot by
    /// construction. Two knots at the same place are refused rather than given an arbitrary direction:
    /// the segment between them has no shape to draw.
    pub fn new(jets: Vec<Jet>) -> Result<Self, CurveError> {
        if jets.len() < 2 {
            return Err(CurveError::TooFewKnots);
        }
        if jets.iter().any(|jet| {
            !jet.position.iter().all(|value| value.is_finite())
                || !jet.tangent.iter().all(|value| value.is_finite())
        }) {
            return Err(CurveError::NotFinite);
        }
        if jets
            .windows(2)
            .any(|pair| same_point(pair[0].position, pair[1].position))
        {
            return Err(CurveError::CoincidentKnots);
        }
        let segments = jets
            .windows(2)
            .map(|pair| Segment {
                p0: pair[0].position,
                m0: pair[0].tangent,
                p1: pair[1].position,
                m1: pair[1].tangent,
            })
            .collect();
        Ok(Self::assemble(segments))
    }

    /// Assembles a curve from segments and computes its tables.
    fn assemble(segments: Vec<Segment>) -> Self {
        let mut cumulative = Vec::with_capacity(segments.len() + 1);
        let mut total = 0.0;
        cumulative.push(0.0);
        for segment in &segments {
            total += segment.length();
            cumulative.push(total);
        }
        let mut curve = Self {
            segments,
            cumulative,
            samples: Vec::new(),
        };
        curve.rebuild_samples();
        curve
    }

    /// Rebuilds the frame table from the curve's own shape.
    ///
    /// # Invariants
    ///
    /// Called after any change to the segments. The first sample keeps the normal it already had, which
    /// is why [`Curve::set_initial_normal`] exists: a split half must inherit the frame its parent had
    /// at the split, or a leaf template lofts rotated against its own midrib.
    fn rebuild_samples(&mut self) {
        let count = self.segments.len() * FRAME_SAMPLES;
        let start = self
            .samples
            .first()
            .map(|sample| sample.normal)
            .unwrap_or_else(|| perpendicular_to([0.0; 3], self.derivative_at(0.0)));
        let mut samples = Vec::with_capacity(count + 1);
        let mut previous: Option<Sample> = None;
        for index in 0..=count {
            let t = index as f32 / count as f32;
            let position = self.position_at(t);
            let tangent = normalized3(self.derivative_at(t));
            let normal = match previous {
                None => perpendicular_to(start, tangent),
                Some(previous) => transport(previous, position, tangent),
            };
            let sample = Sample {
                position,
                tangent,
                normal,
            };
            samples.push(sample);
            previous = Some(sample);
        }
        self.samples = samples;
    }

    /// Sets the normal the frame starts from.
    ///
    /// # Invariants
    ///
    /// The supplied direction is projected perpendicular to the first knot's tangent, so a caller cannot
    /// make an un-orthonormal frame. This is the one piece of frame state a caller owns, and it exists so
    /// that two curves which met at a point continue to agree about which way their sections face.
    pub fn set_initial_normal(&mut self, normal: Vec3) {
        let Some(first) = self.samples.first().copied() else {
            return;
        };
        let mut previous = Sample {
            normal: perpendicular_to(normal, first.tangent),
            ..first
        };
        self.samples[0] = previous;
        for index in 1..self.samples.len() {
            let position = self.samples[index].position;
            let tangent = self.samples[index].tangent;
            let normal = transport(previous, position, tangent);
            let sample = Sample {
                position,
                tangent,
                normal,
            };
            self.samples[index] = sample;
            previous = sample;
        }
    }

    /// The number of segments, which is one fewer than the number of knots.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// The total length.
    pub fn length(&self) -> f32 {
        *self.cumulative.last().unwrap_or(&0.0)
    }

    /// The nearest point on the curve to `point`.
    ///
    /// # Invariants
    ///
    /// **A bracketed search with both budgets stated**: [`CLOSEST_SAMPLES`] positions per segment are compared
    /// through [`Curve::at`], so they are evenly spaced in arc length; the best one brackets the answer, and
    /// [`CLOSEST_STEPS`] golden-section steps narrow the bracket on the position's own distance. A query point
    /// beyond either end answers that end, exactly, because `at(0)` and `at(1)` are the knots.
    ///
    /// **The error is measured and not promised.** `curve_probe` reads the foot on a straight line against its
    /// closed form, the answer at a knot, and the worst departure from a denser search on a curve that bends —
    /// which is the shape of the answer this crate owes a caller (`closest` was deferred with that obligation).
    ///
    /// **The distance is the exact quantity and the fraction is not.** Near a foot the distance is *quadratic* in
    /// `t`, so once the squared distance stops changing at `f32`'s own precision the fraction can still move: a
    /// point 2 cm from a 4 m line pins its foot to about `1e-4` of the curve and no search in `f32` can do better.
    /// The comparisons are therefore made on squared distances, which is one rounding better than on their roots.
    pub fn closest(&self, point: Vec3) -> Closest {
        let samples = self.segment_count().max(1) * CLOSEST_SAMPLES;
        let mut best = 0.0f32;
        let mut best_squared = f32::MAX;
        for index in 0..=samples {
            let t = index as f32 / samples as f32;
            let squared = squared_distance(self.at(t), point);
            if squared < best_squared {
                best_squared = squared;
                best = t;
            }
        }
        let step = 1.0 / samples as f32;
        let mut low = (best - step).max(0.0);
        let mut high = (best + step).min(1.0);
        // Golden section: the distance to a point is unimodal inside a bracket narrow enough to see one
        // minimum, and shrinking the longer side by the same ratio each step needs no derivative.
        const RATIO: f32 = 0.618_034;
        for _ in 0..CLOSEST_STEPS {
            let first = high - (high - low) * RATIO;
            let second = low + (high - low) * RATIO;
            let one = squared_distance(self.at(first), point);
            let other = squared_distance(self.at(second), point);
            if one < other {
                high = second;
            } else {
                low = first;
            }
        }
        let t = clamp(0.5 * (low + high), 0.0, 1.0);
        let position = self.at(t);
        Closest {
            t,
            position,
            distance: length3(subtract(position, point)),
        }
    }

    /// The arc-length fraction of each knot, `0` first and `1` last.
    ///
    /// # Invariants
    ///
    /// This is the inverse of the parameterisation: `curve.at(curve.knot_parameters()[i])` is the `i`th
    /// knot's position to the quadrature's accuracy. An author needs it because every authored band, and
    /// every station a caller writes down, is one of these fractions.
    pub fn knot_parameters(&self) -> Vec<f32> {
        let total = self.length();
        if total <= 0.0 {
            return vec![0.0; self.cumulative.len()];
        }
        self.cumulative
            .iter()
            .map(|length| length / total)
            .collect()
    }

    /// The position at an arc-length fraction.
    pub fn at(&self, t: f32) -> Vec3 {
        let (segment, u) = self.locate(t);
        self.segments[segment].at(u)
    }

    /// The unit tangent at an arc-length fraction.
    pub fn tangent(&self, t: f32) -> Vec3 {
        normalized3(self.derivative_at(t))
    }

    /// The frame at an arc-length fraction.
    ///    /// # Invariants
    ///
    /// Between two frame samples the normal is transported from the earlier one in a single step, so the
    /// answer is continuous in `t` and does not depend on which side of a sample `t` fell.
    pub fn frame(&self, t: f32) -> Frame {
        if self.samples.is_empty() {
            return Frame::of(self.derivative_at(t), [0.0; 3]);
        }
        let t = clamp(t, 0.0, 1.0);
        let highest = self.samples.len() - 1;
        let index = ((t * highest as f32).floor() as usize).min(highest);
        let from = self.samples[index];
        if index == highest {
            return Frame::of(from.tangent, from.normal);
        }
        let position = self.at(t);
        let tangent = normalized3(self.derivative_at(t));
        let normal = transport(from, position, tangent);
        Frame::of(tangent, normal)
    }

    /// The curvature at an arc-length fraction.
    ///
    /// # Invariants
    ///
    /// `|P' × P''| / |P'|³` is invariant under a reparameterisation, so the answer does not depend on the
    /// tangent magnitudes a split left behind.
    pub fn curvature(&self, t: f32) -> f32 {
        let (segment, u) = self.locate(t);
        let first = self.segments[segment].derivative(u);
        let second = self.segments[segment].second(u);
        let speed = length3(first);
        if speed <= f32::EPSILON {
            return 0.0;
        }
        length3(cross3(first, second)) / (speed * speed * speed)
    }

    /// `count` stations spread evenly along the curve's length, with their frames.
    ///
    /// # Invariants
    ///
    /// The first station's frame is the curve's own initial normal and the rest are transported from it,
    /// so [`Curve::stations`] and [`Curve::frame`] agree at every station rather than only at the ends.
    pub fn stations(&self, count: usize) -> Vec<Station> {
        let count = count.max(2);
        let mut out = Vec::with_capacity(count);
        let mut previous: Option<Sample> = None;
        for index in 0..count {
            let t = index as f32 / (count - 1) as f32;
            let position = self.at(t);
            let tangent = normalized3(self.derivative_at(t));
            let normal = match previous {
                None => self
                    .samples
                    .first()
                    .map(|sample| sample.normal)
                    .unwrap_or_else(|| perpendicular_to([0.0; 3], tangent)),
                Some(previous) => transport(previous, position, tangent),
            };
            let sample = Sample {
                position,
                tangent,
                normal,
            };
            out.push(Station {
                t,
                position,
                frame: Frame::of(tangent, normal),
            });
            previous = Some(sample);
        }
        out
    }

    /// The curve as cubic Beziers — the form every format exports.
    ///
    /// # Invariants
    ///
    /// A Hermite segment and a cubic Bezier are the same curve with `p1 = p0 + m0/3` and
    /// `p2 = p3 − m1/3`; nothing is approximated.
    pub fn segments(&self) -> impl Iterator<Item = Bezier> + '_ {
        self.segments.iter().map(Segment::bezier)
    }

    /// The knots, as positions and the derivative leaving each.
    ///
    /// # Invariants
    ///
    /// The tangent is the segment **after** the knot's, and the last knot's is the segment before it.
    /// They are the authored values for a curve built by [`Curve::new`].
    ///
    /// **`Curve::new(curve.jets())` is not a round trip after a split or an insert**, and this is the one
    /// place that matters: each half of a split carries its own parameter interval, so the derivative
    /// *entering* the new knot and the derivative *leaving* it differ in magnitude, and a jet holds one
    /// number. Rebuilding from this list therefore replaces the left-hand magnitude with the right-hand
    /// one and changes the previous segment's shape. A caller that needs the shape back wants [`Curve`]'s
    /// clone, not a list of jets.
    pub fn jets(&self) -> Vec<Jet> {
        let mut out = Vec::with_capacity(self.segments.len() + 1);
        for segment in &self.segments {
            out.push(Jet::new(segment.p0, segment.m0));
        }
        if let Some(last) = self.segments.last() {
            out.push(Jet::new(last.p1, last.m1));
        }
        out
    }

    /// Splits the curve at an arc-length fraction, exactly.
    ///
    /// # Invariants
    ///
    /// Each half is the original cubic restricted to a sub-interval, so the pair draws the same shape to
    /// the last bit the arithmetic allows. The right half's initial normal is the parent's frame at the
    /// split, so a downstream sweep does not find a rotation it did not ask for.
    pub fn split(&self, t: f32) -> (Self, Self) {
        let t = clamp(t, 0.0, 1.0);
        let (index, u) = self.locate(t);
        let bezier = self.segments[index].bezier();
        let first = mix(bezier.p0, bezier.p1, u);
        let second = mix(bezier.p1, bezier.p2, u);
        let third = mix(bezier.p2, bezier.p3, u);
        let fourth = mix(first, second, u);
        let fifth = mix(second, third, u);
        let meet = mix(fourth, fifth, u);
        let left_segment = Segment {
            p0: bezier.p0,
            m0: scale(subtract(first, bezier.p0), 3.0),
            p1: meet,
            m1: scale(subtract(meet, fourth), 3.0),
        };
        let right_segment = Segment {
            p0: meet,
            m0: scale(subtract(fifth, meet), 3.0),
            p1: bezier.p3,
            m1: scale(subtract(bezier.p3, third), 3.0),
        };

        let mut left_segments: Vec<Segment> = self.segments[..index].to_vec();
        left_segments.push(left_segment);
        let mut right_segments: Vec<Segment> = Vec::with_capacity(self.segments.len() - index);
        right_segments.push(right_segment);
        right_segments.extend_from_slice(&self.segments[index + 1..]);

        let split_normal = self.frame(t).normal;
        let start_normal = self
            .samples
            .first()
            .map(|sample| sample.normal)
            .unwrap_or_else(|| perpendicular_to([0.0; 3], self.derivative_at(0.0)));
        let mut left = Self::assemble(left_segments);
        let mut right = Self::assemble(right_segments);
        left.set_initial_normal(start_normal);
        right.set_initial_normal(split_normal);
        (left, right)
    }

    /// Inserts a knot at an arc-length fraction without changing the shape, and returns its index.
    ///
    /// # Invariants
    ///
    /// The new knot's two tangent magnitudes differ, because each half is parameterised on its own
    /// interval. That is not a shape change: every query is by arc length, so nothing a caller reads
    /// depends on it.
    pub fn insert(&mut self, t: f32) -> usize {
        let t = clamp(t, 0.0, 1.0);
        let (index, _) = self.locate(t);
        let start_normal = self
            .samples
            .first()
            .map(|sample| sample.normal)
            .unwrap_or_else(|| perpendicular_to([0.0; 3], self.derivative_at(0.0)));
        let (left, right) = self.split(t);
        let mut segments = left.segments;
        segments.extend(right.segments);
        let mut rebuilt = Self::assemble(segments);
        rebuilt.set_initial_normal(start_normal);
        *self = rebuilt;
        index + 1
    }

    /// The sub-curve between two arc-length fractions.
    ///
    /// # Invariants
    ///
    /// `to <= from` gives a curve of no length at `from` rather than a refusal: a caller that asked for
    /// an empty span gets an empty span, and every query on it answers `from`'s point.
    pub fn sub(&self, from: f32, to: f32) -> Self {
        let from = clamp(from, 0.0, 1.0);
        let to = clamp(to, 0.0, 1.0);
        if to <= from {
            let point = self.at(from);
            let mut empty = Self::assemble(vec![Segment {
                p0: point,
                m0: [0.0; 3],
                p1: point,
                m1: [0.0; 3],
            }]);
            empty.set_initial_normal(self.frame(from).normal);
            return empty;
        }
        if from <= 0.0 && to >= 1.0 {
            return self.clone();
        }
        let (_, right) = self.split(from);
        let span = (to - from) / (1.0 - from);
        let (left, _) = right.split(span.min(1.0));
        left
    }

    /// The segment index and local parameter an arc-length fraction names.
    ///
    /// # Invariants
    ///
    /// **The two ends are the two ends, not one bisection step away from them.** `t = 0` answers the
    /// first segment at `u = 0` and `t = 1` the last at `u = 1`, so [`Curve::at`] returns the first and
    /// last jets' positions bit for bit and [`Curve::tangent`] returns their authored tangents. Without
    /// this, an inversion whose target is the whole length lands on `1 − 2⁻³³` and a closed loop leaves a
    /// `1.2e-10` gap at the seam — invisible, and enough to make "a closed curve's `at(1)` is its
    /// `at(0)`" false.
    fn locate(&self, t: f32) -> (usize, f32) {
        let t = clamp(t, 0.0, 1.0);
        if t <= 0.0 {
            return (0, 0.0);
        }
        if t >= 1.0 {
            return (self.segments.len() - 1, 1.0);
        }
        let target = t * self.length();
        let last = self.segments.len().saturating_sub(1);
        let index = self
            .cumulative
            .partition_point(|&length| length < target)
            .saturating_sub(1)
            .min(last);
        let local = (target - self.cumulative[index]).max(0.0);
        (index, self.segments[index].invert(local))
    }

    /// The derivative at an arc-length fraction, unnormalised.
    fn derivative_at(&self, t: f32) -> Vec3 {
        let (segment, u) = self.locate(t);
        self.segments[segment].derivative(u)
    }

    /// The position at an arc-length fraction.
    fn position_at(&self, t: f32) -> Vec3 {
        let (segment, u) = self.locate(t);
        self.segments[segment].at(u)
    }
}

/// Eight-point Gauss-Legendre nodes on `[-1, 1]`.
const GAUSS_NODES: [f32; 8] = [
    -0.960_289_86,
    -0.796_666_47,
    -0.525_532_4,
    -0.183_434_64,
    0.183_434_64,
    0.525_532_4,
    0.796_666_47,
    0.960_289_86,
];

/// The weights that go with [`GAUSS_NODES`].
const GAUSS_WEIGHTS: [f32; 8] = [
    0.101_228_54,
    0.222_381_03,
    0.313_706_65,
    0.362_683_78,
    0.362_683_78,
    0.313_706_65,
    0.222_381_03,
    0.101_228_54,
];

/// The direction carried from one station to the next without spinning.
///
/// Wang et al.'s double reflection: reflect the previous normal across the plane whose normal is the
/// chord, then reflect the result across the plane that carries the transported tangent onto the new one.
/// Two reflections, and no trigonometry to accumulate error in.
fn transport(previous: Sample, position: Vec3, tangent: Vec3) -> Vec3 {
    let chord = subtract(position, previous.position);
    let chord_squared = dot3(chord, chord);
    if chord_squared <= 1.0e-12 {
        return perpendicular_to(previous.normal, tangent);
    }
    let first = subtract(
        previous.normal,
        scale(chord, 2.0 / chord_squared * dot3(chord, previous.normal)),
    );
    let carried = subtract(
        previous.tangent,
        scale(chord, 2.0 / chord_squared * dot3(chord, previous.tangent)),
    );
    let difference = subtract(tangent, carried);
    let difference_squared = dot3(difference, difference);
    if difference_squared <= 1.0e-12 {
        return perpendicular_to(first, tangent);
    }
    let second = subtract(
        first,
        scale(
            difference,
            2.0 / difference_squared * dot3(difference, first),
        ),
    );
    perpendicular_to(second, tangent)
}

/// Returns `direction` with the component along `axis` removed, normalised.
///
/// # Invariants
///
/// A direction parallel to the axis has no perpendicular part, and the fallback is a fixed perpendicular
/// of the axis rather than a zero vector: a zero normal would collapse every section drawn in this frame
/// to a line.
fn perpendicular_to(direction: Vec3, axis: Vec3) -> Vec3 {
    let axis = normalized3(axis);
    let projected = subtract(direction, scale(axis, dot3(direction, axis)));
    if length3(projected) > 1.0e-6 {
        return normalized3(projected);
    }
    let magnitudes = [axis[0].abs(), axis[1].abs(), axis[2].abs()];
    let mut smallest = 0;
    for candidate in 1..3 {
        if magnitudes[candidate] < magnitudes[smallest] {
            smallest = candidate;
        }
    }
    let basis = match smallest {
        0 => [1.0, 0.0, 0.0],
        1 => [0.0, 1.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    };
    normalized3(cross3(basis, axis))
}

fn same_point(left: Vec3, right: Vec3) -> bool {
    left[0] == right[0] && left[1] == right[1] && left[2] == right[2]
}

fn scale(vector: Vec3, factor: f32) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn mix(from: Vec3, to: Vec3, amount: f32) -> Vec3 {
    add(scale(from, 1.0 - amount), scale(to, amount))
}
