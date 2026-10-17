//! The hand: an edit is a handle and a change, applied in one stated order, reported and never repaired.
//!
//! # The one idea
//!
//! **Manual detail is an offset on a computed value, keyed by a handle that survives the computation.** An
//! [`Edit`] is therefore a [`Handle`] and a [`Change`] and nothing else: no index, no name, no reference to a
//! mesh, and no copy of the geometry. The number it moves is the node's own — its length, its radius, where it
//! hangs — so the authored data is untouched, the offset is recoverable, and a second edit to the same node
//! adds to the same number rather than replacing somebody else's work.
//!
//! # The order is an invariant
//!
//! [`apply`] runs in the design's order with the stages this crate has no data for left out: **prunes first,
//! then offsets and shifts**. A prune runs first so that an edit aimed at a node the same batch prunes is
//! reported as [`Problem::Pruned`] rather than written onto a tombstone where no frame would show it. Grafts,
//! spine bakes and fields are absent from this list because an authored plant has no generator to graft onto
//! and no field to resolve yet; each lands with the thing it edits.
//!
//! # Report, never repair
//!
//! [`apply`] returns an [`EditReport`]: how many edits landed, and per edit that did not, why. An edit is never
//! clamped, never skipped in silence, and never applied to the nearest node that happens to resolve — that
//! difference is what makes a tool an agent can use rather than one it has to guess about.

use shape::math::{Vec3, dot3};
use shape::{Curve, CurveError, Jet};

use crate::handle::Handle;
use crate::node::{Nudges, Prune};
use crate::plant::{Placement, Plant};
use crate::selector::Selector;

/// Which computed number an edit nudges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// The node's own length, in the plant's units.
    Length,
    /// The node's taper, at every station.
    Radius,
    /// Where the node hangs along its parent, as the parent's arc-length fraction.
    Station,
    /// Where it hangs around the parent's section, as a fraction of the loop.
    Slot,
}

impl Property {
    /// The property's name, for a report a reader sees.
    pub fn name(self) -> &'static str {
        match self {
            Property::Length => "length",
            Property::Radius => "radius",
            Property::Station => "station",
            Property::Slot => "slot",
        }
    }
}

/// The frame a [`Change::Shift`] is given in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// The node's own attachment frame: `x` outward from its parent, `y` along the parent's tip.
    Local,
    /// The parent's attachment frame, which is the frame the node's own spine is authored in.
    Parent,
    /// The world's own axes.
    World,
}

impl Frame {
    /// The frame's name, for a report a reader sees.
    pub fn name(self) -> &'static str {
        match self {
            Frame::Local => "local",
            Frame::Parent => "parent",
            Frame::World => "world",
        }
    }
}

/// What an edit does.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A nudge on one computed number: the branch is 4 mm too thin, the twig 5 cm too long.
    Offset {
        /// Which number to move.
        property: Property,
        /// How far, in that property's own units.
        by: f32,
    },
    /// A translation, given in one of the three frames.
    ///
    /// It is stored in the node's own frame and added to whatever other shifts reached it, so two shifts of
    /// one node add up and a later edit to the parent still carries this one.
    Shift {
        /// The frame `by` is written in.
        frame: Frame,
        /// How far, in that frame.
        by: Vec3,
    },
    /// A tombstone: the node stays, its handle keeps naming it, and it draws nothing.
    Prune {
        /// Why it was removed.
        reason: Prune,
    },
    /// A bake: the node's spine from here on, in the node's own frame.
    ///
    /// It **replaces** the authored curve rather than offsetting it, which is V5's third tier: the author has
    /// stopped saying *"nearly right"* and said *"this"*. A nudge on the same node still applies on top,
    /// because the tiers compose; what the bake forbids is a *field* reshaping it later, which is what
    /// [`crate::node::Node::baked`] exists for.
    Spine {
        /// The new spine as its knots, in the coordinates the node's authored spine is in: the first knot's
        /// tangent is the direction the node leaves its parent, and the knots are the whole curve.
        jets: Vec<Jet>,
    },
}

impl Change {
    /// The change's name, for a report a reader sees.
    pub fn name(&self) -> &'static str {
        match self {
            Change::Offset { .. } => "offset",
            Change::Shift { .. } => "shift",
            Change::Prune { .. } => "prune",
            Change::Spine { .. } => "spine",
        }
    }
}

/// One edit: a handle, and what to change there.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    /// The node the edit is aimed at.
    pub at: Handle,
    /// What it does there.
    pub what: Change,
}

