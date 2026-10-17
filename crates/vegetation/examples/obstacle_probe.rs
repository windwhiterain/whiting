//! Measures contact: what an obstacle does to a spine, what it costs, and what thinning takes.
//!
//! Every row is one promise of `.agents/proposals/agentic-vegetation-generation.md` V4's obstacle field and its
//! collision report, plus `thin`, read off the plant and off the occupancy rather than off the field's own numbers.
//!
//! ```text
//! cargo run -p whiting-vegetation --example obstacle_probe
//! ```

use shape::math::length3;
use shape::{Curve, Jet, Scalar};
use whiting_vegetation::contact::{collide, deepest};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::expand::Detail;
use whiting_vegetation::field::Field;
use whiting_vegetation::handle::Handle;
use whiting_vegetation::node::Node;
use whiting_vegetation::plant::Plant;
use whiting_vegetation::thin::thin;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("the push is the penetration", the_push_is_the_penetration()));
    checks.push(("a wall bends a bough out", a_wall_bends_a_bough_out()));
    checks.push((
        "a wall out of reach is nothing",
        a_wall_out_of_reach_is_nothing(),
    ));
    checks.push((
        "the report says what is left",
        the_report_says_what_is_left(),
    ));
    checks.push((
        "amount zero reports without shaping",
        amount_zero_reports_without_shaping(),
    ));
    checks.push((
        "thinning takes the crowded tips",
        thinning_takes_the_crowded_tips(),
    ));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<34} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// A wall whose solid side is `x > 0.55`: the occupancy is the distance from its face.
fn wall(point: [f32; 3]) -> f32 {
    0.55 - point[0]
}

/// A wall nothing reaches.
fn far_wall(point: [f32; 3]) -> f32 {
    50.0 - point[0]
}

fn species() -> Species {
    bamboo(20_261_017)
}

/// A plant of one straight stem across the wall's gradient, so the push has a closed form.
///
/// The stem runs along `+y` at `x = 0.7`, and the wall's face is at `x = 0.55`: every knot of it is `0.15` inside,
/// and the push is along `+x`, across the spine, so no segment is stretched and the tip's displacement is the
/// penetration exactly.
fn stem() -> Plant {
    let spine = Curve::new(vec![
        Jet::new([0.7, 0.0, 0.0], [0.0, 0.5, 0.0]),
        Jet::new([0.7, 0.5, 0.0], [0.0, 0.5, 0.0]),
        Jet::new([0.7, 1.0, 0.0], [0.0, 0.5, 0.0]),
    ])
    .expect("the stem's knots are distinct");
    let taper = Scalar::new(&[(0.0, 0.02), (1.0, 0.01)]).expect("the stem's law is a law");
    Plant::new(Node::root(spine, taper), 1)
}

/// The push is the penetration times the amount, less the re-projection that keeps the stem's segments their own
/// length, and the base does not move.
fn the_push_is_the_penetration() -> bool {
    let plain = stem();
    let root = Handle::root();
    let before = plain.spine(&root).expect("the stem resolves");
    let penetration = 0.15f32;
    let pushed = stem().with_fields(vec![Field::obstacle(wall, 1.0)]);
    let after = pushed.spine(&root).expect("the stem resolves");
    let base = after.jets()[0].position[0] - before.jets()[0].position[0];
    let moved = after.jets().last().expect("a spine has knots").position[0]
        - before.jets().last().expect("a spine has knots").position[0];
    let shortfall = -moved - penetration;
    // The push moves points; the segments it leaves behind are restored to their own lengths, so the *chords* are
    // what a bend preserves — the arc the curve reports is longer than their sum by the bend's own sagitta.
    let chords = |curve: &Curve| -> Vec<f32> {
        curve
            .jets()
            .windows(2)
            .map(|pair| {
                length3([
                    pair[1].position[0] - pair[0].position[0],
                    pair[1].position[1] - pair[0].position[1],
                    pair[1].position[2] - pair[0].position[2],
                ])
            })
            .collect()
    };
    let before_chords = chords(&before);
    let after_chords = chords(&after);
    let mut worst_chord = 0.0f32;
    for (one, other) in before_chords.iter().zip(after_chords.iter()) {
        worst_chord = worst_chord.max((one - other).abs());
    }
    println!("\n# a stem across a wall");
    println!(
        "  the stem stands {penetration:.6} inside: its base moved {base:.3e} along the push and its tip {moved:.6}, which is {shortfall:.3e} short of the penetration"
    );
    println!(
        "  its segments kept their own lengths to {worst_chord:.3e}, so the shortfall is the re-projection's and not the push's"
    );
    println!(
        "  the control is the base: a push that moved the origin would tear the stem off whatever holds it"
    );
    base.abs() < 1e-6 && shortfall.abs() < 0.03 * penetration && worst_chord < 1e-6
}

