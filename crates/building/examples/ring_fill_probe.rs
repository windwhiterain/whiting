//! The ring fill's acceptance: the triangles must cover **exactly the annulus**, printed one by one.
//!
//! # The order of verification, which the last two attempts earned
//!
//! A correct *number* of triangles has already lied once — the greedy walk produced the right count and 140 m² where
//! the annulus was 84 — so this probe prints every triangle's vertices and area **before** it trusts any total. A
//! triangle whose centroid lies outside the annulus is wrong no matter what the sum says, and the centroid test is
//! the local check the sum cannot make.
//!
//! Then the arithmetic: the areas must sum to the outer area minus the inner area.
//!
//! Run with `cargo run -p whiting-building --example ring_fill_probe`.

use whiting_building::fill::ring_fill;
use whiting_building::math::Vec2;

/// Returns a rectangle's outline, wound counter-clockwise.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<Vec2> {
    vec![[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
}

/// Returns a polygon's signed area, positive when counter-clockwise.
fn area(polygon: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..polygon.len() {
        let here = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        twice += here[0] * next[1] - next[0] * here[1];
    }
    twice * 0.5
}

/// Returns a triangle's unsigned area.
fn triangle_area(triangle: [Vec2; 3]) -> f32 {
    let one = [
        triangle[1][0] - triangle[0][0],
        triangle[1][1] - triangle[0][1],
    ];
    let two = [
        triangle[2][0] - triangle[0][0],
        triangle[2][1] - triangle[0][1],
    ];
    (one[0] * two[1] - one[1] * two[0]).abs() * 0.5
}

/// Returns whether a point is inside a simple polygon, by the ray-cast rule.
fn inside(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut held = false;
    for index in 0..polygon.len() {
        let here = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        if (here[1] > point[1]) != (next[1] > point[1]) {
            let along = (next[0] - here[0]) * (point[1] - here[1]) / (next[1] - here[1]) + here[0];
            if point[0] < along {
                held = !held;
            }
        }
    }
    held
}

/// Reports one case: every triangle, then the sum against the annulus.
fn measure(label: &str, outer: &[Vec2], inner: &[Vec2], show: bool) {
    let filled = ring_fill(outer, inner);
    let wanted = (area(outer) - area(inner)).abs();
    let mut covered = 0.0;
    let mut stray = 0;
    for triangle in &filled {
        covered += triangle_area(*triangle);
        let centroid = [
            (triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3.0,
            (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.0,
        ];
        // **The local check**: a triangle's middle must lie in the annulus, which is inside the outline and outside
        // the hole.
        if !inside(centroid, outer) || inside(centroid, inner) {
            stray += 1;
        }
        if show {
            println!(
                "    ({:.1}, {:.1}) ({:.1}, {:.1}) ({:.1}, {:.1})  area {:.2}",
                triangle[0][0],
                triangle[0][1],
                triangle[1][0],
                triangle[1][1],
                triangle[2][0],
                triangle[2][1],
                triangle_area(*triangle)
            );
        }
    }
    println!(
        "  {label}: {} triangle(s), filled {:.4} of {:.4} (difference {:.6}), {stray} stray",
        filled.len(),
        covered,
        wanted,
        (covered - wanted).abs()
    );
}

fn main() {
    println!("# a 10 x 10 outline with a 4 x 4 void, every triangle printed");
    let outer = rectangle(-5.0, -5.0, 5.0, 5.0);
    let centred = rectangle(-2.0, -2.0, 2.0, 2.0);
    measure("centred", &outer, &centred, true);
    measure("off-centre", &outer, &rectangle(-4.0, -1.0, 0.0, 1.0), false);
    measure(
        "triangular void",
        &outer,
        &vec![[-2.0, -2.0], [2.0, -2.0], [0.0, 2.0]],
        false,
    );
    // A void that is not convex is refused rather than guessed.
    measure(
        "a notched void (refused)",
        &outer,
        &vec![[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [0.0, 0.0], [-2.0, 2.0]],
        false,
    );
}
