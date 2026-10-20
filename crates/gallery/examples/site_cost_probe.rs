//! What a site spends its build on: its plantings, their expansions, and the document between them.
//!
//! The question: `site_transect` builds in 30.7 s (`what-the-gallery-costs-to-build`) and no note
//! decomposes it. Its plates do not add up to it either, because a site draws its trees at coarse
//! dials and a dial shrinks the expansion while leaving the growth alone — so the split has to be
//! taken where the site itself is built.
//!
//! Three readings, one build of each, and the third is a difference rather than a phase:
//!
//! | phase | what it is |
//! |---|---|
//! | plantings | every element's species grown and dressed |
//! | expansions | every element's wood and leaves |
//! | assembly | the rest of the site's own build: mesh packing, instance rows, naming |
//!
//! Prints and asserts nothing:
//! `cargo run --release -p whiting-gallery --example site_cost_probe`.

use std::time::Instant;

use whiting_gallery::southwest_sites::{site_transect, transect_plantings};
use whiting_gallery::vegetation_picture::draw;

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let phase = Instant::now();
    let plantings = transect_plantings();
    let planted = ms(phase);
    println!(
        "plantings    {planted:9.1} ms   {} elements",
        plantings.len()
    );

    let phase = Instant::now();
    let mut drawn = Vec::with_capacity(plantings.len());
    let mut each: Vec<f64> = Vec::with_capacity(plantings.len());
    for (index, planting) in plantings.iter().enumerate() {
        let one = Instant::now();
        drawn.push(draw(index, planting));
        each.push(ms(one));
    }
    let expanded = ms(phase);
    println!("expansions   {expanded:9.1} ms   {} drawings", drawn.len());

    // **The site's own build is where the assembly can be seen**, and it builds the two phases above
    // again rather than taking them: a phase is a measurement here, not an input.
    let phase = Instant::now();
    let document = site_transect();
    let total = ms(phase);
    let fields = document.as_struct().map_or(0, |fields| fields.len());
    println!("total        {total:9.1} ms   {fields} fields");
    println!(
        "assembly     {:9.1} ms   (total - plantings - expansions)",
        total - planted - expanded
    );

    // The element cost, largest first, so the tree that owns the site is named rather than inferred.
    let mut order: Vec<usize> = (0..each.len()).collect();
    order.sort_by(|one, other| {
        each[*other]
            .partial_cmp(&each[*one])
            .expect("a measured wall clock is a number")
    });
    println!("\nlargest expansions:");
    for index in order.iter().take(12) {
        println!("{:9.1} ms  element {index}", each[*index]);
    }
    let tail_start = order.len().min(20);
    let tail: f64 = order
        .iter()
        .skip(tail_start)
        .map(|index| each[*index])
        .sum();
    println!(
        "tail         {tail:9.1} ms   over {} elements",
        order.len().saturating_sub(tail_start)
    );
}
