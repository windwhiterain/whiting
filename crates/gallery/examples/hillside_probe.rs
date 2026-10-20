//! Prints the hillside's field, its grade along a line, and the terrace's grade for comparison.
//!
//! # Why this exists
//!
//! The hillside's frame came out as a patchwork of plateaus, and a picture cannot say whether the terrace is in
//! the grade, in the field, or in the mesh that draws them. This probe reads the same three things the scene
//! reads and prints them, so the answer is a number and not an impression.

use whiting_gallery::village;

fn main() {
    println!("{}", village::field_report());
    for (x, z) in [(0.0, -48.0), (0.0, 0.0), (26.0, -44.0), (100.0, 100.0)] {
        println!(
            "  sample({x:6.1},{z:6.1}) = {:8.3}   grade = {:8.3}",
            village::field_sample_at(x, z),
            village::hillside_at(x, z)
        );
    }
    println!("hillside grade along z at x = 0:");
    for step in -12..=12 {
        let z = step as f64 * 4.0;
        println!("  z {z:7.1}  {:8.3}", village::hillside_at(0.0, z));
    }
    println!("hillside grade along x at z = 0:");
    for step in -12..=12 {
        let x = step as f64 * 4.0;
        println!("  x {x:7.1}  {:8.3}", village::hillside_at(x, 0.0));
    }
}
