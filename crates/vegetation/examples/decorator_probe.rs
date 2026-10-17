//! Measures the decorators: where a seat lands, what its frame is, and what it costs.
//!
//! Every row is one promise of `.agents/proposals/agentic-vegetation-generation.md` V3 and D11 — a small authored
//! template at a station, seated by the same numbers a joint is, with no rotation expressible — read off the
//! placements rather than off the seats' own fields.
//!
//! ```text
//! cargo run -p whiting-vegetation --example decorator_probe
//! ```

use shape::math::span;
use shape::mesh::{boundary_loops, euler_characteristic, signed_volume};
use shape::{Curve, Jet, Scalar, Section, Sweep};
use whiting_vegetation::decorator::{Placed, Seat, Template, placed};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::node::Prune;
use whiting_vegetation::ops::{Change, Edit, Property, apply};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("a seat is three numbers", a_seat_is_three_numbers()));
    checks.push(("no rotation is expressible", no_rotation_is_expressible()));
    checks.push(("a template is a solid", a_template_is_a_closed_solid()));
    checks.push(("k seats are one draw", k_seats_are_one_draw()));
    checks.push(("a seat follows its node", a_seat_follows_its_node()));
    checks.push((
        "a tombstone takes its knot",
        a_tombstone_takes_its_decorator(),
    ));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<30} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

fn species() -> Species {
    bamboo(20_261_017)
}

fn bough(index: u32) -> Handle {
    Handle::root().child(index)
}

/// A knot: a short tube that bulges, authored with its base at the origin and its axis along `+x`.
fn knot() -> Template {
    let spine = Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.012, 0.0, 0.0]),
        Jet::new([0.014, 0.0, 0.0], [0.010, 0.0, 0.0]),
        Jet::new([0.026, 0.0, 0.0], [0.004, 0.0, 0.0]),
    ])
    .expect("the knot's knots are distinct");
    let mut sweep = Sweep::new(spine, Section::circle());
    sweep.scale =
        Scalar::new(&[(0.0, 0.010), (0.5, 0.026), (1.0, 0.004)]).expect("the knot's law is a law");
    Template {
        sweep,
        stations: 7,
        around: 12,
    }
}

/// One knot at the base of every bough, sunk a centimetre into the bark.
fn knots(species: &Species) -> Vec<Seat> {
    (0..3)
        .map(|index| {
            let node = species
                .plant
                .get(&bough(index))
                .expect("the bough resolves");
            Seat::new(bough(index), 0, 0.1, node.attachment().slot).sunk(0.01)
        })
        .collect()
}

/// A seat is the attachment a joint is: the frame it lands in is the node's own surface frame there, moved into
/// the world by the node's placement.
fn a_seat_is_three_numbers() -> bool {
    let species = species();
    let seats = knots(&species);
    let placed = placed(&species.plant, &seats);
    let mut worst_frame = 0.0f32;
    let mut worst_skin = 0.0f32;
    for (seat, instance) in seats.iter().zip(placed.iter()) {
        let node = species.plant.get(&seat.node).expect("the node resolves");
        let placement = species
            .plant
            .placement(&seat.node)
            .expect("the node's placement resolves");
        let frame = species
            .plant
            .surface(&seat.node, seat.attachment())
            .expect("the seat's frame resolves");
        // The surface frame is the node's own; the world frame is the node's placement on top of it.
        let world = placement.then(&frame);
        let station = seat.station.clamp(0.0, 1.0);
        let skin = node.taper_now().at(station) - seat.sink;
        let spine = species
            .plant
            .spine(&seat.node)
            .expect("the node's spine resolves");
        let radial = span(instance.placement.origin, spine.at(station));
        worst_skin = worst_skin.max((radial - skin).abs());
        worst_frame = worst_frame.max(span(instance.placement.origin, world.origin));
        worst_frame = worst_frame.max(span(instance.placement.x, world.x));
    }
    println!("\n# a seat");
    println!(
        "  {} seats · their origins against the node's own surface frame {worst_frame:.3e}, and against its skin {worst_skin:.3e}",
        placed.len()
    );
    for (seat, instance) in seats.iter().zip(placed.iter()).take(3) {
        println!(
            "  {:<20} station {:.3} slot {:.3} → origin {:.4} {:.4} {:.4}",
            seat.node.to_string(),
            seat.station,
            seat.slot,
            instance.placement.origin[0],
            instance.placement.origin[1],
            instance.placement.origin[2]
        );
    }
    println!(
        "  the control is the skin: a seat placed by a transform instead of a joint would float off the bark"
    );
    placed.len() == seats.len() && worst_frame < 1e-5 && worst_skin < 1e-5
}

