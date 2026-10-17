//! Measures the brush `vegetation` P2 promises: which handles a [`Selector`] names, and exits
//! non-zero if a reading leaves its threshold.
//!
//! Every row is one threshold and one control that could have failed it, because a row that only
//! says "the number is what I expected" cannot tell a selector that works from one that answers
//! `10` for everything. The plant is the crate's own species — `derive::bamboo` — because a brush
//! is only measured against a real tree: a trunk, three boughs at authored stations, and two branches
//! on each.
//!
//! ```text
//! cargo run -p whiting-vegetation --example select_probe
//! ```
//!
//! [`Selector`]: whiting_vegetation::selector::Selector

use shape::math::Vec3;
use whiting_vegetation::derive::bamboo;
use whiting_vegetation::handle::Handle;
use whiting_vegetation::plant::Plant;
use whiting_vegetation::selector::Selector;

fn main() {
    let plant = the_plant();
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push((
        "every node, in walk order",
        every_node_in_walk_order(&plant),
    ));
    checks.push(("a generation is a level", a_generation_is_a_level(&plant)));
    checks.push((
        "a subtree is the node and below",
        a_subtree_is_the_node_and_below(&plant),
    ));
    checks.push((
        "a station band reads the authored number",
        a_station_band_reads_the_authored_number(&plant),
    ));
    checks.push((
        "a slot band reads the authored number",
        a_slot_band_reads_the_authored_number(&plant),
    ));
    checks.push((
        "a box names the world origin",
        a_box_names_the_world_origin(&plant),
    ));
    checks.push((
        "any and not are over the plant",
        any_and_not_are_over_the_plant(&plant),
    ));
    checks.push(("the two routes are one", the_two_routes_are_one(&plant)));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<40} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// The plant every row measures: the crate's own bamboo, at one seed.
fn the_plant() -> Plant {
    bamboo(20_261_017).plant
}

/// The bough every row that names a subtree or a box reaches for: the second child of the trunk.
fn bough() -> Handle {
    Handle::root().child(1)
}

/// A handle that resolves to nothing: the trunk's eighth child, and it has three.
fn past_the_end() -> Handle {
    Handle::root().child(7)
}

/// The handles, by name, as one line of a table.
fn names(handles: &[Handle]) -> String {
    if handles.is_empty() {
        return "none".to_string();
    }
    handles
        .iter()
        .map(Handle::to_string)
        .collect::<Vec<String>>()
        .join(", ")
}

/// Every selector the rows above use, so the last row compares the two routes on all of them.
fn every_selector(plant: &Plant) -> Vec<(&'static str, Selector)> {
    let crown = crown_box(plant);
    let away = below_the_plant(plant);
    vec![
        ("Every", Selector::Every),
        ("Generation(0)", Selector::Generation(0)),
        ("Generation(1)", Selector::Generation(1)),
        ("Generation(2)", Selector::Generation(2)),
        ("Generation(9)", Selector::Generation(9)),
        ("Descendants(bough 1)", Selector::Descendants(bough())),
        ("Descendants(trunk)", Selector::Descendants(Handle::root())),
        (
            "Descendants(past the end)",
            Selector::Descendants(past_the_end()),
        ),
        (
            "StationBand 0.5..1.0",
            Selector::StationBand { from: 0.5, to: 1.0 },
        ),
        (
            "SlotBand 0.9..1.0",
            Selector::SlotBand { from: 0.9, to: 1.0 },
        ),
        (
            "SlotBand 0.0..0.1",
            Selector::SlotBand { from: 0.0, to: 0.1 },
        ),
        (
            "SlotBand 0.3..0.5",
            Selector::SlotBand { from: 0.3, to: 0.5 },
        ),
        (
            "Inside(bough 1's crown)",
            Selector::Inside {
                corner: crown.0,
                opposite: crown.1,
            },
        ),
        (
            "Inside(a metre below)",
            Selector::Inside {
                corner: away.0,
                opposite: away.1,
            },
        ),
        (
            "Ids([bough 1, branch 0 of it])",
            Selector::Ids(vec![bough(), bough().child(0)]),
        ),
        (
            "Ids([a handle past the end])",
            Selector::Ids(vec![past_the_end()]),
        ),
        (
            "Any([Generation(1), Generation(2)])",
            Selector::Any(vec![Selector::Generation(1), Selector::Generation(2)]),
        ),
        ("Any([])", Selector::Any(Vec::new())),
        (
            "Not(Generation(2))",
            Selector::Not(Box::new(Selector::Generation(2))),
        ),
        ("Not(Every)", Selector::Not(Box::new(Selector::Every))),
    ]
}

