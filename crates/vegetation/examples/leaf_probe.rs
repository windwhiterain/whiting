//! Measures the leaf field's invariants, each with a control that can fail, and exits non-zero if one
//! does.
//!
//! Nothing here is a test file: this is the probe that says whether `vegetation`'s leaves do what
//! their own documentation claims. The rows are `.agents/proposals/agentic-vegetation-generation.md`
//! V2 (a leaf is a distribution field over instances of one template) and D18 (the template is a thin
//! closed solid, so it has a positive signed volume and both of its faces are lit by their own
//! outward normal), plus the four laws a field's rows are read from — the count and the stations, the
//! anchor, the slots, and the frame — and the scale law that turns a station into a leaf.
//!
//! ```text
//! cargo run -p whiting-vegetation --example leaf_probe
//! ```

use std::f32::consts::FRAC_PI_2;

use shape::math::{Vec3, dot3, length3, subtract};
use shape::mesh::{boundary_loops, euler_characteristic, is_manifold, signed_volume};
use shape::{Curve, Jet, Scalar};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::leaf::{GOLDEN_ANGLE, LeafField, LeafInstance, LeafTemplate};
use whiting_vegetation::node::{Attachment, Node};
use whiting_vegetation::plant::{Placement, Plant};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("the count and the stations", the_count_and_the_stations()));
    checks.push((
        "the anchor is on the skin",
        the_anchor_is_on_the_parents_skin(),
    ));
    checks.push(("the slots advance", the_slots_advance()));
    checks.push(("pitch turns forward", pitch_turns_forward()));
    checks.push(("twist turns the face", twist_turns_the_face()));
    checks.push(("frames are orthonormal", every_frame_is_orthonormal()));
    checks.push(("the scale and the jitter", the_scale_and_the_jitter()));
    checks.push(("a blade is a closed solid", a_blade_is_a_closed_solid()));

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

/// A spine of `length` along its own `+y`, standing on its own origin.
fn straight(length: f32) -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.0, length, 0.0]),
        Jet::new([0.0, length, 0.0], [0.0, length, 0.0]),
    ])
    .expect("a straight spine's knots are distinct")
}

/// A law that falls from `base` to `tip` along a spine.
fn taper(base: f32, tip: f32) -> Scalar {
    Scalar::new(&[(0.0, base), (1.0, tip)]).expect("two knots are a law")
}

/// A metre of trunk with three twigs of `0.3` hanging from it: 1 + 3 nodes, three of them generation 1.
fn twig_plant() -> Plant {
    let mut root = Node::root(straight(1.0), taper(0.20, 0.10));
    for (station, slot) in [(0.35f32, 0.25f32), (0.60, 0.70), (0.85, 0.45)] {
        root.push(Node::attached(
            straight(0.3),
            taper(0.05, 0.01),
            Attachment::new(station, slot),
        ));
    }
    Plant::new(root, 20261017)
}

/// The field every row reads: 20 leaves per unit over a band of `(0.3, 0.9)`, and a size that grows.
fn field() -> LeafField {
    let mut field = LeafField::new(1, 0);
    field.band = (0.3, 0.9);
    field.density = Scalar::constant(20.0);
    field.size = Scalar::new(&[(0.0, 0.06), (1.0, 0.14)]).expect("two knots are a law");
    field
}

/// The same field with a density that rises toward the tip, so its stations cannot be a uniform sweep
/// of the band — the control for the row that counts and places.
fn rising_field() -> LeafField {
    let mut field = field();
    field.density = Scalar::new(&[(0.0, 4.0), (1.0, 60.0)]).expect("two knots are a law");
    field
}

/// The world frame on a node's skin at an attachment: outward `x`, the parent's tangent `y`.
fn anchor_frame(plant: &Plant, node: &Handle, station: f32, slot: f32) -> Placement {
    let placement = plant.placement(node).expect("a walked node places");
    let surface = plant
        .surface(node, Attachment::new(station, slot))
        .expect("a walked node's skin resolves");
    placement.then(&surface)
}

/// How far `left` stands from `-right`, which is where a right-handed quarter turn about the forward
/// axis puts the face.
fn against_reversed(left: Vec3, right: Vec3) -> f32 {
    length3(subtract(left, [-right[0], -right[1], -right[2]]))
}

