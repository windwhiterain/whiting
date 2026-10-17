//! Measures the generator: what a rule decides, what the seed decides, and whether a grown plant is a plant.
//!
//! The two species are the experiment. **Bamboo's rule leaves nothing to chance**, so its seed cannot change the
//! tree — that is the row that says the generator reproduces what an author would write. **The shrub's rule
//! leaves the counts, the stations and every limb's length to the seed**, so its two seeds are two shrubs — the
//! row that says the generator is a generator. Between them: the rule's numbers are readable off the tree, a limb
//! is a fraction of what carries it, the result expands into closed solids, and the hand edits it.
//!
//! ```text
//! cargo run -p whiting-vegetation --example species_probe
//! ```

use shape::mesh::{boundary_loops, euler_characteristic, signed_volume};
use whiting_vegetation::derive::{Species, bamboo, shrub};
use whiting_vegetation::expand::{Detail, expand};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::ops::{Change, Edit, Property, apply};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push((
        "rule, not chance",
        the_bamboos_rule_leaves_nothing_to_chance(),
    ));
    checks.push(("the rule's numbers", the_rules_numbers_are_the_trees()));
    checks.push((
        "the seed decides",
        the_seed_decides_what_the_rule_left_open(),
    ));
    checks.push((
        "a joint carries the law",
        a_limb_is_a_fraction_of_what_carries_it(),
    ));
    checks.push((
        "a grown plant is a plant",
        a_grown_plant_expands_into_closed_solids(),
    ));
    checks.push((
        "the hand edits a grown plant",
        the_hand_edits_a_grown_plant(),
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

/// Every live node's handle, spine length and attachment, in walk order.
fn skeleton(species: &Species) -> Vec<(String, f32, f32, f32)> {
    species
        .plant
        .live()
        .map(|(handle, node)| {
            let length = species
                .plant
                .spine(&handle)
                .expect("a live node's spine resolves")
                .length();
            (
                handle.to_string(),
                length,
                node.attachment().station,
                node.attachment().slot,
            )
        })
        .collect()
}

/// A plant's leaves, as the stations and slots a field put them at.
fn leaf_places(species: &Species) -> Vec<(f32, f32)> {
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    expansion
        .leaves
        .iter()
        .flat_map(|group| {
            group
                .leaves
                .iter()
                .map(|leaf| (leaf.station, leaf.slot))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Bamboo's rule has no jitter and no drawing shape, so two seeds are the same tree with two leaf layouts.
fn the_bamboos_rule_leaves_nothing_to_chance() -> bool {
    let one = bamboo(20_261_017);
    let two = bamboo(9_919_999);
    let first = skeleton(&one);
    let second = skeleton(&two);
    let same_tree = first == second;
    let leaves_one = leaf_places(&one);
    let leaves_two = leaf_places(&two);
    let leaves_differ = leaves_one.len() == leaves_two.len()
        && leaves_one
            .iter()
            .zip(leaves_two.iter())
            .any(|(left, right)| left != right);
    println!("\n# a rule without chance");
    println!(
        "  {} nodes at both seeds · the skeleton is identical: {same_tree} · the leaves differ: {leaves_differ}",
        first.len()
    );
    println!(
        "  so the seed reaches the leaf field and nothing else: {} leaves over {} nodes",
        leaves_one.len(),
        first.len()
    );
    println!(
        "  the control is the leaves: a seed that changed nothing would make this a claim about a constant"
    );
    same_tree && leaves_differ && leaves_one.len() > 0
}

/// The rule's bands and steps are readable off the tree: the boughs at the band's ends and between, the slots a
/// step apart.
fn the_rules_numbers_are_the_trees() -> bool {
    let species = bamboo(20_261_017);
    let boughs: Vec<(f32, f32)> = (0..3)
        .map(|index| {
            let node = species
                .plant
                .get(&Handle::root().child(index))
                .expect("the bough resolves");
            (node.attachment().station, node.attachment().slot)
        })
        .collect();
    let branches: Vec<f32> = (0..2)
        .map(|index| {
            species
                .plant
                .get(&Handle::root().child(0).child(index))
                .expect("the branch resolves")
                .attachment()
                .station
        })
        .collect();
    let stations: Vec<f32> = boughs.iter().map(|(station, _)| *station).collect();
    let slots: Vec<f32> = boughs.iter().map(|(_, slot)| *slot).collect();
    let ends = (stations[0] - 0.45).abs() < 1e-6 && (stations[2] - 0.81).abs() < 1e-6;
    let even = (stations[1] - 0.63).abs() < 1e-6;
    let stepping = (slots[1] - 0.333).abs() < 1e-6 && (slots[2] - 0.666).abs() < 1e-6;
    let branch_band = (branches[0] - 0.35).abs() < 1e-6 && (branches[1] - 0.60).abs() < 1e-6;
    println!("\n# the rule's numbers");
    println!("  bough stations {stations:?} · slots {slots:?}");
    println!("  branch stations {branches:?}");
    println!(
        "  the band's ends, the even spread between them, the slot step and the branch band: {ends} {even} {stepping} {branch_band}"
    );
    println!(
        "  the control is the middle bough: a distribution that clumped its children would miss 0.63"
    );
    ends && even && stepping && branch_band
}

/// The shrub's rule leaves counts, stations and lengths to the seed, so two seeds are two shrubs.
fn the_seed_decides_what_the_rule_left_open() -> bool {
    let first = shrub(1);
    let again = shrub(1);
    let second = shrub(2);
    let one = skeleton(&first);
    let repeat = skeleton(&again);
    let other = skeleton(&second);
    let deterministic = one == repeat;
    let differs = one.len() != other.len() || one.iter().zip(other.iter()).any(|(a, b)| a.1 != b.1);
    // The rule's own range: three generations of `3..=4`, `3..=3` and `2..=3`.
    let least = 1 + 3 + 3 * 3 + 3 * 3 * 2;
    let most = 1 + 4 + 4 * 3 + 4 * 3 * 3;
    let inside = (least..=most).contains(&one.len()) && (least..=most).contains(&other.len());
    println!("\n# a rule with chance");
    println!(
        "  shrubs of {} and {} nodes · the same seed twice is identical: {deterministic} · two seeds differ: {differs}",
        one.len(),
        other.len()
    );
    println!("  the rule's own range is {least}..={most}, and both are inside it: {inside}");
    println!(
        "  the control is the repeat: a generator that drew from the clock would fail the first reading"
    );
    deterministic && differs && inside
}

/// A limb's radius at its base is the fraction of the parent's radius the rule names.
fn a_limb_is_a_fraction_of_what_carries_it() -> bool {
    let species = shrub(7);
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let mut worst = 0.0f32;
    let mut measured = 0;
    for solid in &expansion.wood {
        if solid.handle.is_root() {
            continue;
        }
        let parent = solid.handle.parent().expect("a limb has a parent");
        let node = species.plant.get(&solid.handle).expect("the limb resolves");
        let at = node.effective_attachment().station.clamp(0.0, 1.0);
        let carrying = species
            .plant
            .get(&parent)
            .expect("the parent resolves")
            .taper_now()
            .at(at);
        let base = node.taper_now().at(0.0);
        // The rule's fractions, by generation: 0.55 for the first two and 0.60 for the third.
        let expected = if solid.handle.depth() >= 3 {
            0.60
        } else {
            0.55
        };
        worst = worst.max((base / carrying - expected).abs());
        measured += 1;
    }
    println!("\n# a limb and what carries it");
    println!(
        "  {measured} limbs · the worst departure of a limb's base radius from its rule's fraction: {worst:.3e}"
    );
    println!(
        "  the rule is 0.55 of the parent for the first two generations and 0.60 for the third, and the limb reads it"
    );
    println!(
        "  the control is the spread: a limb that ignored its parent would be a constant radius, whatever the generation"
    );
    measured > 0 && worst < 1e-4
}

/// A grown plant expands like an authored one: closed solids, and leaves in groups.
fn a_grown_plant_expands_into_closed_solids() -> bool {
    let species = shrub(3);
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let mut closed = 0;
    let mut worst_volume = f32::MAX;
    for solid in &expansion.wood {
        let surface = solid.loft.view();
        let volume = signed_volume(surface);
        if boundary_loops(surface) == 0 && euler_characteristic(surface) == 2 && volume > 0.0 {
            closed += 1;
        }
        worst_volume = worst_volume.min(volume);
    }
    let leaves: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    let on_limbs = expansion
        .leaves
        .iter()
        .all(|group| species.plant.get(&group.node).is_ok());
    println!("\n# a grown shrub");
    println!(
        "  {} live nodes · {} solids, {closed} of them closed at χ = 2 · the smallest volume {worst_volume:.3e}",
        species.plant.live().count(),
        expansion.wood.len()
    );
    println!(
        "  {leaves} leaves in {} groups, every group on a node that resolves: {on_limbs}",
        expansion.leaves.len()
    );
    println!(
        "  the control is the count: a grown plant with no solids or no leaves would pass a check that only asked for a report"
    );
    closed == expansion.wood.len()
        && expansion.wood.len() == species.plant.live().count()
        && leaves > 0
        && on_limbs
}

/// The hand edits a grown tree: a handle names a path the rule made, so an offset lands on it.
fn the_hand_edits_a_grown_plant() -> bool {
    let mut species = shrub(11);
    let limb = species
        .plant
        .live()
        .find(|(handle, _)| handle.depth() == 1)
        .map(|(handle, _)| handle)
        .expect("the shrub has limbs");
    let before = species
        .plant
        .spine(&limb)
        .expect("the limb resolves")
        .length();
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            limb.clone(),
            Change::Offset {
                property: Property::Length,
                by: -0.05,
            },
        )],
    );
    let after = species
        .plant
        .spine(&limb)
        .expect("the limb resolves")
        .length();
    let moved = before - after;
    println!("\n# the hand on a grown tree");
    println!("  {report} · {limb} went {before:.6} → {after:.6}, shorter by {moved:.6}");
    println!(
        "  the control is the landed edit: a grown node the hand could not name would be a tree nobody can sculpt"
    );
    report.clean() && (moved - 0.05).abs() < 1e-5
}
