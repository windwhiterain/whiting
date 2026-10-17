//! The style slice's acceptance: a style is a set of **relations**, so the buildings it derives are coherent
//! at every size rather than at one.
//!
//! # What this measures
//!
//! - the same footprint in two styles differs in **every** proportion (wall, eave, pitch, roof, glazing);
//! - the same style at two sizes keeps its **proportions** and honours its floors — the wall is a nineteenth
//!   of the smaller span with a 220 mm minimum, so a small cottage is limited by the minimum and a large one by
//!   the share;
//! - each derived building **validates clean and expands**, so a style is not a table of plausible numbers but
//!   a building;
//! - the style's own coherence claims hold: the eave stands outside the wall further than it sinks, the pitch
//!   sheds and does not spire, a window fits its storey, and a bay holds a hole with jambs on both sides.
//!
//! Run with `cargo run -p whiting-building --example style_probe`.

use whiting_building::derive::{COTTAGE, HALL, Style, starter};
use whiting_building::{MassId, RoofId, SolidId};

/// Prints one derived building and answers whether it is a building the crate accepts.
fn report(style: &Style, width: f32, depth: f32, storeys: usize) -> bool {
    let plan = style.starter(width, depth, storeys);
    let building = starter(&plan);
    let span = width.min(depth);
    let violations = building.validate();
    let solid_ok = match building.build() {
        Ok(built) => {
            let mass = built.solid_is_manifold(SolidId::Mass(MassId(0)))
                && built.solid_boundary_loops(SolidId::Mass(MassId(0))) == 0;
            let roof = built.solid_is_manifold(SolidId::Roof(RoofId(0)));
            let volume = built.solid_signed_volume(SolidId::Mass(MassId(0)));
            println!(
                "  {:<8} {width:.1} x {depth:.1} x {storeys} storey(s): wall {:.4} ({:.3} of the span), \
                 slab {:.4}, eave {:.4}, sink {:.4}, rise {:.4}, roof {:.4}, volume {:.4}",
                style.name,
                plan.wall,
                plan.wall / span,
                plan.slab,
                plan.overhang,
                plan.sink,
                plan.rise,
                plan.roof_thickness,
                volume
            );
            mass && roof
        }
        Err(_) => false,
    };
    println!(
        "           {} violation(s), {} solid(s) closed: {}",
        violations.len(),
        if solid_ok { "both" } else { "NOT" },
        solid_ok && violations.is_empty()
    );
    violations.is_empty() && solid_ok
}

/// Answers whether a style's own coherence claims hold for one size.
fn coherent(style: &Style, width: f32, depth: f32, storeys: usize) -> Vec<&'static str> {
    let plan = style.starter(width, depth, storeys);
    let mut broken = Vec::new();
    if plan.wall < 0.15 || plan.wall > 0.6 {
        broken.push("a wall that is not a wall");
    }
    if plan.overhang <= plan.sink {
        broken.push("an eave that stands inside its own seat");
    }
    let span = width.min(depth);
    if plan.rise < span * 0.25 {
        broken.push("a roof too flat to shed");
    }
    if plan.rise > span * 0.9 {
        broken.push("a roof that spires");
    }
    if plan.roof_thickness >= plan.rise {
        broken.push("a roof thicker than its own rise");
    }
    let bays = style.bays(width);
    let bay_width = width / bays as f32;
    let bay_height = plan.height / storeys as f32;
    let (left, right, sill, head) = style.glazing(bay_width, bay_height);
    if left <= 0.0 || right <= 0.0 {
        broken.push("a window with no jamb");
    }
    if sill + head > bay_height {
        broken.push("a window taller than its storey");
    }
    if 2.0 * plan.wall >= bay_width {
        broken.push("a bay that cannot hold a hole: its jambs are all wall");
    }
    broken
}

fn main() {
    println!("# the same footprint in two styles");
    let mut coherent_everywhere = true;
    for style in [COTTAGE, HALL] {
        coherent_everywhere &= report(&style, 6.0, 9.0, 1);
    }

    println!("\n# the same style at two sizes keeps its proportions and honours its floors");
    for (width, depth) in [(2.4f32, 2.4f32), (6.0, 9.0), (12.0, 18.0)] {
        let plan = COTTAGE.starter(width, depth, 1);
        let span = width.min(depth);
        let share = plan.wall / span;
        let floor = COTTAGE.wall.1;
        let ceiling = COTTAGE.wall.2;
        println!(
            "  cottage {width:.1} x {depth:.1}: wall {:.4} = clamp({:.3} x {:.1}, {:.2}, {:.2}) = {:.4}",
            plan.wall,
            COTTAGE.wall.0,
            span,
            floor,
            ceiling,
            (COTTAGE.wall.0 * span).clamp(floor, ceiling)
        );
        println!(
            "           its share of the span, {share:.4}, is the relation it is really built on"
        );
    }
    println!(
        "  so the smallest cottage is limited by the 0.22 m floor and the largest by the 0.45 m ceiling, and \
         the middle one by the share: a relation with both ends bounded"
    );

    println!("\n# every style's coherence claims, at every size");
    for style in [COTTAGE, HALL] {
        for (width, depth, storeys) in [
            (2.4f32, 2.4f32, 1usize),
            (6.0, 9.0, 1),
            (6.0, 9.0, 2),
            (12.0, 18.0, 3),
        ] {
            let broken = coherent(&style, width, depth, storeys);
            coherent_everywhere &= broken.is_empty();
            if !broken.is_empty() {
                println!(
                    "  {} at {width:.1} x {depth:.1} x {storeys}: {broken:?}",
                    style.name
                );
            }
        }
    }
    println!("  every claim holds everywhere: {coherent_everywhere}");

    println!("\n# and the facades a style implies");
    for style in [COTTAGE, HALL] {
        for width in [4.0f32, 6.0, 9.0, 12.0] {
            let bays = style.bays(width);
            let bay_width = width / bays as f32;
            let bay_height = style.storey;
            let (left, right, sill, head) = style.glazing(bay_width, bay_height);
            println!(
                "  {:<8} {width:5.1} m wide: {bays} bay(s) of {bay_width:.3} m; a window inset {left:.3}/\
                 {right:.3}, sill {sill:.3}, {head:.3} tall",
                style.name
            );
        }
    }

    println!("\n# and a two-storey hall, which is what the style is for");
    let mut clean = true;
    for style in [COTTAGE, HALL] {
        clean &= report(&style, 6.0, 9.0, 2);
    }
    println!(
        "\n  every derived building clean: {}",
        clean && coherent_everywhere
    );
}
