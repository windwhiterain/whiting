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
//! # The pins, and what moves them
//!
//! **An id is a position in a scene's own element order, so any change to what a document emits moves every id
//! after it.** The volume refactor is such a change: a mass emits a wall ring and a plate rather than one shell,
//! so `house` id 15 read *the skin of plane 1* where the shell's order had read *plane 0*, and the pin had been
//! stale since that change rather than since the last edit here. A **doorway takes its row with it** as well: a
//! sill standing on the wall's own floor is the ring's base and is not a face of its own, so every scene carrying
//! a doorway is one row shorter than it was — measured, `shed` 67 to 66 rows, `workshop` 124 to 123, and the
//! `identity` scene 78 to 77, with the seven scenes that carry no doorway unmoved.
//!
//! So a pin is re-read after a change that emits elements, never edited to make a run green without that reading
//! being the one the reports quote.
//!
//! **The house window re-read the `house` id 15 pin, on 2026-11-07.** The house grew one opening on edge 1, which
//! is four new rows — left reveal, right reveal, head, sill — inside `mass 0`'s own block and therefore ahead of
//! every id the roof held. `house` id 15 read **`roof 0, the skin of plane 1`** and now reads
//! **`mass 0, the head reveal of opening 0 in edge 1`**, which is the same fact one row-block earlier and is what
//! the pin should have been holding since the window existed: the four quoted readings below name one surface of
//! one element each, and after this one is a reveal rather than a skin. The yard's doorway, by the rule above, took
//! three rows and not four — 101 to 104 against the house's 31 to 35 — which is the doorway claim measured a second
//! time rather than asserted once.
//!
//! Run with `cargo run -p whiting-gallery --example scene_identity_probe`.

use definition::{Schema, SchemaKind};
use whiting_gallery::building_identity::{building_extras, id_in, locate_in, not_authored_extras};

/// The scenes that declare the channel, in the order the gallery lists them.
const SCENES: [&str; 10] = [
    "hall",
    "hall_large",
    "identity",
    "house",
    "shed",
    "stilt",
    "terrace",
    "workshop",
    "wing",
    "yard",
];

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
            "mass 0, the left reveal of opening 1 in edge 2",
        ),
        (
            "house",
            15,
            "mass 0, the head reveal of opening 0 in edge 1",
        ),
        (
            "workshop",
            49,
            "part 0, template mass 0, outer wall over edge 2",
        ),
    ] {
        let answered = locate_in(scene, id);
        let matches = answered.as_deref() == Some(expected);
        clean &= matches;
        println!("  {scene} id {id} → {answered:?}; expected {expected:?}: {matches}");
    }

    println!("\n# the channel is two-way for every scene: {clean}");
}
