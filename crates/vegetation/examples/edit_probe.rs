//! Measures the hand: what an edit reaches, what it refuses, and what a tombstone leaves behind.
//!
//! Every row is one promise of `.agents/proposals/agentic-vegetation-generation.md` V5 — the edit table and
//! its stated order — read off the geometry rather than off the edit's own fields, and every row carries a
//! control that could have failed it.
//!
//! ```text
//! cargo run -p whiting-vegetation --example edit_probe
//! ```

use shape::math::{length3, span, subtract};
use shape::mesh::signed_volume;
use shape::{Jet, Scalar};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::expand::{Detail, Wood, wood};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::node::Prune;
use whiting_vegetation::ops::{Change, Edit, Frame, Problem, Property, apply, paint};
use whiting_vegetation::selector::Selector;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("an offset reaches one node", an_offset_reaches_one_node()));
    checks.push((
        "a length carries its children",
        a_length_nudge_carries_its_children(),
    ));
    checks.push((
        "a station slides the node",
        a_station_nudge_slides_the_node(),
    ));
    checks.push(("three frames, three places", a_shift_reads_its_frame()));
    checks.push(("a prune is a tombstone", a_prune_is_a_tombstone()));
    checks.push((
        "a prune outranks an offset",
        a_prune_outranks_an_offset_on_the_same_node(),
    ));
    checks.push(("a stale handle is named", a_stale_handle_is_named()));
    checks.push((
        "a value out of range is refused",
        a_value_out_of_range_is_refused(),
    ));
    checks.push(("a bake replaces the spine", a_bake_replaces_the_spine()));
    checks.push((
        "a bake shape refuses is kept out",
        a_bake_shape_refuses_is_not_applied(),
    ));
    checks.push((
        "the brush writes the edits",
        a_brush_writes_the_edit_table(),
    ));
    checks.push(("a curve is a law added", a_curve_is_a_law_added()));
    checks.push(("two curves add up", two_curves_add_up()));
    checks.push((
        "a curve of zero changes nothing",
        a_curve_of_zero_changes_nothing(),
    ));
    checks.push((
        "a law that thins to nothing is refused",
        a_law_that_thins_to_nothing_is_refused(),
    ));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<32} {}", if *passed { "pass" } else { "FAIL" });
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

fn branch(bough_index: u32, branch_index: u32) -> Handle {
    bough(bough_index).child(branch_index)
}

/// The solid a handle names, in an expansion.
fn solid_of<'a>(solids: &'a [Wood], handle: &Handle) -> &'a Wood {
    solids
        .iter()
        .find(|solid| &solid.handle == handle)
        .expect("the plant has a solid for every live node")
}

/// A ring's radius: the furthest vertex from the ring's own centroid.
fn radius_at(loft: &shape::Loft, ring: usize) -> f32 {
    let mut centroid = [0.0f32; 3];
    for slot in 0..loft.around() {
        let (point, _) = loft.vertex(ring, slot);
        for axis in 0..3 {
            centroid[axis] += point[axis] / loft.around() as f32;
        }
    }
    let mut radius = 0.0f32;
    for slot in 0..loft.around() {
        let (point, _) = loft.vertex(ring, slot);
        radius = radius.max(length3(subtract(point, centroid)));
    }
    radius
}

/// An offset on one node's radius moves that node's rings and nothing else in the plant.
fn an_offset_reaches_one_node() -> bool {
    let before = species();
    let solids_before = wood(&before.plant, Detail::DEFAULT);
    let mut after = species();
    let report = apply(
        &mut after.plant,
        &[Edit::new(
            bough(1),
            Change::Offset {
                property: Property::Radius,
                by: 0.004,
            },
        )],
    );
    let solids_after = wood(&after.plant, Detail::DEFAULT);
    let ring = Detail::DEFAULT.stations / 2;
    let thin = radius_at(&solid_of(&solids_before, &bough(1)).loft, ring);
    let thick = radius_at(&solid_of(&solids_after, &bough(1)).loft, ring);
    let growth = thick - thin;
    let mut moved = Vec::new();
    for solid in &solids_before {
        let same = solid_of(&solids_after, &solid.handle);
        let was = radius_at(&solid.loft, ring);
        let now = radius_at(&same.loft, ring);
        if (was - now).abs() > 1e-6 {
            moved.push(solid.handle.to_string());
        }
    }
    println!("\n# an offset on one node");
    println!(
        "  {report} · the bough's radius at ring {ring}: {thin:.6} → {thick:.6}, a growth of {growth:.3e}"
    );
    println!(
        "  the rings that moved: {moved:?} — one of {} solids, so the control is that a radius offset is not a plant-wide scale",
        solids_before.len()
    );
    println!(
        "  the radius is measured from each ring's own centroid, so a solid that only moved reads as unchanged"
    );
    report.clean()
        && (growth - 0.004).abs() < 1e-6
        && moved.len() == 1
        && moved[0] == bough(1).to_string()
}

