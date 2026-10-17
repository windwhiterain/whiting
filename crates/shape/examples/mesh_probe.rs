//! Measures the mesh predicates, each with a control that can fail, and exits non-zero if one does.
//!
//! Nothing here is a test file: this is the probe that says whether `shape::mesh` does what its own
//! documentation claims, and every reading is printed beside the number it is compared against.
//! The predicates are `.agents/proposals/shape-primitives.md` D5, and the geometry is built here
//! rather than loaded, so that every triangle's winding is visible in the source rather than in a
//! file this probe cannot read.
//!
//! ```text
//! cargo run -p whiting-shape --example mesh_probe
//! ```

use whiting_shape::math::Vec3;
use whiting_shape::mesh::{
    Triangles, boundary_loops, euler_characteristic, is_manifold, same_direction_edges,
    signed_volume, winding_is_consistent,
};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("closed cube", the_closed_cube()));
    checks.push((
        "reversed winding",
        a_reversed_winding_is_a_fault_of_orientation(),
    ));
    checks.push(("open cube", the_cube_with_one_triangle_missing()));
    checks.push(("open prism", the_open_prism()));
    checks.push(("closed prism", the_closed_prism()));
    checks.push(("three-way edge", an_edge_three_triangles_share()));
    checks.push(("repeated corner", a_repeated_corner_index()));
    checks.push((
        "unused position",
        an_unreferenced_position_is_not_a_vertex(),
    ));

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

/// The eight corners of a unit cube: `x` east, `y` up, `z` south, the bottom ring first.
const CUBE_POSITIONS: [Vec3; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0],
];

/// The twelve triangles of that cube, two per face and each wound counter-clockwise seen from
/// outside — which is the winding the volume row is measured against.
const CUBE_CORNERS: [[u32; 3]; 12] = [
    [0, 1, 3],
    [1, 2, 3],
    [4, 7, 6],
    [4, 6, 5],
    [0, 4, 5],
    [0, 5, 1],
    [2, 6, 7],
    [2, 7, 3],
    [0, 3, 7],
    [0, 7, 4],
    [1, 5, 6],
    [1, 6, 2],
];

/// A closed genus-zero shell: every reading it should give, against the number it should read.
fn the_closed_cube() -> bool {
    let surface = Triangles {
        positions: &CUBE_POSITIONS,
        corners: &CUBE_CORNERS,
    };
    let volume = signed_volume(surface);
    let characteristic = euler_characteristic(surface);
    let loops = boundary_loops(surface);
    let manifold = is_manifold(surface);
    let consistent = winding_is_consistent(surface);
    let defects = same_direction_edges(surface);

    println!("\n# a unit cube, 8 corners and 12 outward-wound triangles");
    println!("  signed_volume {volume:.6} against 1 · threshold 1e-6");
    println!("  euler_characteristic {characteristic} against 2");
    println!("  boundary_loops {loops} against 0 · is_manifold {manifold} against true");
    println!(
        "  winding_is_consistent {consistent} against true · same_direction_edges {defects} against 0"
    );
    println!(
        "  every one of the 18 edges is shared by two triangles wound the two ways, which is what makes the \
         signed volume a volume and not a number"
    );

    (volume - 1.0).abs() < 1.0e-6
        && characteristic == 2
        && loops == 0
        && manifold
        && consistent
        && defects == 0
}

/// The control: the same cube with one triangle wound the other way. Only the orientation readings
/// may move, and the row fails if a topological one does.
fn a_reversed_winding_is_a_fault_of_orientation() -> bool {
    let mut corners = CUBE_CORNERS;
    let upright = corners[3];
    corners[3] = [upright[0], upright[2], upright[1]];
    let surface = Triangles {
        positions: &CUBE_POSITIONS,
        corners: &corners,
    };
    let volume = signed_volume(surface);
    let loops = boundary_loops(surface);
    let manifold = is_manifold(surface);
    let consistent = winding_is_consistent(surface);
    let defects = same_direction_edges(surface);
    let moved = (volume - 1.0).abs();

    println!(
        "\n# the control: that cube with its fourth triangle ({},{},{}) → ({},{},{}) wound the other way",
        upright[0], upright[1], upright[2], corners[3][0], corners[3][1], corners[3][2]
    );
    println!(
        "  winding_is_consistent {consistent} against false · same_direction_edges {defects} against 1 or more"
    );
    println!(
        "  signed_volume {volume:.6} against 1 · moved by {moved:.6}, over the 1e-3 the row asks for"
    );
    println!(
        "  is_manifold {manifold} and boundary_loops {loops} — the defect is orientation, so a topology keyed on \
         it would report a hole that is not there"
    );

    !consistent && defects >= 1 && moved > 1.0e-3 && manifold && loops == 0
}

/// A shell with a hole in it: the freed edges meet one another, so the hole is one loop.
fn the_cube_with_one_triangle_missing() -> bool {
    let mut corners = CUBE_CORNERS.to_vec();
    corners.remove(0);
    let surface = Triangles {
        positions: &CUBE_POSITIONS,
        corners: &corners,
    };
    let characteristic = euler_characteristic(surface);
    let loops = boundary_loops(surface);
    let manifold = is_manifold(surface);

    println!("\n# the cube's shell with one triangle taken off");
    println!(
        "  boundary_loops {loops} against 1 · euler_characteristic {characteristic} against 1"
    );
    println!(
        "  the three edges the triangle held are now each used once, and they meet at two of the cube's corners, \
         so the hole is one loop and not three"
    );
    println!("  is_manifold {manifold}, and that is right: an open shell is a manifold");

    loops == 1 && characteristic == 1
}

