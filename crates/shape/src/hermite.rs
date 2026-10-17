//! The cubic Hermite basis: one polynomial, two callers.
//!
//! # The one idea
//!
//! A [`crate::curve::Curve`] and a [`crate::scalar::Scalar`] are the same polynomial in different
//! spaces: four basis functions of a segment parameter in `[0, 1]`, weighted by two values and two
//! slopes. The basis is written once, here, so that a change to the polynomial changes both, and so
//! that neither derives its coefficients from the other's source.
//!
//! # Which function is which
//!
//! [`value`] and [`slope`] return the four weights in the same order: the start's value, the start's
//! slope, the end's value, the end's slope. A slope's unit is the caller's, and that is the only
//! place the two callers differ — `curve` multiplies the slope weights by the interval's direction,
//! `scalar` by the interval's width. Both callers keep `t` in `[0, 1]` inside a span, and both get
//! exactly `1` and `0` at the span's ends, which is what makes a knot's value exact rather than
//! approached.

/// The four weights at `t`: start value, start slope, end value, end slope.
pub(crate) fn value(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        2.0 * t3 - 3.0 * t2 + 1.0,
        t3 - 2.0 * t2 + t,
        -2.0 * t3 + 3.0 * t2,
        t3 - t2,
    ]
}

/// The four weights of the basis' derivative at `t`, in the same order as [`value`].
pub(crate) fn slope(t: f32) -> [f32; 4] {
    let t2 = t * t;
    [
        6.0 * t2 - 6.0 * t,
        3.0 * t2 - 4.0 * t + 1.0,
        -6.0 * t2 + 6.0 * t,
        3.0 * t2 - 2.0 * t,
    ]
}

/// The four weights of the basis' second derivative at `t`, in the same order as [`value`].
pub(crate) fn second(t: f32) -> [f32; 4] {
    [
        12.0 * t - 6.0,
        6.0 * t - 4.0,
        -12.0 * t + 6.0,
        6.0 * t - 2.0,
    ]
}
