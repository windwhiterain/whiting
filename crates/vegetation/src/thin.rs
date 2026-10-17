//! Thinning: what a crown can afford when its branches crowd each other.
//!
//! # The one idea
//!
//! **A branch whose tip stands closer to a branch already kept than `spacing` is dropped.** The decision is made
//! in space and not by rank, which is the difference between thinning and a smaller `count` band: a generator that
//! overproduces and then thins can vary its numbers by seed, and the crowding — which no band knows about, because
//! it depends on where the branches actually went — is what decides. That is why this is a function over a grown
//! plant and not a number in a rule.
//!
//! # Two properties that make it usable
//!
//! - **Every decision is made before any is applied**, so the branch a thinning keeps is chosen against the plant
//!   as it was, and the list does not depend on the order the prunes land.
//! - **It is a pruning edit like any other**: the tombstones are `Prune::Thinned`, and a second call finds nothing
//!   to do because the pruned nodes are not walked. A thinning that could not be repeated would be a thinning
//!   whose report could never be checked.

use shape::Vec3;
use shape::math::{length3, subtract};

use crate::handle::Handle;
use crate::node::Prune;
use crate::ops::{Change, Edit, apply};
use crate::plant::Plant;

/// What a thinning took.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Thin {
    /// Every node the thinning pruned, in walk order.
    pub pruned: Vec<Handle>,
}

/// Drops every child whose tip stands closer than `spacing` to a sibling already kept.
///
/// # Invariants
///
/// - **The first child of a crowding group survives**, because the walk is in the children's own order and the
///   first tip has nothing kept to be close to. So a thinning is deterministic: the same plant and the same
///   spacing prune the same handles, whatever order a map happens to iterate in.
/// - **Only siblings compete.** Two branches of different parents may touch and are both kept: crowding is what a
///   parent's own children do, and a rule against cousins would need a resolution that the author has not stated.
/// - **The tips are the drawn ones**, read through [`Plant::spine`], so a branch a field bent is judged where it
///   ended up and not where the rule put it.
/// - **The prune is the same edit a hand writes**, `Prune::Thinned`, applied through [`crate::ops::apply`], so a
///   report can say what went and a caller can keep the list.
pub fn thin(plant: &mut Plant, spacing: f32) -> Thin {
    let mut edits = Vec::new();
    let mut pruned = Vec::new();
    if spacing <= 0.0 {
        return Thin { pruned };
    }
    let parents: Vec<Handle> = plant.live().map(|(handle, _)| handle).collect();
    for parent in parents {
        let count = plant
            .get(&parent)
            .map(|node| node.children().len())
            .unwrap_or(0);
        let mut kept: Vec<Vec3> = Vec::new();
        for index in 0..count {
            let child = parent.child(index as u32);
            let Ok(node) = plant.get(&child) else {
                continue;
            };
            if node.pruned().is_some() {
                continue;
            }
            let Ok(spine) = plant.spine(&child) else {
                continue;
            };
            let tip = spine.at(1.0);
            let crowded = kept
                .iter()
                .any(|other| length3(subtract(tip, *other)) < spacing);
            if crowded {
                pruned.push(child.clone());
                edits.push(Edit::new(
                    child,
                    Change::Prune {
                        reason: Prune::Thinned,
                    },
                ));
            } else {
                kept.push(tip);
            }
        }
    }
    apply(plant, &edits);
    Thin { pruned }
}

/// The closest distance between any two of a node's children's tips, or `None` when it has fewer than two.
///
/// A caller that wants to know whether a plant needs thinning asks this before it prunes anything: a minimum below
/// its spacing is exactly what [`thin`] would act on.
pub fn closest_tips(plant: &Plant, parent: &Handle) -> Option<f32> {
    let count = plant.get(parent).ok()?.children().len();
    let mut tips: Vec<Vec3> = Vec::new();
    for index in 0..count {
        let child = parent.child(index as u32);
        if plant
            .get(&child)
            .map(|node| node.pruned().is_some())
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(spine) = plant.spine(&child) else {
            continue;
        };
        tips.push(spine.at(1.0));
    }
    let mut closest: Option<f32> = None;
    for (index, tip) in tips.iter().enumerate() {
        for other in tips.iter().skip(index + 1) {
            let distance = length3(subtract(*tip, *other));
            closest = Some(match closest {
                Some(held) => held.min(distance),
                None => distance,
            });
        }
    }
    closest
}
