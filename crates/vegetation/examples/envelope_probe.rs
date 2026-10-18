//! Measures the envelope and the report: where a crown ends, what a trim takes, and what the numbers say.
//!
//! Every row is one promise of `.agents/proposals/agentic-vegetation-generation.md` V4 — the envelope as a radius
//! curve, refinable to a section, with a trim that reports rather than repairs — read off the plant and off the
//! envelope's own answers rather than off its knots.
//!
//! ```text
//! cargo run -p whiting-vegetation --example envelope_probe
//! ```

use shape::Section;
use shape::math::{length3, subtract};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::envelope::{Envelope, Outline, trim};
use whiting_vegetation::expand::Detail;
use whiting_vegetation::handle::Handle;
use whiting_vegetation::report::{BANDS, report};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("the radius is the envelope", the_radius_is_the_envelope()));
    checks.push(("a section refines it", a_section_refines_the_envelope()));
    checks.push(("the trim pulls tips in", the_trim_pulls_tips_in()));
    checks.push((
        "an unsavable node is pressed",
        an_unsavable_node_is_pressed(),
    ));
    checks.push(("the report agrees", the_report_agrees()));
    checks.push(("the crown reaches the plant", the_crown_reaches_the_plant()));
    checks.push((
        "a shape law varies along the trunk",
        a_shape_law_varies_along_the_trunk(),
    ));
    checks.push((
        "the coarse form is the circle",
        the_coarse_form_is_the_circle(),
    ));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<30} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

fn species() -> Species {
    bamboo(20_261_017)
}

/// A crown that reaches 0.55 at the base and 0.35 at the top: tighter than this plant's boughs.
fn tight() -> Envelope {
    Envelope::radial(&[(0.0, 0.55), (0.5, 0.45), (1.0, 0.35)])
}

/// A crown with room: nothing in this plant reaches it.
fn roomy() -> Envelope {
    Envelope::radial(&[(0.0, 1.6), (0.5, 1.4), (1.0, 1.3)])
}

/// A point at a station, offset across the trunk by `across` in the frame's normal.
fn across_the_trunk(
    plant: &whiting_vegetation::plant::Plant,
    station: f32,
    across: f32,
) -> [f32; 3] {
    let trunk = plant.spine(&Handle::root()).expect("the trunk resolves");
    let point = trunk.at(station);
    let frame = trunk.frame(station);
    [
        point[0] + frame.normal[0] * across,
        point[1] + frame.normal[1] * across,
        point[2] + frame.normal[2] * across,
    ]
}

/// On a circular envelope the outside distance is the radial excess, exactly.
fn the_radius_is_the_envelope() -> bool {
    let species = species();
    let crown = tight();
    let at = 0.5f32;
    let allowed = crown.radius.at(at);
    let inside = crown.outside(
        &species.plant,
        across_the_trunk(&species.plant, at, allowed - 0.1),
    );
    let on_it = crown.outside(
        &species.plant,
        across_the_trunk(&species.plant, at, allowed),
    );
    let beyond = crown.outside(
        &species.plant,
        across_the_trunk(&species.plant, at, allowed + 1.0),
    );
    let axis = crown.outside(&species.plant, across_the_trunk(&species.plant, at, 0.0));
    println!("\n# the radius is the envelope");
    println!(
        "  at station {at} the law allows {allowed:.6}: a point 0.1 inside reads {inside:.3e}, on the boundary {on_it:.3e}, a metre beyond {beyond:.6}"
    );
    println!("  a point on the trunk's own axis reads {axis:.3e}");
    println!(
        "  the control is the metre: the outside distance is the excess itself, not the ratio or a squared one"
    );
    println!(
        "  a boundary point reads a few millionths, because `closest` pins a fraction only as well as the distance is flat"
    );
    inside == 0.0 && on_it < 1e-4 && (beyond - 1.0).abs() < 1e-3 && axis == 0.0
}

/// The same radius law drawn with a section is a different crown: a refinement is a data change.
fn a_section_refines_the_envelope() -> bool {
    let species = species();
    let at = 0.5f32;
    let radius = 1.0f32;
    let latitude = Envelope::radial(&[(0.0, radius), (1.0, radius)]);
    let upright =
        Envelope::radial(&[(0.0, radius), (1.0, radius)]).with_section(Section::ellipse(2.0, 0.5));
    let wide = across_the_trunk(&species.plant, at, 1.5);
    let circle_x = latitude.outside(&species.plant, wide);
    let ellipse_x = upright.outside(&species.plant, wide);
    // The same distance, but in the trunk's binormal direction instead: the narrow axis.
    let trunk = species
        .plant
        .spine(&Handle::root())
        .expect("the trunk resolves");
    let point = trunk.at(at);
    let frame = trunk.frame(at);
    let along = [
        point[0] + frame.binormal[0] * 1.5,
        point[1] + frame.binormal[1] * 1.5,
        point[2] + frame.binormal[2] * 1.5,
    ];
    let circle_y = latitude.outside(&species.plant, along);
    let ellipse_y = upright.outside(&species.plant, along);
    println!("\n# a section refines the envelope");
    println!(
        "  a circle of radius 1 and the same law drawn with a 2:1 section, at 1.5 across the normal: {circle_x:.6} and {ellipse_x:.6}"
    );
    println!("  and at 1.5 along the binormal: {circle_y:.6} and {ellipse_y:.6}");
    println!(
        "  the control is the two axes: a refinement that only scaled the circle would answer the same on both"
    );
    (circle_x - 0.5).abs() < 1e-4
        && ellipse_x == 0.0
        && (circle_y - 0.5).abs() < 1e-4
        && (ellipse_y - 1.0).abs() < 1e-4
}

