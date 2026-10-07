//! Where a massif comes from when nobody has authored one yet.
//!
//! # What a derivation is worth
//!
//! Nothing carries a correctness guarantee. The purpose of a generated massif is to give the
//! author — human or agent — something concrete to edit, which is exactly what R3 asks for.
//! Once generated, its lines are ordinary editable lines and nothing records that they were
//! derived.
//!
//! # The shape that is generated
//!
//! A **fan**: ridges radiating from one summit to a ring of tips, with a valley wedged between
//! each neighbouring pair, and a footprint threaded through the tips and the valley mouths. It
//! is the one ridge arrangement whose correspondence can be written down in closed form — the
//! two ridges a wedge lies between are its angular neighbours, and the point of a straight
//! radial ridge that a valley node faces is that node's projection onto the ridge — so it is
//! the arrangement a derivation can build and be right about.
//!
//! Spur ridges hanging off a parent are **not** generated. The model expresses them — `Attach`
//! and a station on each side — and an author builds them, but which wedge a spur splits is a
//! measurement of the ridge tree's planar embedding rather than a fact about its topology, and
//! a derivation that guessed it would produce lines the author has to check rather than lines
//! the author can start editing.

use std::f32::consts::TAU;

use crate::Massif;
use crate::line::{LineRef, Station};
use crate::math::{Vec2, distance, offset};
use crate::skeleton::{
    FootprintNode, Ridge, RidgeId, RidgeNode, Skeleton, Valley, ValleyId, ValleyNode,
};
use crate::tessellation::Tessellation;

/// How a massif is generated.
///
/// # Invariants
///
/// Every field is a number the author would otherwise be choosing by hand, and every one of them
/// is reachable afterwards through an edit. Nothing here is a shape the model cannot express; it
/// is a starting point, spelled out so that two runs with the same value produce the same
/// massif.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Starter {
    /// How many ridges radiate from the summit.
    pub ridges: usize,
    /// How many nodes each ridge is authored with.
    pub nodes: usize,
    /// The plan distance from the summit to a ridge tip.
    pub reach: f32,
    /// The elevation of the summit.
    pub summit: f32,
    /// The elevation a ridge reaches at its tip.
    pub tip: f32,
    /// How much further out a valley mouth sits than the corner behind it.
    pub bulge: f32,
    /// The bearing the first ridge runs along, in radians.
    pub bearing: f32,
    /// How far round the circle the ridge set is spread, in radians.
    ///
    /// # Invariants
    ///
    /// It is a **full turn**, and the ridges are placed at `index / count` of it rather than
    /// across its ends. A fan spread over less than a turn leaves a closing wedge between the
    /// last ridge and the first that is wider than the mountain, and the corner of that wedge —
    /// which is `reach / cos` of its half angle — is then hundreds of units out. That is a
    /// footprint with a spike on the back of it, and the number that would have warned the author
    /// is one they had no reason to expect. A massif with a front and a back is made by editing
    /// the ridges, where the effect is visible.
    pub spread: f32,
    /// How far a ridge's bearing may wander from its even place, in radians.
    pub irregularity: f32,
    /// How far a ridge's tip elevation may differ from `tip`, in proportion.
    pub relief: f32,
    /// The seed the irregularity and the relief are read from.
    pub seed: i64,
}

impl Default for Starter {
    /// Returns a fan of seven ridges across a little over half a turn.
    ///
    /// Seven is enough that two neighbouring wedges read as different slopes rather than as one
    /// surface with a line on it, and few enough that the massif is a massif rather than a cone.
    /// The spread is under a full turn because a fan that closes on itself is a star seen from
    /// directly overhead, and a massif is looked at from the side.
    fn default() -> Self {
        Self {
            ridges: 7,
            nodes: 13,
            reach: 120.0,
            summit: 95.0,
            tip: 6.0,
            bulge: 0.06,
            bearing: -2.55,
            spread: TAU,
            irregularity: 0.26,
            relief: 0.55,
            seed: 11,
        }
    }
}

