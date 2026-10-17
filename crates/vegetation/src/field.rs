//! The forces that shape a grown plant: V5's stage 6, between the hand's edits and expansion.
//!
//! # The one idea
//!
//! **A field is a push on a curve, and the plant is what all of them together make.** It reads the spine the
//! rule and the hand left, the frame that spine is drawn in, and returns a new spine — so a field never holds
//! state, never edits the tree, and never needs to know what kind of plant it is shaping. Two fields on one
//! plant apply one after the other, each reading the last one's result, which is what makes gravity and a
//! light direction compose without either knowing about the other.
//!
//! # Two ways to say where a push comes from
//!
//! A [`Field::Tropism`] is a push toward a **direction**, the same everywhere: gravity, a light, a prevailing
//! wind. A [`Field::Obstacle`] is a push out of **a place**, and it is a function rather than a shape because the
//! crate does not know what the place is: a caller with a `terrain::HeightField` passes its sampler, a caller with
//! a building passes its mesh's signed distance. Both families push with the same weight and the same bend, which
//! is why they are one enum and one pipeline rather than two subsystems.
//!
//! # A push, not a stretch
//!
//! The displacement grows as `t²` along the spine: nothing at the base, so a branch leaves its parent in the
//! direction the rule gave it, and everything at the tip. The **segment lengths are then restored**, so the
//! field bends the limb rather than pulling it longer — a droop that also grew the branch would be a field
//! doing two things, and a plant's proportions are the rule's business, not the wind's.
//!
//! # What a field may not touch
//!
//! A spine an author **baked** is not shaped: the third tier of the design says an authored spine is authored
//! from here on. [`crate::plant::Plant::shape`] is the one place that reads the marker, so every field inherits
//! the rule without repeating it.

use shape::math::{dot3, length3, subtract};
use shape::{Curve, Jet};

use crate::plant::Placement;

/// The step [`outward`] measures an occupancy's gradient with, in the plant's units.
///
/// One millimetre: small enough that a wall's gradient is a wall's, large enough that `f32`'s own noise in a
/// sampled occupancy does not become a direction.
const GRADIENT_STEP: f32 = 1.0e-3;

/// A force that shapes a plant, expressed in the world and applied in the frame a spine is drawn in.
#[derive(Debug, Clone, Copy)]
pub enum Field {
    /// A push toward a world direction, growing along each spine: gravity, a light a plant grows toward, a
    /// prevailing wind.
    ///
    /// Gravity and phototropism are one field with two directions, which is the whole reason this is a variant
    /// of one family and not two: a scene that adds a window light adds a `Tropism`, not a subsystem.
    Tropism {
        /// The world direction the push goes toward. It is normalised, so only its direction matters.
        toward: [f32; 3],
        /// How far a spine's tip is pushed, **as a fraction of the spine's own length**, so a droop is the same
        /// shape at every scale: a shrub's twig and a tree's bough bend alike, and no limb is folded by a force
        /// meant for a branch ten times its size.
        amount: f32,
    },
    /// A push out of something solid: a wall, a building's shell, the space a neighbour already occupies.
    ///
    /// # Invariants
    ///
    /// The occupancy is a **signed distance in the world**: negative inside the obstacle, positive outside, and
    /// the magnitude is the distance to its surface. A spine is pushed along the occupancy's own gradient by the
    /// penetration the knot stands in, so a branch that only grazes a wall is moved a little and one that is
    /// buried in it is moved a lot — and the push is measured where the wood *is*, which is why this is the one
    /// field that has to be given the world's own geometry rather than a direction.
    ///
    /// **A function pointer, not a closure**, because a field is `Copy` and comparable: a caller with state to
    /// carry keeps it in the scene and calls [`crate::contact::collide`] with a closure instead, which is the same
    /// occupancy read as a report rather than as a force.
    Obstacle {
        /// The occupancy, evaluated at a world point: negative inside, positive outside.
        occupancy: fn([f32; 3]) -> f32,
        /// How much of the penetration the spine is pushed out by. `1.0` puts the spine on the surface, `1.2`
        /// leaves a fifth of the penetration as clearance, and `0.0` declares an obstacle and moves nothing —
        /// which is how a scene asks *"what does this hit?"* without shaping the plant to answer.
        amount: f32,
    },
}