/// The count is the rounded integral of the density law over the band, and the stations are where the
/// cumulative law reaches a half spacing — not a uniform sweep of the band, which the second half of
/// this row is the control for.
fn the_count_and_the_stations() -> bool {
    let plant = twig_plant();
    let even = field();
    let rising = rising_field();
    let even_groups = even.groups(&plant);
    let rising_groups = rising.groups(&plant);
    // 20 per unit of a 0.3 twig over a 0.6 band: `round(20 · 0.6 · 0.3) = 4`, one every 0.15.
    let expected = 4;
    let spacing = (even.band.1 - even.band.0) / expected as f32;
    let mut inside = true;
    let mut increasing = true;
    let mut worst_first = 0.0f32;
    println!("\n# the count and the stations");
    println!("  a density of 20 per unit, evenly placed");
    for group in &even_groups {
        println!(
            "  {:<14} {} leaves: {}",
            group.node,
            group.leaves.len(),
            stations(&group.leaves)
        );
        for (rank, leaf) in group.leaves.iter().enumerate() {
            inside &= leaf.station >= even.band.0 && leaf.station <= even.band.1;
            increasing &= rank == 0 || leaf.station > group.leaves[rank - 1].station;
        }
        if let Some(first) = group.leaves.first() {
            worst_first = worst_first.max((first.station - (even.band.0 + 0.5 * spacing)).abs());
        }
    }
    let counted = even_groups.len() == 3
        && even_groups
            .iter()
            .all(|group| group.leaves.len() == expected);
    println!("  three nodes of generation 1, {expected} leaves each: {counted}");
    println!(
        "  every station inside ({:.1}, {:.1}) and after the last: {inside}, {increasing}",
        even.band.0, even.band.1
    );
    println!(
        "  the first is half a spacing in, at {:.6} against {:.6}: worst {worst_first:.3e}",
        even.band.0 + 0.5 * spacing,
        even_groups[0].leaves[0].station
    );

    let middle = 0.5 * (rising.band.0 + rising.band.1);
    let mut below = 0;
    let mut above = 0;
    let mut shrinking = true;
    let tip_gaps = gaps(&rising_groups[0].leaves);
    for group in &rising_groups {
        inside &= group
            .leaves
            .iter()
            .all(|leaf| leaf.station >= rising.band.0 && leaf.station <= rising.band.1);
        increasing &= group
            .leaves
            .windows(2)
            .all(|pair| pair[1].station > pair[0].station);
        shrinking &= gaps(&group.leaves).windows(2).all(|pair| pair[1] < pair[0]);
        for leaf in &group.leaves {
            if leaf.station < middle {
                below += 1;
            } else {
                above += 1;
            }
        }
    }
    println!("  a density that rises toward the tip, placed by the same table");
    for group in &rising_groups {
        println!(
            "  {:<14} {} leaves: {}",
            group.node,
            group.leaves.len(),
            stations(&group.leaves)
        );
    }
    println!(
        "  the gaps shrink along it: {} — a uniform sweep would hold them even",
        tip_gaps
            .iter()
            .map(|gap| format!("{gap:.4}"))
            .collect::<Vec<String>>()
            .join(" ")
    );
    println!(
        "  {below} leaves below the band's middle and {above} above: the law, not a constant, put \
         them there"
    );
    println!(
        "  both fields keep the stations inside the band and strictly increasing: {inside}, {increasing}"
    );
    counted
        && inside
        && increasing
        && worst_first < 1.0e-6
        && shrinking
        && tip_gaps.len() >= 2
        && above > below
}

/// The gaps between one node's stations.
fn gaps(leaves: &[LeafInstance]) -> Vec<f32> {
    leaves
        .windows(2)
        .map(|pair| pair[1].station - pair[0].station)
        .collect()
}

/// One node's stations, to read in a row.
fn stations(leaves: &[LeafInstance]) -> String {
    leaves
        .iter()
        .map(|leaf| format!("{:.6}", leaf.station))
        .collect::<Vec<String>>()
        .join(" ")
}