/// The trim writes length nudges: every trimmed node's tip comes inside, and the plant gets smaller.
fn the_trim_pulls_tips_in() -> bool {
    let mut species = species();
    let crown = tight();
    let before = report(
        &species.plant,
        Detail::DEFAULT,
        &species.fields,
        Some(&crown),
    );
    let worst_before = before.worst_exceedance();
    let mut tallest = 0.0f32;
    for (handle, _) in species.plant.live() {
        let tip = species
            .plant
            .spine(&handle)
            .expect("a live spine resolves")
            .at(1.0);
        tallest = tallest.max(tip[1]);
    }
    let trimmed = trim(&mut species.plant, &crown);
    let after = report(
        &species.plant,
        Detail::DEFAULT,
        &species.fields,
        Some(&crown),
    );
    let worst_after = after.worst_exceedance();
    let mut tallest_after = 0.0f32;
    for (handle, _) in species.plant.live() {
        let tip = species
            .plant
            .spine(&handle)
            .expect("a live spine resolves")
            .at(1.0);
        tallest_after = tallest_after.max(tip[1]);
    }
    let removed: f32 = trimmed.trimmed.iter().map(|(_, amount)| amount).sum();
    println!("\n# the trim");
    println!(
        "  {} nodes trimmed, {removed:.6} of length taken in total, {} pressed",
        trimmed.trimmed.len(),
        trimmed.pressed.len()
    );
    println!(
        "  the worst exceedance went {worst_before:.6} → {worst_after:.6}, and the tallest tip {tallest:.6} → {tallest_after:.6}"
    );
    for (handle, amount) in trimmed.trimmed.iter().take(3) {
        println!("  {handle} lost {amount:.6}");
    }
    println!(
        "  the control is the length: a trim that reported itself without changing a spine would leave the tallest tip where it was"
    );
    // The nodes the trim could reach are inside now; the pressed ones are the author's, and the report says so.
    let mut worst_trimmed = f32::MIN;
    for (handle, _) in &trimmed.trimmed {
        let tip = species
            .plant
            .spine(handle)
            .expect("a trimmed spine resolves")
            .at(1.0);
        worst_trimmed = worst_trimmed.max(crown.outside(&species.plant, tip));
    }
    println!(
        "  the worst exceedance among the trimmed nodes is {worst_trimmed:.3e}, and the {} pressed ones are reported rather than cut",
        trimmed.pressed.len()
    );
    worst_before > 0.0
        && worst_trimmed <= 1e-4
        && !trimmed.trimmed.is_empty()
        && tallest_after < tallest
        && removed > 0.0
}

/// A node whose origin stands outside cannot be shortened into the crown: it is pressed and reported.
fn an_unsavable_node_is_pressed() -> bool {
    let mut species = species();
    // A crown so narrow that the boughs are hanging outside it at their own origins.
    let narrow = Envelope::radial(&[(0.0, 0.005), (1.0, 0.005)]);
    let bough = Handle::root().child(0);
    let before = species
        .plant
        .spine(&bough)
        .expect("the bough resolves")
        .length();
    let trimmed = trim(&mut species.plant, &narrow);
    let after = species
        .plant
        .spine(&bough)
        .expect("the bough resolves")
        .length();
    let pressed_the_bough = trimmed.pressed.iter().any(|(handle, _)| handle == &bough);
    println!("\n# a node that cannot be saved");
    println!(
        "  {} trimmed and {} pressed · the first bough is {} and its length is {before:.6} → {after:.6}",
        trimmed.trimmed.len(),
        trimmed.pressed.len(),
        if pressed_the_bough {
            "pressed"
        } else {
            "not pressed"
        }
    );
    println!(
        "  the control is the length: a trim that shrank an unsavable node would move it toward the trunk for nothing"
    );
    pressed_the_bough && (before - after).abs() < 1e-6
}