/// Two fields are equal when they are the same law with the same numbers.
///
/// # Invariants
///
/// An obstacle is compared by **the function's own address**, which is the only thing a function pointer has: a
/// field carrying a second occupancy is a different field even when the two functions agree everywhere, and this
/// says so rather than guessing from behaviour.
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Field::Tropism {
                    toward: one,
                    amount: first,
                },
                Field::Tropism {
                    toward: two,
                    amount: second,
                },
            ) => one == two && first == second,
            (
                Field::Obstacle {
                    occupancy: one,
                    amount: first,
                },
                Field::Obstacle {
                    occupancy: two,
                    amount: second,
                },
            ) => std::ptr::fn_addr_eq(*one, *two) && first == second,
            _ => false,
        }
    }
}

impl Field {
    /// The weight of the world: a push toward `-y`.
    pub fn gravity(amount: f32) -> Self {
        Field::Tropism {
            toward: [0.0, -1.0, 0.0],
            amount,
        }
    }

    /// A push toward a light, a window, or the sun.
    pub fn light(toward: [f32; 3], amount: f32) -> Self {
        Field::Tropism { toward, amount }
    }

    /// A push out of whatever `occupancy` calls solid.
    pub fn obstacle(occupancy: fn([f32; 3]) -> f32, amount: f32) -> Self {
        Field::Obstacle { occupancy, amount }
    }

    /// The spine this field makes of `curve`, which is drawn in the frame `placement` puts in the world.
    ///
    /// # Invariants
    ///
    /// - The base of the spine does not move: the weight is `t²`, which is zero and flat at `t = 0`.
    /// - **Every segment keeps the length it had**, so the push bends the spine and cannot stretch it.
    /// - The initial normal is the input's, carried: a field turns the spine, and a section's roll is not the
    ///   field's to change.
    /// - **A field whose push is zero everywhere returns the curve untouched**, so a plant with no obstacle in
    ///   reach is bit-identical to the plant without the field. That is the property a scene relies on to add a
    ///   field and keep a picture it already has.
    pub fn shape(&self, curve: &Curve, placement: &Placement) -> Curve {
        if curve.length() <= 0.0 {
            return curve.clone();
        }
        let offsets = self.offsets(curve, placement);
        if offsets.iter().all(|offset| *offset == [0.0; 3]) {
            return curve.clone();
        }
        let jets = curve.jets();
        let mut points: Vec<[f32; 3]> = jets.iter().map(|jet| jet.position).collect();
        let chords: Vec<f32> = points
            .windows(2)
            .map(|pair| length3(subtract(pair[1], pair[0])))
            .collect();
        for (index, offset) in offsets.iter().enumerate() {
            for axis in 0..3 {
                points[index][axis] += offset[axis];
            }
        }
        keep_lengths(&mut points, &chords);
        let tangents = central_differences(&points);
        let shaped: Vec<Jet> = points
            .into_iter()
            .zip(tangents)
            .map(|(position, tangent)| Jet::new(position, tangent))
            .collect();
        let mut bent = Curve::new(shaped).expect("a bent valid curve is a valid curve");
        bent.set_initial_normal(curve.frame(0.0).normal);
        bent
    }