/// A leaf hangs on the parent's skin, `sink` for none: its origin is the attachment resolved in the
/// world, and the distance back to the parent's own spine is the parent's taper there.
fn the_anchor_is_on_the_parents_skin() -> bool {
    let plant = twig_plant();
    let groups = field().groups(&plant);
    let mut worst_origin = 0.0f32;
    let mut worst_radius = 0.0f32;
    let mut leaves = 0;
    println!("\n# the anchor is on the parent's skin");
    for group in &groups {
        let node = plant.get(&group.node).expect("a walked node resolves");
        let spine = plant
            .spine(&group.node)
            .expect("a walked node's spine resolves");
        for leaf in &group.leaves {
            leaves += 1;
            let parent = anchor_frame(&plant, &group.node, leaf.station, leaf.slot);
            worst_origin =
                worst_origin.max(length3(subtract(leaf.placement.origin, parent.origin)));
            let radius = length3(subtract(leaf.placement.origin, spine.at(leaf.station)));
            worst_radius = worst_radius.max((radius - node.taper().at(leaf.station)).abs());
        }
    }
    let station = groups[0].leaves[0].station;
    let taper = plant
        .get(&groups[0].node)
        .expect("a walked node resolves")
        .taper()
        .clone();
    let moved = (taper.at(station + 0.05) - taper.at(station)).abs();
    println!(
        "  {leaves} instances, every origin against its own attachment, composed: {worst_origin:.3e}"
    );
    println!("  every radius against the parent's taper law: worst {worst_radius:.3e}");
    println!(
        "  control: 0.05 of station moves the radius {moved:.3e}, so the law is not a constant"
    );
    worst_origin < 1.0e-6 && worst_radius < 1.0e-6 && moved > 1.0e-3
}

/// Each leaf advances the golden angle around the parent's loop, from a phase of its node's own, so
/// two nodes of one field do not align their spirals.
fn the_slots_advance() -> bool {
    let plant = twig_plant();
    let groups = field().groups(&plant);
    let mut worst_step = 0.0f32;
    let mut pairs = 0;
    for group in &groups {
        for pair in group.leaves.windows(2) {
            pairs += 1;
            let step = (pair[1].slot - pair[0].slot).rem_euclid(1.0);
            worst_step = worst_step.max((step - GOLDEN_ANGLE).abs());
        }
    }
    let phases: Vec<String> = groups
        .iter()
        .map(|group| format!("{:.6}", group.leaves[0].slot))
        .collect();
    let apart = groups.iter().enumerate().all(|(index, first)| {
        groups[index + 1..]
            .iter()
            .all(|other| (first.leaves[0].slot - other.leaves[0].slot).abs() > 1.0e-3)
    });
    println!("\n# the slots advance by the phyllotaxy");
    println!("  {pairs} consecutive pairs against {GOLDEN_ANGLE:.8}: worst {worst_step:.3e}");
    println!(
        "  the first slot of each node, the phase: {} · apart: {apart}",
        phases.join(" ")
    );
    println!(
        "  the phase is the node's first draw, before any jitter, so the spirals do not line up"
    );
    worst_step < 1.0e-6 && apart
}

/// `pitch` turns the leaf's forward from the parent's outward direction toward the parent's tip.
fn pitch_turns_forward() -> bool {
    let plant = twig_plant();
    let mut level = field();
    level.pitch = Scalar::constant(0.0);
    let mut raised = field();
    raised.pitch = Scalar::constant(FRAC_PI_2);
    let level = level.groups(&plant);
    let raised = raised.groups(&plant);
    let flat = level[0].leaves[0];
    let steep = raised[0].leaves[0];
    let parent = anchor_frame(&plant, &level[0].node, flat.station, flat.slot);
    let flat_outward = dot3(flat.placement.x, parent.x);
    let flat_tangent = dot3(flat.placement.x, parent.y);
    let steep_tangent = dot3(steep.placement.x, parent.y);
    let steep_outward = dot3(steep.placement.x, parent.x);
    println!("\n# pitch turns forward toward the parent's tip");
    println!("  the same node, the same leaf, one field each");
    println!(
        "  pitch 0: forward against the outward {flat_outward:.6}, against the tangent {flat_tangent:.6}"
    );
    println!(
        "  pitch {}: forward against the tangent {steep_tangent:.6}, against the outward {steep_outward:.3e}",
        FRAC_PI_2
    );
    println!("  a pitch of a quarter turn is the leaf pointing at the parent's tip, to 1e-6");
    (flat_outward - 1.0).abs() < 1.0e-6
        && flat_tangent.abs() < 1.0e-6
        && (steep_tangent - 1.0).abs() < 1.0e-6
        && steep_outward.abs() < 1.0e-6
}