/// Every node a plant has, in the plant's own order, and nothing else.
fn every_node_in_walk_order(plant: &Plant) -> bool {
    let named = Selector::Every.handles(plant);
    let walked: Vec<Handle> = plant.walk().map(|(handle, _)| handle).collect();
    let control = Selector::Generation(1).handles(plant);
    let same = named == walked;
    println!("\n# Every");
    println!(
        "  the walk yields {} nodes, Every names {}",
        walked.len(),
        named.len()
    );
    println!("  the same handles in the same order: {same}");
    println!("  {}", names(&named));
    println!(
        "  the control, Generation(1), names {} of them — a `handles` that ignored its selector \
         would pass this row and fail that one",
        control.len()
    );
    same && named.len() == 10 && control.len() == 3
}

/// A generation is every node at one depth, and a depth no node has is an empty brush.
fn a_generation_is_a_level(plant: &Plant) -> bool {
    let trunk = Selector::Generation(0).handles(plant);
    let boughs = Selector::Generation(1).handles(plant);
    let branches = Selector::Generation(2).handles(plant);
    let past = Selector::Generation(9).handles(plant);
    let depths_read_back =
        |handles: &[Handle]| -> Vec<usize> { handles.iter().map(Handle::depth).collect() };
    let at_its_depth = depths_read_back(&boughs).iter().all(|depth| *depth == 1)
        && depths_read_back(&branches).iter().all(|depth| *depth == 2)
        && depths_read_back(&trunk) == vec![0];
    let covers_the_plant = trunk.len() + boughs.len() + branches.len() == plant.walk().count();
    println!("\n# Generation");
    println!(
        "  0: {}  ·  1: {}  ·  2: {}",
        names(&trunk),
        names(&boughs),
        names(&branches)
    );
    println!("  every handle reads back at its own depth: {at_its_depth}");
    println!(
        "  the three levels cover all {} nodes: {covers_the_plant}",
        plant.walk().count()
    );
    println!(
        "  the control, Generation(9), names {} — a depth past the tree is an empty brush, not an error",
        names(&past)
    );
    at_its_depth
        && trunk.len() == 1
        && boughs.len() == 3
        && branches.len() == 6
        && past.is_empty()
        && covers_the_plant
}

/// A subtree includes the node itself, and a handle that no longer resolves names nothing.
fn a_subtree_is_the_node_and_below(plant: &Plant) -> bool {
    let base = bough();
    let subtree = Selector::Descendants(base.clone()).handles(plant);
    let whole = Selector::Descendants(Handle::root()).handles(plant);
    let orphan = Selector::Descendants(past_the_end()).handles(plant);
    let under = subtree
        .iter()
        .all(|handle| handle.path().starts_with(base.path()));
    println!("\n# Descendants");
    println!("  trunk/bough 1 and below: {}", names(&subtree));
    println!("  the trunk and below: {} nodes", whole.len());
    println!("  every name is the bough's path or a step below it: {under}");
    println!(
        "  the control, a handle past the end, names {} — a path that does not resolve is not a \
         prefix of one that does",
        names(&orphan)
    );
    under
        && subtree.len() == 3
        && subtree.contains(&base)
        && whole.len() == plant.walk().count()
        && orphan.is_empty()
}