/// Builds a massif from nothing.
///
/// # Invariants
///
/// The seed reaches the bearings and the tip elevations and nothing else. The *shape* — how many
/// ridges, how far they reach, how the footprint closes — is the caller's, so changing the seed
/// changes the mountain rather than the kind of thing the mountain is.
pub fn starter(seed: i64, params: Starter) -> Massif {
    let mut params = params;
    params.seed = seed;
    Massif::new(fan(params)).with_tessellation(Tessellation::default())
}

impl Massif {
    /// Builds a massif from nothing, which is the same as [`starter`].
    pub fn starter(seed: i64, params: Starter) -> Self {
        starter(seed, params)
    }

    /// Regenerates this massif's valleys from its ridges, which is the same as
    /// [`derive_valleys`].
    pub fn derive_valleys(&mut self) -> usize {
        derive_valleys(self)
    }
}

/// Regenerates the valleys of a massif from its ridges.
///
/// # Invariants
///
/// It **replaces** every valley rather than merging with the existing ones, which makes it a
/// "start over" operation and not an incremental one: a derivation has no way to know which parts
/// of an edited valley the author meant to keep. It is therefore the answer to "the valleys are
/// wrong", never to "the valleys are nearly right".
///
/// The domain is a fan: ridges whose tips are in angular order around one summit. A skeleton
/// outside that domain keeps the valleys it had, and the count of ridges that could not be placed
/// is returned, because rewriting valleys from a measurement the skeleton does not support would
/// replace an edited structure with a guess.
pub fn derive_valleys(massif: &mut Massif) -> usize {
    let Some(fan) = Fan::read(&massif.skeleton) else {
        return massif.skeleton.ridges.len();
    };
    massif.skeleton.valleys = fan.valleys();
    0
}

/// A ridge fan: one summit, one set of bearings, one reach and one node count.
///
/// # Invariants
///
/// Every ridge has the **same reach and the same node count**, and that is what makes the wedge
/// walk closed form. A wedge's corner is where both of its stations reach the tips, and with two
/// different reaches the two stations reach their tips at two different points along the
/// bisector, so the corner would have to be chosen rather than computed. Unequal ridges are an
/// edit an author makes afterwards, where they can see what it does.
struct Fan {
    /// Where every ridge starts.
    summit: Vec2,
    /// The elevation at the summit.
    summit_elevation: f32,
    /// The plan distance from the summit to every tip.
    reach: f32,
    /// How many nodes every ridge and every valley is authored with.
    nodes: usize,
    /// How much further out a mouth sits than the corner behind it.
    bulge: f32,
    /// Each ridge's bearing, in the order the wedges read them.
    bearings: Vec<f32>,
    /// Each ridge's tip elevation.
    tips: Vec<f32>,
}

impl Fan {
    /// Reads a fan off a skeleton, or nothing when the skeleton is not one.
    ///
    /// # Invariants
    ///
    /// A fan is recognised by facts and by no measurement: every ridge starts at one point, the
    /// ridges' tip bearings advance in the order of their ids, and every ridge is a straight
    /// radial line. The first two are what make a wedge's ridges angular neighbours; the third is
    /// what makes the projection a closed form rather than an approximation. A skeleton failing
    /// any of them is left alone rather than refused, because the author may have meant it.
    fn read(skeleton: &Skeleton) -> Option<Self> {
        if skeleton.ridges.len() < 3 {
            return None;
        }
        let summit = skeleton.ridges[0].nodes.first()?.position;
        let nodes = skeleton.ridges[0].nodes.len();
        let mut bearings = Vec::with_capacity(skeleton.ridges.len());
        let mut tips = Vec::with_capacity(skeleton.ridges.len());
        let mut previous: Option<f32> = None;
        let mut reach = 0.0f32;
        for ridge in &skeleton.ridges {
            if ridge.nodes.len() != nodes {
                return None;
            }
            let start = ridge.nodes.first()?;
            let end = ridge.nodes.last()?;
            if distance(start.position, summit) > crate::sampling::WELD {
                return None;
            }
            let bearing = (end.position[1] - summit[1]).atan2(end.position[0] - summit[0]);
            let spoke = distance(summit, end.position);
            if spoke <= f32::EPSILON {
                return None;
            }
            if reach <= f32::EPSILON {
                reach = spoke;
            } else if (spoke - reach).abs() > reach * 1.0e-3 {
                return None;
            }
            // Every node in line with the two ends: a ridge that bends is not the straight radial
            // line the projection is exact for.
            for node in &ridge.nodes {
                let offset = crate::math::difference(node.position, summit);
                let along = crate::math::dot(offset, [bearing.cos(), bearing.sin()]);
                let across = crate::math::cross([bearing.cos(), bearing.sin()], offset);
                if across.abs() > reach * 1.0e-3 || along < -reach * 1.0e-3 {
                    return None;
                }
            }
            if let Some(previous) = previous {
                let advance = wrap(bearing - previous);
                if advance <= 0.0 {
                    return None;
                }
            }
            previous = Some(bearing);
            bearings.push(bearing);
            tips.push(end.elevation);
        }
        Some(Self {
            summit,
            summit_elevation: skeleton.ridges[0].nodes[0].elevation,
            reach,
            nodes,
            bulge: Starter::default().bulge,
            bearings,
            tips,
        })
    }

