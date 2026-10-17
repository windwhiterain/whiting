//! Measures the fields: what a push does to a spine, what it does to what hangs from it, and what it may not
//! touch.
//!
//! Every row is one promise of `.agents/proposals/agentic-vegetation-generation.md` V5's stage 6 — *apply fields
//! and the envelope* — read off the shaped spine rather than off the field's own numbers. The instrument is a
//! **lateral** push, because a lateral push is where a bend is visible; gravity is measured against it, and the
//! difference between the two is the law that only the component *across* a spine bends it.
//!
//! ```text
//! cargo run -p whiting-vegetation --example field_probe
//! ```

use shape::math::{dot3, length3, normalized3, span, subtract};
use shape::{Curve, Jet};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::field::Field;
use whiting_vegetation::handle::Handle;
use whiting_vegetation::ops::{Change, Edit, apply};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("across, not along", a_push_bends_only_across()));
    checks.push((
        "a push along does nothing",
        a_push_along_a_spine_does_nothing(),
    ));
    checks.push(("the weight is t squared", the_weight_is_t_squared()));
    checks.push(("a bend does not stretch", a_bend_does_not_stretch()));
    checks.push((
        "a child follows the bend",
        a_child_follows_its_parents_bend(),
    ));
    checks.push(("a baked spine is immune", a_baked_spine_is_immune()));
    checks.push(("two fields compose", two_fields_compose()));
    checks.push(("gravity droops a bough", gravity_droops_a_bough()));
    checks.push(("an ancestor straightens", an_ancestor_straightens()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<28} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// The plant every row reshapes, under a set of fields.
fn under(fields: Vec<Field>) -> Species {
    let mut species = bamboo(20_261_017);
    species.plant = species.plant.with_fields(fields);
    species
}

/// A lateral push: the instrument, because a bend is visible across a spine.
fn wind(amount: f32) -> Field {
    Field::light([1.0, 0.0, 0.0], amount)
}

/// The jets' own chord lengths: what a bend must not change.
fn chords(curve: &Curve) -> Vec<f32> {
    let points: Vec<[f32; 3]> = curve.jets().iter().map(|jet| jet.position).collect();
    points
        .windows(2)
        .map(|pair| length3(subtract(pair[1], pair[0])))
        .collect()
}

/// A lateral push moves the trunk's tip by the field's reach, across the spine, and leaves its base alone.
fn a_push_bends_only_across() -> bool {
    let amount = 0.15f32;
    let plain = under(Vec::new());
    let pushed = under(vec![wind(amount)]);
    let root = Handle::root();
    let before = plain.plant.spine(&root).expect("the trunk resolves");
    let after = pushed.plant.spine(&root).expect("the trunk resolves");
    let length = before.length();
    let base = span(before.jets()[0].position, after.jets()[0].position);
    let tip = subtract(
        after.jets().last().expect("a spine has knots").position,
        before.jets().last().expect("a spine has knots").position,
    );
    let moved = length3(tip);
    let sideways = dot3(normalized3(tip), [1.0, 0.0, 0.0]);
    let reach = amount * length;
    println!("\n# a lateral push on the trunk");
    println!(
        "  a trunk of {length:.6} under a {amount} push moves its tip {moved:.6} for a reach of {reach:.6}, so {:.4} of it",
        moved / reach
    );
    println!(
        "  the displacement points along the push to {sideways:.6} — not to one, because the part along the leaning trunk is removed — and the base moved {base:.3e}"
    );
    println!(
        "  the control is the base: a push that moved the origin would move the plant off the ground"
    );
    base < 1e-6 && (moved / reach - 1.0).abs() < 0.05 && sideways > 0.95
}

/// Gravity on an upright trunk finds almost nothing across it: a spine along the field is compressed, not pulled.
fn a_push_along_a_spine_does_nothing() -> bool {
    let amount = 0.1f32;
    let plain = under(Vec::new());
    let root = Handle::root();
    let before = plain.plant.spine(&root).expect("the trunk resolves");
    let length = before.length();
    let along = under(vec![Field::gravity(amount)]);
    let sideways = under(vec![wind(amount)]);
    let moved = |species: &Species| {
        let after = species.plant.spine(&root).expect("the trunk resolves");
        span(
            before.jets().last().expect("a spine has knots").position,
            after.jets().last().expect("a spine has knots").position,
        )
    };
    let down = moved(&along);
    let across = moved(&sideways);
    let reach = amount * length;
    println!("\n# a push along a spine");
    println!(
        "  gravity moves the upright trunk's tip {down:.6} and a lateral push of the same strength moves it {across:.6}"
    );
    println!(
        "  that is {:.2}% of the reach for gravity against {:.2}% for the lateral push, and the trunk leans enough to give gravity that much",
        100.0 * down / reach,
        100.0 * across / reach
    );
    println!(
        "  the control is the lateral push: a law that moved a spine along itself would make gravity the larger of the two"
    );
    down / reach < 0.15 && across / reach > 0.9
}

/// The weight grows as `t²`: at the middle knot the displacement is that knot's fraction squared.
fn the_weight_is_t_squared() -> bool {
    let plain = under(Vec::new());
    let pushed = under(vec![wind(0.15)]);
    let root = Handle::root();
    let before = plain.plant.spine(&root).expect("the trunk resolves");
    let after = pushed.plant.spine(&root).expect("the trunk resolves");
    let jets = before.jets().len();
    let middle = jets / 2;
    let tip = span(
        before.jets()[jets - 1].position,
        after.jets()[jets - 1].position,
    );
    let half = span(
        before.jets()[middle].position,
        after.jets()[middle].position,
    );
    let ratio = half / tip;
    let fractions = after.knot_parameters();
    let expected = fractions[middle] * fractions[middle];
    println!("\n# the weight along a spine");
    println!(
        "  at knot {middle} of {jets} the displacement is {half:.6} against the tip's {tip:.6}, a ratio of {ratio:.6}"
    );
    println!(
        "  the knot's own fraction is {:.6} and its square is {expected:.6}",
        fractions[middle]
    );
    println!(
        "  the departure from the square is {:.2}%, which is the re-projection that keeps each segment its own length moving the knots a little further",
        100.0 * (ratio - expected).abs()
    );
    println!(
        "  the control is the fraction itself, {:.6}: a linear weight would read that",
        fractions[middle]
    );
    (ratio - expected).abs() < 2e-2 && ratio < 0.6
}

/// A push bends the spine: every segment keeps the length it had.
fn a_bend_does_not_stretch() -> bool {
    let plain = under(Vec::new());
    let pushed = under(vec![wind(0.15), Field::gravity(0.2)]);
    let mut worst = 0.0f32;
    let mut nodes = 0;
    for (handle, _) in plain.plant.live() {
        let before = chords(&plain.plant.spine(&handle).expect("a live spine resolves"));
        let after = chords(&pushed.plant.spine(&handle).expect("a live spine resolves"));
        for (one, other) in before.iter().zip(after.iter()) {
            worst = worst.max((one - other).abs());
        }
        nodes += 1;
    }
    println!("\n# a bend and a stretch");
    println!("  {nodes} nodes · the worst segment length change under two fields: {worst:.3e}");
    println!(
        "  the control is the displacement: both fields move every node, so a stretch would show here"
    );
    worst < 1e-5 && nodes > 0
}

/// A node that hangs from a bent spine is carried by it and still sits on its skin.
fn a_child_follows_its_parents_bend() -> bool {
    let plain = under(Vec::new());
    let pushed = under(vec![wind(0.15)]);
    let bough = Handle::root().child(0);
    let branch = bough.child(0);
    let before = plain
        .plant
        .placement(&branch)
        .expect("the branch resolves")
        .origin;
    let after = pushed
        .plant
        .placement(&branch)
        .expect("the branch resolves")
        .origin;
    let moved = span(before, after);
    let node = pushed
        .plant
        .get(&branch)
        .expect("the branch resolves")
        .effective_attachment();
    let station = node.station.clamp(0.0, 1.0);
    let skin = pushed
        .plant
        .get(&bough)
        .expect("the bough resolves")
        .taper_now()
        .at(station)
        - node.sink;
    let spine = pushed.plant.spine(&bough).expect("the bough resolves");
    let radial = span(after, spine.at(station));
    println!("\n# a child on a bent parent");
    println!(
        "  {branch} moved {moved:.6} under the field and sits {radial:.3e} from the bent bough's spine against a taper less the sink of {skin:.6}"
    );
    println!(
        "  the control is the move: a child placed against the straight spine would leave its parent's bark"
    );
    moved > 1e-3 && (radial - skin).abs() < 1e-5
}

/// A spine an author baked is not shaped by a field.
fn a_baked_spine_is_immune() -> bool {
    let mut species = bamboo(20_261_017);
    let trunk = Handle::root();
    let baked = Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.05, 0.7, 0.0]),
        Jet::new([0.05, 0.9, 0.02], [0.02, 0.6, 0.01]),
    ])
    .expect("the baked spine is a curve");
    let report = apply(
        &mut species.plant,
        &[Edit::new(
            trunk.clone(),
            Change::Spine { jets: baked.jets() },
        )],
    );
    let without = species.plant.spine(&trunk).expect("the trunk resolves");
    let mut pushed = species.clone();
    pushed.plant = species
        .plant
        .clone()
        .with_fields(vec![wind(0.5), Field::gravity(0.5)]);
    let with = pushed.plant.spine(&trunk).expect("the trunk resolves");
    let mut worst = 0.0f32;
    for (one, other) in without.jets().iter().zip(with.jets().iter()) {
        worst = worst.max(span(one.position, other.position));
    }
    let marked = pushed
        .plant
        .get(&trunk)
        .expect("the trunk resolves")
        .baked();
    println!("\n# a baked spine under two fields");
    println!("  {report} · the spine is marked baked: {marked}");
    println!(
        "  a half-strength push and a half-strength weight move it by {worst:.3e}: the third tier is authored from here on"
    );
    println!(
        "  the control is that strength: either field would move an authored spine by a fifth of its length"
    );
    report.clean() && marked && worst < 1e-7
}

