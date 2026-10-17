//! Measures the hand's whole vocabulary on one plant: bake, thickness, cut, slide, graft, seat.
//!
//! The script is [`derive::sculpted`], the same one the `sculpted` scene draws, so this probe and that frame are one
//! claim seen twice: `.agents/proposals/agentic-vegetation-generation.md`'s three tiers — rule, nudge, bake — plus
//! the generator's own graft, composed on one plant rather than measured tier by tier.
//!
//! ```text
//! cargo run -p whiting-vegetation --example sculpt_probe
//! ```

use shape::math::span;
use shape::mesh::{boundary_loops, euler_characteristic, signed_volume};
use whiting_vegetation::decorator::placed;
use whiting_vegetation::derive::sculpted;
use whiting_vegetation::expand::{Detail, expand};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::node::Prune;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("the script lands clean", the_script_lands_clean()));
    checks.push(("a bake is drawn as authored", a_bake_is_drawn_as_authored()));
    checks.push(("a thickness is on the skin", a_thickness_is_on_the_skin()));
    checks.push(("a tombstone still resolves", a_tombstone_still_resolves()));
    checks.push(("a graft is filled at once", a_graft_is_filled_at_once()));
    checks.push(("a sculpted plant is a plant", a_sculpted_plant_is_a_plant()));
    checks.push(("a seat rides the graft", a_seat_rides_the_graft()));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<32} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

fn bough(index: u32) -> Handle {
    Handle::root().child(index)
}

/// Every edit landed and nothing was refused.
fn the_script_lands_clean() -> bool {
    let sculpted = sculpted(20_261_017);
    let live = sculpted.species.plant.live().count();
    let walked = sculpted.species.plant.walk().count();
    println!("\n# the script");
    println!(
        "  {} · the graft landed at {} · {live} live nodes of {walked} walked",
        sculpted.report, sculpted.grafted
    );
    println!(
        "  the control is the count: ten nodes grew, one was grafted, one was cut — a refusal would leave the walk and the live tree the same size"
    );
    sculpted.report.clean() && walked == 11 && live == 10
}

/// The baked spine is the one the hand wrote, not a rule's.
fn a_bake_is_drawn_as_authored() -> bool {
    let sculpted = sculpted(20_261_017);
    let drawn = sculpted
        .species
        .plant
        .get(&bough(0))
        .expect("the bough resolves")
        .spine_now();
    let mut worst = 0.0f32;
    for (one, other) in sculpted.authored.jets().iter().zip(drawn.jets().iter()) {
        worst = worst.max(span(one.position, other.position));
    }
    let baked = sculpted
        .species
        .plant
        .get(&bough(0))
        .expect("the bough resolves")
        .baked();
    println!("\n# the bake");
    println!(
        "  the bough's drawn spine is the authored one to {worst:.3e}, and the spine is marked baked: {baked}"
    );
    println!(
        "  the control is the shape: the rule's bough was a different curve, so a bake that did not land would read metres"
    );
    worst < 1e-6 && baked
}

/// A radius nudge is drawn on the skin the children hang from.
fn a_thickness_is_on_the_skin() -> bool {
    let plain = whiting_vegetation::derive::bamboo(20_261_017);
    let sculpted = sculpted(20_261_017);
    let target = bough(0).child(0);
    let before = plain
        .plant
        .get(&target)
        .expect("the branch resolves")
        .taper_now()
        .at(0.0);
    let after = sculpted
        .species
        .plant
        .get(&target)
        .expect("the branch resolves")
        .taper_now()
        .at(0.0);
    println!("\n# a thickness");
    println!(
        "  the branch's base went {before:.6} → {after:.6}, which is {:+.6}",
        after - before
    );
    println!(
        "  the control is the authored radius: a nudge that replaced one instead of adding to it would read 0.001"
    );
    (after - before - 0.010).abs() < 1e-6 && before > 0.0
}

