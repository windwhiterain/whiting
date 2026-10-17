//! A law of one number along a spine: a taper, a crown's reach, a twist.
//!
//! # The one idea
//!
//! **A [`Scalar`] is a [`crate::curve::Curve`] with the space taken out of it, and it takes its `t`
//! from the curve it belongs to.** A sweep multiplies its section by `scale.at(station.t)` and
//! rotates it by `twist.at(station.t)`, so the number a branch's taper or an envelope's reach is
//! written in is a fraction of **the branch's own length** — the same `t` every other authored
//! number uses ([`crate::curve`] D2). The superior's sentence for the envelope — *a radius curve in
//! the curve's parameter space* — is this type, and nothing else about it is vegetation's.
//!
//! # Why it is not `Curve`
//!
//! `Curve`'s `t` is a fraction of its own length, and that is what makes a station move with the
//! geometry. A scalar's variation is not a length: a radius that goes 1 → 0.2 → 1 has a total
//! variation, but normalising by it would make the parameter depend on the values, and a radius
//! that is flat over the first half would spend the whole parameter range where nothing happens.
//! So a `Scalar`'s `t` is the **normalised knot parameter**, the knots must start at `0` and end at
//! `1` (a law is total), and it is deliberately *not* the same contract as `Curve`'s `t` — which is
//! why the two are separate types sharing one basis rather than one type with a flag.
//!
//! # A slope is limited by default
//!
//! [`Scalar::new`] takes values only and chooses the slopes: one-sided at the ends, central inside,
//! then limited so that **each span stays inside the two values that bound it** (the
//! Fritsch-Carlson condition, `alpha² + beta² <= 9` per span, with a slope that points away from the
//! span's direction zeroed). A radius interpolated from three knots therefore never dips below zero
//! between two positive ones, which is the one thing an authored radius must not do: a negative
//! radius folds a section through its own spine. [`Scalar::with_slopes`] is the unlimited form, for
//! a corner or an overshoot the author means.

use crate::hermite;
use crate::math::clamp;

/// One span between two knots, in the knots' own parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    t0: f32,
    v0: f32,
    s0: f32,
    t1: f32,
    v1: f32,
    s1: f32,
}

/// Why a scalar could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarError {
    /// Fewer than two knots: there is nothing to evaluate between.
    TooFewKnots,
    /// A knot holds a value that is not finite.
    NotFinite,
    /// Two consecutive knots stand at the same parameter, so that span has no width.
    NotIncreasing,
    /// The first knot is not at `0`, or the last is not at `1`.
    NotTotal,
}

/// A number along a spine: a law of the spine's arc-length fraction.
#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    spans: Vec<Span>,
}

impl Scalar {
    /// A law that says the same number everywhere.
    ///
    /// It is one span of zero slope rather than a variant, so every query of a `Scalar` is one query.
    pub fn constant(value: f32) -> Self {
        assert!(
            value.is_finite(),
            "a scalar's value must be finite, and a constant is not data that can be reported"
        );
        Self {
            spans: vec![Span {
                t0: 0.0,
                v0: value,
                s0: 0.0,
                t1: 1.0,
                v1: value,
                s1: 0.0,
            }],
        }
    }

    /// A law through the given knots, with the slopes chosen and limited for you.
    ///
    /// # Invariants
    ///
    /// The knots are `(t, value)` and must start at `0`, end at `1`, and increase strictly. Every
    /// span stays inside the range of the two values that bound it, so a radius given three positive
    /// knots is positive everywhere between them. A span whose two values are equal is constant, and
    /// a knot that is a local extremum gets a zero slope, which is what stops a spike from ringing.
    pub fn new(knots: &[(f32, f32)]) -> Result<Self, ScalarError> {
        validate(knots)?;
        let last = knots.len() - 1;
        let mut slopes: Vec<f32> = Vec::with_capacity(knots.len());
        slopes.push(secant(knots[0], knots[1]));
        for index in 1..last {
            slopes.push(
                (knots[index + 1].1 - knots[index - 1].1)
                    / (knots[index + 1].0 - knots[index - 1].0),
            );
        }
        slopes.push(secant(knots[last - 1], knots[last]));
        limit(knots, &mut slopes);
        Ok(assemble(knots, &slopes))
    }

    /// A law through the given `(t, value, slope)` knots, with no limiting.
    ///
    /// The unlimited form, for an author who means the overshoot: a bump that rises faster than its
    /// neighbours' secants, or a corner whose slope is zero on one side and not the other.
    pub fn with_slopes(knots: &[(f32, f32, f32)]) -> Result<Self, ScalarError> {
        let pairs: Vec<(f32, f32)> = knots.iter().map(|knot| (knot.0, knot.1)).collect();
        validate(&pairs)?;
        for knot in knots {
            if !knot.2.is_finite() {
                return Err(ScalarError::NotFinite);
            }
        }
        let slopes: Vec<f32> = knots.iter().map(|knot| knot.2).collect();
        Ok(assemble(&pairs, &slopes))
    }