/// Two fields on one plant compose: the second reads the first's bent spine and pushes again.
fn two_fields_compose() -> bool {
    let root = Handle::root();
    let gravity = Field::gravity(0.2);
    let light = wind(0.15);
    let plain = under(Vec::new());
    let one = under(vec![gravity]);
    let other = under(vec![light]);
    let both = under(vec![gravity, light]);
    let start = plain
        .plant
        .spine(&root)
        .expect("the trunk resolves")
        .jets()
        .last()
        .expect("a spine has knots")
        .position;
    let tip = |species: &Species| {
        species
            .plant
            .spine(&root)
            .expect("the trunk resolves")
            .jets()
            .last()
            .expect("a spine has knots")
            .position
    };
    let pushed = [subtract(tip(&one), start), subtract(tip(&other), start)];
    let sum = [
        pushed[0][0] + pushed[1][0],
        pushed[0][1] + pushed[1][1],
        pushed[0][2] + pushed[1][2],
    ];
    let together = subtract(tip(&both), start);
    let departure = span(sum, together);
    let size = length3(sum);
    println!("\n# two fields");
    println!(
        "  gravity moves the tip {:.6}, the light {:.6}, and the two together {:.6} where their sum is {:.6}",
        length3(pushed[0]),
        length3(pushed[1]),
        length3(together),
        size
    );
    println!(
        "  the composed push departs from the sum by {departure:.3e}, because the second field reads the first field's bent tangents"
    );
    println!(
        "  the control is the composition: two fields that overwrote each other would read the larger one alone"
    );
    departure / size < 0.05 && size > 0.2
}