/// `twist` rolls the face about the leaf's own forward and leaves the forward where it was.
fn twist_turns_the_face() -> bool {
    let plant = twig_plant();
    let level = field();
    let mut rolled = field();
    rolled.twist = Scalar::constant(FRAC_PI_2);
    let level = level.groups(&plant);
    let rolled = rolled.groups(&plant);
    let straight = level[0].leaves[0];
    let turned = rolled[0].leaves[0];
    let side_onto_face = length3(subtract(turned.placement.y, straight.placement.z));
    let face_onto_reversed_side = against_reversed(turned.placement.z, straight.placement.y);
    let forward_kept = length3(subtract(turned.placement.x, straight.placement.x));
    println!("\n# twist turns the face about forward");
    println!("  the same node, the same leaf, one field each");
    println!("  the side against the untwisted face: {side_onto_face:.3e}");
    println!("  the face against the reversed untwisted side: {face_onto_reversed_side:.3e}");
    println!("  forward unchanged by the twist: {forward_kept:.3e}");
    println!(
        "  a right-handed quarter turn about +forward swaps the two, so the face lands on -side"
    );
    side_onto_face < 1.0e-6 && face_onto_reversed_side < 1.0e-6 && forward_kept < 1.0e-6
}

/// Every instance's frame is a rigid one: unit axes, and no two of them leaning on each other.
fn every_frame_is_orthonormal() -> bool {
    let plant = twig_plant();
    let mut rolled = field();
    rolled.twist = Scalar::constant(1.1);
    let fields = [field(), rolled];
    let mut worst_length = 0.0f32;
    let mut worst_dot = 0.0f32;
    let mut instances = 0;
    for field in &fields {
        for group in field.groups(&plant) {
            for leaf in &group.leaves {
                let placement = leaf.placement;
                worst_length = worst_length
                    .max((length3(placement.x) - 1.0).abs())
                    .max((length3(placement.y) - 1.0).abs())
                    .max((length3(placement.z) - 1.0).abs());
                worst_dot = worst_dot
                    .max(dot3(placement.x, placement.y).abs())
                    .max(dot3(placement.y, placement.z).abs())
                    .max(dot3(placement.z, placement.x).abs());
                instances += 1;
            }
        }
    }
    println!("\n# every frame is orthonormal");
    println!(
        "  {instances} instances over a twist of 0 and 1.1: worst |axis| - 1 {worst_length:.3e}"
    );
    println!("  worst pairwise dot: {worst_dot:.3e}");
    println!("  a placement never scales or shears what it places, so both are to 1e-6");
    instances > 0 && worst_length < 1.0e-6 && worst_dot < 1.0e-6
}

/// The scale is the size law at the leaf's own station, times the node's jitter, and the same field
/// run twice gives the same numbers.
fn the_scale_and_the_jitter() -> bool {
    let plant = twig_plant();
    let even = field();
    let shaky = LeafField {
        jitter: 0.2,
        ..field()
    };
    let even_groups = even.groups(&plant);
    let shaky_groups = shaky.groups(&plant);
    let again = shaky.groups(&plant);
    let exact = even_groups
        .iter()
        .flat_map(|group| group.leaves.iter())
        .all(|leaf| leaf.scale == even.size.at(leaf.station));
    let mut widest = 0.0f32;
    let mut scales: Vec<f32> = Vec::new();
    for leaf in shaky_groups.iter().flat_map(|group| group.leaves.iter()) {
        widest = widest.max((leaf.scale / shaky.size.at(leaf.station) - 1.0).abs());
        scales.push(leaf.scale);
    }
    let varied = scales.iter().filter(|&&scale| scale != scales[0]).count();
    let repeated = shaky_groups
        .iter()
        .zip(again.iter())
        .all(|(first, second)| {
            first
                .leaves
                .iter()
                .zip(second.leaves.iter())
                .all(|(left, right)| {
                    left.scale == right.scale
                        && left.station == right.station
                        && left.slot == right.slot
                })
        });
    println!("\n# the scale law and the jitter");
    println!("  jitter 0: every scale is size.at(station) exactly: {exact}");
    println!("  jitter 0.2: the widest departure from size.at(station) is {widest:.3} of it");
    println!(
        "  {varied} of {} scales differ from the first",
        scales.len()
    );
    println!("  the same field twice: identical stations, slots and scales: {repeated}");
    println!("  the jitter draw is unconditional, so editing the jitter moves no other number");
    exact && widest <= 0.2 + 1.0e-6 && varied >= 2 && repeated
}