/// A station band is the number the author wrote on the attachment, not a position the plant
/// computed, and the count is the walk's own.
fn a_station_band_reads_the_authored_number(plant: &Plant) -> bool {
    let (from, to) = (0.5f32, 1.0f32);
    let band = Selector::StationBand { from, to };
    let named = band.handles(plant);
    // The probe's own count, from the walk and the authored numbers, with no selector involved.
    let counted = plant
        .walk()
        .filter(|(_, node)| {
            let station = node.attachment().station;
            station >= from && station <= to
        })
        .count();
    let mut every_station_in_band = true;
    let mut stations = Vec::new();
    for handle in &named {
        let node = plant.get(handle).expect("a named handle resolves");
        let station = node.attachment().station;
        every_station_in_band &= station >= from && station <= to;
        stations.push(format!("{station:.2}"));
    }
    println!("\n# StationBand {from:?}..{to:?}");
    for handle in &named {
        let node = plant.get(handle).expect("a named handle resolves");
        println!(
            "  {:<20} station {:.2} · slot {:.3}",
            handle.to_string(),
            node.attachment().station,
            node.attachment().slot
        );
    }
    println!("  every named station read back inside the band: {every_station_in_band}");
    println!(
        "  {} names · the walk's own count is {counted} · stations: {}",
        named.len(),
        stations.join(" ")
    );
    println!(
        "  the trunk's station is 0.00 and so falls outside a band that starts at {from}: a band \
         reads an attachment, and the root carries the root attachment's own numbers"
    );
    named.len() == counted && every_station_in_band && named.len() == 5
}

/// A slot band is the authored slot on the attachment, and it is cut at the seam rather than joined
/// across it: `[0.9, 1.0]` and `[0.0, 0.1]` are two different brushes.
///
/// # Invariants
///
/// The thresholds are about the interval and not about this plant's slots: every band's names are
/// read back inside that band and counted against the walk, and the two seam bands must be
/// **disjoint** — which is what a wrapping implementation could not be.
fn a_slot_band_reads_the_authored_number(plant: &Plant) -> bool {
    println!("\n# SlotBand, and the seam");
    let seam = read_slot_band(plant, 0.9, 1.0);
    let past = read_slot_band(plant, 0.0, 0.1);
    let middle = read_slot_band(plant, 0.3, 0.5);
    // The control: a band that wrapped onto 0.0 would name the slot-0 nodes from both sides of 1.0,
    // and the two lists would meet.
    let disjoint = seam.0.iter().all(|handle| !past.0.contains(handle));
    println!("  the two seam bands are disjoint: {disjoint}");
    println!(
        "  this plant's slots are 0.000, 0.300, 0.333, 0.500 and 0.666, so the seam band finds \
         none ({}) while the band just past it finds the slot-0 nodes ({})",
        seam.0.len(),
        past.0.len()
    );
    println!("  the row asks about the interval, not about this plant's slots");
    let every_slot_in_band = seam.1 && past.1 && middle.1;
    let counts_agree = seam.2 && past.2 && middle.2;
    every_slot_in_band && counts_agree && disjoint
}

/// One slot band's reading: the handles it names, whether every named slot is inside it, and
/// whether the count is the walk's own — with the row printed where it is measured.
fn read_slot_band(plant: &Plant, from: f32, to: f32) -> (Vec<Handle>, bool, bool) {
    let named = Selector::SlotBand { from, to }.handles(plant);
    let counted = plant
        .walk()
        .filter(|(_, node)| {
            let slot = node.attachment().slot;
            slot >= from && slot <= to
        })
        .count();
    let mut slots = Vec::new();
    let mut in_band = true;
    for handle in &named {
        let slot = plant
            .get(handle)
            .expect("a named handle resolves")
            .attachment()
            .slot;
        in_band &= slot >= from && slot <= to;
        slots.push(format!("{slot:.3}"));
    }
    let agrees = named.len() == counted;
    println!(
        "  [{from}, {to}] names {} ({}) at slots [{}] — the walk's own count is {counted}",
        named.len(),
        names(&named),
        slots.join(" ")
    );
    (named, in_band, agrees)
}