impl Edit {
    /// An edit of one node.
    pub fn new(at: Handle, what: Change) -> Self {
        Self { at, what }
    }

    /// The change's name, for a table of edits a caller prints before applying them.
    pub fn kind(&self) -> &'static str {
        self.what.name()
    }
}

/// Why an edit did not land.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// The handle names no node.
    Orphaned {
        /// The handle the edit was aimed at.
        at: Handle,
    },
    /// The node is a tombstone: an edit on it would be invisible, so it is reported instead.
    Pruned {
        /// The handle the edit was aimed at.
        at: Handle,
    },
    /// A station or a slot would leave `[0, 1]`, which is not a place on a parent.
    OutOfRange {
        /// The handle the edit was aimed at.
        at: Handle,
        /// Which number would have left its range.
        property: Property,
        /// The value it would have had.
        value: f32,
    },
    /// A length would have stopped being positive, which is a spine with nothing to sweep.
    NotPositive {
        /// The handle the edit was aimed at.
        at: Handle,
        /// Which number would have stopped being positive.
        property: Property,
        /// The value it would have had.
        value: f32,
    },
    /// A baked spine could not be built, so the node keeps the one it had.
    SpineRefused {
        /// The handle the edit was aimed at.
        at: Handle,
        /// Why `shape` refused the curve.
        error: CurveError,
    },
}

/// What `shape` refused, in a sentence a report can print.
fn why(error: CurveError) -> &'static str {
    match error {
        CurveError::TooFewKnots => "fewer than two knots",
        CurveError::NotFinite => "a knot that is not finite",
        CurveError::CoincidentKnots => "two consecutive knots at the same place",
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Orphaned { at } => write!(formatter, "{at} names no node"),
            Problem::Pruned { at } => write!(
                formatter,
                "{at} is pruned, so an edit there would show nowhere"
            ),
            Problem::OutOfRange {
                at,
                property,
                value,
            } => write!(
                formatter,
                "{at}'s {} would be {value}, which leaves [0, 1]",
                property.name()
            ),
            Problem::NotPositive {
                at,
                property,
                value,
            } => write!(
                formatter,
                "{at}'s {} would be {value}, which is not positive",
                property.name()
            ),
            Problem::SpineRefused { at, error } => {
                write!(formatter, "{at}'s new spine was refused: {}", why(*error))
            }
        }
    }
}

/// What an application reached, and what it could not.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EditReport {
    /// How many edits landed.
    pub applied: usize,
    /// One entry per edit that did not, in the order the batch was applied.
    pub problems: Vec<Problem>,
}

impl EditReport {
    /// Whether every edit in the batch landed.
    pub fn clean(&self) -> bool {
        self.problems.is_empty()
    }
}

impl std::fmt::Display for EditReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} edit(s) landed, {} did not",
            self.applied,
            self.problems.len()
        )
    }
}

/// Applies a batch of edits, in the stated order, and reports what did not land.
///
/// # Invariants
///
/// **The stages run in [`stage`]'s order — prunes, then spine bakes, then the nudges — and the batch's own
/// order inside a stage**, so the report reads in the order a caller wrote it and an edit aimed at a node the
/// same batch prunes is reported rather than written onto a tombstone. An edit whose handle does not resolve,
/// whose node is a tombstone, or whose value would leave its range changes nothing and appears in the
/// report's problems.
pub fn apply(plant: &mut Plant, edits: &[Edit]) -> EditReport {
    let mut report = EditReport::default();
    for stage in 0..=LAST_STAGE {
        for edit in edits {
            if stage_of(&edit.what) == stage {
                apply_one(plant, edit, &mut report);
            }
        }
    }
    report
}

/// The last stage an edit can belong to, so [`apply`] can walk them without naming each one.
const LAST_STAGE: u8 = 2;

/// Which stage an edit belongs to. Lower runs first.
///
/// # Invariants
///
/// The order is V5's, with the stages this crate has no data for left out: **(1) prunes, (2) spine bakes,
/// (3) offsets and shifts.** A prune runs before the nudges so that a nudge on a pruned node is reported; a
/// bake runs before the nudges so that a nudge on a baked node applies to the authored spine rather than being
/// thrown away by it.
fn stage_of(what: &Change) -> u8 {
    match what {
        Change::Prune { .. } => 0,
        Change::Spine { .. } => 1,
        Change::Offset { .. } | Change::Shift { .. } => 2,
    }
}