/// A blade's midrib: it starts at the origin along `+x` and arcs up inside the `xy` plane, which is
/// where the contract says a blade lives.
fn blade_midrib() -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.35, 0.0, 0.0]),
        Jet::new([0.70, 0.05, 0.0], [0.30, 0.05, 0.0]),
        Jet::new([1.00, 0.14, 0.0], [0.20, 0.03, 0.0]),
    ])
    .expect("the midrib's knots are distinct")
}

/// The blade is a closed solid with a positive volume, so both of its faces are lit by their own
/// outward normal rather than by a shading trick.
fn a_blade_is_a_closed_solid() -> bool {
    let midrib = blade_midrib();
    // Zero at both ends: the base and the tip are points, which is the tip case, and the caps stay.
    let width = Scalar::new(&[(0.0, 0.0), (0.4, 0.5), (1.0, 0.0)]).expect("three knots are a law");
    let thickness = 0.02;
    let template = LeafTemplate::blade(midrib.clone(), width, thickness, 9, 12);
    let loft = template.loft();
    let volume = signed_volume(loft.view());
    let closed = boundary_loops(loft.view()) == 0
        && is_manifold(loft.view())
        && euler_characteristic(loft.view()) == 2;
    let (base, _) = loft.vertex(0, 0);
    let base_error = length3(subtract(base, midrib.at(0.0)));
    let mut base_x = 0.0f32;
    let mut widest_side = 0.0f32;
    let mut thickest_face = 0.0f32;
    for ring in 0..loft.rings() {
        for slot in 0..loft.around() {
            let (point, uv) = loft.vertex(ring, slot);
            if ring == 0 {
                base_x = base_x.max(point[0].abs());
            }
            // The midrib arcs into the template's own `y`, so the blade's own extent is measured
            // against the spine point at the vertex's own station rather than against the origin.
            let spine_point = midrib.at(uv[0]);
            widest_side = widest_side.max((point[1] - spine_point[1]).abs());
            thickest_face = thickest_face.max((point[2] - spine_point[2]).abs());
        }
    }
    let (tip, _) = loft.vertex(loft.rings() - 1, 0);
    let mut tip_spread = 0.0f32;
    for slot in 0..loft.around() {
        let (point, _) = loft.vertex(loft.rings() - 1, slot);
        tip_spread = tip_spread.max(length3(subtract(point, tip)));
    }
    println!("\n# a blade: a midrib and a lens, pointed at both ends");
    println!(
        "  boundary loops {} · manifold {} · χ = {} · signed volume {volume:.6}",
        boundary_loops(loft.view()),
        is_manifold(loft.view()),
        euler_characteristic(loft.view())
    );
    println!(
        "  {} rings of {} = {} vertices, the caps reusing the end rings' own",
        loft.rings(),
        loft.around(),
        loft.positions().len()
    );
    println!("  the first ring's slot 0 against the midrib's first point: {base_error:.3e}");
    println!("  the base ring's own |x|: {base_x:.3e}, the midrib starts at the origin");
    println!(
        "  the tip ring's spread: {tip_spread:.3e} — the tip case, with no special case in the walk"
    );
    println!(
        "  the blade's own width reaches |y| {widest_side:.6} and its face |z| {thickest_face:.6}: \
         the width is the template's side and the face its z, so a transposed section would swap them"
    );
    closed
        && volume > 0.0
        && base_error < 1.0e-6
        && base_x < 1.0e-6
        && tip_spread < 1.0e-6
        && widest_side > 10.0 * thickest_face
}