/// A length nudge shortens one spine and carries everything that hangs from it.
fn a_length_nudge_carries_its_children() -> bool {
    let before = species();
    let length_before = before
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    let origins_before: Vec<[f32; 3]> = (0..2)
        .map(|index| {
            before
                .plant
                .placement(&branch(0, index as u32))
                .expect("the branch resolves")
                .origin
        })
        .collect();
    let mut after = species();
    let report = apply(
        &mut after.plant,
        &[Edit::new(
            bough(0),
            Change::Offset {
                property: Property::Length,
                by: -0.15,
            },
        )],
    );
    let length_after = after
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    let origins_after: Vec<[f32; 3]> = (0..2)
        .map(|index| {
            after
                .plant
                .placement(&branch(0, index as u32))
                .expect("the branch resolves")
                .origin
        })
        .collect();
    let shortened = length_before - length_after;
    let mut least_move = f32::MAX;
    let mut worst_skin = 0.0f32;
    for index in 0..2 {
        let moved = span(origins_before[index], origins_after[index]);
        least_move = least_move.min(moved);
        // The branch still sits on the bough's bark, now drawn at the shorter length.
        let branch_node = after
            .plant
            .get(&branch(0, index as u32))
            .expect("the branch resolves");
        let station = branch_node.effective_attachment().station.clamp(0.0, 1.0);
        let skin = after
            .plant
            .get(&bough(0))
            .expect("the bough resolves")
            .taper_now()
            .at(station)
            - branch_node.effective_attachment().sink;
        let spine = after.plant.spine(&bough(0)).expect("the bough resolves");
        let radial = span(origins_after[index], spine.at(station));
        worst_skin = worst_skin.max((radial - skin).abs());
    }
    println!("\n# a length nudge");
    println!(
        "  {report} · the bough is {shortened:.6} shorter where it was asked to be 0.15 shorter"
    );
    println!(
        "  both branches moved, the least by {least_move:.6} m, and both still sit on the shorter bough's bark to {worst_skin:.3e}"
    );
    println!(
        "  the control is the move: a nudge that shortened the spine without carrying the branches would read 0.000000"
    );
    report.clean() && (shortened - 0.15).abs() < 1e-4 && least_move > 0.01 && worst_skin < 1e-5
}

/// A station nudge slides a node along its parent, to the far end of the parent rather than off it.
fn a_station_nudge_slides_the_node() -> bool {
    let before = species();
    let origin_before = before
        .plant
        .placement(&bough(2))
        .expect("the bough resolves")
        .origin;
    let trunk = before
        .plant
        .spine(&Handle::root())
        .expect("the trunk resolves");
    let trunk_length = trunk.length();
    let mut after = species();
    let report = apply(
        &mut after.plant,
        &[Edit::new(
            bough(2),
            Change::Offset {
                property: Property::Station,
                by: 0.1,
            },
        )],
    );
    let placement = after
        .plant
        .placement(&bough(2))
        .expect("the bough resolves");
    let moved = span(origin_before, placement.origin);
    let expected = 0.1 * trunk_length;
    let station = after
        .plant
        .get(&bough(2))
        .expect("the bough resolves")
        .effective_attachment();
    let at = station.station.clamp(0.0, 1.0);
    let skin = after
        .plant
        .get(&Handle::root())
        .expect("the trunk resolves")
        .taper_now()
        .at(at)
        - station.sink;
    let radial = span(placement.origin, trunk.at(at));
    println!("\n# a station nudge");
    println!(
        "  {report} · the bough moved {moved:.6} m where a tenth of the trunk's {trunk_length:.6} m is {expected:.6}"
    );
    println!(
        "  it sits at station {at:.4} on the trunk's bark: {radial:.3e} from the spine against the taper less the sink, {skin:.6}"
    );
    report.clean() && (moved - expected).abs() < 1e-4 && (radial - skin).abs() < 1e-4
}

