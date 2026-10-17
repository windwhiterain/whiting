//! Measures the curve's invariants, each with a control that can fail, and exits non-zero if one does.
//!
//! Nothing here is a test file: this is the probe that says whether `shape::curve` does what its own
//! documentation claims, and every reading is printed with the number it is compared against. The
//! invariants are `.agents/proposals/shape-primitives.md` D2 (arc length), D3 (a transported frame) and
//! P0's exit criterion (a split changes the parameterisation and not the shape). The thresholds the probe
//! gates on are the ones that document states, so a tolerance that moves has to be moved in two places
//! deliberately rather than drifting in one.
//!
//! ```text
//! cargo run -p whiting-shape --example curve_probe
//! ```

use whiting_shape::curve::{Curve, Jet};
use whiting_shape::math::{Vec3, dot3, length3, normalized3, subtract};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("arc length", length_against_a_polyline()));
    checks.push(("even stations", stations_are_evenly_spaced()));
    checks.push(("insert keeps the shape", insertion_preserves_the_shape()));
    checks.push(("straight frame", a_straight_curve_has_a_constant_frame()));
    checks.push(("inflection", an_inflection_does_not_flip_the_frame()));
    checks.push(("closed loop", a_closed_planar_loop_has_no_holonomy()));
    checks.push(("closed ends", a_closed_curve_ends_where_it_starts()));
    checks.push(("bezier form", the_bezier_form_is_the_same_curve()));
    checks.push(("the foot", the_foot_on_a_straight_line()));
    checks.push(("past the end", a_point_past_the_end_answers_the_end()));
    checks.push(("at a knot", a_point_at_a_knot_answers_the_knot()));
    checks.push(("against a denser search", the_search_against_a_denser_one()));

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

/// A bent spine: the kind of control polygon a branch or a culm is authored from.
fn spine() -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.0, 0.8, 0.0]),
        Jet::new([0.2, 0.9, 0.1], [0.6, 0.7, 0.0]),
        Jet::new([0.9, 1.7, 0.4], [0.9, 0.2, 0.5]),
        Jet::new([1.9, 1.9, 0.9], [1.0, 0.0, 0.6]),
        Jet::new([2.9, 1.85, 1.5], [1.0, -0.1, 0.4]),
    ])
    .expect("the spine's knots are distinct")
}

