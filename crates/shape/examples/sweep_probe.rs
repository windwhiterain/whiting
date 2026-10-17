//! Measures the sweep's invariants, each with a control that can fail, and exits non-zero if one does.
//!
//! Nothing here is a test file: this is the probe that says whether `shape::sweep` does what its own
//! documentation claims. The rows are `.agents/proposals/shape-primitives.md` D4 (a spine plus a section
//! law, with `(u, v)` owned by the loft), D5 (`signed_volume`, the predicate that turns an inside-out
//! solid into a named reading) and the sweep handoff's exit criterion: a tube, a blade and an envelope
//! built from one call, the vertices that disagree about `u` being zero, and a closed sweep's volume
//! against the frustum it is.
//!
//! ```text
//! cargo run -p whiting-shape --example sweep_probe
//! ```

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use whiting_shape::curve::{Curve, Jet};
use whiting_shape::math::{Vec2, cross3, dot3, length3, normalized3, subtract};
use whiting_shape::mesh::{
    boundary_loops, euler_characteristic, is_manifold, same_direction_edges, signed_volume,
    winding_is_consistent,
};
use whiting_shape::scalar::Scalar;
use whiting_shape::section::Section;
use whiting_shape::sweep::Sweep;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("frustum volume", a_taper_has_the_frustum_volume()));
    checks.push((
        "solid or tube",
        a_capped_sweep_is_a_solid_and_an_uncapped_one_is_a_tube(),
    ));
    checks.push(("u and v", a_vertex_knows_its_station_and_its_slot()));
    checks.push(("ring plane", a_ring_lies_in_its_stations_plane()));
    checks.push(("twist", twist_rotates_the_section_in_its_plane()));
    checks.push((
        "section frame",
        the_section_follows_the_spines_stated_frame(),
    ));
    checks.push(("blade tip", a_blade_is_a_closed_solid_with_a_pointed_tip()));
    checks.push(("envelope", refining_an_envelope_is_a_section_change()));

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

/// A straight spine of unit length along `+y`: the one shape whose swept volume has a closed form.
fn straight(length: f32) -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.0, length, 0.0]),
        Jet::new([0.0, length, 0.0], [0.0, length, 0.0]),
    ])
    .expect("the straight spine's knots are distinct")
}

/// A spine that bends once: the control for every row that must not assume a straight axis.
fn bent() -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.08, 1.2, 0.0]),
        Jet::new([0.08, 1.2, 0.0], [0.14, 1.2, 0.03]),
        Jet::new([0.28, 2.4, 0.08], [0.12, 0.9, 0.05]),
    ])
    .expect("the bent spine's knots are distinct")
}

/// A quarter arc of radius 1: a leaf's midrib, which starts level and ends vertical.
fn midrib() -> Curve {
    let steps = 4;
    let step = FRAC_PI_2 / steps as f32;
    let jets = (0..=steps)
        .map(|index| {
            let angle = -FRAC_PI_2 + index as f32 * step;
            let (sin, cos) = angle.sin_cos();
            Jet::new([cos, 1.0 + sin, 0.0], [-sin * step, cos * step, 0.0])
        })
        .collect();
    Curve::new(jets).expect("the midrib's knots are distinct")
}

/// A unit circle with a linear taper from 1 to 0.5.
fn tapered_tube() -> Sweep {
    let mut sweep = Sweep::new(straight(1.0), Section::circle());
    sweep.scale = Scalar::new(&[(0.0, 1.0), (1.0, 0.5)]).expect("two knots are a law");
    sweep
}

/// A crown's reach: one radius curve in the trunk's parameter space.
fn crown_reach() -> Scalar {
    Scalar::new(&[(0.0, 0.05), (0.45, 0.55), (1.0, 0.9)]).expect("three knots are a law")
}