/// A box reads the world origin, normalises its two corners, and a box a metre away names nothing.
fn a_box_names_the_world_origin(plant: &Plant) -> bool {
    let (corner, opposite) = crown_box(plant);
    let named = Selector::Inside { corner, opposite }.handles(plant);
    let inside = |point: Vec3| {
        (0..3).all(|axis| {
            let low = corner[axis].min(opposite[axis]);
            let high = corner[axis].max(opposite[axis]);
            point[axis] >= low && point[axis] <= high
        })
    };
    let origins_inside = named.iter().all(|handle| {
        inside(
            plant
                .placement(handle)
                .expect("a named handle places")
                .origin,
        )
    });
    let named_the_bough = named.contains(&bough());
    let reversed = Selector::Inside {
        corner: opposite,
        opposite: corner,
    }
    .handles(plant);
    let (away_corner, away_opposite) = below_the_plant(plant);
    let away = Selector::Inside {
        corner: away_corner,
        opposite: away_opposite,
    }
    .handles(plant);
    let lowest = lowest_origin(plant);
    let away_top = away_corner[1].max(away_opposite[1]);
    let clearance = lowest[1] - away_top;
    println!("\n# Inside");
    println!("  the box around trunk/bough 1's crown: {corner:?} .. {opposite:?}");
    println!(
        "  {} of {} nodes: {}",
        named.len(),
        plant.walk().count(),
        names(&named)
    );
    println!("  every named origin read back inside the box: {origins_inside}");
    println!(
        "  the corners given the other way round: the same {} names",
        reversed.len()
    );
    println!(
        "  the control, its top face at y = {away_top:.2}, names {}; the plant's lowest origin is \
         at y = {:.3}, a clearance of {clearance:.3} m",
        names(&away),
        lowest[1]
    );
    origins_inside
        && named_the_bough
        && !named.is_empty()
        && named.len() < plant.walk().count()
        && reversed == named
        && away.is_empty()
}

/// The complement is over the plant's nodes, so `Not(Every)` is empty and an empty `Any` joins to
/// nothing.
fn any_and_not_are_over_the_plant(plant: &Plant) -> bool {
    let union =
        Selector::Any(vec![Selector::Generation(1), Selector::Generation(2)]).handles(plant);
    let boughs = Selector::Generation(1).handles(plant);
    let branches = Selector::Generation(2).handles(plant);
    let above = Selector::Not(Box::new(Selector::Generation(2))).handles(plant);
    let nothing = Selector::Not(Box::new(Selector::Every)).handles(plant);
    let empty_join = Selector::Any(Vec::new()).handles(plant);
    let trunk = Selector::Generation(0).handles(plant);
    let is_the_join = union.len() == boughs.len() + branches.len()
        && union
            .iter()
            .all(|handle| boughs.contains(handle) || branches.contains(handle));
    let above_is_the_rest = above.len() == trunk.len() + boughs.len()
        && above.iter().all(|handle| !branches.contains(handle));
    println!("\n# Any and Not");
    println!(
        "  Any([Generation(1), Generation(2)]) names {}: {}",
        union.len(),
        names(&union)
    );
    println!(
        "  Not(Generation(2)) names {}: {}",
        above.len(),
        names(&above)
    );
    println!(
        "  Not(Every) names {} and Any([]) names {}",
        names(&nothing),
        names(&empty_join)
    );
    println!("  the join is the two levels with no overlap: {is_the_join}");
    println!("  the complement is everything above the branches: {above_is_the_rest}");
    union.len() == 9
        && above.len() == 4
        && nothing.is_empty()
        && empty_join.is_empty()
        && is_the_join
        && above_is_the_rest
}