    /// Returns every ridge of the fan.
    fn ridges(&self) -> Vec<Ridge> {
        (0..self.bearings.len())
            .map(|index| {
                let direction = self.direction(self.bearings[index]);
                let ridge_nodes: Vec<RidgeNode> = (0..self.nodes)
                    .map(|step| {
                        let fraction = step as f32 / (self.nodes - 1) as f32;
                        RidgeNode::new(
                            offset(self.summit, scale(direction, self.reach * fraction)),
                            self.ridge_elevation(index, fraction),
                        )
                        .with_half_width(self.reach * 0.05)
                        .with_sharpness(0.55)
                    })
                    .collect();
                Ridge::root(ridge_nodes)
            })
            .collect()
    }

    /// Returns every valley of the fan, one per neighbouring pair.
    fn valleys(&self) -> Vec<Valley> {
        let wedges = self.wedges();
        let outline = self.footprint(&wedges);
        self.bound(wedges, &outline)
    }

    /// Returns every wedge of the fan, in ridge order.
    fn wedges(&self) -> Vec<Wedge> {
        (0..self.bearings.len())
            .map(|index| self.wedge(index, (index + 1) % self.bearings.len()))
            .collect()
    }

    /// Binds each wedge's mouth to the rim that was threaded through it.
    ///
    /// # Invariants
    ///
    /// The mouths are placed before the rim exists and the stations naming the rim cannot be
    /// written until it does, so the rim is built first and its arc-length parameters are read
    /// back into the valleys here. The rim alternates tip and mouth, so wedge `index`'s mouth is
    /// vertex `2 * index + 1` of it.
    fn bound(&self, wedges: Vec<Wedge>, outline: &[FootprintNode]) -> Vec<Valley> {
        let mut spans = Vec::with_capacity(outline.len());
        let mut total = 0.0;
        for (index, node) in outline.iter().enumerate() {
            spans.push(total);
            total += distance(node.position, outline[(index + 1) % outline.len()].position);
        }
        wedges
            .into_iter()
            .enumerate()
            .map(|(index, wedge)| {
                let parameter = if total <= f32::EPSILON {
                    0.0
                } else {
                    spans[(2 * index + 1).min(spans.len() - 1)] / total
                };
                wedge.into_valley(parameter)
            })
            .collect()
    }

    /// Returns the rim, threaded through the tips and the mouths.
    ///
    /// # Invariants
    ///
    /// The rim alternates tip and mouth and the mouths sit **further out than the corners behind
    /// them**, which is what makes it a simple loop: a mouth drawn inside the chord between its
    /// two tips would fold the boundary back on itself, and the thing the author would then be
    /// asked to fix is the footprint rather than the mountain.
    ///
    /// The loop is wound so that the inside is on its **left**. The expansion winds every face to
    /// look up, so a rim wound the other way would put the mesh's plan area at the negation of the
    /// footprint's and make the coverage check read a covered massif as inside out.
    fn footprint(&self, wedges: &[Wedge]) -> Vec<FootprintNode> {
        let mut outline = Vec::with_capacity(self.bearings.len() * 2);
        for (index, wedge) in wedges.iter().enumerate() {
            outline.push(FootprintNode::new(
                offset(
                    self.summit,
                    scale(self.direction(self.bearings[index]), self.reach),
                ),
                self.tips[index],
            ));
            outline.push(FootprintNode::new(wedge.mouth, wedge.mouth_elevation));
        }
        outline
    }