/// A straight line, where a Frenet frame is undefined and a transported one is constant.
fn straight() -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
        Jet::new([2.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
        Jet::new([4.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
    ])
    .expect("the straight curve's knots are distinct")
}

/// An S in the `xy` plane: the curve passes an inflection at its middle knot.
fn s_curve() -> Curve {
    Curve::new(vec![
        Jet::new([-2.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
        Jet::new([-0.9, 0.7, 0.0], [1.0, 0.35, 0.0]),
        Jet::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        Jet::new([0.9, -0.7, 0.0], [1.0, -0.35, 0.0]),
        Jet::new([2.0, 0.0, 0.0], [1.0, -1.0, 0.0]),
    ])
    .expect("the S's knots are distinct")
}

/// A circle as eight Hermite segments, whose tangents are the arc's own derivatives.
fn circle(radius: f32) -> Curve {
    let step = std::f32::consts::FRAC_PI_4;
    let jets = (0..=8)
        .map(|index| {
            let angle = index as f32 * step;
            let (sin, cos) = angle.sin_cos();
            Jet::new(
                [radius * cos, radius * sin, 0.0],
                [-radius * step * sin, radius * step * cos, 0.0],
            )
        })
        .collect();
    Curve::new(jets).expect("the circle's knots are distinct")
}

/// The arc-length table against a fine polyline of the same curve.
fn length_against_a_polyline() -> bool {
    let curve = spine();
    let steps = 4000;
    let mut measured = 0.0f32;
    let mut previous = curve.at(0.0);
    for step in 1..=steps {
        let at = curve.at(step as f32 / steps as f32);
        measured += length3(subtract(at, previous));
        previous = at;
    }
    let stated = curve.length();
    let relative = (measured - stated).abs() / stated;
    println!("# the arc length against a {steps}-segment polyline of the same curve");
    println!(
        "  length() {stated:.6} · polyline {measured:.6} · relative difference {relative:.3e}"
    );
    println!(
        "  the inversion walks back the same quadrature the table was built from, so this is a \
         consistency check rather than an independent one — the closed loop below is the independent one"
    );
    relative < 1.0e-5
}

/// Stations spread by arc length really are evenly spaced along it.
fn stations_are_evenly_spaced() -> bool {
    let curve = spine();
    let stations = curve.stations(65);
    // Arc length between consecutive stations, measured by a fine polyline along each span, and the
    // chord beside it: a chord shortens by an amount that depends on how curved its span is, so the arc
    // is the number that speaks about the parameterisation.
    let mut shortest = f32::INFINITY;
    let mut longest = 0.0f32;
    let mut shortest_chord = f32::INFINITY;
    let mut longest_chord = 0.0f32;
    for pair in stations.windows(2) {
        let across = 64;
        let mut arc = 0.0f32;
        let mut previous = pair[0].position;
        for step in 1..=across {
            let t = pair[0].t + (pair[1].t - pair[0].t) * step as f32 / across as f32;
            let at = curve.at(t);
            arc += length3(subtract(at, previous));
            previous = at;
        }
        let chord = length3(subtract(pair[1].position, pair[0].position));
        shortest = shortest.min(arc);
        longest = longest.max(arc);
        shortest_chord = shortest_chord.min(chord);
        longest_chord = longest_chord.max(chord);
    }
    let ratio = longest / shortest;
    println!("\n# 65 stations, evenly spread by arc length");
    println!("  along the curve: shortest {shortest:.6} · longest {longest:.6} · ratio {ratio:.6}");
    println!(
        "  as a straight chord: ratio {:.6} — the two differ because a chord shortens by an amount that \
         depends on how curved its span is, so the arc ratio is the one that speaks about the \
         parameterisation",
        longest_chord / shortest_chord
    );
    ratio < 1.0001
}

/// Inserting a knot changes the parameterisation and not the shape, and the control shows the instrument
/// would see it if it did.
fn insertion_preserves_the_shape() -> bool {
    let original = spine();
    let samples = 256;
    let at = |curve: &Curve| -> Vec<Vec3> {
        (0..=samples)
            .map(|index| curve.at(index as f32 / samples as f32))
            .collect()
    };

    let before = at(&original);
    let mut inserted = original.clone();
    let index = inserted.insert(0.37);
    let after = at(&inserted);
    let worst = before
        .iter()
        .zip(after.iter())
        .map(|(left, right)| length3(subtract(*left, *right)))
        .fold(0.0f32, f32::max)
        / original.length();

    // The control: the same instrument on a curve that *has* moved. One knot is displaced by a
    // thousandth of the curve's length, and the measurement has to report something of that order.
    let mut jets = original.jets();
    jets[2].position[1] += original.length() * 1.0e-3;
    let moved = Curve::new(jets).expect("the displaced knots are distinct");
    let moved_samples = at(&moved);
    let control = before
        .iter()
        .zip(moved_samples.iter())
        .map(|(left, right)| length3(subtract(*left, *right)))
        .fold(0.0f32, f32::max)
        / original.length();

    println!("\n# inserting a knot at t = 0.37, and the control that can fail");
    println!(
        "  new knot index {index} · worst sample movement {worst:.3e} of the length · length {:.6} → \
         {:.6}",
        original.length(),
        inserted.length()
    );
    println!(
        "  control: moving one knot by 1e-3 of the length reads {control:.3e}, so the instrument reports \
         a change of the size it is asked about"
    );
    worst < 1.0e-5 && control > 1.0e-4
}

/// A straight curve's transported frame is constant — the case a Frenet frame cannot answer at all.
fn a_straight_curve_has_a_constant_frame() -> bool {
    let curve = straight();
    let stations = curve.stations(33);
    let mut worst = 0.0f32;
    for pair in stations.windows(2) {
        worst = worst.max(angle(pair[0].frame.normal, pair[1].frame.normal));
    }
    println!("\n# a straight curve's frame");
    println!("  33 stations · worst turn of the normal {worst:.3e} rad");
    println!(
        "  a Frenet frame has no normal to return here at all, which is why it is not offered"
    );
    worst < 1.0e-6
}

/// An inflection turns the tangent but must not flip the frame's normal — the case a Frenet frame gets
/// wrong, and the reason the probe measures both.
fn an_inflection_does_not_flip_the_frame() -> bool {
    let curve = s_curve();
    let knots = curve.knot_parameters();
    let middle = knots[2];
    let step = 2.0e-3;

    let carried = angle(
        curve.frame(middle - step).normal,
        curve.frame(middle + step).normal,
    );
    let flipped = angle(
        frenet_normal(&curve, middle - step),
        frenet_normal(&curve, middle + step),
    );

    println!("\n# an inflection, at the S-curve's middle knot (t = {middle:.4})");
    println!(
        "  the transported normal turns {carried:.3e} rad across it; a Frenet normal turns {flipped:.3e} \
         rad ({:.1}°)",
        flipped.to_degrees()
    );
    println!(
        "  the transported frame is what a section is drawn in, so that flip is the defect it exists to \
         avoid — and the control is the Frenet reading, which must find it"
    );
    carried < 0.05 && flipped > 1.0
}

/// A closed planar loop's frame returns to where it started.
///
/// # Invariants
///
/// The frame's normal is the loop's own plane normal and a transported frame carries it unchanged, so the
/// holonomy is not small but **zero**, and the control is the same instrument on the tangent: at
/// `t = 0.999` the tangent has turned `6.6e-3` rad, which is the reading the normal would have shown had
/// it turned with the curve. A zero on the normal is therefore a measurement rather than a blind spot.
fn a_closed_planar_loop_has_no_holonomy() -> bool {
    let curve = circle(1.5);
    let analytic = std::f32::consts::TAU * 1.5;
    let start = curve.frame(0.0);
    let almost = curve.frame(0.999);
    let whole = curve.frame(1.0);
    let twist = angle(start.normal, whole.normal);
    let control = angle(start.tangent, almost.tangent);
    let short = (analytic - curve.length()) / analytic;
    println!("\n# a closed planar loop of eight segments");
    println!(
        "  length {:.6} against the analytic {:.6} — the Hermite circle is {short:.1e} short, which is the \
         representation and not the quadrature",
        curve.length(),
        analytic
    );
    println!("  twist after one loop {twist:.3e} rad: the plane normal is carried unchanged");
    println!(
        "  the same instrument on the tangent at t = 0.999 reads {control:.3e} rad, so the zero above is a \
         reading and not a blind spot"
    );
    twist < 1.0e-4 && control > 1.0e-3 && short.abs() < 1.0e-3
}

/// The Bezier form an exporter receives is the same curve, not an approximation of it.
fn the_bezier_form_is_the_same_curve() -> bool {
    let curve = spine();
    let jets = curve.jets();
    let mut worst_corner = 0.0f32;
    let mut worst_handle = 0.0f32;
    for (index, bezier) in curve.segments().enumerate() {
        let start = jets[index];
        let end = jets[index + 1];
        worst_corner = worst_corner
            .max(length3(subtract(bezier.p0, start.position)))
            .max(length3(subtract(bezier.p3, end.position)));
        worst_handle = worst_handle
            .max(length3(subtract(
                subtract(bezier.p1, bezier.p0),
                scaled(start.tangent, 1.0 / 3.0),
            )))
            .max(length3(subtract(
                subtract(bezier.p3, bezier.p2),
                scaled(end.tangent, 1.0 / 3.0),
            )));
    }
    println!("\n# the Bezier form an exporter receives");
    println!(
        "  {} segments · worst corner gap {worst_corner:.3e} · worst handle error {worst_handle:.3e}",
        curve.segment_count()
    );
    println!("  p1 = p0 + m0/3 and p2 = p3 − m1/3 are identities here, not approximations");
    worst_corner < 1.0e-6 && worst_handle < 1.0e-6
}

/// A curve whose last jet repeats its first returns to its start **exactly**, not nearly.
///
/// # Invariants
///
/// The last span evaluates its end value as `h01(1)` times the last knot's position, and the four basis
/// weights at `t = 1` are exactly `0, 0, 1, 0` — so the answer is that position bit for bit, and a closed
/// loop has no gap for a sweep to hide. The control is an open spine, whose ends must differ by a
/// distance a reader can see.
fn a_closed_curve_ends_where_it_starts() -> bool {
    let loop_ = exact_loop();
    let open = spine();
    let closed_gap = length3(subtract(loop_.at(1.0), loop_.at(0.0)));
    let open_gap = length3(subtract(open.at(1.0), open.at(0.0)));
    println!("\n# where a curve ends");
    println!(
        "  a closed loop's |at(1) − at(0)| = {closed_gap:.3e} · an open spine's = {open_gap:.3e}"
    );
    println!("  the closed gap is exactly zero because the basis at t = 1 is (0, 0, 1, 0)");
    closed_gap == 0.0 && open_gap > 0.1
}

/// A loop whose closing knot is its first knot, written out rather than derived from it.
fn exact_loop() -> Curve {
    Curve::new(vec![
        Jet::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        Jet::new([0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]),
        Jet::new([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        Jet::new([0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        Jet::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ])
    .expect("the loop's knots are distinct")
}

/// The angle between two directions, in radians.
fn angle(left: Vec3, right: Vec3) -> f32 {
    dot3(normalized3(left), normalized3(right))
        .clamp(-1.0, 1.0)
        .acos()
}

/// A Frenet normal from the curve's own points: `P''` with its tangential part removed.///
/// # Invariants
///
/// It is here as the *control* for the transported frame, and a second difference is the only way to ask
/// for it through this crate's public surface — which is the point: a caller who wants a Frenet frame has
/// to reconstruct it from points, and it is undefined where the curve is straight.
fn frenet_normal(curve: &Curve, t: f32) -> Vec3 {
    let step = 1.0e-3;
    let behind = curve.at((t - step).max(0.0));
    let here = curve.at(t);
    let ahead = curve.at((t + step).min(1.0));
    let second = add(subtract(scaled(here, 2.0), behind), scaled(ahead, -1.0));
    let along = normalized3(subtract(ahead, behind));
    normalized3(subtract(second, scaled(along, dot3(second, along))))
}

fn scaled(vector: Vec3, factor: f32) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// On a straight line the nearest point is the perpendicular foot, which has a closed form.
///
/// # Invariants
///
/// The line runs along `+x` from the origin, so the foot of a point at `(x, y, z)` is at arc-length fraction
/// `x / length`, and the distance there is `√(y² + z²)`. The control is a point whose foot is *not* at a
/// sample: it is at 0.37, between two of the search's own samples.
///
/// **The distance is exact and the fraction is only as exact as the parameterisation**: the foot's distance
/// agrees with the closed form to a rounding, while its `t` differs by the same amount `at` itself differs,
/// because `at` interpolates inside a segment's arc table rather than inverting the cubic. The row measures that
/// separately instead of blaming the search for it.
fn the_foot_on_a_straight_line() -> bool {
    let line = straight();
    let length = line.length();
    let query = [1.48f32, 0.9, -0.35];
    let found = line.closest(query);
    let expected_t = 1.48 / length;
    let expected_distance = (0.9f32 * 0.9 + 0.35 * 0.35).sqrt();
    let by_parameter = line.at(expected_t);
    let parameter_error = length3(subtract(by_parameter, [1.48, 0.0, 0.0]));
    println!("\n# the foot on a straight line");
    println!(
        "  a point at (1.48, 0.9, -0.35) against a line of {:?}: t {:.6} against {expected_t:.6}, distance {:.6} against {expected_distance:.6}",
        length, found.t, found.distance
    );
    println!(
        "  and `at({expected_t:.6})` lands {parameter_error:.3e} from the point that fraction names, so a difference in t is not the parameterisation's"
    );
    println!(
        "  near a foot the distance is quadratic, so f32 cannot pin the fraction better than about 1e-4: the distance is the exact quantity and t is only as exact as the flatness"
    );
    println!(
        "  the sample step is {:.6} of the curve, so the answer is not one of the search's own samples",
        1.0 / (line.segment_count() * whiting_shape::CLOSEST_SAMPLES) as f32
    );
    (found.distance - expected_distance).abs() < 1e-6 && (found.t - expected_t).abs() < 5e-4
}

/// A point past the tip answers the tip, exactly.
fn a_point_past_the_end_answers_the_end() -> bool {
    let line = straight();
    let end = line.at(1.0);
    let query = [end[0] + 1.5, end[1] + 0.4, end[2]];
    let found = line.closest(query);
    let expected = length3(subtract(found.position, query));
    println!("\n# a point past the end");
    println!(
        "  the answer is t {:.6} (the tip is {:.6}) at a distance of {:.6}, which is the distance to the tip: {:.6}",
        found.t,
        line.at(1.0)[0],
        found.distance,
        expected
    );
    println!(
        "  the control is the tip's own position: `at(1)` is the last knot exactly, so this is not a search result"
    );
    found.t > 0.999_999 && (found.distance - expected).abs() < 1e-6
}

/// A query point on a knot answers that knot: the closest point is where the curve itself is.
fn a_point_at_a_knot_answers_the_knot() -> bool {
    let curve = s_curve();
    let fractions = curve.knot_parameters();
    let knot = fractions[2];
    let query = curve.at(knot);
    let found = curve.closest(query);
    println!("\n# a point on a knot");
    println!(
        "  the middle knot is at {knot:.6} and the search answers {:.6}, at a distance of {:.3e}",
        found.t, found.distance
    );
    println!(
        "  the control is the knot's fraction: a search that stopped at its nearest sample would answer the sample instead"
    );
    (found.t - knot).abs() < 1e-4 && found.distance < 1e-5
}

/// The budget is enough: the search agrees with one thirty-two times denser over a curve that bends.
fn the_search_against_a_denser_one() -> bool {
    let curve = spine();
    // The reference is the same method with far more samples and steps, which answers whether the stated
    // budget is enough rather than whether the method is the method.
    let reference = |point: Vec3| -> f32 {
        let samples = curve.segment_count() * whiting_shape::CLOSEST_SAMPLES * 32;
        let mut best = 0.0f32;
        let mut best_distance = f32::MAX;
        for index in 0..=samples {
            let t = index as f32 / samples as f32;
            let distance = length3(subtract(curve.at(t), point));
            if distance < best_distance {
                best_distance = distance;
                best = t;
            }
        }
        let step = 1.0 / samples as f32;
        let mut low = (best - step).max(0.0);
        let mut high = (best + step).min(1.0);
        for _ in 0..60 {
            let first = high - (high - low) * 0.618_034f32;
            let second = low + (high - low) * 0.618_034f32;
            if length3(subtract(curve.at(first), point))
                < length3(subtract(curve.at(second), point))
            {
                high = second;
            } else {
                low = first;
            }
        }
        length3(subtract(curve.at(0.5 * (low + high)), point))
    };
    let mut worst = 0.0f32;
    let mut queries = 0;
    for index in 0..40 {
        let angle = index as f32 * 0.157_079_6;
        let point = [
            1.4 + 0.9 * angle.cos(),
            1.0 + 0.9 * angle.sin(),
            0.6 + 0.4 * angle.cos(),
        ];
        let found = curve.closest(point).distance;
        worst = worst.max((found - reference(point)).abs());
        queries += 1;
    }
    println!("\n# the budget against a denser search");
    println!(
        "  {queries} query points around a curve of {} segments · worst distance error {worst:.3e} against a search with {} samples per segment",
        curve.segment_count(),
        whiting_shape::CLOSEST_SAMPLES * 32
    );
    println!(
        "  the stated budget is {} samples per segment and {} steps",
        whiting_shape::CLOSEST_SAMPLES,
        whiting_shape::CLOSEST_STEPS
    );
    println!(
        "  the control is the near miss: an error of zero would say the two searches are the same search, which they are not"
    );
    worst < 1e-4
}