/// A bough that grew into a wall is bent out of it, and the report agrees.
fn a_wall_bends_a_bough_out() -> bool {
    let plain = species();
    let before = collide(&plain.plant, wall, Detail::DEFAULT);
    let pushed = species()
        .plant
        .with_fields(vec![Field::obstacle(wall, 1.0)]);
    let after = collide(&pushed, wall, Detail::DEFAULT);
    let depth_before = deepest(&before);
    let depth_after = deepest(&after);
    let mut worst = 0.0f32;
    for (handle, _) in plain.plant.live() {
        let one = plain.plant.spine(&handle).expect("a live spine resolves");
        let other = pushed.spine(&handle).expect("a live spine resolves");
        for (first, second) in one.jets().iter().zip(other.jets().iter()) {
            worst = worst.max(length3([
                first.position[0] - second.position[0],
                first.position[1] - second.position[1],
                first.position[2] - second.position[2],
            ]));
        }
    }
    println!("\n# a wall the boughs grew into");
    println!(
        "  {} contacts before, deepest {depth_before:.6}; {} after, deepest {depth_after:.6}",
        before.len(),
        after.len()
    );
    println!(
        "  the worst knot moved {worst:.6}, so the field did something rather than being ignored"
    );
    println!(
        "  the control is the depth: a field that pushed everything equally would move knots with no contact too"
    );
    before.len() > 0
        && worst > 1e-3
        && (depth_after < depth_before * 0.5 || after.len() < before.len() / 2)
}

/// A wall nothing reaches bends nothing, bit for bit: the property a scene relies on.
fn a_wall_out_of_reach_is_nothing() -> bool {
    let plain = species();
    let pushed = species()
        .plant
        .with_fields(vec![Field::obstacle(far_wall, 1.0)]);
    let mut worst = 0.0f32;
    for (handle, _) in plain.plant.live() {
        let one = plain.plant.spine(&handle).expect("a live spine resolves");
        let other = pushed.spine(&handle).expect("a live spine resolves");
        for (first, second) in one.jets().iter().zip(other.jets().iter()) {
            worst = worst.max(length3([
                first.position[0] - second.position[0],
                first.position[1] - second.position[1],
                first.position[2] - second.position[2],
            ]));
        }
    }
    let contacts = collide(&pushed, far_wall, Detail::DEFAULT).len();
    println!("\n# a wall out of reach");
    println!(
        "  every knot is where it was to {worst:.3e}, and the report finds {contacts} contacts"
    );
    println!(
        "  the control is the equality: a field that touched every spine would leave a picture it was added to changed"
    );
    worst == 0.0 && contacts == 0
}

/// The report says what the field could not fix, which is the one number an author acts on.
fn the_report_says_what_is_left() -> bool {
    let plain = species();
    let before = collide(&plain.plant, wall, Detail::DEFAULT);
    // One push per spine is not a solver: whatever the field cannot reach, the report says.
    let pushed = species()
        .plant
        .with_fields(vec![Field::obstacle(wall, 1.0), Field::obstacle(wall, 1.0)]);
    let after = collide(&pushed, wall, Detail::DEFAULT);
    let once = species()
        .plant
        .with_fields(vec![Field::obstacle(wall, 1.0)]);
    let once_left = deepest(&collide(&once, wall, Detail::DEFAULT));
    println!("\n# what is left after the push");
    println!(
        "  {} contacts, deepest {:.6}; after one push {} and deepest {once_left:.6}; after a second {} and deepest {:.6}",
        before.len(),
        deepest(&before),
        collide(&once, wall, Detail::DEFAULT).len(),
        after.len(),
        deepest(&after)
    );
    for contact in after.iter().take(3) {
        println!(
            "  still inside: {} at station {:.3} by {:.6}",
            contact.handle, contact.station, contact.depth
        );
    }
    println!(
        "  the control is the second push: a report that always answered zero would say the same after one push as after two"
    );
    !before.is_empty() && deepest(&after) < deepest(&before) && deepest(&after) <= once_left
}

