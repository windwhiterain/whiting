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
use document::building_identity::{building_extras, locate_in, not_authored_extras, table_in};

/// The scenes that declare the channel, in the order the gallery lists them.
const SCENES: [&str; 69] = [
    "chuandou_house",
    "caravan_inn",
    "compound",
    "compound_court",
    "fengshui_tower",
    "hall",
    "dai_hamlet",
    "hall_large",
    "identity",
    "house",
    "salt_village",
    "salt_yard",
    "saltworks",
    "shed",
    "stilt",
    "dai_temple",
    "dong_water",
    "dong_square",
    "dong_village",
    "dai_stupa",
    "drum_tower",
    "dragon_kiln",
    "earthen_house",
    "bai_gate",
    "bai_court",
    "bai_lane",
    "benzu_miao",
    "benzu_miao_court",
    "bamboo_house",
    "jingpo_mouth",
    "jingpo_village",
    "lisu_steps",
    "lisu_yard",
    "lisu_village",
    "log_house",
    "village_west",
    "tuzhang_mouth",
    "tuzhang_village",
    "village_square",
    "hillside_village",
    "longhouse",
    "miao_stilts",
    "miao_village",
    "mushroom_house",
    "naxi_court",
    "paifang",
    "plateau_village",
    "pottery_village",
    "qiang_double",
    "qiang_tower",
    "rain_bridge",
    "stilt_gallery",
    "yikeyin_gable",
    "yikeyin",
    "yikeyin_court",
    "street",
    "street_arcade",
    "terrace",
    "slate_court",
    "slate_street",
    "slate_gate",
    "slate_house",
    "tunpu_gate",
    "tunpu_village",
    "tibetan_house",
    "watermill",
    "workshop",
    "wing",
    "yard",
];

/// Answers whether a point schema carries the `authored_id` field the channel writes.
fn carries(schema: &Schema) -> bool {
    matches!(&schema.kind, SchemaKind::Struct { fields } if fields.contains_key("authored_id"))
}

fn main() {
    // **The scenes to walk are the command line's, or every scene.** The table costs about a minute
    // a scene, so a run that pins one new scene should not have to expand the other fifteen: pass the
    // name and this walks that one. With no arguments it walks all of them, which is what the audit
    // asks for and what keeps the total honest.
    let asked: Vec<String> = std::env::args().skip(1).collect();
    let walk: Vec<&str> = if asked.is_empty() {
        SCENES.to_vec()
    } else {
        SCENES
            .iter()
            .copied()
            .filter(|scene| asked.iter().any(|name| name == scene))
            .collect()
    };
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
    // **A scene the gallery names but this list does not is a scene no instrument measures**, and the same is true the other way round: the list is a
    // second register of the gallery and the two are checked against each other here, because rounds 207, 221, 223 and 231 each added a scene to three of
    // the four registrations and **the fourth one failed silently** — `building_of` answered `None`, the loop below `continue`d, and the run reported one
    // scene fewer with no error at all.
    // **The check is about buildings and not about scenes**: the gallery also publishes studies — `dunes`, `stilllife`, `tiles` and their siblings — which
    // are not architecture and carry no `naming` arm by design. What must hold is that **every name the gallery publishes and `naming` answers for is
    // measured here**, and that every name listed here is one the gallery publishes.
    let published = whiting_gallery::Gallery::names();
    let buildable: Vec<&str> = published
        .iter()
        .copied()
        .filter(|name| whiting_gallery::naming::building_of(name).is_some())
        .collect();
    let missing_from_probe: Vec<&&str> = buildable
        .iter()
        .filter(|name| !SCENES.contains(name))
        .collect();
    let unknown_to_gallery: Vec<&&str> = SCENES
        .iter()
        .filter(|name| !published.contains(name))
        .collect();
    println!(
        "  the gallery names {} scene(s), {} of them buildings; this probe lists {}: unmeasured {:?}, unknown {:?}",
        published.len(),
        buildable.len(),
        SCENES.len(),
        missing_from_probe,
        unknown_to_gallery
    );
    clean &= missing_from_probe.is_empty() && unknown_to_gallery.is_empty();
    let asked_for = walk.len();
    let mut measured = 0_usize;
    for scene in walk {
        let Some(building) = whiting_gallery::naming::building_of(scene) else {
            clean = false;
            println!("  {scene}: NOT NAMED — `naming::building_of` has no arm for it");
            continue;
        };
        // **The document is built once and the table read from it.** `locate_in` and `id_in` are the
        // tools' own entry points and each one builds the document to answer a single question — the
        // right cost for a caller that asks once, and nine thousand rebuilds for a walk that asks every
        // id and then every name back. Measured on the street: 143 s a scene through those two, and
        // under a second from one table.
        let Some(rows) = table_in(&building) else {
            clean = false;
            println!("  {scene}: THE DOCUMENT DOES NOT BUILD");
            continue;
        };
        let contiguous = rows
            .iter()
            .enumerate()
            .all(|(at, (held, _))| *held == at as u32 + 1);
        let named = rows.iter().all(|(_, name)| !name.is_empty());
        // The same table read the other way: one index from name to id, so the round trip is a
        // lookup and not a search.
        let by_name: std::collections::HashMap<&str, u32> =
            rows.iter().map(|(id, name)| (name.as_str(), *id)).collect();
        let round_trips = rows
            .iter()
            .all(|(held, name)| by_name.get(name.as_str()) == Some(held));
        clean &= !rows.is_empty() && contiguous && named && round_trips;
        measured += 1;
        println!(
            "  {:<13} {:>4} row(s), ids 1..={}, contiguous {}, every name non-empty {}, every name round-trips {}",
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

    // **Every scene the walk asked for printed a table, or the run failed.** This is the count that catches a silently skipped registration, and it is the
    // one number that must be read beside the gallery's own.
    println!(
        "  the walk asked for {} scene(s) and {} printed a table: {}",
        asked_for,
        measured,
        measured == asked_for
    );
    clean &= measured == asked_for;

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
        let answered = whiting_gallery::naming::building_of(scene)
            .and_then(|building| locate_in(&building, id));
        let matches = answered.as_deref() == Some(expected);
        clean &= matches;
        println!("  {scene} id {id} → {answered:?}; expected {expected:?}: {matches}");
    }

    println!("\n# the channel is two-way for every scene: {clean}");
}