    /// Returns the wedge between two neighbouring ridges.
    ///
    /// # Invariants
    ///
    /// Every station is a **projection and not a search**: a node stands on the bearing halfway
    /// between the two ridges, so the point of a straight radial ridge nearest to it is the one at
    /// `cos` of the half angle, and the node whose two stations have both reached their tips is
    /// the one at `reach / cos`. A derivation that searched for the nearest point would need a
    /// measurement per node and would still be wrong for the fan it was handed.
    ///
    /// The last two nodes are what closes the wedge: the **corner**, whose two ribs reach the two
    /// tips, and the **mouth** on the rim. Between them the two belts share the valley's last
    /// segment and cover the rim arc from one tip to the other, so the region bounded by the two
    /// tips and the corner is covered rather than left as a hole between four faces.
    fn wedge(&self, left: usize, right: usize) -> Wedge {
        let separation = wrap(self.bearings[right] - self.bearings[left]) * 0.5;
        let middle = self.bearings[left] + separation;
        let direction = self.direction(middle);
        let cosine = separation.cos().max(1.0e-3);
        let corner = self.reach / cosine;
        let mouth = corner.max(self.reach) * (1.0 + self.bulge);
        let steps = self.nodes.saturating_sub(1).max(2);

        let mut nodes = Vec::with_capacity(steps + 2);
        nodes.push((
            self.summit,
            self.summit_elevation,
            Station::ridge(RidgeId(left), 0.0),
            Station::ridge(RidgeId(right), 0.0),
        ));
        for step in 1..=steps {
            let fraction = step as f32 / steps as f32;
            nodes.push((
                offset(self.summit, scale(direction, corner * fraction)),
                self.valley_elevation(fraction),
                Station::ridge(RidgeId(left), fraction),
                Station::ridge(RidgeId(right), fraction),
            ));
        }
        Wedge {
            mouth: offset(self.summit, scale(direction, mouth)),
            mouth_elevation: self.valley_elevation(1.0),
            nodes,
        }
    }

    /// Returns the elevation a ridge has at a fraction of its run.
    ///
    /// # Invariants
    ///
    /// The fraction is raised to a power **below one**, so the ridge loses most of its height
    /// early and then runs out nearly level. A crest is steep near its summit and eases into its
    /// shoulder; a linear fall is a cone.
    fn ridge_elevation(&self, index: usize, fraction: f32) -> f32 {
        self.summit_elevation + (self.tips[index] - self.summit_elevation) * fraction.powf(0.75)
    }

    /// Returns the elevation a valley has at a fraction of its run.
    ///
    /// # Invariants
    ///
    /// The fraction is raised to a power **above one**, so the valley runs nearly level out of the
    /// summit and then drops. That is the inverse of the ridge's own profile, which is what puts
    /// the valley floor below the crest at every fraction rather than only at the ends.
    fn valley_elevation(&self, fraction: f32) -> f32 {
        let base = self.summit_elevation * 0.10;
        self.summit_elevation - (self.summit_elevation - base) * fraction.powf(1.7)
    }

    /// Returns the unit direction at a bearing.
    fn direction(&self, bearing: f32) -> Vec2 {
        [bearing.cos(), bearing.sin()]
    }
}

/// One wedge's plan, before its stations can name the footprint.
struct Wedge {
    /// Where the wedge's mouth stands on the rim.
    mouth: Vec2,
    /// The elevation at the mouth.
    mouth_elevation: f32,
    /// The nodes, each with the two ridge stations it faces.
    nodes: Vec<(Vec2, f32, Station, Station)>,
}