/// A pruned branch is a tombstone: the walk names it, the drawing does not, and its handle still resolves.
fn a_tombstone_still_resolves() -> bool {
    let sculpted = sculpted(20_261_017);
    let cut = bough(2).child(1);
    let named = sculpted
        .species
        .plant
        .walk()
        .any(|(handle, _)| handle == cut);
    let drawn = sculpted
        .species
        .plant
        .live()
        .any(|(handle, _)| handle == cut);
    let resolves =
        sculpted.species.plant.get(&cut).is_ok() && sculpted.species.plant.spine(&cut).is_ok();
    let reason = sculpted
        .species
        .plant
        .get(&cut)
        .expect("the pruned node resolves")
        .pruned();
    println!("\n# a tombstone");
    println!(
        "  {cut}: named by the walk {named}, drawn by the plant {drawn}, still resolves {resolves}, its reason {reason:?}"
    );
    println!(
        "  the control is the pair: a prune that dropped the node would lose the handle an author's next edit is aimed at"
    );
    named && !drawn && resolves && reason == Some(Prune::ByAuthor)
}

/// The grafted branch is the rule's own, and the leaf field fills it without being told.
fn a_graft_is_filled_at_once() -> bool {
    let sculpted = sculpted(20_261_017);
    let mine = sculpted
        .species
        .plant
        .spine(&sculpted.grafted)
        .expect("the grafted node resolves")
        .length();
    let theirs = sculpted
        .species
        .plant
        .spine(&bough(1).child(0))
        .expect("the sibling resolves")
        .length();
    let expansion = expand(
        &sculpted.species.plant,
        Detail::DEFAULT,
        &sculpted.species.fields,
    );
    let leaves = expansion
        .leaves
        .iter()
        .find(|group| group.node == sculpted.grafted)
        .map(|group| group.leaves.len())
        .unwrap_or(0);
    println!("\n# the graft");
    println!(
        "  {} is {mine:.6} long against its sibling's {theirs:.6}, and the field filled it with {leaves} leaves",
        sculpted.grafted
    );
    println!(
        "  the control is the fill: a graft the field did not see would be a branch nobody draws"
    );
    (mine - theirs).abs() < 1e-6 && leaves > 0
}

/// Every solid the plant now draws is still a closed solid with a volume, and the leaves still group.
fn a_sculpted_plant_is_a_plant() -> bool {
    let sculpted = sculpted(20_261_017);
    let expansion = expand(
        &sculpted.species.plant,
        Detail::DEFAULT,
        &sculpted.species.fields,
    );
    let live = sculpted.species.plant.live().count();
    let mut closed = 0;
    let mut smallest = f32::MAX;
    for solid in &expansion.wood {
        let surface = solid.loft.view();
        let volume = signed_volume(surface);
        if boundary_loops(surface) == 0
            && shape::mesh::is_manifold(surface)
            && euler_characteristic(surface) == 2
            && volume > 0.0
        {
            closed += 1;
            smallest = smallest.min(volume);
        }
    }
    let leaves: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    println!("\n# a sculpted plant");
    println!(
        "  {live} live nodes · {} solids, {closed} closed at χ = 2 · smallest volume {smallest:.3e} · {leaves} leaves in {} groups",
        expansion.wood.len(),
        expansion.leaves.len()
    );
    println!(
        "  the control is the bake: a spine the hand wrote is still swept by the same section and taper"
    );
    closed == live && expansion.wood.len() == live && leaves > 0 && smallest > 0.0
}

/// The decorator's seat rides the grafted branch, on its own skin.
fn a_seat_rides_the_graft() -> bool {
    let sculpted = sculpted(20_261_017);
    let instances = placed(&sculpted.species.plant, &sculpted.seats);
    let skin = {
        let node = sculpted
            .species
            .plant
            .get(&sculpted.grafted)
            .expect("the graft resolves");
        node.taper_now().at(0.05) - sculpted.seats[0].sink
    };
    let spine = sculpted
        .species
        .plant
        .spine(&sculpted.grafted)
        .expect("the graft resolves");
    let radial = instances
        .first()
        .map(|instance| span(instance.placement.origin, spine.at(0.05)))
        .unwrap_or(f32::MAX);
    println!("\n# a seat on a graft");
    println!(
        "  the knot sits {radial:.3e} from the grafted branch's spine against a taper less the sink of {skin:.3e}"
    );
    println!(
        "  the control is the node: a seat stored against the plant would not follow a node the hand had added"
    );
    instances.len() == 1 && (radial - skin).abs() < 1e-5
}
