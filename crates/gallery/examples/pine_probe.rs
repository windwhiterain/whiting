//! The pine's boughs, measured: which depth-1 solids are inside out, folded or childless.
//!
//! A frame showed two boughs of the Yunnan pine drawn black and bare. Black wood is an inside-out
//! solid and bare wood is a node whose children did not place, so this probe prints, for every bough,
//! the three numbers that tell those apart: the signed volume of its solid, the length of its placed
//! spine, and the count of its children. Run:
//!
//! ```text
//! cargo run -p whiting-gallery --example pine_probe
//! ```

use shape::mesh::signed_volume;
use vegetation::expand::{Detail, wood};
use vegetation::handle::Handle;
use whiting_gallery::southwest::{Foliage, yunnan_pine};

fn main() {
    let species = yunnan_pine(21, Foliage::Needle);
    let solids = wood(&species.plant, Detail::DEFAULT);
    let mut worst = (Handle::root(), 0.0f32);
    for solid in &solids {
        let handle = &solid.handle;
        if handle.depth() != 1 {
            continue;
        }
        let volume = signed_volume(solid.loft.view());
        let spine = species
            .plant
            .spine(handle)
            .expect("a handle the walk yielded resolves");
        let node = species
            .plant
            .get(handle)
            .expect("a handle the walk yielded resolves");
        if volume < worst.1 {
            worst = (handle.clone(), volume);
        }
        println!(
            "{handle:<12} volume {volume:+.3e}  spine {:6.3} m  children {:2}  rings {:2}",
            spine.length(),
            node.children().len(),
            solid.loft.rings()
        );
    }
    println!("\nworst bough volume: {} at {:+.3e}", worst.0, worst.1);
}
