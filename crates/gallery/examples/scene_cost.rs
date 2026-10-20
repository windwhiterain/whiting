//! What every gallery scene costs to build, in the order they cost.
//!
//! The question: `alpine_oak` is 8.5 s and the note that decomposes it calls it the slow half of the
//! gallery — but the scenes are built by name on request and no suite walks them, so nothing has
//! ever listed their builds side by side. This does.
//!
//! One build per name, wall clock, and nothing else: a scene whose build is slow is named here and
//! decomposed elsewhere. A scene that panics is reported and the sweep goes on, because a probe that
//! dies on the fourth of eighty-five names would answer nothing about the other eighty-one.
//!
//! Prints and asserts nothing: `cargo run --release -p whiting-gallery --example scene_cost [filter]`,
//! where `filter` keeps the names containing it.

use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};

use whiting_gallery::Gallery;

/// Every name [`Gallery::build`] answers, taken from its own match arms.
const NAMES: &[&str] = &[
    "bamboo",
    "bamboo_wind",
    "bamboo_crown",
    "bamboo_knots",
    "grove",
    "sculpted",
    "segmentation",
    "garden_wall",
    "aspiring",
    "thinned",
    "brushed",
    "arching",
    "panelled",
    "bands",
    "yunnan_pine",
    "yunnan_pine_bough",
    "yunnan_pine_base",
    "dove_tree",
    "dove_tree_bough",
    "dove_tree_base",
    "giant_rhododendron",
    "giant_rhododendron_bough",
    "giant_rhododendron_base",
    "site_montane",
    "site_valley",
    "site_subalpine",
    "site_transect",
    "alpine_oak",
    "alpine_oak_bough",
    "alpine_oak_base",
    "lijian_spruce",
    "lijian_spruce_bough",
    "lijian_spruce_base",
    "cangshan_fir",
    "cangshan_fir_bough",
    "cangshan_fir_base",
    "tree_fern",
    "tree_fern_frond",
    "tree_fern_base",
    "kapok",
    "kapok_flower",
    "kapok_bough",
    "kapok_base",
    "southwest_birch",
    "southwest_birch_bough",
    "southwest_birch_base",
    "evelyn_keteleeria",
    "evelyn_keteleeria_bough",
    "evelyn_keteleeria_base",
    "armand_pine",
    "armand_pine_bough",
    "armand_pine_base",
    "qiong_bamboo",
    "qiong_bamboo_culm",
    "qiong_bamboo_base",
    "dunes",
    "foldyard",
    "hermite",
    "hermite_massif",
    "compound",
    "compound_court",
    "hall",
    "hall_large",
    "house",
    "identity",
    "landscape",
    "lanterns",
    "monoliths",
    "shed",
    "street",
    "street_arcade",
    "stilt",
    "terrace",
    "wing",
    "workshop",
    "yard",
    "yikeyin",
    "yikeyin_court",
    "spotlight",
    "stilllife",
    "sunlight",
    "sweep",
    "terraces",
    "tidewater",
    "tiles",
];

fn main() {
    let filter = std::env::args().nth(1);
    let mut rows: Vec<(String, f64, usize)> = Vec::new();
    for name in NAMES {
        if let Some(filter) = &filter {
            if !name.contains(filter.as_str()) {
                continue;
            }
        }
        // Printed before the build so a scene slow enough to look like a hang is named while it runs.
        print!("  … {name}\n");
        let _ = std::io::stdout().flush();
        let phase = std::time::Instant::now();
        let built = catch_unwind(AssertUnwindSafe(|| Gallery::build(name)));
        let elapsed = phase.elapsed().as_secs_f64() * 1000.0;
        match built {
            Ok(scene) => {
                let fields = scene.as_struct().map_or(0, |fields| fields.len());
                println!("{elapsed:10.1} ms  {fields:4} fields  {name}");
                rows.push((name.to_string(), elapsed, fields));
            }
            Err(_) => println!("          … panicked       {name}"),
        }
        let _ = std::io::stdout().flush();
    }

    rows.sort_by(|one, other| {
        other
            .1
            .partial_cmp(&one.1)
            .expect("a measured wall clock is a number")
    });
    println!("\nranked, slowest first:");
    for (name, elapsed, fields) in &rows {
        println!("{elapsed:10.1} ms  {fields:4} fields  {name}");
    }
}
