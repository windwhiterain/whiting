//! Measures the scalar law's invariants, each with a control that can fail, and exits non-zero if one
//! does.
//!
//! Nothing here is a test file: this is the probe that says whether `shape::scalar` does what its own
//! documentation claims. The invariants are `.agents/proposals/shape-primitives.md` D4/D6 (a section's
//! scale is a law of the spine's arc-length fraction) and the two properties the type promises: the
//! limiter keeps a span inside its own values, and the basis reproduces a polynomial it is given the
//! data for. Each reading is printed with the number it is compared against.
//!
//! ```text
//! cargo run -p whiting-shape --example scalar_probe
//! ```

use whiting_shape::scalar::{Scalar, ScalarError};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("constant", a_constant_is_one_number()));
    checks.push(("two knots", two_knots_are_a_straight_line()));
    checks.push(("limiter", a_limited_span_stays_inside_its_values()));
    checks.push(("C1 at a knot", a_knot_is_continuous_in_slope()));
    checks.push(("quadratic", the_basis_reproduces_a_quadratic()));
    checks.push(("offset", an_offset_moves_the_number_and_not_the_shape()));
    checks.push(("clamped ends", the_ends_are_clamped()));
    checks.push(("errors", the_errors_are_the_reported_ones()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<24} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// A constant law answers with its one number everywhere, to the rounding of the basis sum.
///
/// # Invariants
///
/// It is **not** bit-identical, and that is the reading rather than a defect: the four weights sum to
/// `1` in exact arithmetic and to `1` plus one rounding in `f32`, so a constant is constant to about
/// `1e-7` of itself. This row gates on that number instead of asserting a zero the evaluation cannot
/// give, and the zero it *does* give is the slope's, which is exact because every term in it is
/// multiplied by a zero slope.
fn a_constant_is_one_number() -> bool {
    let scalar = Scalar::constant(2.5);
    let mut worst = 0.0f32;
    let mut slope = 0.0f32;
    for step in 0..=100 {
        let t = step as f32 / 100.0;
        worst = worst.max((scalar.at(t) - 2.5).abs());
        slope = slope.max(scalar.slope(t).abs());
    }
    println!("\n# a constant law");
    println!("  worst deviation over 101 samples {worst:.3e} · worst slope {slope:.3e}");
    println!(
        "  the deviation is the basis sum's rounding; the slope is exactly zero, term by term"
    );
    worst < 1e-6 && slope == 0.0
}

/// Two knots are a cone's taper: a straight line, and the limiter must leave it straight.
fn two_knots_are_a_straight_line() -> bool {
    let scalar = Scalar::new(&[(0.0, 1.0), (1.0, 0.5)]).expect("two knots are a law");
    let start = scalar.at(0.0);
    let quarter = scalar.at(0.25);
    let end = scalar.at(1.0);
    let slope = scalar.slope(0.5);
    println!("\n# two knots: a taper");
    println!(
        "  at(0) = {start:.6} · at(0.25) = {quarter:.6} (a line says 0.875) · at(1) = {end:.6}"
    );
    println!("  slope everywhere {slope:.6} (a line says −0.5)");
    (start - 1.0).abs() < 1e-6
        && (quarter - 0.875).abs() < 1e-6
        && (end - 0.5).abs() < 1e-6
        && (slope + 0.5).abs() < 1e-6
}

/// A spike: the limiter keeps the span inside the values that bound it, and the same slopes without it
/// do not.
fn a_limited_span_stays_inside_its_values() -> bool {
    let limited =
        Scalar::new(&[(0.0, 0.0), (0.5, 1.0), (1.0, 0.9)]).expect("three knots are a law");
    let control = Scalar::with_slopes(&[(0.0, 0.0, 2.0), (0.5, 1.0, 0.9), (1.0, 0.9, -0.2)])
        .expect("the control's knots are a law");
    let mut limited_max = f32::MIN;
    let mut control_max = f32::MIN;
    for step in 0..=1000 {
        let t = step as f32 / 1000.0;
        limited_max = limited_max.max(limited.at(t));
        control_max = control_max.max(control.at(t));
    }
    println!("\n# a spike: the limiter, and the same knot slopes without it");
    println!("  limited maximum {limited_max:.6} (the data's own maximum is 1.0)");
    println!(
        "  control maximum {control_max:.6} — its knot slope points against its span, which the limiter zeroes"
    );
    limited_max <= 1.0 + 1e-6 && control_max > 1.01
}

/// The slope is the derivative from both sides of a knot, measured by finite differences.
fn a_knot_is_continuous_in_slope() -> bool {
    let scalar = quadratic();
    let knots = [(0.0f32, 0.0f32), (0.1, 0.01), (0.55, 0.3025), (1.0, 1.0)];
    let step = 1e-3;
    let mut worst_difference = 0.0f32;
    let mut worst_exact = 0.0f32;
    for knot in &knots[1..knots.len() - 1] {
        let behind = (scalar.at(knot.0) - scalar.at(knot.0 - step)) / step;
        let ahead = (scalar.at(knot.0 + step) - scalar.at(knot.0)) / step;
        let exact = 2.0 * knot.0;
        worst_difference = worst_difference
            .max((behind - exact).abs())
            .max((ahead - exact).abs());
        worst_exact = worst_exact.max((scalar.slope(knot.0) - exact).abs());
    }
    println!("\n# a knot, from both sides");
    println!("  worst finite difference from 2t {worst_difference:.3e} (the step is {step:.0e})");
    println!("  worst slope from the law itself {worst_exact:.3e}");
    worst_difference < 5e-3 && worst_exact < 1e-5
}

/// A Hermite span reproduces a cubic exactly from its two values and two slopes, so a quadratic with
/// exact derivatives must come back unchanged — including across unevenly spaced knots, which is what
/// catches a slope scaled by the wrong width.
fn the_basis_reproduces_a_quadratic() -> bool {
    let scalar = quadratic();
    let mut worst_value = 0.0f32;
    let mut worst_slope = 0.0f32;
    for step in 0..=1000 {
        let t = step as f32 / 1000.0;
        worst_value = worst_value.max((scalar.at(t) - t * t).abs());
        worst_slope = worst_slope.max((scalar.slope(t) - 2.0 * t).abs());
    }
    println!("\n# a quadratic through four unevenly spaced knots");
    println!("  worst value error {worst_value:.3e} · worst slope error {worst_slope:.3e}");
    println!("  knots at 0, 0.1, 0.55 and 1: an even spacing would hide a width scaled wrong");
    worst_value < 1e-6 && worst_slope < 1e-5
}

/// `at` pins its argument, so a caller that interpolates past an end reads the end and not an
/// extrapolation.
fn the_ends_are_clamped() -> bool {
    let scalar = quadratic();
    let before = scalar.at(-1.0) == scalar.at(0.0);
    let after = scalar.at(2.0) == scalar.at(1.0);
    println!("\n# outside the law");
    println!("  at(−1) = at(0) {before} · at(2) = at(1) {after}");
    println!("  a law is total: it is defined on [0, 1] and pins rather than extrapolates");
    before && after
}

/// Every refusal is the one the type documents.
fn the_errors_are_the_reported_ones() -> bool {
    let cases: [(&str, Result<Scalar, ScalarError>, ScalarError); 5] = [
        (
            "one knot",
            Scalar::new(&[(0.0, 1.0)]),
            ScalarError::TooFewKnots,
        ),
        (
            "not finite",
            Scalar::new(&[(0.0, 1.0), (1.0, f32::NAN)]),
            ScalarError::NotFinite,
        ),
        (
            "not increasing",
            Scalar::new(&[(0.0, 1.0), (0.0, 2.0)]),
            ScalarError::NotIncreasing,
        ),
        (
            "not total",
            Scalar::new(&[(0.1, 1.0), (1.0, 2.0)]),
            ScalarError::NotTotal,
        ),
        (
            "slope not finite",
            Scalar::with_slopes(&[(0.0, 1.0, 0.0), (1.0, 2.0, f32::INFINITY)]),
            ScalarError::NotFinite,
        ),
    ];
    let mut passed = true;
    println!("\n# what a law refuses");
    for (name, result, expected) in &cases {
        let agrees = result == &Err(*expected);
        passed &= agrees;
        println!(
            "  {name:<16} {expected:?} {}",
            if agrees { "pass" } else { "FAIL" }
        );
    }
    passed
}

/// An offset law is the law plus the number, everywhere, with its shape untouched.
///
/// # Invariants
///
/// The values differ by exactly `by` at every sample and the slopes are equal, so a nudge cannot change what
/// the law *does* — and the control is that the two laws are not the same law: a tolerance that passed on an
/// unshifted copy would call a real offset a success.
fn an_offset_moves_the_number_and_not_the_shape() -> bool {
    let law = quadratic();
    let by = -0.25f32;
    let shifted = law.offset(by);
    let mut worst_value = 0.0f32;
    let mut worst_slope = 0.0f32;
    let mut biggest = 0.0f32;
    for step in 0..=200 {
        let t = step as f32 / 200.0;
        let difference = (law.at(t) + by) - shifted.at(t);
        worst_value = worst_value.max(difference.abs());
        worst_slope = worst_slope.max((law.slope(t) - shifted.slope(t)).abs());
        biggest = biggest.max((law.at(t) - shifted.at(t)).abs());
    }
    println!("\n# an offset law");
    println!(
        "  worst value difference from the law less {by} {worst_value:.3e} · worst slope difference {worst_slope:.3e}"
    );
    println!(
        "  the slopes agree to a rounding rather than exactly, because the slope is evaluated from the shifted knots"
    );
    println!(
        "  the two laws differ by {biggest:.6} where they are compared, so the row is not blind"
    );
    worst_value < 1e-6 && worst_slope < 1e-6 && biggest > 0.2
}

/// A quadratic as four `(t, value, slope)` knots: `v = t²`, `dv/dt = 2t`.
fn quadratic() -> Scalar {
    Scalar::with_slopes(&[
        (0.0, 0.0, 0.0),
        (0.1, 0.01, 0.2),
        (0.55, 0.3025, 1.1),
        (1.0, 1.0, 2.0),
    ])
    .expect("the quadratic's knots are a law")
}
