//! Measures the section's invariants, each against a threshold and each with a control that could have
//! failed it, and exits non-zero if one does.
//!
//! Nothing here is a test file: this is the probe that says whether `shape::section` does what its own
//! documentation claims, and every reading is printed with the number it is compared against. The
//! contract it grades is D4's — the section is a shape the sweep scales, `v` is a fraction of arc
//! length as `t` is along the spine, and `v` increases counter-clockwise for every section.
//!
//! The arc-length rows compare against instruments the module shares no code with: the lens's fractions
//! are re-integrated here by a 200 000-step midpoint rule accumulated in `f64`, and its perimeter is
//! checked against Ramanujan's closed form. **One threshold moves deliberately.** The lens row states
//! `1e-3` against Ramanujan, which no correct section can meet: at a 50:1 aspect that formula is
//! `1.1e-2` short of the ellipse's own length, and the `24`-gon's chords are a further `2.1e-3` short of
//! it. The row therefore grades against the formula at the tolerance *it* holds at that aspect, and the
//! control inside it grades the same instrument at a `2:1` aspect where the formula agrees to `1e-5` —
//! so a number that moves between those two rows is the closed form's error and not the module's.
//!
//! ```text
//! cargo run -p whiting-shape --example section_probe
//! ```

use whiting_shape::math::{Vec2, clamp, cross, difference, distance, dot, length, lerp};
use whiting_shape::section::{Outline, Section, SectionError};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("circle ring", circle_ring()));
    checks.push(("flat ring", flat_ring()));
    checks.push(("square outline", square_ring()));
    checks.push(("winding normalised", clockwise_is_the_same_loop()));
    checks.push(("L outline", l_shape_ring()));
    checks.push(("errors refused", errors_are_refused()));
    checks.push(("flat is an ellipse", flat_is_an_ellipse()));
    checks.push(("reach", reach_in_a_direction()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<24} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} row(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// How many steps the independent integrations take.
const STEPS: usize = 200_000;

/// A unit circle: every point at radius one, every chord the same length, and `v` at zero on `(+x, 0)`.
fn circle_ring() -> bool {
    let ring = Section::circle().ring(16);
    let mut worst_radius = 0.0f32;
    let mut chords = Vec::with_capacity(ring.len());
    for index in 0..ring.len() {
        worst_radius = worst_radius.max((length(ring[index]) - 1.0).abs());
        chords.push(distance(ring[index], ring[(index + 1) % ring.len()]));
    }
    let mean = chords.iter().sum::<f32>() / chords.len() as f32;
    let worst_chord = chords.iter().fold(0.0f32, |worst, chord| {
        worst.max((chord - mean).abs() / mean)
    });
    let first = distance(ring[0], [1.0, 0.0]);
    let area = shoelace(&ring);
    let stated = 0.5 * 16.0 * (std::f32::consts::TAU / 16.0).sin();

    println!(
        "\n# circle().ring(16) — the one section whose uniform angle is its uniform arc length"
    );
    println!(
        "  16 points · worst |p| − 1 {worst_radius:.3e} · chord {mean:.6}, worst spread \
         {worst_chord:.3e} relative to the mean"
    );
    println!(
        "  ring[0] is ({:.7}, {:.7}), {first:.3e} from (1, 0) — `v = 0` is a stated place, not an artefact",
        ring[0][0], ring[0][1]
    );
    println!(
        "  shoelace area {area:.7} against {stated:.7} for the exact 16-gon · difference \
         {:.3e} · positive: {}",
        (area - stated).abs(),
        area > 0.0
    );
    worst_radius < 1.0e-6
        && worst_chord < 1.0e-6
        && first < 1.0e-7
        && (area - stated).abs() < 1.0e-6
}

/// A blade as a lens: `flat` is an ellipse, and `v` is spread by arc length rather than by angle.
fn flat_ring() -> bool {
    let (half_width, thickness, around) = (1.0f32, 0.02f32, 24usize);
    let ring = Section::flat(half_width, thickness).ring(around);
    let (x, y) = (half_width as f64, thickness as f64);
    let total = ellipse_arc(x, y, std::f64::consts::TAU, STEPS);

    let mut worst_off = 0.0f32;
    let mut worst_fraction = 0.0f64;
    let mut worst_control = 0.0f64;
    for (index, point) in ring.iter().enumerate() {
        let v = index as f64 / around as f64;
        worst_off = worst_off
            .max(((point[0] / half_width).powi(2) + (point[1] / thickness).powi(2) - 1.0).abs());
        let theta = theta_of(*point, x, y);
        worst_fraction = worst_fraction.max((ellipse_arc(x, y, theta, STEPS) / total - v).abs());
        let control = ellipse_arc(x, y, std::f64::consts::TAU * v, STEPS) / total;
        worst_control = worst_control.max((control - v).abs());
    }

    let perimeter = recovered_perimeter(x, y, &ring);
    let chords: f32 = (0..around)
        .map(|index| distance(ring[index], ring[(index + 1) % around]))
        .sum();
    let stated = ramanujan(x, y);
    let control_ring = Section::ellipse(1.0, 0.5).ring(around);
    let control_stated = ramanujan(1.0, 0.5);
    let control_perimeter = recovered_perimeter(1.0, 0.5, &control_ring);

    println!(
        "\n# flat(1.0, 0.02).ring(24) — a lens, which is how a blade is a closed solid and not a sheet"
    );
    println!("  24 points · worst (x/a)² + (y/b)² − 1 {worst_off:.3e}");
    println!(
        "  arc fraction by an independent {STEPS}-step integration from θ = 0: worst \
         |measured − i/24| {worst_fraction:.3e}"
    );
    println!(
        "  control, the same 24 points at a uniform θ = 2πi/24: worst |measured − i/24| \
         {worst_control:.3e} — that is the reading a `ring` which sampled the angle would give, and it \
         is the row that can fail"
    );
    println!(
        "  the lens's perimeter, recovered from its own ring {perimeter:.6} · Ramanujan {stated:.6} · \
         difference {:.3e}",
        (perimeter - stated).abs()
    );
    println!(
        "  the 24 chords themselves measure {chords:.6}, a further {:.3e} short of that — a polygonal \
         deficit, so the row grades the recovered arc length rather than the chord sum",
        perimeter - chords as f64
    );
    println!(
        "  control, the same instrument on ellipse(1.0, 0.5): recovered {control_perimeter:.6} against \
         Ramanujan {control_stated:.6}, difference {:.3e} — so the 50:1 difference above is Ramanujan's \
         own error at a 50:1 aspect, not this quadrature's",
        (control_perimeter - control_stated).abs()
    );
    worst_off < 1.0e-6
        && worst_fraction < 2.0e-3
        && worst_control > 2.0e-3
        && (perimeter - stated).abs() < 2.0e-2
        && (control_perimeter - control_stated).abs() < 1.0e-4
}

/// A square authored counter-clockwise: the ring lands on its edges, a quarter of a side apart.
fn square_ring() -> bool {
    let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let section = Section::outline(&points).expect("the square has four distinct finite points");
    let ring = section.ring(16);

    let worst_off = worst_distance_to(&ring, &points);
    let mut worst_chord = 0.0f32;
    for index in 0..ring.len() {
        worst_chord =
            worst_chord.max((distance(ring[index], ring[(index + 1) % ring.len()]) - 0.25).abs());
    }
    let area = shoelace(&ring);
    let start_exact = ring[0] == points[0];

    println!("\n# outline([(0,0), (1,0), (1,1), (0,1)]).ring(16) — the loop's own table, exactly");
    println!(
        "  16 points · worst distance to the boundary {worst_off:.3e} · worst chord − 0.25 \
         {worst_chord:.3e}"
    );
    println!(
        "  ring[0] is ({}, {}) and is points[0] exactly: {start_exact} · shoelace area {area:.7} against \
         1.0, difference {:.3e}",
        ring[0][0],
        ring[0][1],
        (area - 1.0).abs()
    );
    worst_off < 1.0e-6 && worst_chord < 1.0e-6 && start_exact && (area - 1.0).abs() < 1.0e-6
}

/// The same square listed clockwise: a section is a shape, so the winding an author typed is not it.
fn clockwise_is_the_same_loop() -> bool {
    let counter = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let clockwise = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    let first = Section::outline(&counter).expect("the square is closed and finite");
    let second = Section::outline(&clockwise).expect("the square is closed and finite");
    let (left, right) = (first.ring(16), second.ring(16));
    let worst = left
        .iter()
        .zip(right.iter())
        .map(|(one, other)| length(difference(*other, *one)))
        .fold(0.0f32, f32::max);
    let start_exact = right[0] == [0.0, 0.0];

    println!("\n# the same square listed clockwise — the normalisation row");
    println!(
        "  stored as {} after normalising, which is the counter-clockwise list itself",
        format_points(outline_of(&second).points())
    );
    println!(
        "  ring[0] is ({}, {}) — the author's first point is kept first: {start_exact}",
        right[0][0], right[0][1]
    );
    println!(
        "  point-by-point worst difference between the two rings {worst:.3e}, so a sweep's faces cannot \
         depend on the order the points were typed in"
    );
    worst < 1.0e-6 && start_exact
}

/// An L, so the ring has to cross a corner: three points on edges, none of them on a vertex by luck.
fn l_shape_ring() -> bool {
    let points = [
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ];
    let section = Section::outline(&points).expect("the L has six distinct finite points");
    let ring = section.ring(12);
    let worst_off = worst_distance_to(&ring, &points);
    let start_exact = ring[0] == points[0];
    let chords: Vec<f32> = (0..ring.len())
        .map(|index| distance(ring[index], ring[(index + 1) % ring.len()]))
        .collect();
    let mean = chords.iter().sum::<f32>() / chords.len() as f32;
    let worst_chord = chords.iter().fold(0.0f32, |worst, chord| {
        worst.max((chord - mean).abs() / mean)
    });

    println!("\n# an L-shaped outline of six points, ring(12)");
    println!(
        "  perimeter {:.6} · 12 points · worst distance to the boundary {worst_off:.3e}",
        outline_of(&section).perimeter()
    );
    println!(
        "  chord {mean:.6}, worst spread {worst_chord:.3e} relative to the mean — and the chords here \
         must differ: the ones spanning the two inner corners are shorter than the arc they stand for, \
         which is what a polyline corner is and why the section is measured by arc length"
    );
    println!(
        "  ring[0] is ({}, {}) and is points[0] exactly: {start_exact} — the corners land on vertices \
         rather than near them, because the table is measured from the points",
        ring[0][0], ring[0][1]
    );
    worst_off < 1.0e-6 && start_exact
}

/// What the loop's own table refuses, and why each refusal is a division rather than a taste.
fn errors_are_refused() -> bool {
    let few = Section::outline(&[[0.0, 0.0], [1.0, 0.0]]);
    let not_finite = Section::outline(&[[0.0, 0.0], [1.0, 0.0], [f32::NAN, 1.0]]);
    let repeated = Section::outline(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]);
    let repeated_inside = Section::outline(&[[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    let infinite = Section::outline(&[[0.0, 0.0], [1.0, 0.0], [1.0, f32::INFINITY]]);

    println!("\n# what a refused outline says instead of accepting it");
    println!("  two points                          → {}", report(&few));
    println!(
        "  a NaN coordinate                   → {}",
        report(&not_finite)
    );
    println!(
        "  an infinite coordinate              → {}",
        report(&infinite)
    );
    println!(
        "  the last point repeating the first  → {}",
        report(&repeated)
    );
    println!(
        "  two interior points equal           → {}",
        report(&repeated_inside)
    );
    println!(
        "  the closing pair is the case a caller writes by accident and the inversion would divide by \
         zero on, so it is refused with the interior one"
    );
    few == Err(SectionError::TooFewPoints)
        && not_finite == Err(SectionError::NotFinite)
        && infinite == Err(SectionError::NotFinite)
        && repeated == Err(SectionError::RepeatedPoint)
        && repeated_inside == Err(SectionError::RepeatedPoint)
}

/// A circle and a blade are one ellipse at two aspect ratios, and `flat` is not a second kind of shape.
fn flat_is_an_ellipse() -> bool {
    let (x, y, around) = (0.5f32, 0.01f32, 16usize);
    let lens = Section::flat(x, y).ring(around);
    let ellipse = Section::ellipse(x, y).ring(around);
    let worst = lens
        .iter()
        .zip(ellipse.iter())
        .map(|(one, other)| length(difference(*other, *one)))
        .fold(0.0f32, f32::max);
    let (lens_x, lens_y) = extents(&lens);
    let (ellipse_x, ellipse_y) = extents(&ellipse);

    println!("\n# ellipse(0.5, 0.01) and flat(0.5, 0.01) — the unification row");
    println!("  flat(0.5, 0.01)       max |x| {lens_x:.7} · max |y| {lens_y:.7}");
    println!("  ellipse(0.5, 0.01)    max |x| {ellipse_x:.7} · max |y| {ellipse_y:.7}");
    println!(
        "  point-by-point worst difference {worst:.3e} — the blade is the ellipse, so a sweep has one \
         arm to scale and no second shape to keep in step"
    );
    worst == 0.0 && lens_x == ellipse_x && lens_y == ellipse_y
}

/// The probe's own arc-length instrument: a midpoint rule on `|dP/dθ|`, accumulated in `f64`.
///
/// # Invariants
///
/// It shares no code with the module's table, and `f64` is not decoration: a 200 000-term `f32` sum
/// carries about `1e-4` of its own rounding, which is the order of the thing being measured.
fn ellipse_arc(x: f64, y: f64, theta: f64, steps: usize) -> f64 {
    let width = theta / steps as f64;
    let mut total = 0.0;
    for step in 0..steps {
        let mid = (step as f64 + 0.5) * width;
        total += width * (x * mid.sin()).hypot(y * mid.cos());
    }
    total
}

/// The point's own parameter on the ellipse, read as `atan2(y / b, x / a)` and folded into `[0, 2π)`.
fn theta_of(point: Vec2, x: f64, y: f64) -> f64 {
    (point[1] as f64 / y)
        .atan2(point[0] as f64 / x)
        .rem_euclid(std::f64::consts::TAU)
}

/// The perimeter a section reports for itself, recovered through its own ring.
///
/// # Invariants
///
/// The ellipse arm has no `perimeter` — a `Section` is a shape and the sweep scales it — so the ring is
/// the only window onto the table. The module claims point `i` is at fraction `i / around`, and this
/// divides the independently measured arc to each such point by the fraction it claimed, so the average
/// is the table's total and not one placement.
fn recovered_perimeter(x: f64, y: f64, ring: &[Vec2]) -> f64 {
    let around = ring.len() as f64;
    let total: f64 = ring
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, point)| {
            let theta = theta_of(*point, x, y);
            ellipse_arc(x, y, theta, STEPS) / (index as f64 / around)
        })
        .sum();
    total / (around - 1.0)
}

/// Ramanujan's approximation to an ellipse's circumference.
fn ramanujan(x: f64, y: f64) -> f64 {
    std::f64::consts::PI * (3.0 * (x + y) - ((3.0 * x + y) * (x + 3.0 * y)).sqrt())
}

/// The signed area of a closed polygon, positive for the counter-clockwise winding.
fn shoelace(points: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..points.len() {
        twice += cross(points[index], points[(index + 1) % points.len()]);
    }
    0.5 * twice
}

/// The largest distance from any of the ring's points to the polygon's boundary.
fn worst_distance_to(ring: &[Vec2], polygon: &[Vec2]) -> f32 {
    ring.iter()
        .map(|point| {
            (0..polygon.len())
                .map(|index| {
                    distance_to_segment(
                        *point,
                        polygon[index],
                        polygon[(index + 1) % polygon.len()],
                    )
                })
                .fold(f32::INFINITY, f32::min)
        })
        .fold(0.0f32, f32::max)
}

/// The distance from a point to a segment, which is the boundary's own distance and not a vertex's.
fn distance_to_segment(point: Vec2, from: Vec2, to: Vec2) -> f32 {
    let edge = difference(to, from);
    let span = dot(edge, edge);
    let along = if span > 0.0 {
        clamp(dot(difference(point, from), edge) / span, 0.0, 1.0)
    } else {
        0.0
    };
    length(difference(point, lerp(from, to, along)))
}

/// The largest `|x|` and `|y|` a ring reaches.
fn extents(ring: &[Vec2]) -> (f32, f32) {
    ring.iter().fold((0.0f32, 0.0f32), |(wide, tall), point| {
        (wide.max(point[0].abs()), tall.max(point[1].abs()))
    })
}

/// The stored points, formatted the way they were authored.
fn format_points(points: &[Vec2]) -> String {
    let listed: Vec<String> = points
        .iter()
        .map(|point| format!("({}, {})", point[0], point[1]))
        .collect();
    format!("[{}]", listed.join(", "))
}

/// The outline a section stores, for the readings only an outline has.
fn outline_of(section: &Section) -> &Outline {
    match section {
        Section::Outline(outline) => outline,
        Section::Ellipse { .. } => panic!("this row reads an outline"),
    }
}

/// An error as the word for it, or the loop that was built where none should have been.
fn report(result: &Result<Section, SectionError>) -> String {
    match result {
        Err(SectionError::TooFewPoints) => "TooFewPoints".to_string(),
        Err(SectionError::NotFinite) => "NotFinite".to_string(),
        Err(SectionError::RepeatedPoint) => "RepeatedPoint".to_string(),
        Ok(_) => "a section, which it must not be".to_string(),
    }
}

/// How far a section reaches in a direction: the closed form for an ellipse, the first edge for an outline.
///
/// # Invariants
///
/// A unit circle reaches one in every direction; a 2:1 ellipse reaches its two half axes on the axes and
/// `1 / √(0.5/4 + 0.5)` on the diagonal; a square outline reaches one on an axis and `√2` at its corner; a flat lens
/// reaches its half width and its thickness. The control is a direction of no length, which answers zero rather
/// than a guess.
fn reach_in_a_direction() -> bool {
    let circle = Section::circle();
    let mut worst_circle = 0.0f32;
    for step in 0..16 {
        let angle = step as f32 * std::f32::consts::TAU / 16.0;
        let reach = circle.reach([angle.cos(), angle.sin()]);
        worst_circle = worst_circle.max((reach - 1.0).abs());
    }
    let ellipse = Section::ellipse(2.0, 1.0);
    let along_x = ellipse.reach([1.0, 0.0]);
    let along_y = ellipse.reach([0.0, 1.0]);
    let diagonal = ellipse.reach([1.0, 1.0]);
    let expected_diagonal = 1.0 / (0.5f32 / 4.0 + 0.5).sqrt();
    let flat = Section::flat(1.0, 0.1);
    let flat_wide = flat.reach([1.0, 0.0]);
    let flat_thin = flat.reach([0.0, 1.0]);
    let square = Section::outline(&[[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]])
        .expect("the square is a loop");
    let square_axis = square.reach([1.0, 0.0]);
    let square_corner = square.reach([1.0, 1.0]);
    let nothing = circle.reach([0.0, 0.0]);
    println!("\n# how far a section reaches");
    println!("  a unit circle, 16 directions: worst |reach - 1| {worst_circle:.3e}");
    println!(
        "  a 2:1 ellipse: {along_x:.6} along x, {along_y:.6} along y, {diagonal:.6} on the diagonal against {expected_diagonal:.6}"
    );
    println!("  a flat lens of half width 1 and thickness 0.1: {flat_wide:.6} and {flat_thin:.6}");
    println!(
        "  a square outline: {square_axis:.6} on an axis and {square_corner:.6} at its corner, which is the √2 the ray travels"
    );
    println!(
        "  a direction of no length answers {nothing:.6}, and the control is that it is not one"
    );
    worst_circle < 1e-6
        && (along_x - 2.0).abs() < 1e-6
        && (along_y - 1.0).abs() < 1e-6
        && (diagonal - expected_diagonal).abs() < 1e-6
        && (flat_wide - 1.0).abs() < 1e-6
        && (flat_thin - 0.1).abs() < 1e-6
        && (square_axis - 1.0).abs() < 1e-6
        && (square_corner - std::f32::consts::SQRT_2).abs() < 1e-6
        && nothing == 0.0
}