    /// What this field asks of every knot, in the frame the spine is drawn in.
    ///
    /// # Invariants
    ///
    /// Every offset is a **vector in the spine's own frame**, which is what lets the two families share one
    /// pipeline: a tropism's direction is read into that frame once, and an obstacle's gradient is read into it
    /// per knot, because an obstacle's direction is different at every place the wood stands.
    fn offsets(&self, curve: &Curve, placement: &Placement) -> Vec<[f32; 3]> {
        let jets = curve.jets();
        let fractions = curve.knot_parameters();
        let mut offsets = vec![[0.0f32; 3]; jets.len()];
        match *self {
            Field::Tropism { toward, amount } => {
                let size = length3(toward);
                if amount == 0.0 || size < 1.0e-9 {
                    return offsets;
                }
                let local =
                    placement.unrotate([toward[0] / size, toward[1] / size, toward[2] / size]);
                let reach = amount * curve.length();
                for (index, fraction) in fractions.iter().enumerate() {
                    let direction = jets[index].tangent;
                    let heading = length3(direction);
                    let weight = fraction * fraction;
                    if heading < 1.0e-9 || weight == 0.0 {
                        continue;
                    }
                    let unit = [
                        direction[0] / heading,
                        direction[1] / heading,
                        direction[2] / heading,
                    ];
                    // **Only the part across the spine bends it**: a limb along the field is compressed rather
                    // than pulled, so a trunk under gravity does not leave the ground and a branch pointing at
                    // the sun does not grow longer.
                    let along = dot3(local, unit);
                    for axis in 0..3 {
                        offsets[index][axis] = (local[axis] - unit[axis] * along) * reach * weight;
                    }
                }
            }
            Field::Obstacle { occupancy, amount } => {
                if amount == 0.0 {
                    return offsets;
                }
                for (index, jet) in jets.iter().enumerate() {
                    let weight = fractions[index] * fractions[index];
                    if weight == 0.0 {
                        continue;
                    }
                    let world = placement.place(jet.position);
                    let value = occupancy(world);
                    if value >= 0.0 {
                        continue;
                    }
                    let Some(out) = outward(occupancy, world) else {
                        continue;
                    };
                    let local = placement.unrotate(out);
                    for axis in 0..3 {
                        offsets[index][axis] = local[axis] * -value * amount * weight;
                    }
                }
            }
        }
        offsets
    }
}

/// The direction an occupancy grows in at a point, from central differences on it.
///
/// # Invariants
///
/// **A direction of no length is no direction**: a flat occupancy (a constant, or the inside of a slab measured
/// exactly) answers `None`, and a spine with nothing to be pushed out of is left where it is rather than pushed
/// somewhere arbitrary.
fn outward(occupancy: fn([f32; 3]) -> f32, point: [f32; 3]) -> Option<[f32; 3]> {
    let mut gradient = [0.0f32; 3];
    for axis in 0..3 {
        let mut low = point;
        let mut high = point;
        low[axis] -= GRADIENT_STEP;
        high[axis] += GRADIENT_STEP;
        gradient[axis] = occupancy(high) - occupancy(low);
    }
    let size = length3(gradient);
    if size < 1.0e-9 {
        return None;
    }
    Some([gradient[0] / size, gradient[1] / size, gradient[2] / size])
}

/// Moves the points so every segment is as long as `chords` says it was.
///
/// # Invariants
///
/// The first point stays where it is and each next one is placed along the displaced segment's own direction,
/// so the spine's shape is what changed and its proportions are not.
fn keep_lengths(points: &mut [[f32; 3]], chords: &[f32]) {
    for index in 0..chords.len() {
        let step = subtract(points[index + 1], points[index]);
        let size = length3(step);
        let direction = if size > 1.0e-9 {
            [step[0] / size, step[1] / size, step[2] / size]
        } else {
            [0.0; 3]
        };
        for axis in 0..3 {
            points[index + 1][axis] = points[index][axis] + direction[axis] * chords[index];
        }
    }
}

/// Each knot's tangent, from the points either side of it.
///
/// # Invariants
///
/// The knots are one unit of the knot parameter apart, so the difference over both neighbours divided by two is
/// the derivative there; the ends use the one side they have. This is the same convention `Curve::new` takes —
/// a jet's tangent is the derivative with respect to the knot parameter — so a displaced spine is a curve whose
/// tangents are its own geometry rather than the old tangents moved.
fn central_differences(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut tangents = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let low = index.saturating_sub(1);
        let high = (index + 1).min(points.len() - 1);
        let span = (high - low) as f32;
        if span == 0.0 {
            tangents.push([0.0; 3]);
            continue;
        }
        let step = subtract(points[high], points[low]);
        tangents.push([step[0] / span, step[1] / span, step[2] / span]);
    }
    tangents
}