/// One translation written in three frames lands in three places, and the world one is the world's.
fn a_shift_reads_its_frame() -> bool {
    let by = [0.05f32, 0.0, 0.0];
    let target = branch(0, 0);
    let mut origins: Vec<[f32; 3]> = Vec::new();
    for frame in [Frame::Local, Frame::Parent, Frame::World] {
        let mut species = species();
        let report = apply(
            &mut species.plant,
            &[Edit::new(target.clone(), Change::Shift { frame, by })],
        );
        let placement = species
            .plant
            .placement(&target)
            .expect("the branch resolves");
        println!(
            "\n# a shift in the {} frame\n  {report} · the branch's origin is now {:.6} {:.6} {:.6}",
            frame.name(),
            placement.origin[0],
            placement.origin[1],
            placement.origin[2]
        );
        origins.push(placement.origin);
    }
    let unshifted = species()
        .plant
        .placement(&target)
        .expect("the branch resolves")
        .origin;
    let world = span(unshifted, origins[2]);
    let local_versus_world = span(origins[0], origins[2]);
    let parent_versus_world = span(origins[1], origins[2]);
    let local_versus_parent = span(origins[0], origins[1]);
    println!(
        "  the world frame moves it by {world:.6} for a shift of {:.6}, and the three frames differ by {local_versus_world:.3e}, {parent_versus_world:.3e} and {local_versus_parent:.3e}",
        length3(by)
    );
    // A child of the root has no parent frame of its own to differ from the world's: the root's placement is
    // the identity, so reading the trunk's frame and the world's are the same rotation. That is a reading
    // rather than a defect, and the row states it so the next reader does not call it one.
    let mut parent_plant = species();
    apply(
        &mut parent_plant.plant,
        &[Edit::new(
            bough(0),
            Change::Shift {
                frame: Frame::Parent,
                by,
            },
        )],
    );
    let mut world_plant = species();
    apply(
        &mut world_plant.plant,
        &[Edit::new(
            bough(0),
            Change::Shift {
                frame: Frame::World,
                by,
            },
        )],
    );
    let parent_origin = parent_plant
        .plant
        .placement(&bough(0))
        .expect("the bough resolves")
        .origin;
    let world_origin = world_plant
        .plant
        .placement(&bough(0))
        .expect("the bough resolves")
        .origin;
    let indistinguishable = span(parent_origin, world_origin);
    println!(
        "  for a child of the root the parent and world frames land {indistinguishable:.3e} apart, because the root's placement is the identity"
    );
    println!(
        "  the control is the difference: a shift that ignored its frame would land all three in one place"
    );
    (world - length3(by)).abs() < 1e-6
        && local_versus_world > 1e-3
        && parent_versus_world > 1e-3
        && local_versus_parent > 1e-3
        && indistinguishable < 1e-6
}

/// A prune is a tombstone: the walk still names the node, the plant stops drawing it, nothing renumbers.
fn a_prune_is_a_tombstone() -> bool {
    let mut species = species();
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            bough(1),
            Change::Prune {
                reason: Prune::ByAuthor,
            },
        )],
    );
    let named: Vec<Handle> = species.plant.walk().map(|(handle, _)| handle).collect();
    let live: Vec<Handle> = species.plant.live().map(|(handle, _)| handle).collect();
    let solids = wood(&species.plant, Detail::DEFAULT);
    let tombstone = species
        .plant
        .get(&bough(1))
        .expect("a tombstone still resolves")
        .pruned();
    let child_still_named = named.contains(&branch(1, 0));
    let child_not_drawn = !live.contains(&branch(1, 0));
    println!("\n# a prune");
    println!(
        "  {report} · the walk names {} nodes and the plant draws {} · the bough's reason is {tombstone:?}",
        named.len(),
        live.len()
    );
    println!(
        "  its branch is still named: {child_still_named}, and no longer drawn: {child_not_drawn}, and the wood has {} solids",
        solids.len()
    );
    println!("  the control is the walk: a removal from a list would renumber the boughs after it");
    report.clean()
        && named.len() == 10
        && live.len() == 7
        && solids.len() == 7
        && tombstone == Some(Prune::ByAuthor)
        && child_still_named
        && child_not_drawn
}