/// The report's counts are the expansion's, and its crown is in the trunk's own parameter space.
fn the_report_agrees() -> bool {
    let species = species();
    let roomy = roomy();
    let measured = report(
        &species.plant,
        Detail::DEFAULT,
        &species.fields,
        Some(&roomy),
    );
    let nodes = species.plant.live().count();
    let fits = measured.worst_exceedance() <= 0.0;
    let bands = measured.crown.len() == BANDS;
    let low_to_high = measured
        .crown
        .windows(2)
        .all(|pair| pair[0].station < pair[1].station);
    println!("\n# the report");
    println!(
        "  {nodes} nodes · {} solids · {} triangles · {} leaves in {} groups",
        measured.wood_solids, measured.triangles, measured.leaves, measured.groups
    );
    println!("  the roomy crown fits: {fits} · {bands} bands, low to high: {low_to_high}");
    for reach in &measured.crown {
        println!(
            "  station {:.3}: reached {:.6}, allowed {:.6}",
            reach.station, reach.reached, reach.allowed
        );
    }
    println!(
        "  the control is the allowance: a report that read the envelope's knots in another space would not fit a crown that does"
    );
    measured.nodes == nodes
        && measured.wood_solids == nodes
        && measured.triangles > 0
        && measured.leaves > 0
        && fits
        && bands
        && low_to_high
}

/// The crown's own reading is the plant's widest reach: the report and a direct measurement agree.
fn the_crown_reaches_the_plant() -> bool {
    let species = species();
    let trunk = species
        .plant
        .spine(&Handle::root())
        .expect("the trunk resolves");
    let mut widest = 0.0f32;
    for (handle, _) in species.plant.live() {
        let tip = species
            .plant
            .spine(&handle)
            .expect("a live spine resolves")
            .at(1.0);
        let nearest = trunk.closest(tip);
        widest = widest.max(length3(subtract(tip, nearest.position)));
    }
    let measured = report(
        &species.plant,
        Detail::DEFAULT,
        &species.fields,
        Some(&roomy()),
    );
    let reached = measured
        .crown
        .iter()
        .map(|reach| reach.reached)
        .fold(0.0f32, f32::max);
    println!("\n# the crown against a direct measurement");
    println!(
        "  the report's widest band reads {reached:.6} and a walk over every tip reads {widest:.6}"
    );
    println!(
        "  the control is the difference: a report measuring something else — a leaf, a solid's vertex — would read differently"
    );
    (reached - widest).abs() < 1e-5
}

/// The shape law: a family of outlines keyed along the trunk, blended between its keys.
///
/// # Invariants
///
/// An ellipse answered in its narrow direction is its own half-axis, so the three readings are the law itself: one at
/// the base, a quarter at the top, and the mean half way — **a blend and not a step**, which is what makes the shape a
/// surface rather than two envelopes stacked.
fn a_shape_law_varies_along_the_trunk() -> bool {
    let shape = Outline::of(&[
        (0.0, Section::ellipse(1.0, 1.0)),
        (1.0, Section::ellipse(1.0, 0.25)),
    ]);
    let (base, middle, top) = (
        shape.reach(0.0, [0.0, 1.0]),
        shape.reach(0.5, [0.0, 1.0]),
        shape.reach(1.0, [0.0, 1.0]),
    );
    let widest = shape.widest(1.0);
    println!("\n# a shape law");
    println!(
        "  the narrow direction reaches {base:.6} at the base, {middle:.6} half way, {top:.6} at the top; the widest direction at the top is {widest:.6}"
    );
    println!(
        "  the control is the middle: a step would read one end's value, and the mean is 0.625000"
    );
    (base - 1.0).abs() < 1e-6
        && (top - 0.25).abs() < 1e-6
        && (middle - 0.625).abs() < 1e-6
        && (widest - 1.0).abs() < 1e-6
}

/// The coarse envelope is the circle, and refining it to a circle changes nothing.
fn the_coarse_form_is_the_circle() -> bool {
    let radius = [(0.0f32, 0.52f32), (0.5, 0.40), (1.0, 0.28)];
    let coarse = Envelope::radial(&radius);
    let refined = Envelope::radial(&radius).with_section(Section::circle());
    let circle = Outline::CIRCLE.reach(0.37, [0.6, -0.8]);
    println!("\n# the coarse form");
    println!(
        "  the empty shape law reaches {circle:.6} in every direction, and a circle section reaches the same, so the coarse envelope is the refined one"
    );
    println!(
        "  the control is the data: a coarse crown is one number per station, which is what `radial` writes"
    );
    let mut worst = 0.0f32;
    for step in 0..16 {
        let station = step as f32 / 15.0;
        let angle = step as f32 * 0.7;
        let direction = [angle.cos(), angle.sin()];
        worst = worst.max(
            (coarse.shape.reach(station, direction) - refined.shape.reach(station, direction))
                .abs(),
        );
    }
    println!(
        "  the two laws differ in their data — an empty family against two circle keys — and agree in every reading to {worst:.3e}, which is the invariant that matters"
    );
    (circle - 1.0).abs() < 1e-6 && worst < 1e-6
}
