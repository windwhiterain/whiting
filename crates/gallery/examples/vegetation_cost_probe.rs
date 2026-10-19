//! Where a vegetation plate's build time actually goes.
//!
//! The question: `alpine_oak`'s document build is measured at ~47 s in a debug build, which is
//! absurd for one tree, and the recorded explanation
//! (`vegetation-scene-build-cost.md`) cites lines that have since moved and a mechanism the hot
//! paths no longer have — `expand`, `leaf` and `wood` all share one `Derived` table now. So the
//! decomposition has to be measured again rather than repeated.
//!
//! This probe answers it in five phases of one plate: the species' own construction (rule and
//! seed), the plant's wood, its leaf fields, the whole expansion, and the whole picture. Each is
//! timed with the shape it produced beside it, so a phase that is slow has a count to blame.
//!
//! It prints and asserts nothing: `cargo run -p whiting-gallery --example vegetation_cost_probe`,
//! and again with `--release`, because the debug constant is not the one an author pays.

use std::time::Instant;

use vegetation::expand::{Detail, expand, wood};
use vegetation::leaf::{LeafField, LeafGroup};

/// The seed the whole-tree plate is built at.
const SEED: u64 = 34;

/// The whole-tree plate's dial, as `southwest_oak`'s own constant has it.
const DETAIL: Detail = Detail {
    stations: 8,
    around: 24,
    spacing: 0.45,
};

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

fn groups(fields: &[LeafField], plant: &vegetation::plant::Plant) -> Vec<LeafGroup> {
    fields
        .iter()
        .flat_map(|field| field.groups(plant))
        .collect()
}

fn main() {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("alpine_oak, {profile} build");

    let phase = Instant::now();
    let species = whiting_gallery::southwest_oak::alpine_oak(SEED);
    let species_ms = ms(phase);
    let nodes = species.plant.live().count();
    println!(
        "  species     {species_ms:9.1} ms   {nodes} live nodes, {} blade templates, {} leaf fields, {} decorators",
        species.templates.len(),
        species.fields.len(),
        species.decorators.len(),
    );

    let phase = Instant::now();
    let solids = wood(&species.plant, DETAIL);
    let wood_ms = ms(phase);
    let rings: usize = solids.iter().map(|one| one.loft.positions().len()).sum();
    println!(
        "  wood        {wood_ms:9.1} ms   {} solids, {rings} vertices",
        solids.len()
    );

    let phase = Instant::now();
    let placed = groups(&species.fields, &species.plant);
    let leaves_ms = ms(phase);
    let blades: usize = placed.iter().map(|group| group.leaves.len()).sum();
    println!(
        "  leaves      {leaves_ms:9.1} ms   {} groups, {blades} blades",
        placed.len()
    );

    let phase = Instant::now();
    let expansion = expand(&species.plant, DETAIL, &species.fields);
    let expand_ms = ms(phase);
    let expanded_blades: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    println!(
        "  expand      {expand_ms:9.1} ms   {} solids, {} groups, {expanded_blades} blades",
        expansion.wood.len(),
        expansion.leaves.len(),
    );

    let phase = Instant::now();
    let picture = whiting_gallery::southwest_oak::alpine_oak_picture();
    let picture_ms = ms(phase);
    println!(
        "  picture     {picture_ms:9.1} ms   {} top-level fields",
        picture.as_struct().map_or(0, |fields| fields.len())
    );

    // The phases above are one run each, and the picture re-derives the species and the
    // expansion, so this is the number that bounds the whole build rather than their sum.
    println!(
        "  ---- species + wood + leaves + expand = {:.1} ms; picture alone = {picture_ms:.1} ms",
        species_ms + wood_ms + leaves_ms + expand_ms
    );
}