/// Gravity droops a bough even though it cannot bend an upright trunk: the bough departs from the vertical.
fn gravity_droops_a_bough() -> bool {
    let amount = 0.2f32;
    let plain = under(Vec::new());
    let pressed = under(vec![Field::gravity(amount)]);
    let bough = Handle::root().child(0);
    let tip = |species: &Species| {
        species
            .plant
            .spine(&bough)
            .expect("the bough resolves")
            .jets()
            .last()
            .expect("a spine has knots")
            .position
    };
    let drop = tip(&plain)[1] - tip(&pressed)[1];
    let length = plain
        .plant
        .spine(&bough)
        .expect("the bough resolves")
        .length();
    let reach = amount * length;
    println!("\n# gravity on a bough");
    println!(
        "  the bough's tip dropped {drop:.6}, which is {:.2}% of a {reach:.6} reach: a limb at an angle to the weight feels it",
        drop / reach * 100.0
    );
    println!(
        "  the control is the trunk: the same field moves the upright trunk's tip by a few per cent of its reach"
    );
    drop / reach > 0.2 && drop / reach < 1.1
}

/// The ancestor field pushes a limb along the direction its parent carried, or away from it.
///
/// # Invariants
///
/// The direction is the frame's own `y`, which is the parent's tangent at the attachment, so the row measures the
/// angle between a bough's own departure and the parent's direction: a positive `toward` must **close** it (the
/// limb straightens along its parent) and a negative one must **open** it, and zero must leave it exactly as the
/// rule drew it.
fn an_ancestor_straightens() -> bool {
    let plain = under(Vec::new());
    let straight = under(vec![Field::ancestor(0.6)]);
    let opened = under(vec![Field::ancestor(-0.6)]);
    let bough = Handle::root().child(0);
    let angle = |species: &Species| -> f32 {
        let curve = species.plant.spine(&bough).expect("the bough resolves");
        let start = curve.jets()[0].tangent;
        let heading = length3(start);
        // The parent's own direction is the culm's tangent, which is the frame's `y` for every attached node.
        let up = [0.0f32, 1.0, 0.0];
        let dot = (start[0] * up[0] + start[1] * up[1] + start[2] * up[2]) / heading;
        dot.clamp(-1.0, 1.0).acos()
    };
    let (plain, straight, opened) = (angle(&plain), angle(&straight), angle(&opened));
    println!("\n# an ancestor");
    println!(
        "  the bough leaves the culm at {plain:.6} rad: toward 0.6 closes it to {straight:.6}, away opens it to {opened:.6}"
    );
    println!(
        "  the control is the sign: one direction with two signs has to move the angle two ways"
    );
    straight < plain && opened > plain
}