impl Wedge {
    /// Returns the wedge as a valley, with its mouth bound to the footprint.
    fn into_valley(self, mouth_parameter: f32) -> Valley {
        let mouth = Station::new(LineRef::Footprint, mouth_parameter);
        let mut nodes: Vec<ValleyNode> = self
            .nodes
            .into_iter()
            .map(|(position, elevation, left, right)| {
                ValleyNode::new(position, elevation, left, right)
            })
            .collect();
        nodes.push(ValleyNode::new(
            self.mouth,
            self.mouth_elevation,
            mouth,
            mouth,
        ));
        Valley::new(nodes)
    }
}

/// Builds the whole fan: ridges, valleys and footprint together.
///
/// # Invariants
///
/// The three are generated in one pass because they are one decision. A valley is pinned to the
/// ridge points on either side of it, so a ridge generated without the valley facing it is a ridge
/// with nothing to build against, and a footprint generated without either is a boundary around
/// lines that do not reach it.
fn fan(params: Starter) -> Skeleton {
    let count = params.ridges.max(3);
    let nodes = params.nodes.max(3);

    // The bearings are spread and then jittered **in order**, so the ridges stay in the order they
    // were laid out in and a wedge is still between its two angular neighbours. Jitter deep enough
    // to reorder them would be jitter that changes the topology of the massif.
    //
    // The even place is `index / count` of the spread and not `index / (count - 1)`: the fan is a
    // ring, so the gap from the last ridge back to the first has to be the same size as the others
    // rather than the whole of what is left.
    let bearings: Vec<f32> = (0..count)
        .map(|index| {
            let even = index as f32 / count as f32;
            params.bearing
                + params.spread * even
                + params.irregularity * (hash01(index as i64, 3, params.seed) - 0.5)
        })
        .collect();
    let tips: Vec<f32> = (0..count)
        .map(|index| {
            let relief = hash01(index as i64, 7, params.seed) - 0.5;
            (params.tip * (1.0 + params.relief * relief * 2.0) + params.summit * 0.10)
                .max(params.summit * 0.02)
        })
        .collect();

    let fan = Fan {
        summit: [0.0, 0.0],
        summit_elevation: params.summit,
        reach: params.reach,
        nodes,
        bulge: params.bulge,
        bearings,
        tips,
    };
    // One set of wedges, read once: the valleys and the rim are two readings of the same plan, so
    // building them from two separately computed sets would let the mouth a station names be a
    // different point from the mouth the rim threads through.
    let wedges = fan.wedges();
    let outline = fan.footprint(&wedges);
    let valleys = fan.bound(wedges, &outline);
    Skeleton::new(fan.ridges(), valleys, outline)
}

/// Scales a plan direction.
fn scale(direction: Vec2, factor: f32) -> Vec2 {
    [direction[0] * factor, direction[1] * factor]
}

/// Returns an angle wrapped into zero to a full turn.
fn wrap(angle: f32) -> f32 {
    let mut wrapped = angle % TAU;
    while wrapped < 0.0 {
        wrapped += TAU;
    }
    wrapped
}

/// Returns a deterministic value in zero to one from two integers and a seed.
///
/// # Invariants
///
/// The same arguments always give the same value, and the three inputs are mixed apart rather
/// than summed, so a bearing and an elevation drawn from one seed are not two readings of one
/// number. A derivation that correlated them would put every tall ridge in one place.
fn hash01(x: i64, y: i64, seed: i64) -> f32 {
    let mut value = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (seed as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    (value >> 11) as f32 / (1u64 << 53) as f32
}

/// Returns the lines a valley's two sides face.
///
/// # Invariants
///
/// It is the answer to "is this valley between the ridges I think it is", which is the question an
/// author asks after a derivation and the one a violation list does not answer.
pub fn faces(skeleton: &Skeleton, valley: ValleyId) -> Vec<LineRef> {
    let mut lines: Vec<LineRef> = skeleton.valleys[valley.0]
        .nodes
        .iter()
        .flat_map(|node| [node.left.line, node.right.line])
        .collect();
    lines.sort();
    lines.dedup();
    lines
}