/// A prune in a batch outranks an offset on the same node: the offset is reported, not written.
fn a_prune_outranks_an_offset_on_the_same_node() -> bool {
    let mut species = species();
    let report = apply(
        &mut species.plant,
        &[
            Edit::new(
                bough(1),
                Change::Offset {
                    property: Property::Radius,
                    by: 0.01,
                },
            ),
            Edit::new(
                bough(1),
                Change::Prune {
                    reason: Prune::Thinned,
                },
            ),
        ],
    );
    let nudged = species
        .plant
        .get(&bough(1))
        .expect("the bough resolves")
        .nudges()
        .radius;
    let pruned_problems = report
        .problems
        .iter()
        .filter(|problem| matches!(problem, Problem::Pruned { .. }))
        .count();
    println!("\n# the order, on one node");
    println!(
        "  {report} · the prune landed, the offset did not, and the radius nudge is {nudged:.6}"
    );
    for problem in &report.problems {
        println!("  {problem}");
    }
    println!(
        "  the control is the batch's own list order: the offset is written first and the prune second, and the order of the tree wins"
    );
    report.applied == 1 && pruned_problems == 1 && nudged == 0.0
}

/// A handle the plant does not have is named rather than ignored, and it changes nothing.
fn a_stale_handle_is_named() -> bool {
    let mut species = species();
    let stale = bough(9);
    let before = wood(&species.plant, Detail::DEFAULT)
        .iter()
        .map(|solid| signed_volume(solid.loft.view()))
        .fold(0.0f32, f32::max);
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            stale.clone(),
            Change::Offset {
                property: Property::Radius,
                by: 0.01,
            },
        )],
    );
    let after = wood(&species.plant, Detail::DEFAULT)
        .iter()
        .map(|solid| signed_volume(solid.loft.view()))
        .fold(0.0f32, f32::max);
    let named = report
        .problems
        .iter()
        .any(|problem| matches!(problem, Problem::Orphaned { at } if at == &stale));
    println!("\n# a handle that names nothing");
    println!("  {report} · the largest solid's volume is {before:.6} → {after:.6}");
    for problem in &report.problems {
        println!("  {problem}");
    }
    println!(
        "  the control is the unchanged volume: a report that named the handle and edited something anyway would read as a move"
    );
    report.applied == 0 && named && (before - after).abs() < 1e-9
}

/// A number outside the range its property has is refused with the value it would have had.
fn a_value_out_of_range_is_refused() -> bool {
    let mut species = species();
    let report = apply(
        &mut species.plant,
        &[
            Edit::new(
                bough(2),
                Change::Offset {
                    property: Property::Station,
                    by: 0.9,
                },
            ),
            Edit::new(
                branch(0, 0),
                Change::Offset {
                    property: Property::Length,
                    by: -100.0,
                },
            ),
        ],
    );
    let station = species
        .plant
        .get(&bough(2))
        .expect("the bough resolves")
        .effective_attachment()
        .station;
    let length = species
        .plant
        .spine(&branch(0, 0))
        .expect("the branch resolves")
        .length();
    let refused = report
        .problems
        .iter()
        .filter(|problem| {
            matches!(
                problem,
                Problem::OutOfRange { .. } | Problem::NotPositive { .. }
            )
        })
        .count();
    println!("\n# a number outside its range");
    println!(
        "  {report} · the bough's station is still {station:.6} and the branch is still {length:.6} m long"
    );
    for problem in &report.problems {
        println!("  {problem}");
    }
    println!(
        "  the control is the refusal: a clamp would have left the station at 1.0 and the length at a positive floor"
    );
    report.applied == 0 && refused == 2 && (station - 0.81).abs() < 1e-6 && length > 0.0
}