    /// The number at the spine's arc-length fraction `t`, with `t` pinned into `[0, 1]`.
    pub fn at(&self, t: f32) -> f32 {
        let (span, s) = self.locate(t);
        let width = span.t1 - span.t0;
        let [h00, h10, h01, h11] = hermite::value(s);
        span.v0 * h00 + span.s0 * width * h10 + span.v1 * h01 + span.s1 * width * h11
    }

    /// How fast the number changes with `t`, in numbers per unit of the spine's length fraction.
    pub fn slope(&self, t: f32) -> f32 {
        let (span, s) = self.locate(t);
        let width = span.t1 - span.t0;
        let [d00, d10, d01, d11] = hermite::slope(s);
        (span.v0 * d00 + span.s0 * width * d10 + span.v1 * d01 + span.s1 * width * d11) / width
    }

    /// The span holding `t` and the local parameter inside it.
    ///
    /// # Invariants
    ///
    /// `partition_point` finds the first span that ends after `t`, so a query exactly at a knot
    /// answers with that knot's stored value rather than the neighbouring span's limit: a knot is
    /// exact, which is what lets an authored number be read back unchanged.
    fn locate(&self, t: f32) -> (&Span, f32) {
        let t = clamp(t, 0.0, 1.0);
        let index = self
            .spans
            .partition_point(|span| span.t1 <= t)
            .min(self.spans.len() - 1);
        let span = &self.spans[index];
        let width = span.t1 - span.t0;
        let s = if width > 0.0 {
            (t - span.t0) / width
        } else {
            1.0
        };
        (span, s)
    }
}

/// The slope of the straight line between two knots.
fn secant(from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.1 - from.1) / (to.0 - from.0)
}

/// Checks what a law needs to be total, finite and evaluable.
fn validate(knots: &[(f32, f32)]) -> Result<(), ScalarError> {
    if knots.len() < 2 {
        return Err(ScalarError::TooFewKnots);
    }
    if knots
        .iter()
        .any(|knot| !knot.0.is_finite() || !knot.1.is_finite())
    {
        return Err(ScalarError::NotFinite);
    }
    if knots.windows(2).any(|pair| pair[1].0 <= pair[0].0) {
        return Err(ScalarError::NotIncreasing);
    }
    if knots[0].0 != 0.0 || knots[knots.len() - 1].0 != 1.0 {
        return Err(ScalarError::NotTotal);
    }
    Ok(())
}

/// Limits the slopes so that no span leaves the range of its own two values.
///
/// # Invariants
///
/// The condition is Fritsch-Carlson's, applied per span and in place: a slope that points away from
/// the span's direction is zeroed, and a pair whose `alpha² + beta²` exceeds `9` is scaled by
/// `3 / sqrt(alpha² + beta²)`. A knot shared by two spans can therefore be damped twice, which can
/// only make a span flatter — never steeper — so the guarantee holds span by span.
fn limit(knots: &[(f32, f32)], slopes: &mut [f32]) {
    for index in 0..knots.len() - 1 {
        let (t0, v0) = knots[index];
        let (t1, v1) = knots[index + 1];
        let delta = (v1 - v0) / (t1 - t0);
        let mut start = slopes[index];
        let mut end = slopes[index + 1];
        if delta == 0.0 {
            start = 0.0;
            end = 0.0;
        } else {
            let mut alpha = start / delta;
            let mut beta = end / delta;
            if alpha < 0.0 {
                start = 0.0;
                alpha = 0.0;
            }
            if beta < 0.0 {
                end = 0.0;
                beta = 0.0;
            }
            let magnitude = (alpha * alpha + beta * beta).sqrt();
            if magnitude > 3.0 {
                let factor = 3.0 / magnitude;
                start *= factor;
                end *= factor;
            }
        }
        slopes[index] = start;
        slopes[index + 1] = end;
    }
}

/// Zips the knots and slopes into spans.
fn assemble(knots: &[(f32, f32)], slopes: &[f32]) -> Scalar {
    let spans = (0..knots.len() - 1)
        .map(|index| Span {
            t0: knots[index].0,
            v0: knots[index].1,
            s0: slopes[index],
            t1: knots[index + 1].0,
            v1: knots[index + 1].1,
            s1: slopes[index + 1],
        })
        .collect();
    Scalar { spans }
}