/// The frame is the surface's, whole: its `x` is outward, its `y` is the parent's tangent, and no angle appears.
fn no_rotation_is_expressible() -> bool {
    let species = species();
    let seats = knots(&species);
    let placed = placed(&species.plant, &seats);
    let mut worst_axis = 0.0f32;
    let mut different = 0;
    let mut reference: Option<[f32; 3]> = None;
    for (seat, instance) in seats.iter().zip(placed.iter()) {
        let placement = species
            .plant
            .placement(&seat.node)
            .expect("the node's placement resolves");
        let frame = species
            .plant
            .surface(&seat.node, seat.attachment())
            .expect("the seat's frame resolves");
        let world = placement.then(&frame);
        worst_axis = worst_axis.max(span(instance.placement.x, world.x));
        worst_axis = worst_axis.max(span(instance.placement.y, world.y));
        worst_axis = worst_axis.max(span(instance.placement.z, world.z));
        match reference {
            None => reference = Some(instance.placement.x),
            Some(first) => {
                if span(first, instance.placement.x) > 1e-3 {
                    different += 1;
                }
            }
        }
    }
    println!("\n# no rotation is expressible");
    println!(
        "  every placement's axes are its surface frame's to {worst_axis:.3e}: x outward, y along the parent, z their cross product"
    );
    println!(
        "  {different} of {} seats have a different frame from the first, so the frame is derived and not chosen",
        placed.len().saturating_sub(1)
    );
    println!(
        "  the control is `Seat`: it carries a station, a slot, a sink and a scale, and there is no angle anywhere in it"
    );
    worst_axis < 1e-6 && different > 0
}

/// A decorator's template is a closed solid, so a scene can draw it as a mesh rather than a sheet.
fn a_template_is_a_closed_solid() -> bool {
    let template = knot();
    let loft = template.loft();
    let surface = loft.view();
    let volume = signed_volume(surface);
    let closed = boundary_loops(surface) == 0
        && shape::mesh::is_manifold(surface)
        && euler_characteristic(surface) == 2
        && volume > 0.0;
    println!("\n# a decorator's template");
    println!(
        "  the knot is {} rings of {} at {} triangles: χ = {}, closed at {:.0} boundary loops, volume {:.3e}",
        loft.rings(),
        loft.around(),
        loft.triangles().len(),
        euler_characteristic(surface),
        boundary_loops(surface),
        volume
    );
    println!(
        "  the control is the volume: a template authored as a sheet would have a zero volume and two boundary loops"
    );
    closed && volume > 0.0
}

/// `k` decorators of one template are one mesh and `k` transforms, exactly as `k` leaves are.
fn k_seats_are_one_draw() -> bool {
    let species = species();
    // Every branch of every bough gets a knot: six seats, one template.
    let mut seats = Vec::new();
    for bough_index in 0..3 {
        for branch_index in 0..2 {
            let node = bough(bough_index).child(branch_index);
            seats.push(Seat::new(node, 0, 0.05, 0.5).sunk(0.002));
        }
    }
    let placed = placed(&species.plant, &seats);
    let groups: Vec<(Handle, usize)> = {
        let mut pairs: Vec<(Handle, usize)> = placed
            .iter()
            .map(|instance: &Placed| (instance.node.clone(), instance.template))
            .collect();
        pairs.sort_by(|one, other| one.0.to_string().cmp(&other.0.to_string()));
        pairs.dedup();
        pairs
    };
    let template = knot();
    let loft = template.loft();
    let baked = loft.triangles().len() * placed.len();
    println!("\n# k seats, one mesh");
    println!(
        "  {} seats of one template · the template is {} triangles and the placed instances are {} transforms",
        placed.len(),
        loft.triangles().len(),
        placed.len()
    );
    println!(
        "  baked instead they would be {baked} triangles, and the pairs a scene groups by are {} — one per seat here, because each sits on its own node",
        groups.len()
    );
    println!(
        "  the control is the template: a decorator whose shape changed per instance would need a mesh per instance"
    );
    placed.len() == 6 && groups.len() == 6 && loft.triangles().len() > 0
}

/// A seat is placed against the skin the plant draws, so an edit that moves its node carries it along.
fn a_seat_follows_its_node() -> bool {
    let mut species = species();
    let seats = knots(&species);
    let before = placed(&species.plant, &seats);
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            bough(1),
            Change::Offset {
                property: Property::Station,
                by: 0.12,
            },
        )],
    );
    let after = placed(&species.plant, &seats);
    let moved = span(before[1].placement.origin, after[1].placement.origin);
    // And it is still on the bark: the radial distance to the bough's spine is the taper less the sink.
    let node = species.plant.get(&bough(1)).expect("the bough resolves");
    let station = 0.1f32;
    let skin = node.taper_now().at(station) - seats[1].sink;
    let spine = species.plant.spine(&bough(1)).expect("the bough resolves");
    let radial = span(after[1].placement.origin, spine.at(station));
    println!("\n# a seat and an edit");
    println!(
        "  {report} · the second knot moved {moved:.6} and sits {radial:.3e} from the bough's spine against a taper less the sink of {skin:.6}"
    );
    println!(
        "  the control is the move: a decorator stored in the world would stay where it was planted"
    );
    report.clean() && moved > 1e-3 && (radial - skin).abs() < 1e-5
}

/// A tombstone takes its decorator with it, and only its own.
fn a_tombstone_takes_its_decorator() -> bool {
    let mut species = species();
    let seats = knots(&species);
    let before = placed(&species.plant, &seats);
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            bough(1),
            Change::Prune {
                reason: Prune::ByAuthor,
            },
        )],
    );
    let after = placed(&species.plant, &seats);
    let lost = before.len() - after.len();
    let bough_gone = !after.iter().any(|instance| instance.node == bough(1));
    let others = after.iter().any(|instance| instance.node == bough(0));
    println!("\n# a tombstone's decorator");
    println!(
        "  {report} · {} seats before and {} after: {} lost, the pruned one gone: {bough_gone}, the others still placed: {others}",
        before.len(),
        after.len(),
        lost
    );
    println!(
        "  the control is the count: a decorator placed from a list of handles would draw on a node the plant no longer draws"
    );
    report.clean() && lost == 1 && bough_gone && others
}
