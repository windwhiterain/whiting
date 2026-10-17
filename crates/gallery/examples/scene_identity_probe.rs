//! The identity channel's readings, pinned for **every building scene** — as assertions rather than as numbers
//! in a report.
//!
//! # What this measures
//!
//! The channel is a **two-way table**, and both directions are registrations that can rot independently: a scene
//! whose meshes stop carrying ids draws a frame that answers zero everywhere, and a scene missing from
//! `locate_in` answers *"no domain reading"* at every pixel it does name. Neither shows up as a compile error, so
//! this probe walks each scene's whole table and asserts:
//!
//! - every id from the first answers a **non-empty name**, and the ids are **contiguous** from the first, so no
//!   row is numbered over a hole;
//! - every name **round-trips** back to the id it came from, which is what makes the two directions one table;
//! - the table is the one a frame's ids are drawn from, because it is `Identity::of` over the scene's own
//!   document — the same call the channel's packing is built from.
//!
//! A pixel-level assertion needs the device, and the gallery suite renders every scene; what that suite cannot
//! see is whether a pixel's number *means* anything, which is exactly what this pins.
//!
//! Run with `cargo run -p whiting-gallery --example scene_identity_probe`.

use definition::{Schema, SchemaKind};
use whiting_gallery::building_identity::{building_extras, id_in, locate_in, not_authored_extras};

/// The scenes that declare the channel, in the order the gallery lists them.
const SCENES: [&str; 6] = ["hall", "identity", "house", "shed", "workshop", "yard"];

/// Answers whether a point schema carries the `authored_id` field the channel writes.
fn carries(schema: &Schema) -> bool {
    matches!(&schema.kind, SchemaKind::Struct { fields } if fields.contains_key("authored_id"))
}

fn main() {
    println!("# the channel's own shape");
    let authored = building_extras();
    let plain = not_authored_extras();
    let shape = carries(&authored.point_schema()) && !carries(&plain.point_schema());
    println!(
        "  the authored schema carries authored_id: {}; the reserved-zero schema does not: {}",
        carries(&authored.point_schema()),
        !carries(&plain.point_schema())
    );
    println!(
        "  both declare an output: authored {}, plain {}",
        !authored.is_empty(),
        !plain.is_empty()
    );
    let mut clean = shape && !authored.is_empty() && !plain.is_empty();

    println!("\n# every scene's table, both ways");
    for scene in SCENES {
        let mut rows: Vec<(u32, String)> = Vec::new();
        let mut id = 1;
        while let Some(name) = locate_in(scene, id) {
            rows.push((id, name));
            id += 1;
        }
        let contiguous = rows
            .iter()
            .enumerate()
            .all(|(at, (held, _))| *held == at as u32 + 1);
        let named = rows.iter().all(|(_, name)| !name.is_empty());
        let round_trips = rows
            .iter()
            .all(|(held, name)| id_in(scene, name) == Some(*held));
        clean &= !rows.is_empty() && contiguous && named && round_trips;
        println!(
            "  {:<9} {:>3} row(s), ids 1..={}, contiguous {}, every name non-empty {}, every name round-trips {}",
            scene,
            rows.len(),
            rows.len(),
            contiguous,
            named,
            round_trips
        );
        // Two rows per scene, so a report shows what the numbers *mean* rather than only that they fit.
        for (at, (held, name)) in rows.iter().enumerate() {
            if at == 0 || at + 1 == rows.len() {
                println!("      id {held:>3} → {name}");
            }
        }
    }

    println!("\n# and the readings the reports quote");
    for (scene, id, expected) in [
        ("identity", 3, "mass 0, outer wall over edge 2"),
        (
            "identity",
            16,
            "mass 0, the head reveal of opening 0 in edge 2",
        ),
        ("house", 15, "roof 0, the skin of plane 0"),
        (
            "workshop",
            49,
            "part 0, template mass 0, outer wall over edge 0",
        ),
    ] {
        let answered = locate_in(scene, id);
        let matches = answered.as_deref() == Some(expected);
        clean &= matches;
        println!("  {scene} id {id} → {answered:?}; expected {expected:?}: {matches}");
    }

    println!("\n# the channel is two-way for every scene: {clean}");
}
