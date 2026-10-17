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

/// A force that shapes a plant, expressed in the world and applied in the frame a spine is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// The spine this field makes of `curve`, which is drawn in the frame `placement` puts in the world.
    ///
    /// # Invariants
    ///
    /// - The base of the spine does not move: the weight is `t²`, which is zero and flat at `t = 0`.
    /// - **The push is what the spine feels across it**: only the part of the field perpendicular to each knot's
    ///   own tangent displaces it. A limb along the field is compressed rather than pulled, so a trunk under
    ///   gravity does not leave the ground and a branch pointing at the sun does not grow longer — a force that
    ///   moved a spine along itself would be a stretch wearing a bend's clothes.
    /// - The tip moves by `amount ×` the spine's length times the sine of the angle between them, because the
    ///   weight there is one and the across component is a unit push times that sine.
    /// - **Every segment keeps the length it had**, so the push bends the spine and cannot stretch it.
    /// - The initial normal is the input's, carried: a field turns the spine, and a section's roll is not the
    ///   field's to change.
    pub fn shape(&self, curve: &Curve, placement: &Placement) -> Curve {
        let Field::Tropism { toward, amount } = *self;
        let length = curve.length();
        if amount == 0.0 || length <= 0.0 {
            return curve.clone();
        }
        let local = [
            dot3(toward, placement.x),
            dot3(toward, placement.y),
            dot3(toward, placement.z),
        ];
        let size = length3(local);
        if size < 1.0e-6 {
            return curve.clone();
        }
        let push = [local[0] / size, local[1] / size, local[2] / size];
        let reach = amount * length;
        let jets = curve.jets();
        let fractions = curve.knot_parameters();
        let mut points: Vec<[f32; 3]> = jets.iter().map(|jet| jet.position).collect();
        let chords: Vec<f32> = points
            .windows(2)
            .map(|pair| length3(subtract(pair[1], pair[0])))
            .collect();
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
            let along = dot3(push, unit);
            for axis in 0..3 {
                let across = push[axis] - unit[axis] * along;
                points[index][axis] += across * reach * weight;
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
            [0.0, 0.0, 0.0]
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