/// A bake replaces the node's spine, and everything that hangs from it follows the new skin.
fn a_bake_replaces_the_spine() -> bool {
    let mut species = species();
    let authored = species
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    // A hand-written S in the bough's own frame: out, up and out again, with a shape the authored curve has
    // nowhere. These are the node's own coordinates, which is the space a bake is written in.
    let jets = vec![
        Jet::new([0.0, 0.0, 0.0], [0.30, 0.40, 0.0]),
        Jet::new([0.25, 0.30, 0.0], [0.10, 0.35, 0.0]),
        Jet::new([0.40, 0.62, 0.0], [0.05, 0.10, 0.0]),
    ];
    let report = apply(
        &mut species.plant,
        &[Edit::new(bough(0), Change::Spine { jets })],
    );
    let baked = species
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    let marked = species
        .plant
        .get(&bough(0))
        .expect("the bough resolves")
        .baked();
    let mut worst_skin = 0.0f32;
    for index in 0..2 {
        let handle = branch(0, index as u32);
        let node = species.plant.get(&handle).expect("the branch resolves");
        let attachment = node.effective_attachment();
        let at = attachment.station.clamp(0.0, 1.0);
        let skin = species
            .plant
            .get(&bough(0))
            .expect("the bough resolves")
            .taper_now()
            .at(at)
            - attachment.sink;
        let origin = species
            .plant
            .placement(&handle)
            .expect("the branch resolves")
            .origin;
        let spine = species.plant.spine(&bough(0)).expect("the bough resolves");
        worst_skin = worst_skin.max((span(origin, spine.at(at)) - skin).abs());
    }
    println!("\n# a bake");
    println!(
        "  {report} · the bough's drawn length went {authored:.6} → {baked:.6}, and the node is marked baked: {marked}"
    );
    println!(
        "  both branches still hang on the new skin to {worst_skin:.3e}, so a bake moves what hangs from it"
    );
    println!(
        "  the control is the length: a bake that was ignored would read the authored length back"
    );
    report.clean() && marked && (baked - authored).abs() > 0.05 && worst_skin < 1e-5
}

/// A spine `shape` refuses changes nothing and is reported with the reason.
fn a_bake_shape_refuses_is_not_applied() -> bool {
    let mut species = species();
    let before = species
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    // Two knots at the same place: a segment with no direction, which `shape` refuses by name.
    let jets = vec![
        Jet::new([0.1, 0.2, 0.0], [0.3, 0.0, 0.0]),
        Jet::new([0.1, 0.2, 0.0], [0.0, 0.3, 0.0]),
    ];
    let report = apply(
        &mut species.plant,
        &[Edit::new(bough(0), Change::Spine { jets })],
    );
    let after = species
        .plant
        .spine(&bough(0))
        .expect("the bough resolves")
        .length();
    let marked = species
        .plant
        .get(&bough(0))
        .expect("the bough resolves")
        .baked();
    let refused = matches!(report.problems.as_slice(), [Problem::SpineRefused { .. }]);
    println!("\n# a bake that shape refuses");
    println!(
        "  {report} · the length is still {after:.6} (it was {before:.6}) and the node is marked baked: {marked}"
    );
    for problem in &report.problems {
        println!("  {problem}");
    }
    println!(
        "  the control is the marker: a refusal that had set it would let a later field think the spine was authored"
    );
    report.applied == 0 && refused && (after - before).abs() < 1e-6 && !marked
}

/// The brush is a predicate that writes the edit table: one edit per handle it names, and the same table.
fn a_brush_writes_the_edit_table() -> bool {
    let mut species = species();
    let selector = Selector::Generation(2);
    let named = selector.handles(&species.plant);
    let edits = paint(
        &species.plant,
        &selector,
        Change::Offset {
            property: Property::Radius,
            by: 0.001,
        },
    );
    let report = apply(&mut species.plant, &edits);
    let nudged = named
        .iter()
        .filter(|handle| {
            species
                .plant
                .get(handle)
                .map(|node| node.nudges().radius == 0.001)
                .unwrap_or(false)
        })
        .count();
    let above = (0..3).all(|index| {
        species
            .plant
            .get(&bough(index))
            .map(|node| node.nudges().radius == 0.0)
            .unwrap_or(false)
    });
    let trunk = species
        .plant
        .get(&Handle::root())
        .map(|node| node.nudges().radius == 0.0)
        .unwrap_or(false);
    println!("\n# the brush");
    println!(
        "  Generation(2) names {} handles and the brush writes {} edits, one each, in walk order",
        named.len(),
        edits.len()
    );
    println!(
        "  {report} · {nudged} of them carry the nudge, and the boughs and the trunk are untouched: {}",
        above && trunk
    );
    println!(
        "  the control is the untouched nodes: a brush that ignored its selector would have nudged the trunk too"
    );
    edits.len() == named.len()
        && report.applied == named.len()
        && nudged == named.len()
        && above
        && trunk
}

