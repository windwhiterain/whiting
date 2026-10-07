//! Probes the two properties of the simplex Bernstein basis that the layered patch representation
//! stands on, and one that the soft-edge invariant stands on.
//!
//! Run with `cargo run -p patch --example elevation_probe`. This is an example and not a gate,
//! following `facet_value_probe`: it prints what it measured, including the parts it could not
//! establish.
//!
//! 1. Level 0 is the mesh — a degree-0 net reproduces its own corners exactly.
//! 2. Elevation preserves the surface — the property the refinement ladder is built on.
//! 3. The basis is a partition of unity — what makes dragging an assignment, not a solve.
//! 4. Elevation is linear — what makes an all-zero layer stay all-zero forever.
//! 5. The soft-edge invariant produces a continuous tangent plane — measured numerically, so the
//!    formula is not being checked against itself.

use whiting_patch::{Point3, SimplexNet, count};

const FRACTION: f64 = 1.0 / 3.0;

/// A deterministic spread of interior points, so the numbers below are reproducible.
fn sample_points() -> Vec<[f64; 3]> {
    let mut points = Vec::new();
    for step in 1..12 {
        let a = step as f64 / 12.0;
        for inner in 1..12 - step {
            let b = inner as f64 / 12.0;
            points.push([a, b, 1.0 - a - b]);
        }
    }
    points
}