/// A three-lobed loop, counter-clockwise: the envelope's refinement.
fn lobed() -> Vec<Vec2> {
    (0..24)
        .map(|index| {
            let angle = index as f32 / 24.0 * TAU;
            let radius = 1.0 + 0.25 * (3.0 * angle).cos();
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect()
}

/// A closed sweep is a frustum of an `n`-gonal pyramid, and its volume is positive.
fn a_taper_has_the_frustum_volume() -> bool {
    let sides = 32usize;
    let loft = tapered_tube().sample(2, sides);
    let measured = signed_volume(loft.view());
    let step = TAU / sides as f32;
    let start_area = 0.5 * sides as f32 * step.sin();
    let end_area = 0.5 * sides as f32 * 0.25 * step.sin();
    let expected = (1.0 / 3.0) * (start_area + (start_area * end_area).sqrt() + end_area);
    let round = PI * (1.0 + 0.5 + 0.25) / 3.0;
    let error = (measured - expected).abs() / expected;
    println!("\n# a taper, graded against the frustum it is");
    println!(
        "  signed volume {measured:.6} · the {sides}-gon frustum {expected:.6} · relative error {error:.3e}"
    );
    println!(
        "  the round frustum is {round:.6}, {:.3e} higher: that difference is the {sides}-gon and not the walk",
        (round - expected) / expected
    );
    println!("  a positive reading is the winding's contract: counter-clockwise seen from outside");
    measured > 0.0 && error < 1e-5
}

/// Capping the ends is the whole difference between a solid and a tube.
fn a_capped_sweep_is_a_solid_and_an_uncapped_one_is_a_tube() -> bool {
    let mut sweep = Sweep::new(straight(1.0), Section::circle());
    sweep.scale =
        Scalar::new(&[(0.0, 1.0), (0.35, 0.8), (1.0, 0.4)]).expect("three knots are a law");
    let solid = sweep.sample(4, 24);
    let mut open = Sweep::new(straight(1.0), Section::circle());
    open.scale =
        Scalar::new(&[(0.0, 1.0), (0.35, 0.8), (1.0, 0.4)]).expect("three knots are a law");
    open.capped = false;
    let tube = open.sample(4, 24);
    let solid_ok = boundary_loops(solid.view()) == 0
        && is_manifold(solid.view())
        && winding_is_consistent(solid.view())
        && same_direction_edges(solid.view()) == 0
        && euler_characteristic(solid.view()) == 2;
    let tube_ok = boundary_loops(tube.view()) == 2
        && euler_characteristic(tube.view()) == 0
        && is_manifold(tube.view());
    println!("\n# a solid and a tube");
    println!(
        "  capped: {} boundary loop(s) · {} same-direction edge(s) · χ = {}",
        boundary_loops(solid.view()),
        same_direction_edges(solid.view()),
        euler_characteristic(solid.view())
    );
    println!(
        "  uncapped: {} boundary loop(s) · χ = {} · the same rings, and nothing else",
        boundary_loops(tube.view()),
        euler_characteristic(tube.view())
    );
    println!("  the fan reuses the end rings' vertices, so both lofts hold the same vertex count");
    solid_ok && tube_ok && solid.positions().len() == tube.positions().len()
}

/// A vertex's `(u, v)` is its station and its slot, exactly.
fn a_vertex_knows_its_station_and_its_slot() -> bool {
    let sweep = tapered_tube();
    let rings = 5;
    let around = 12;
    let loft = sweep.sample(rings, around);
    let stations = sweep.spine.stations(rings);
    let mut wrong_u = 0;
    let mut wrong_v = 0;
    for ring in 0..loft.rings() {
        for slot in 0..loft.around() {
            let (_, uv) = loft.vertex(ring, slot);
            if uv[0] != stations[ring].t {
                wrong_u += 1;
            }
            if uv[1] != slot as f32 / around as f32 {
                wrong_v += 1;
            }
        }
    }
    println!("\n# every vertex's parameter");
    println!(
        "  {wrong_u} of {} vertices disagree about u, {wrong_v} about v",
        rings * around
    );
    println!(
        "  u is the ring's station and v is slot / around, read from the same array as the position"
    );
    wrong_u == 0 && wrong_v == 0
}

/// A ring lies in its station's plane, is centred on the station, and has the scale's radius.
fn a_ring_lies_in_its_stations_plane() -> bool {
    let mut sweep = Sweep::new(bent(), Section::circle());
    sweep.scale = crown_reach();
    let rings = 7;
    let around = 24;
    let loft = sweep.sample(rings, around);
    let stations = sweep.spine.stations(rings);
    let mut worst_along = 0.0f32;
    let mut worst_radius = 0.0f32;
    let mut worst_normal = 1.0f32;
    for (ring, station) in stations.iter().enumerate() {
        let mut centroid = [0.0f32; 3];
        for slot in 0..around {
            let (point, _) = loft.vertex(ring, slot);
            for axis in 0..3 {
                centroid[axis] += point[axis] / around as f32;
            }
        }
        worst_along = worst_along
            .max(dot3(subtract(centroid, station.position), station.frame.tangent).abs());
        let scale = sweep.scale.at(station.t);
        for slot in 0..around {
            let (point, _) = loft.vertex(ring, slot);
            worst_radius = worst_radius.max((length3(subtract(point, centroid)) - scale).abs());
        }
        let (a, _) = loft.vertex(ring, 0);
        let (b, _) = loft.vertex(ring, 1);
        let (c, _) = loft.vertex(ring, 2);
        let face = normalized3(cross3(subtract(b, a), subtract(c, a)));
        worst_normal = worst_normal.min(dot3(face, station.frame.tangent).abs());
    }
    println!("\n# each ring in its station's plane");
    println!("  worst centroid offset along the tangent {worst_along:.3e}");
    println!("  worst radius error against the scale law {worst_radius:.3e}");
    println!("  the ring's own plane normal against the tangent: worst |cos| {worst_normal:.6}");
    worst_along < 1e-5 && worst_radius < 1e-5 && worst_normal > 0.9999
}

/// `twist` turns the section in its own plane and nowhere else.
fn twist_rotates_the_section_in_its_plane() -> bool {
    let station = straight(1.0).stations(2)[0];
    let flat = Sweep::new(straight(1.0), Section::flat(1.0, 0.01));
    let rolled = {
        let mut sweep = Sweep::new(straight(1.0), Section::flat(1.0, 0.01));
        sweep.twist = Scalar::constant(FRAC_PI_2);
        sweep
    };
    let plain = flat.sample(2, 16);
    let turned = rolled.sample(2, 16);
    let (plain_point, _) = plain.vertex(0, 0);
    let (turned_point, _) = turned.vertex(0, 0);
    let plain_along = normalized3(subtract(plain_point, station.position));
    let turned_along = normalized3(subtract(turned_point, station.position));
    println!("\n# a blade rolled a quarter turn");
    println!(
        "  slot 0 lies along the frame's normal by {:.6}, and along its binormal by {:.6} after π/2",
        dot3(plain_along, station.frame.normal),
        dot3(turned_along, station.frame.binormal)
    );
    println!("  a positive twist turns from x toward y, which is the same sense v increases in");
    dot3(plain_along, station.frame.normal) > 0.999
        && dot3(turned_along, station.frame.binormal) > 0.999
}

/// Which way a section's `x` points is the spine's statement, not the crate's.
fn the_section_follows_the_spines_stated_frame() -> bool {
    let mut east = straight(1.0);
    east.set_initial_normal([1.0, 0.0, 0.0]);
    let mut north = straight(1.0);
    north.set_initial_normal([0.0, 0.0, 1.0]);
    let east_sweep = Sweep::new(east, Section::circle());
    let north_sweep = Sweep::new(north, Section::circle());
    let east_loft = east_sweep.sample(2, 16);
    let north_loft = north_sweep.sample(2, 16);
    let east_station = east_sweep.spine.stations(2)[0];
    let north_station = north_sweep.spine.stations(2)[0];
    let east_along = normalized3(subtract(east_loft.vertex(0, 0).0, east_station.position));
    let north_along = normalized3(subtract(north_loft.vertex(0, 0).0, north_station.position));
    let east_error = 1.0 - dot3(east_along, east_station.frame.normal);
    let north_error = 1.0 - dot3(north_along, north_station.frame.normal);
    let apart = dot3(east_station.frame.normal, north_station.frame.normal).abs();
    println!("\n# the section's x is the spine's frame normal");
    println!(
        "  slot 0 against its own frame normal: east {east_error:.3e} · north {north_error:.3e}"
    );
    println!(
        "  the two frames' normals disagree by |cos| {apart:.3e}, so the check can tell them apart"
    );
    east_error < 1e-6 && north_error < 1e-6 && apart < 1e-3
}

/// A blade is a closed solid whose tip collapses to a point without a special case.
fn a_blade_is_a_closed_solid_with_a_pointed_tip() -> bool {
    let mut sweep = Sweep::new(midrib(), Section::flat(1.0, 0.01));
    sweep.scale =
        Scalar::new(&[(0.0, 0.0), (0.45, 1.0), (1.0, 0.0)]).expect("three knots are a law");
    let rings = 5;
    let around = 16;
    let loft = sweep.sample(rings, around);
    let volume = signed_volume(loft.view());
    let closed = boundary_loops(loft.view()) == 0
        && is_manifold(loft.view())
        && winding_is_consistent(loft.view())
        && euler_characteristic(loft.view()) == 2;
    let (first, _) = loft.vertex(rings - 1, 0);
    let mut tip = 0.0f32;
    for slot in 0..around {
        let (point, _) = loft.vertex(rings - 1, slot);
        tip = tip.max(length3(subtract(point, first)));
    }
    println!("\n# a blade: a midrib and a lens, tapered to a point at both ends");
    println!(
        "  {closed} closed at χ = {} · signed volume {volume:.6} · tip spread {tip:.3e}",
        euler_characteristic(loft.view())
    );
    println!(
        "  {} vertices for {rings} × {around} rings: the caps reuse the end rings, so the grid is the loft",
        loft.positions().len()
    );
    println!("  a zero scale collapses the ring, the fan stays, and the surface stays closed");
    closed && volume > 0.0 && tip < 1e-6 && loft.positions().len() == rings * around
}

/// Refining an envelope replaces the section and changes nothing else.
fn refining_an_envelope_is_a_section_change() -> bool {
    let rings = 6;
    let around = 32;
    let mut coarse = Sweep::new(bent(), Section::circle());
    coarse.scale = crown_reach();
    let mut refined = Sweep::new(
        bent(),
        Section::outline(&lobed()).expect("the lobes are a loop"),
    );
    refined.scale = crown_reach();
    let coarse = coarse.sample(rings, around);
    let refined = refined.sample(rings, around);
    let coarse_volume = signed_volume(coarse.view());
    let refined_volume = signed_volume(refined.view());
    let both_closed = [&coarse, &refined]
        .iter()
        .all(|loft| boundary_loops(loft.view()) == 0 && euler_characteristic(loft.view()) == 2);
    let change = (refined_volume - coarse_volume).abs() / coarse_volume;
    println!("\n# an envelope, coarse and refined");
    println!("  a circle of reach r(t): volume {coarse_volume:.6}");
    println!(
        "  a three-lobed outline of the same reach: volume {refined_volume:.6}, {change:.3e} apart"
    );
    println!(
        "  both built by one call with the same ring count and the same vertex count: {} and {}",
        coarse.positions().len(),
        refined.positions().len()
    );
    both_closed
        && coarse.positions().len() == refined.positions().len()
        && coarse.positions().len() == rings * around
        && change > 1e-3
}