/// The law the curve rows add: four millimetres at the base, nothing at the tip.
fn thickening() -> Scalar {
    Scalar::new(&[(0.0, 0.004), (1.0, 0.0)]).expect("two knots are a law")
}

/// A node's base and tip radius, as it is drawn.
fn radii(plant: &whiting_vegetation::plant::Plant, handle: &Handle) -> (f32, f32) {
    let taper = plant.get(handle).expect("the node resolves").taper_now();
    (taper.at(0.0), taper.at(1.0))
}

/// A curve is an offset whose number is a law: added at every station, not replacing the profile.
fn a_curve_is_a_law_added() -> bool {
    let mut species = species();
    let target = Handle::root().child(0);
    let (base, tip) = radii(&species.plant, &target);
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            target.clone(),
            Change::Curve {
                curve: thickening(),
            },
        )],
    );
    let (after_base, after_tip) = radii(&species.plant, &target);
    println!("\n# a curve");
    println!(
        "  {report} · the bough's base went {base:.6} → {after_base:.6} and its tip {tip:.6} → {after_tip:.6}"
    );
    println!(
        "  the control is the law's own shape: a curve added everywhere would move the tip too, and the tip's knot is zero"
    );
    report.clean() && (after_base - base - 0.004).abs() < 1e-6 && (after_tip - tip).abs() < 1e-6
}

/// Two curves on one node add, which is what makes the verb composable rather than last-wins.
fn two_curves_add_up() -> bool {
    let mut species = species();
    let target = Handle::root().child(0);
    let (base, _) = radii(&species.plant, &target);
    let half = Scalar::new(&[(0.0, 0.002), (1.0, 0.0)]).expect("two knots are a law");
    apply(
        &mut species.plant,
        &[
            Edit::new(
                target.clone(),
                Change::Curve {
                    curve: half.clone(),
                },
            ),
            Edit::new(target.clone(), Change::Curve { curve: half }),
        ],
    );
    let (after, _) = radii(&species.plant, &target);
    println!("\n# two curves");
    println!(
        "  two laws of two millimetres add on one node: the base went {base:.6} → {after:.6}, which is {:+.6}",
        after - base
    );
    println!("  the control is the sum: a last-wins verb would read +0.002 here");
    (after - base - 0.004).abs() < 1e-6
}

/// The neutral element: a law of zero leaves the profile exactly as it was.
fn a_curve_of_zero_changes_nothing() -> bool {
    let mut species = species();
    let target = Handle::root().child(1).child(0);
    let before = species
        .plant
        .get(&target)
        .expect("the node resolves")
        .taper_now();
    apply(
        &mut species.plant,
        &[Edit::new(
            target.clone(),
            Change::Curve {
                curve: Scalar::constant(0.0),
            },
        )],
    );
    let after = species
        .plant
        .get(&target)
        .expect("the node resolves")
        .taper_now();
    let mut worst = 0.0f32;
    for step in 0..=32 {
        let t = step as f32 / 32.0;
        worst = worst.max((before.at(t) - after.at(t)).abs());
    }
    println!("\n# a curve of zero");
    println!("  the taper moved by {worst:.3e} at its worst station");
    println!(
        "  the control is the neutral element: a verb that could not be cancelled would be a verb that always writes"
    );
    worst < 1e-6
}

/// A law that would thin the branch past nothing is refused, and the profile is left alone.
fn a_law_that_thins_to_nothing_is_refused() -> bool {
    let mut species = species();
    let target = Handle::root().child(1).child(0);
    let (base, tip) = radii(&species.plant, &target);
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            target.clone(),
            Change::Curve {
                curve: Scalar::constant(-1.0),
            },
        )],
    );
    let (after_base, after_tip) = radii(&species.plant, &target);
    let refused = matches!(report.problems.first(), Some(Problem::NotPositive { .. }));
    println!("\n# a law that thins to nothing");
    println!(
        "  {report} · the branch reads {base:.6}/{tip:.6} before and {after_base:.6}/{after_tip:.6} after"
    );
    println!(
        "  the control is the refusal: a swept section of negative radius would draw the solid turned inside out"
    );
    refused && report.applied == 0 && after_base == base && after_tip == tip
}