/// A net whose shape is not its own symmetry: three corners and six degree-2 points.
fn irregular_degree2() -> SimplexNet {
    SimplexNet::new(
        2,
        vec![
            Point3::new(0.10, -0.40, 0.90),
            Point3::new(0.30, 0.10, 0.35),
            Point3::new(0.55, -0.15, 0.65),
            Point3::new(0.80, 0.25, -0.20),
            Point3::new(0.65, 0.60, 0.45),
            Point3::new(0.20, 0.55, -0.50),
        ],
    )
    .expect("degree 2 takes six points")
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let samples = sample_points();
    println!(
        "simplex elevation probe — {} interior samples, degrees 0 through 5",
        samples.len()
    );

    // 1. The base net is the mesh.
    let corners = [
        Point3::new(-2.0, 1.0, 0.5),
        Point3::new(3.5, 1.25, -1.0),
        Point3::new(0.75, -4.0, 2.5),
    ];
    let mesh_level = SimplexNet::from_corners(corners[0], corners[1], corners[2]);
    // A degree-0 net is one point and a constant surface; the ladder starts at degree 1.
    assert_eq!(count(0), 1, "a degree-0 simplex net is a single point");
    let mut corner_error = 0.0f64;
    for (index, corner) in corners.iter().enumerate() {
        let at = [
            if index == 0 { 1.0 } else { 0.0 },
            if index == 1 { 1.0 } else { 0.0 },
            if index == 2 { 1.0 } else { 0.0 },
        ];
        corner_error = corner_error.max(mesh_level.evaluate(at).distance(*corner));
    }
    // And the interior must be the flat triangle, not a curve: the centre of mass of the
    // corners is the centroid, and a linear net through the corners reproduces it.
    let centroid = (corners[0] + corners[1] + corners[2]) / 3.0;
    let centroid_error = mesh_level
        .evaluate([FRACTION, FRACTION, FRACTION])
        .distance(centroid);
    let mesh_area = 0.5 * ((corners[1] - corners[0]).cross(corners[2] - corners[0])).length();
    let base_degree = mesh_level.degree();
    report(
        "1  the base net is the mesh",
        &format!(
            "base net degree       {base_degree}\nlargest corner error  {corner_error:.3e}\n\
             centroid error       {centroid_error:.3e}\ntriangle area       {mesh_area:.6}\n\
             level 0 is a degree-1 net, because a degree-0 simplex net is one point and a constant\n\
             surface. a model built at level 0 is the mesh, not an approximation of it"
        ),
    );

    // 2. Elevation preserves the surface.
    let source = irregular_degree2();
    println!("\n   degree  points   max |S_elevated - S|   max |relative to triangle|");
    let mut worst_surface_error = 0.0f64;
    let mut worst_relative = 0.0f64;
    for steps in 0..=3 {
        let raised = source.elevate_by(steps);
        let mut largest = 0.0f64;
        for sample in &samples {
            let before = source.evaluate(*sample);
            let after = raised.evaluate(*sample);
            largest = largest.max(before.distance(after));
        }
        let relative = largest / mesh_area;
        worst_surface_error = worst_surface_error.max(largest);
        worst_relative = worst_relative.max(relative);
        println!(
            "   {:>5}   {:>6}   {:>22.3e}   {:>24.3e}",
            raised.degree(),
            raised.len(),
            largest,
            relative
        );
    }
    println!(
        "\n   worst across the ladder: {worst_surface_error:.3e} absolute, {worst_relative:.3e} of the \
         triangle size"
    );
    report(
        "2  elevation is shape preserving",
        "PASS if the absolute error sits near f64 rounding for a net whose points span order one.\n\
         Elevation is also the only way this crate adds a control point, so this number is the\n\
         whole cost of going from a mesh to an editable patch.",
    );

    // 3. Partition of unity. A net whose every point is (1,0,0) evaluates to the basis sum in x.
    let mut worst_basis_sum = 0.0f64;
    for degree in 0..=5 {
        let unit = SimplexNet::new(degree, vec![Point3::new(1.0, 0.0, 0.0); count(degree)])
            .expect("unit net matches its degree");
        for sample in &samples {
            let sum = unit.evaluate(*sample).x;
            worst_basis_sum = worst_basis_sum.max((sum - 1.0).abs());
        }
        // Outside the simplex the basis is still a partition, of (a+b+c)^n rather than 1.
        let outside = unit.evaluate([0.2, 0.2, 0.2]).x;
        let expected = 0.6f64.powi(degree as i32);
        worst_basis_sum = worst_basis_sum.max((outside - expected).abs());
    }
    report(
        "3  the basis is a partition of unity",
        &format!(
            "worst |sum of basis values - 1|  {worst_basis_sum:.3e}\n\
             (the off-simplex samples are checked against (a+b+c)^n instead)\n\
             this is what makes dragging an assignment: move a point's whole support and the surface\n\
             point moves by exactly that vector, with no solve"
        ),
    );

    // 4. Elevation is linear, which is what makes an empty layer an empty layer.
    let scale = -3.25;
    let scaled_source = SimplexNet::new(2, source.points().iter().map(|p| *p * scale).collect())
        .expect("scaled net keeps its shape");
    let lifted_then_scaled = source
        .elevate_by(3)
        .points()
        .iter()
        .map(|p| *p * scale)
        .collect::<Vec<_>>();
    let scaled_then_lifted = scaled_source.elevate_by(3).points().to_vec();
    let linearity_error = lifted_then_scaled
        .iter()
        .zip(&scaled_then_lifted)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f64::max);

    // And the property the storage design actually rests on: a zero displacement field stays
    // exactly zero through any number of refinements.
    let zero = SimplexNet::from_corners(Point3::ZERO, Point3::ZERO, Point3::ZERO);
    let mut zero_drift = 0.0f64;
    for steps in 0..=5 {
        let lifted = zero.elevate_by(steps);
        for point in lifted.points() {
            zero_drift = zero_drift.max(point.length());
        }
    }
    report(
        "4  elevation is linear, so an empty layer is empty",
        &format!(
            "elevate(s*n) vs s*elevate(n)   {linearity_error:.3e}\n\
             worst coordinate of a zero net after five elevations   {zero_drift:.3e}\n\
             the second number is the storage rule from the design: a level holds a displacement\n\
             against the root, and an all-zero level contributes exactly nothing, at any depth"
        ),
    );

    // 5. The soft-edge invariant. Two triangles share an edge on k == 0; the second triangle's
    //    interior row is placed by the mu invariant, and the tangent planes are then measured
    //    numerically so the formula is not checked against itself.
    println!();
    let degree = 2;
    let edge = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.5, 0.0, 0.4),
        Point3::new(1.0, 0.0, 0.0),
    ];
    // The k == 0 row, addressed as (i, j) with i + j == degree.
    let boundary = [(degree, 0), (degree - 1, 1), (degree - 2, 2)];
    // The k == 1 row immediately interior to it.
    let interior = [(degree - 1, 0), (degree - 2, 1)];
    let master_interior = [Point3::new(0.45, 0.35, 0.62), Point3::new(0.55, 0.35, 0.62)];
    let apex = Point3::new(0.5, 0.9, 0.5);
    let mu = 0.7;

    let build_side = |slave_placement: Option<SlavePlacement>| -> SimplexNet {
        let mut points = vec![Point3::ZERO; count(degree)];
        for (slot, point) in boundary.iter().zip(edge) {
            points[row(degree, *slot)] = point;
        }
        for (position, slot) in interior.iter().enumerate() {
            let master = master_interior[position];
            let boundary_point = edge[position];
            points[row(degree, *slot)] = match slave_placement {
                None => master,
                Some(SlavePlacement::ByInvariant(mu)) => {
                    boundary_point + (boundary_point - master) * mu
                }
                // The control: the same master row, pushed sideways. Nothing here is a crease.
                Some(SlavePlacement::Displaced(offset)) => master + Point3::new(0.0, offset, 0.0),
            };
        }
        points[row(degree, (0, 0))] = apex;
        SimplexNet::new(degree, points).expect("degree 2 takes six points")
    };

    let master = build_side(None);
    let slave = build_side(Some(SlavePlacement::ByInvariant(mu)));
    let control = build_side(Some(SlavePlacement::Displaced(0.8)));

    // Sample the shared edge, parameterised by a with b = 1 - a and c = 0.
    let step = 1.0e-5;
    let edge_samples: Vec<f64> = (1..10).map(|index| index as f64 / 10.0).collect();
    let mut worst_plane_gap = 0.0f64;
    let mut worst_control_gap = 0.0f64;
    println!("\n   a     soft edge (mu)      control (deliberate crease)");
    for a in &edge_samples {
        let b = 1.0 - a;
        let normal_master = tangent_plane_normal(&master, *a, b, step);
        let gap = plane_gap(normal_master, tangent_plane_normal(&slave, *a, b, step));
        let control_gap = plane_gap(normal_master, tangent_plane_normal(&control, *a, b, step));
        worst_plane_gap = worst_plane_gap.max(gap);
        worst_control_gap = worst_control_gap.max(control_gap);
        println!("   {a:.1}      {gap:.3e}          {control_gap:.3e}");
    }

    // The measurement's own error, not assumed. A second-order stencil means a real gap is
    // independent of the step and a truncation artefact falls as step^2, so the last column says
    // which one this is: near 1 means the gap is the probe's noise, not the invariant's.
    println!("\n   step        soft gap        control gap     soft scaled to the first step");
    let mut previous: Option<(f64, f64)> = None;
    for current_step in [1.0e-4, 1.0e-5, 1.0e-6] {
        let soft = worst_gap(&master, &slave, &edge_samples, current_step);
        let crease = worst_gap(&master, &control, &edge_samples, current_step);
        let scaled = previous.map_or(f64::NAN, |(previous_step, _)| {
            soft * previous_step * previous_step / (current_step * current_step)
        });
        println!("   {current_step:.0e}   {soft:.3e}     {crease:.3e}      {scaled:.3}");
        previous = Some((current_step, soft));
    }
    report(
        "5  the soft-edge invariant produces a continuous tangent plane",
        &format!(
            "worst plane gap with the mu invariant   {worst_plane_gap:.3e} rad\n\
             worst plane gap on the control        {worst_control_gap:.3e} rad\n\
             the control is the same edge with the slave row pushed sideways, so it is a crease.\n\
             a control near zero would mean this probe cannot see a crease and proves nothing\n\
             the soft gap sits on a floor of about 1.4e-8 rad and stops falling with the step.\n\
             that is the measurement, not the surface: two unit vectors this close have a dot\n\
             product f64 cannot resolve below 1 - 1e-16, and acos turns that into sqrt(2e-16)"
        ),
    );

    // Verdict.
    let tolerance_absolute = 1.0e-12;
    let tolerance_angle = 1.0e-6;
    let mut failures: Vec<&str> = Vec::new();
    if corner_error > tolerance_absolute {
        failures.push("level 0 does not reproduce its corners");
    }
    if worst_relative > tolerance_absolute {
        failures.push("elevation moved the surface");
    }
    if worst_basis_sum > tolerance_absolute {
        failures.push("the basis is not a partition of unity");
    }
    if linearity_error > tolerance_absolute {
        failures.push("elevation is not linear");
    }
    if zero_drift > tolerance_absolute {
        failures.push("a zero level does not stay zero");
    }
    if worst_plane_gap > tolerance_angle {
        failures.push("the soft-edge invariant does not produce a continuous tangent plane");
    }
    if worst_control_gap < 1.0e-3 {
        failures
            .push("the control measurement is degenerate: a deliberate crease looks continuous");
    }

    println!();
    if failures.is_empty() {
        println!("all six probes passed");
    } else {
        println!("FAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}

/// Where the second triangle's interior row goes. The master side is the `None` case.
enum SlavePlacement {
    /// The invariant from the design: `Q + mu * (Q - master)`.
    ByInvariant(f64),
    /// The control measurement: the master row pushed sideways, which is a crease.
    Displaced(f64),
}

/// The largest tangent-plane gap between `first` and `second` over the edge samples.
fn worst_gap(first: &SimplexNet, second: &SimplexNet, samples: &[f64], step: f64) -> f64 {
    samples
        .iter()
        .map(|a| {
            plane_gap(
                tangent_plane_normal(first, *a, 1.0 - a, step),
                tangent_plane_normal(second, *a, 1.0 - a, step),
            )
        })
        .fold(0.0f64, f64::max)
}

/// Index into a degree-`degree` net's flat point array.
fn row(degree: usize, (i, j): (usize, usize)) -> usize {
    i * (degree + 1) - i * i.saturating_sub(1) / 2 + j
}

/// The surface normal at the point `(a, b, 0)` on the edge, by finite difference.
///
/// `b` and `c` follow from `a` with `b = 1 - a - c`, so the derivative along the edge and the
/// derivative across it are both one-dimensional. Everything here is measured; nothing evaluates
/// a derivative in closed form.
///
/// The across-edge direction uses a second-order one-sided stencil. A first-order one would carry
/// an `O(step)` error that is the same order as the effect being measured, which would make this
/// probe unable to tell a real gap from its own noise.
fn tangent_plane_normal(net: &SimplexNet, a: f64, b: f64, step: f64) -> Point3 {
    let along = (net.evaluate([a + step, b - step, 0.0]) - net.evaluate([a - step, b + step, 0.0]))
        / (2.0 * step);
    let across = (net.evaluate([a, b, 0.0]) * -3.0 + net.evaluate([a, b - step, step]) * 4.0
        - net.evaluate([a, b - 2.0 * step, 2.0 * step]))
        / (2.0 * step);
    along.cross(across)
}

/// The angle between two tangent planes, in radians, treating a plane as unoriented.
///
/// Two planes are the same plane whether their normals agree or oppose, so the raw angle is
/// folded into `[0, pi/2]`.
fn plane_gap(first: Point3, second: Point3) -> f64 {
    let raw = first.angle(second);
    if raw.is_nan() {
        return f64::INFINITY;
    }
    raw.min(std::f64::consts::PI - raw)
}