/// One edit, with its checks and its report line.
fn apply_one(plant: &mut Plant, edit: &Edit, report: &mut EditReport) {
    let (length, nudged, attachment) = {
        let node = match plant.get(&edit.at) {
            Ok(node) => node,
            Err(_) => {
                report.problems.push(Problem::Orphaned {
                    at: edit.at.clone(),
                });
                return;
            }
        };
        if node.pruned().is_some() {
            report.problems.push(Problem::Pruned {
                at: edit.at.clone(),
            });
            return;
        }
        (
            node.spine().length(),
            node.nudges().length,
            node.effective_attachment(),
        )
    };
    match &edit.what {
        Change::Prune { reason } => {
            plant
                .get_mut(&edit.at)
                .expect("the handle resolved above")
                .prune(*reason);
            report.applied += 1;
        }
        Change::Offset { property, by } => {
            match property {
                Property::Length => {
                    let value = length + nudged + by;
                    if value <= 0.0 {
                        report.problems.push(Problem::NotPositive {
                            at: edit.at.clone(),
                            property: *property,
                            value,
                        });
                        return;
                    }
                }
                Property::Station => {
                    let value = attachment.station + by;
                    if !(0.0..=1.0).contains(&value) {
                        report.problems.push(Problem::OutOfRange {
                            at: edit.at.clone(),
                            property: *property,
                            value,
                        });
                        return;
                    }
                }
                Property::Slot => {
                    let value = attachment.slot + by;
                    if !(0.0..=1.0).contains(&value) {
                        report.problems.push(Problem::OutOfRange {
                            at: edit.at.clone(),
                            property: *property,
                            value,
                        });
                        return;
                    }
                }
                Property::Radius => {}
            }
            let nudges: &mut Nudges = plant
                .get_mut(&edit.at)
                .expect("the handle resolved above")
                .nudges_mut();
            match property {
                Property::Length => nudges.length += by,
                Property::Radius => nudges.radius += by,
                Property::Station => nudges.station += by,
                Property::Slot => nudges.slot += by,
            }
            report.applied += 1;
        }
        Change::Spine { jets } => match Curve::new(jets.clone()) {
            Ok(spine) => {
                plant
                    .get_mut(&edit.at)
                    .expect("the handle resolved above")
                    .set_spine(spine);
                report.applied += 1;
            }
            Err(error) => report.problems.push(Problem::SpineRefused {
                at: edit.at.clone(),
                error,
            }),
        },
        Change::Shift { frame, by } => {
            let local = framed(plant, &edit.at, *frame, *by);
            let shift = plant
                .get_mut(&edit.at)
                .expect("the handle resolved above")
                .shift_mut();
            for axis in 0..3 {
                shift[axis] += local[axis];
            }
            report.applied += 1;
        }
    }
}

/// A translation `by`, written in `frame`, as the node's own frame reads it.///
/// # Invariants
///
/// A shift is stored in the node's own frame, so each other frame is undone by one rotation: the parent's
/// frame by the surface frame's own axes — which are that rotation, since a rotation's inverse is its
/// transpose — and the world's by the node's placement. **The root has no parent, and its frame is the
/// world's**, so `Frame::Parent` on the root means the same as `Frame::World` there rather than being an
/// error a caller has to discover.
fn framed(plant: &Plant, at: &Handle, frame: Frame, by: Vec3) -> Vec3 {
    match frame {
        Frame::Local => by,
        Frame::Parent => {
            let Some(parent) = at.parent() else {
                return by;
            };
            let Ok(node) = plant.get(at) else {
                return by;
            };
            let Ok(surface) = plant.surface(&parent, node.effective_attachment()) else {
                return by;
            };
            [
                dot3(by, surface.x),
                dot3(by, surface.y),
                dot3(by, surface.z),
            ]
        }
        Frame::World => {
            let placement = plant.placement(at).unwrap_or(Placement::IDENTITY);
            [
                dot3(by, placement.x),
                dot3(by, placement.y),
                dot3(by, placement.z),
            ]
        }
    }
}

/// The edits a brush writes: one per handle the selector names, all with the same change.
///
/// # Invariants
///
/// **The brush is a predicate that writes the edit table, not a second editing path.** A caller can print the
/// list before applying it, filter it, log it, or hand it to the same [`apply`], which is what makes a brush
/// safe to aim: what it will do is a list of handles before it does it. The list is in walk order and without
/// duplicates, because [`Selector::handles`] is.
pub fn paint(plant: &Plant, selector: &Selector, change: Change) -> Vec<Edit> {
    selector
        .handles(plant)
        .into_iter()
        .map(|at| Edit::new(at, change.clone()))
        .collect()
}
