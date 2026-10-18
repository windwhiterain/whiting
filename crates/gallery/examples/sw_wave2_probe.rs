//! How many instance rows each second-wave plate costs, measured before the first render.
//!
//! The scene document pays per instance row, so the counts are read off the same expansion the picture
//! builds rather than discovered by rendering one. Run:
//!
//! ```text
//! cargo run -p whiting-gallery --example sw_wave2_probe
//! ```

use vegetation::decorator::{Seat, scatter_in};
use vegetation::derive::Species;
use vegetation::expand::Detail;
use vegetation::handle::Handle;
use vegetation::leaf::LeafField;
use vegetation::plant::Plant;
use whiting_gallery::southwest_wave2::*;

fn rows_of(name: &str, leaves: &[LeafField], seats: usize, detail: Detail, plant: &Plant) {
    let leaf_rows: usize = leaves
        .iter()
        .map(|field| {
            field
                .groups(plant)
                .iter()
                .map(|group| group.leaves.len())
                .sum::<usize>()
        })
        .sum();
    let groups: usize = leaves.iter().map(|field| field.groups(plant).len()).sum();
    let nodes = plant.live().count();
    let wood_triangles: usize = vegetation::expand::wood(plant, detail)
        .iter()
        .map(|w| w.loft.view().corners.len() / 3)
        .sum();
    let total = leaf_rows + seats;
    println!(
        "{name:<26} nodes {nodes:>6}  leaf rows {leaf_rows:>7}  leaf groups {groups:>6}  seats {seats:>6}  wood tris {wood_triangles:>9}  total {total:>7}"
    );
}

fn birch_catkins(species: &Species, depth: usize) -> Vec<Seat> {
    scatter_in(&species.plant, depth, 0, 0.96, (1, 2), (0.36, 1.0))
}

fn birch_trees() {
    let detail = Detail {
        stations: 6,
        around: 12,
        spacing: 0.5,
    };
    let base_detail = Detail {
        stations: 8,
        around: 14,
        spacing: 0.08,
    };

    let tree = southwest_birch(63, BirchFoliage::Leaf);
    rows_of(
        "southwest_birch",
        &tree.fields,
        birch_catkins(&tree, 2).len(),
        detail,
        &tree.plant,
    );

    let bough = southwest_birch_bough(63);
    rows_of(
        "southwest_birch_bough",
        &bough.fields,
        birch_catkins(&bough, 1).len(),
        detail,
        &bough.plant,
    );

    let base = southwest_birch(63, BirchFoliage::Leaf);
    rows_of(
        "southwest_birch_base",
        &base.fields,
        birch_catkins(&base, 2).len(),
        base_detail,
        &base.plant,
    );
}

fn keteleeria_seats(species: &Species) -> Vec<Seat> {
    let mut seats = scatter_in(&species.plant, 3, 0, 0.94, (1, 2), (0.12, 1.0));
    seats.extend(scatter_in(&species.plant, 1, 0, 0.90, (1, 2), (0.45, 1.0)));
    seats.extend(
        [(0.980, 0.14), (0.990, 0.61), (0.997, 0.37), (1.000, 0.85)]
            .into_iter()
            .map(|(station, slot)| Seat::new(Handle::root(), 0, station, slot)),
    );
    seats
}

fn keteleeria_trees() {
    let detail = Detail {
        stations: 6,
        around: 12,
        spacing: 0.5,
    };
    let base_detail = Detail {
        stations: 6,
        around: 16,
        spacing: 0.22,
    };

    let tree = evelyn_keteleeria(71, KeteleeriaFoliage::Plume);
    rows_of(
        "evelyn_keteleeria",
        &tree.fields,
        keteleeria_seats(&tree).len(),
        detail,
        &tree.plant,
    );

    let bough = evelyn_keteleeria_bough(71);
    rows_of(
        "evelyn_keteleeria_bough",
        &bough.fields,
        scatter_in(&bough.plant, 1, 0, 0.94, (0, 1), (0.12, 1.0)).len(),
        detail,
        &bough.plant,
    );

    let base = evelyn_keteleeria(71, KeteleeriaFoliage::Plume);
    rows_of(
        "evelyn_keteleeria_base",
        &base.fields,
        keteleeria_seats(&base).len(),
        base_detail,
        &base.plant,
    );
}

fn main() {
    println!("--- 西南桦 Betula alnoides");
    birch_trees();
    println!("\n--- 滇油杉 Keteleeria evelyniana");
    keteleeria_trees();
}