/// `handles` is `walk().filter(matches)` on every selector above, twice, in walk order and without
/// repeats: one row, because a selector that answers `matches` and `handles` differently is two
/// tools.
fn the_two_routes_are_one(plant: &Plant) -> bool {
    let walked: Vec<Handle> = plant.walk().map(|(handle, _)| handle).collect();
    let mut agree = true;
    println!("\n# the two routes");
    println!(
        "  {:<34} {:>4} {:>6} {:>6}",
        "selector", "named", "again", "route"
    );
    for (label, selector) in every_selector(plant) {
        let named = selector.handles(plant);
        let again = selector.handles(plant);
        let filtered: Vec<Handle> = plant
            .walk()
            .filter(|(handle, node)| selector.matches(plant, handle, node))
            .map(|(handle, _)| handle)
            .collect();
        agree &= named == again && named == filtered;
        // Walk order and no repeats are the walk's own properties, read back independently of the
        // order `handles` produced.
        let mut rank = 0usize;
        let mut in_walk_order = true;
        let mut unique = true;
        for (index, handle) in named.iter().enumerate() {
            unique &= !named[..index].contains(handle);
            match walked.iter().position(|other| other == handle) {
                Some(found) if found >= rank => rank = found + 1,
                _ => in_walk_order = false,
            }
        }
        agree &= in_walk_order && unique && named.iter().all(|handle| walked.contains(handle));
        println!(
            "  {label:<34} {:>4} {:>6} {:>6}",
            named.len(),
            if named == again { "same" } else { "DIFFER" },
            if named == filtered { "same" } else { "DIFFER" }
        );
    }
    println!("  every selector: two calls agree, and `handles` is the walk filtered by `matches`");
    println!(
        "  every list is a subsequence of the walk, so it is in walk order and has no repeats"
    );
    // A stale handle is the one place the two routes can read differently, and both readings are
    // the contract: `matches` asks whether the list contains the handle, and the walk cannot reach
    // it. The call below is deliberately mismatched — the handle and the node do not travel
    // together — because there is no node for a stale handle, and a membership test is the only
    // thing left for it to answer.
    let stale = Selector::Ids(vec![past_the_end()]);
    let asked = stale.matches(plant, &past_the_end(), plant.root());
    let named_by_handles = stale.handles(plant);
    let subtracts_nothing = Selector::Not(Box::new(stale)).handles(plant);
    println!(
        "  a stale handle: `matches` on it answers {asked} because it is in the list, and \
         `handles` cannot answer for it at all"
    );
    println!(
        "  so Not(that list) subtracts nothing and names {} — Not is over the plant's nodes",
        subtracts_nothing.len()
    );
    agree && asked && named_by_handles.is_empty() && subtracts_nothing.len() == walked.len()
}

/// A box around one bough's crown: half a metre across, centred on the bough's own world origin.
fn crown_box(plant: &Plant) -> (Vec3, Vec3) {
    let origin = plant
        .placement(&bough())
        .expect("a walked node places")
        .origin;
    let half = 0.35f32;
    (
        [origin[0] - half, origin[1] - half, origin[2] - half],
        [origin[0] + half, origin[1] + half, origin[2] + half],
    )
}

/// The same box, whose **top face** is a metre below the plant's lowest origin: nothing in a plant
/// hangs below where it starts, so the gap is a metre rather than a few centimetres.
fn below_the_plant(plant: &Plant) -> (Vec3, Vec3) {
    let (corner, opposite) = crown_box(plant);
    let top = lowest_origin(plant)[1] - 1.0;
    let bottom = top + corner[1] - opposite[1];
    (
        [corner[0], bottom, corner[2]],
        [opposite[0], top, opposite[2]],
    )
}

/// The lowest of the plant's ten origins, axis by axis.
fn lowest_origin(plant: &Plant) -> Vec3 {
    plant
        .walk()
        .map(|(handle, _)| {
            plant
                .placement(&handle)
                .expect("a walked node places")
                .origin
        })
        .fold([f32::MAX; 3], |lowest, origin| {
            [
                lowest[0].min(origin[0]),
                lowest[1].min(origin[1]),
                lowest[2].min(origin[2]),
            ]
        })
}
