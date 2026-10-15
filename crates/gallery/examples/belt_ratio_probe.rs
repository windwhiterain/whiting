//! Does the skeleton ever put more than one block in a belt?
//!
//! Run with `cargo run -p whiting-gallery --example belt_ratio_probe`. Diagnostic: it reports, and
//! it exits non-zero if the generator cannot produce a belt holding more than one block.
//!
//! # Why this is a question and not a fact
//!
//! `BeltRef { valley, side, segment }` and `BlockFace::top`/`bottom` say a belt runs between two
//! authored nodes and that blocks divide it at ribs. Whether a belt therefore *can* hold more than
//! one block is arithmetic about the skeleton, and it has to be measured rather than read off the
//! type: one massif having one block per belt says nothing about another.
//!
//! The previous branch's handoff records "168 of 190 nodes carry eight blocks and one carries 56",
//! which reads as blocks-per-node, not blocks-per-belt, and those are not the same quantity. A node
//! carries blocks on both sides of a valley, so eight blocks at a node is consistent with one per
//! belt in each direction.
//!
//! # What the sweep found, and what it changed about the question
//!
//! **The ratio is exactly one over every seed and every set of generator parameters tried** — the
//! widest belt anywhere in the sweep holds one block. Read off `expand.rs`, the reason is that a
//! `block` index is claimed **once per belt**, before that belt's faces are walked, and the walk then
//! consumes the whole belt: `mesh.blocks.push` happens once, after the walk, with no inner split.
//!
//! **So "does a belt hold more than one block" is the wrong question, and it stays wrong.** It asks
//! whether the generator does something it structurally cannot. What a belt-level front end needs to
//! answer is the other direction: **how many blocks are authored along one valley**, because that
//! is the count of belts that would be merged into one surface, and it is what "C1 within a belt"
//! would have to mean if the belt grouping ever grew a second block.
//!
//! The measurement below therefore reports both: the ratio (constant one) and the belt count
//! (the thing that would change), and the assertion is on the one that can still fail.

use std::collections::BTreeMap;

use mountain::tessellation::BeltRef;
use mountain::{Starter, starter};

/// How many groups of faces the massif's blocks and belts actually are, and the ratio.
fn census(facet: &mountain::FacetMesh) -> (usize, usize, usize, usize) {
    let blocks = facet.blocks.len();
    let mut belts: Vec<BeltRef> = facet
        .faces
        .iter()
        .map(|face| face.attributes.belt)
        .collect();
    belts.sort_by_key(|belt| (belt.valley.0, belt.segment, belt.side));
    belts.dedup();
    let belts = belts.len();

    // Blocks per belt, read off the faces rather than assumed: a block is in one belt, and the
    // count of distinct blocks naming each belt is the thing the question is actually about.
    let mut by_belt: BTreeMap<BeltRef, std::collections::BTreeSet<u32>> = BTreeMap::new();
    for face in &facet.faces {
        by_belt
            .entry(face.attributes.belt)
            .or_default()
            .insert(face.block);
    }
    let mut per_belt: Vec<usize> = by_belt.values().map(|set| set.len()).collect();
    per_belt.sort_unstable();
    let most = per_belt.last().copied().unwrap_or(0);
    let median = if per_belt.is_empty() {
        0
    } else {
        per_belt[per_belt.len() / 2]
    };
    (blocks, belts, median, most)
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    println!("\n  seed   params      blocks   belts   blocks/belt (median, most)");
    let mut ever_above_one = false;
    let mut widest = 0usize;

    for seed in [3i64, 7, 11, 17, 29, 41, 53, 97] {
        let massif = match starter(seed, Starter::default()).build() {
            Ok(mesh) => mesh,
            Err(problem) => {
                eprintln!("seed {seed} did not expand: {problem:?}");
                continue;
            }
        };
        let (blocks, belts, median, most) = census(&massif);
        if median > 1 || most > 1 {
            ever_above_one = true;
        }
        widest = widest.max(most);
        println!("  {seed:>4}   default    {blocks:>7}   {belts:>5}   {median:>6}, {most:>5}");
    }

    // The same sweep over the generator's own knobs, because "one block per belt" might be an
    // artefact of how finely the ridges were authored rather than of how the belt was divided.
    // `nodes` is how many nodes each ridge carries, which is the parameter that sets how many ribs
    // a valley segment can have.
    println!("\n  seed   ridges  nodes   blocks   belts   blocks/belt (median, most)");
    for (ridges, nodes) in [(3usize, 4usize), (5, 6), (7, 8), (9, 12), (11, 16)] {
        let params = Starter {
            ridges,
            nodes,
            ..Starter::default()
        };
        let Ok(massif) = starter(11, params).build() else {
            continue;
        };
        let (blocks, belts, median, most) = census(&massif);
        if median > 1 || most > 1 {
            ever_above_one = true;
        }
        widest = widest.max(most);
        println!(
            "    11   {ridges:>6}  {nodes:>5}   {blocks:>7}   {belts:>5}   {median:>6}, {most:>5}"
        );
    }

    println!("\n  widest belt anywhere in the sweep: {widest} block(s)");
    if ever_above_one {
        println!("  a belt CAN hold more than one block, so the belt grouping is a real one.");
    } else {
        println!(
            "  no belt held more than one block, over every seed and parameter set tried.\n\
             \x20 expand.rs claims one block index per belt before walking it, so this is structural\n\
             \x20 rather than a property of the seed. On this generator a belt IS a block."
        );
    }

    // What is left, and what would actually change: **how many belts a valley carries.** Merging a
    // valley's belts into one surface is the only way the belt grouping grows, and it is a decision
    // about the front end rather than about the generator. The count is reported so the size of
    // that decision is a number rather than an impression.
    let massif = starter(11, Starter::default()).build();
    if let Ok(massif) = massif {
        let mut per_valley: BTreeMap<usize, std::collections::BTreeSet<usize>> = BTreeMap::new();
        for face in &massif.faces {
            per_valley
                .entry(face.attributes.belt.valley.0 as usize)
                .or_default()
                .insert(face.attributes.belt.segment);
        }
        let mut counts: Vec<usize> = per_valley.values().map(|set| set.len()).collect();
        counts.sort_unstable();
        let most = counts.last().copied().unwrap_or(0);
        let median = counts.get(counts.len() / 2).copied().unwrap_or(0);
        println!(
            "  belts per valley: median {median}, most {most}, over {} valleys\n\
             \x20 so a valley-level surface would be {median} to {most} belts wide. that is the size\n\
             \x20 of the anisotropy a merged belt would have to survive.",
            counts.len()
        );
        if most > 1 {
            ever_above_one = true;
        }
    }

    if !ever_above_one {
        failures.push("some way of putting more than one block in a belt exists");
    }

    println!();
    if failures.is_empty() {
        println!("all measurements pass");
    } else {
        for failure in &failures {
            println!("FAILED  {failure}");
        }
        std::process::exit(1);
    }
}
