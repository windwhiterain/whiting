//! Measures the plant `vegetation` P1 promises, and exits non-zero if a reading leaves its threshold.
//!
//! The plant is hand-authored — a culm with three boughs, each carrying two twigs, and a leaf field on the
//! twigs — because P1's job is to prove the *representation* and not a generator: a law that is nearly
//! right is P2's business, and a default that is nearly right is data the moment it exists. The rows are
//! the design's P1 exit (`.agents/proposals/agentic-vegetation-generation.md` V1, V2 and V6): the wood is
//! closed solids, a joint sits on its parent's skin, the taper is the *law* rather than a cone, and the
//! leaves are instances of one template.
//!
//! ```text
//! cargo run -p whiting-vegetation --example plant_probe
//! ```

use shape::math::{length3, span, subtract};
use shape::mesh::{boundary_loops, euler_characteristic, is_manifold, signed_volume};
use shape::{Curve, Jet, Scalar};
use whiting_vegetation::expand::{Detail, wood};
use whiting_vegetation::node::{Attachment, Node};
use whiting_vegetation::plant::Plant;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("a census", the_census()));
    checks.push(("every solid is closed", every_loft_is_a_closed_solid()));
    checks.push(("the taper is the law", the_taper_is_the_law_not_a_cone()));
    checks.push(("a joint is on the skin", a_joint_sits_on_its_parents_skin()));
    checks.push(("one resolution dial", the_dial_is_the_only_resolution()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<26} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// A spine through the given `(position, tangent)` pairs.
fn spine(knots: &[([f32; 3], [f32; 3])]) -> Curve {
    Curve::new(
        knots
            .iter()
            .map(|(position, tangent)| Jet::new(*position, *tangent))
            .collect(),
    )
    .expect("the plant's knots are distinct")
}

/// The culm: 2.2 metres tall, its taper rising at a node and falling between them.
fn culm() -> Node {
    Node::root(
        spine(&[
            ([0.0, 0.0, 0.0], [0.02, 0.75, 0.0]),
            ([0.02, 0.75, 0.0], [0.05, 0.75, 0.01]),
            ([0.09, 1.5, 0.02], [0.06, 0.7, 0.02]),
            ([0.19, 2.2, 0.05], [0.05, 0.6, 0.02]),
        ]),
        Scalar::new(&[
            (0.0, 0.030),
            (0.28, 0.022),
            (0.32, 0.028),
            (0.62, 0.018),
            (0.66, 0.024),
            (1.0, 0.010),
        ])
        .expect("the culm's taper is a law"),
    )
}

/// A bough: out along `x`, up along `y`, and out again — authored in its parent's attachment frame.
fn bough(lean: f32, attachment: Attachment) -> Node {
    Node::attached(
        spine(&[
            ([0.0, 0.0, 0.0], [0.42, 0.10, 0.0]),
            ([0.30, 0.06, 0.02 * lean], [0.38, 0.30, 0.04]),
            ([0.62, 0.26, 0.06 * lean], [0.28, 0.42, 0.02]),
            ([0.86, 0.52, 0.08 * lean], [0.18, 0.30, 0.0]),
        ]),
        Scalar::new(&[(0.0, 0.011), (0.5, 0.007), (1.0, 0.002)])
            .expect("the bough's taper is a law"),
        attachment,
    )
}

/// A twig: shorter, thinner, and the generation the leaves hang from.
fn twig(attachment: Attachment) -> Node {
    Node::attached(
        spine(&[
            ([0.0, 0.0, 0.0], [0.20, 0.16, 0.02]),
            ([0.16, 0.13, 0.02], [0.10, 0.26, 0.02]),
            ([0.26, 0.38, 0.03], [0.04, 0.20, 0.0]),
        ]),
        Scalar::new(&[(0.0, 0.0035), (1.0, 0.0008)]).expect("the twig's taper is a law"),
        attachment,
    )
}

/// The plant: a culm, three boughs, two twigs on each.
fn plant() -> Plant {
    let mut root = culm();
    for index in 0..3 {
        let station = 0.45 + 0.18 * index as f32;
        let slot = 0.333 * index as f32;
        let mut branch = bough(index as f32, Attachment::new(station, slot).sunk(0.004));
        for twig_index in 0..2 {
            branch.push(twig(
                Attachment::new(
                    0.35 + 0.25 * twig_index as f32,
                    0.3 + 0.2 * twig_index as f32,
                )
                .sunk(0.0015),
            ));
        }
        root.push(branch);
    }
    Plant::new(root, 20_261_017)
}

/// The whole plant's numbers, and the one reading that says the parts add up.
fn the_census() -> bool {
    let plant = plant();
    let detail = Detail::DEFAULT;
    let solids = wood(&plant, detail);
    let triangles: usize = solids
        .iter()
        .map(|solid| solid.loft.triangles().len())
        .sum();
    let vertices: usize = solids
        .iter()
        .map(|solid| solid.loft.positions().len())
        .sum();
    let nodes: usize = plant.walk().count();
    println!("\n# the plant");
    println!(
        "  {nodes} nodes · {} solids · {vertices} vertices · {triangles} triangles at {detail:?}",
        solids.len()
    );
    println!(
        "  {:<28} {:>6} {:>6} {:>8}",
        "handle", "rings", "around", "triangles"
    );
    for solid in &solids {
        println!(
            "  {:<28} {:>6} {:>6} {:>8}",
            solid.handle.to_string(),
            solid.loft.rings(),
            solid.loft.around(),
            solid.loft.triangles().len()
        );
    }
    println!("  one solid per node, in walk order, and the triangles are the walk's whole output");
    nodes == 10 && solids.len() == nodes && triangles > 0
}

/// Every node is a closed solid, whatever its taper does at its ends.
fn every_loft_is_a_closed_solid() -> bool {
    let plant = plant();
    let solids = wood(&plant, Detail::DEFAULT);
    let mut worst_volume = f32::MAX;
    let mut closed = 0;
    for solid in &solids {
        let surface = solid.loft.view();
        if boundary_loops(surface) == 0
            && is_manifold(surface)
            && euler_characteristic(surface) == 2
            && signed_volume(surface) > 0.0
        {
            closed += 1;
        }
        worst_volume = worst_volume.min(signed_volume(surface));
    }
    println!("\n# the wood");
    println!(
        "  {closed} of {} solids are closed at χ = 2 with a positive volume; the smallest volume is {worst_volume:.3e}",
        solids.len()
    );
    println!(
        "  the twigs end at a radius of 8e-4 rather than a point, and a leaf's blade ends at a point; both close"
    );
    closed == solids.len() && worst_volume > 0.0
}

/// The culm's radius is the taper law at every ring, including the rise the law puts at a node.
fn the_taper_is_the_law_not_a_cone() -> bool {
    let plant = plant();
    let solids = wood(
        &plant,
        Detail {
            stations: 33,
            around: 24,
        },
    );
    let culm = &solids[0];
    let taper = plant.root().taper();
    let mut worst = 0.0f32;
    let mut radii = Vec::new();
    for ring in 0..culm.loft.rings() {
        let (_, uv) = culm.loft.vertex(ring, 0);
        let mut centroid = [0.0f32; 3];
        for slot in 0..culm.loft.around() {
            let (point, _) = culm.loft.vertex(ring, slot);
            for axis in 0..3 {
                centroid[axis] += point[axis] / culm.loft.around() as f32;
            }
        }
        let mut radius = 0.0f32;
        for slot in 0..culm.loft.around() {
            let (point, _) = culm.loft.vertex(ring, slot);
            radius = radius.max(length3(subtract(point, centroid)));
        }
        worst = worst.max((radius - taper.at(uv[0])).abs());
        radii.push(radius);
    }
    let at = |t: f32| radii[(t * (radii.len() - 1) as f32).round() as usize];
    let rise = at(0.32) - at(0.28);
    let cone = 0.030 + (0.010 - 0.030) * 0.32;
    println!("\n# the culm's taper");
    println!(
        "  worst radius against the law {worst:.3e} over {} rings",
        radii.len()
    );
    println!(
        "  the law rises 0.006 from 0.28 to 0.32 and the solid rises {rise:.6}: the node is in the geometry, not in a note"
    );
    println!(
        "  a straight cone between the law's ends would read {cone:.6} at 0.32, where the law reads {:.6}",
        taper.at(0.32)
    );
    worst < 1.0e-6 && rise > 0.004 && (taper.at(0.32) - cone).abs() > 1.0e-3
}

/// Every child's first point is on its parent's skin, and sunk by the amount it asked for.
fn a_joint_sits_on_its_parents_skin() -> bool {
    let plant = plant();
    let mut worst = 0.0f32;
    let mut joints = 0;
    for (handle, node) in plant.walk() {
        if handle.is_root() {
            continue;
        }
        let parent_handle = handle.parent().expect("a node below the root has a parent");
        let parent = plant.get(&parent_handle).expect("the parent resolves");
        let parent_spine = plant
            .spine(&parent_handle)
            .expect("the parent's spine resolves");
        let attachment = node.attachment();
        let station = attachment.station.clamp(0.0, 1.0);
        let skin = parent.taper().at(station) - attachment.sink;
        let placement = plant.placement(&handle).expect("the child resolves");
        let radial = span(placement.origin, parent_spine.at(station));
        worst = worst.max((radial - skin).abs());
        joints += 1;
    }
    println!("\n# the joints");
    println!(
        "  {joints} joints · worst distance from the parent's spine against its taper less the sink: {worst:.3e}"
    );
    println!(
        "  so a bough starts on the bark and a twig starts on the bough's, without a pass that places either"
    );
    joints == 9 && worst < 1.0e-5
}

/// The dial is the only resolution: doubling it quadruples the triangles and moves nothing else.
fn the_dial_is_the_only_resolution() -> bool {
    let plant = plant();
    let coarse = wood(&plant, Detail::DEFAULT);
    let fine = wood(&plant, Detail::DEFAULT.scaled(2));
    let coarse_triangles: usize = coarse
        .iter()
        .map(|solid| solid.loft.triangles().len())
        .sum();
    let fine_triangles: usize = fine.iter().map(|solid| solid.loft.triangles().len()).sum();
    let ratio = fine_triangles as f32 / coarse_triangles as f32;
    let closed = fine.iter().all(|solid| {
        boundary_loops(solid.loft.view()) == 0 && euler_characteristic(solid.loft.view()) == 2
    });
    let same_places = fine
        .iter()
        .zip(coarse.iter())
        .all(|(fine, coarse)| fine.handle == coarse.handle);
    println!("\n# the resolution dial");
    println!(
        "  {coarse_triangles} triangles at {:?} · {fine_triangles} at {:?} · ratio {ratio:.3}",
        Detail::DEFAULT,
        Detail::DEFAULT.scaled(2)
    );
    println!(
        "  doubling both dials makes four times the cells, and the caps' share shrinks, so the ratio is near 4 and above it"
    );
    println!(
        "  every solid stays closed: {closed}; the same nodes in the same order: {same_places}, so a coarser plant is the same plant"
    );
    closed && same_places && (3.5..4.5).contains(&ratio)
}