/// An obstacle declared with an amount of zero reports and does not shape: the same plant, bit for bit.
fn amount_zero_reports_without_shaping() -> bool {
    let plain = species();
    let declared = species()
        .plant
        .with_fields(vec![Field::obstacle(wall, 0.0)]);
    let mut worst = 0.0f32;
    for (handle, _) in plain.plant.live() {
        let one = plain.plant.spine(&handle).expect("a live spine resolves");
        let other = declared.spine(&handle).expect("a live spine resolves");
        for (first, second) in one.jets().iter().zip(other.jets().iter()) {
            worst = worst.max(length3([
                first.position[0] - second.position[0],
                first.position[1] - second.position[1],
                first.position[2] - second.position[2],
            ]));
        }
    }
    let contacts = collide(&plain.plant, wall, Detail::DEFAULT).len();
    println!("\n# an obstacle that only asks");
    println!(
        "  with an amount of zero every knot is where it was to {worst:.3e}, and the report still finds {contacts} contacts"
    );
    println!(
        "  the control is the count: a declared obstacle that shaped the plant would be a field nobody could ask a question of"
    );
    worst == 0.0 && contacts > 0
}

/// Thinning drops the tips a parent cannot afford, and a second call finds nothing to do.
fn thinning_takes_the_crowded_tips() -> bool {
    let mut species = species();
    let before_nodes = species.plant.live().count();
    // Every sibling group as it stands: what a thinning will judge, taken before it judges anything.
    let groups: Vec<(Handle, Vec<(Handle, [f32; 3])>)> = species
        .plant
        .live()
        .map(|(parent, _)| {
            let count = species
                .plant
                .get(&parent)
                .map(|node| node.children().len())
                .unwrap_or(0);
            let tips = (0..count)
                .filter_map(|index| {
                    let child = parent.child(index as u32);
                    let spine = species.plant.spine(&child).ok()?;
                    Some((child, spine.at(1.0)))
                })
                .collect();
            (parent, tips)
        })
        .collect();
    let mut closest = f32::MAX;
    for (_, tips) in &groups {
        for (index, (_, one)) in tips.iter().enumerate() {
            for (_, other) in tips.iter().skip(index + 1) {
                closest = closest.min(length3([
                    one[0] - other[0],
                    one[1] - other[1],
                    one[2] - other[2],
                ]));
            }
        }
    }
    let spacing = closest * 2.0;
    let taken = thin(&mut species.plant, spacing);
    let after_nodes = species.plant.live().count();
    let again = thin(&mut species.plant, spacing);
    let pruned: Vec<Handle> = taken.pruned.clone();
    // Every pruned tip had a sibling kept within the spacing, judged on the geometry before the pruning.
    let mut justified = true;
    for (parent, tips) in &groups {
        let _ = parent;
        for (child, tip) in tips {
            if !pruned.contains(child) {
                continue;
            }
            let near = tips.iter().any(|(other, other_tip)| {
                other != child
                    && !pruned.contains(other)
                    && length3([
                        tip[0] - other_tip[0],
                        tip[1] - other_tip[1],
                        tip[2] - other_tip[2],
                    ]) < spacing
            });
            justified = justified && near;
        }
    }
    println!("\n# thinning");
    println!(
        "  the closest pair of sibling tips was {closest:.6}, so the spacing is {spacing:.6}: {} nodes pruned, the plant {before_nodes} → {after_nodes}",
        taken.pruned.len()
    );
    for handle in taken.pruned.iter().take(3) {
        println!("  {handle} went");
    }
    println!(
        "  every pruned tip had a kept sibling inside the spacing: {justified}; a second thinning pruned {} more",
        again.pruned.len()
    );
    println!(
        "  the control is the second call: a rule that fed on its own output would keep taking branches"
    );
    !taken.pruned.is_empty()
        && after_nodes < before_nodes
        && justified
        && again.pruned.is_empty()
        && spacing > closest
}