/// The twelve side quads of a prism, with neither end closed.
fn the_open_prism() -> bool {
    let sides = 12;
    let (positions, corners) = prism_shell(sides, 1.0, 1.0);
    let surface = Triangles {
        positions: &positions,
        corners: &corners,
    };
    let volume = signed_volume(surface);
    let characteristic = euler_characteristic(surface);
    let loops = boundary_loops(surface);
    let manifold = is_manifold(surface);

    println!("\n# a 12-gon prism of radius 1 and height 1, side quads only");
    println!(
        "  boundary_loops {loops} against 2 · euler_characteristic {characteristic} against 0"
    );
    println!(
        "  is_manifold {manifold} against true — the two end rings share no vertex, so they are two loops"
    );
    println!(
        "  signed_volume {volume:.6}, printed and not graded: the surface is open, so the number is the prism plus \
         whatever the two holes leave outside the origin rather than the prism"
    );

    loops == 2 && characteristic == 0 && manifold
}

/// The same prism with a fan closing each end, against the volume its polygon actually encloses.
fn the_closed_prism() -> bool {
    let sides = 12;
    let (radius, height) = (1.0f64, 1.0f64);
    let (positions, corners) = closed_prism(sides, radius as f32, height as f32);
    let surface = Triangles {
        positions: &positions,
        corners: &corners,
    };
    let volume = signed_volume(surface);
    let exact = sides as f64 / 2.0
        * radius
        * radius
        * (std::f64::consts::TAU / sides as f64).sin()
        * height;
    let difference = (volume as f64 - exact).abs();
    let relative = (exact - std::f64::consts::PI * radius * radius * height).abs()
        / (std::f64::consts::PI * radius * radius * height);
    let loops = boundary_loops(surface);
    let characteristic = euler_characteristic(surface);

    println!("\n# the same prism with a fan closing each end");
    println!(
        "  boundary_loops {loops} against 0 · euler_characteristic {characteristic} against 2"
    );
    println!(
        "  signed_volume {volume:.6} against the exact prism {exact:.6} = (n/2)·r²·sin(2π/n)·h · threshold 1e-5 · \
         difference {difference:.3e}"
    );
    println!(
        "  the control: that prism is {:.2}% short of the disc of area π·r²·h it stands in for, because a polygon's \
         area falls short of a disc's rather than approximating it, and the deficit shrinks like 1/n²",
        relative * 100.0
    );

    loops == 0 && characteristic == 2 && difference < 1.0e-5 && relative > 0.0 && relative < 0.1
}

/// One edge carrying three triangles: the count `is_manifold` exists to catch.
fn an_edge_three_triangles_share() -> bool {
    let positions: [Vec3; 5] = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
    ];
    let corners: [[u32; 3]; 3] = [[0, 1, 2], [0, 3, 1], [0, 1, 4]];
    let surface = Triangles {
        positions: &positions,
        corners: &corners,
    };
    let manifold = is_manifold(surface);
    let defects = same_direction_edges(surface);

    println!("\n# three triangles across the edge 0–1");
    println!(
        "  is_manifold {manifold} against false — that edge carries three triangles and no edge may carry three"
    );
    println!(
        "  same_direction_edges {defects}, which is a second fault and not the one this row is about: two of the \
         three wind 0→1 and the third 1→0"
    );

    !manifold
}

/// A triangle naming the same vertex twice: an edge whose two ends are one index.
fn a_repeated_corner_index() -> bool {
    let positions: [Vec3; 2] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let corners: [[u32; 3]; 1] = [[0, 0, 1]];
    let surface = Triangles {
        positions: &positions,
        corners: &corners,
    };
    let manifold = is_manifold(surface);
    let volume = signed_volume(surface);

    println!("\n# one triangle naming the same corner index twice");
    println!(
        "  is_manifold {manifold} against false — the edge 0–0 has no second end to be shared with"
    );
    println!("  signed_volume {volume:.6}, which is the zero a degenerate triangle contributes");

    !manifold
}

/// The control that `V` counts the indices a triangle names: a position nobody names is not a vertex.
fn an_unreferenced_position_is_not_a_vertex() -> bool {
    let mut positions = CUBE_POSITIONS.to_vec();
    positions.push([0.5, 0.5, 0.5]);
    let surface = Triangles {
        positions: &positions,
        corners: &CUBE_CORNERS,
    };
    let characteristic = euler_characteristic(surface);

    println!("\n# the closed cube with one more position, which no triangle names");
    println!(
        "  euler_characteristic {characteristic} against 2, with positions {} long against the cube's 8 — V counts \
         the indices a corner refers to, so the answer is a property of the surface and not of its buffer",
        positions.len()
    );

    characteristic == 2
}

/// A regular prism's two rings and its side quads, wound outward and with neither end closed.
fn prism_shell(sides: u32, radius: f32, height: f32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let positions = (0..2 * sides)
        .map(|index| {
            let angle = std::f32::consts::TAU * (index % sides) as f32 / sides as f32;
            let (sin, cos) = angle.sin_cos();
            [radius * cos, height * (index / sides) as f32, radius * sin]
        })
        .collect();
    let mut corners = Vec::new();
    for side in 0..sides {
        let next = (side + 1) % sides;
        corners.push([side, sides + side, sides + next]);
        corners.push([side, sides + next, next]);
    }
    (positions, corners)
}

/// The same prism with a fan closing each end, the bottom one facing down and the top one up.
fn closed_prism(sides: u32, radius: f32, height: f32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let (mut positions, mut corners) = prism_shell(sides, radius, height);
    let bottom = positions.len() as u32;
    positions.push([0.0, 0.0, 0.0]);
    let top = positions.len() as u32;
    positions.push([0.0, height, 0.0]);
    for side in 0..sides {
        let next = (side + 1) % sides;
        corners.push([bottom, side, next]);
        corners.push([top, sides + next, sides + side]);
    }
    (positions, corners)
}
