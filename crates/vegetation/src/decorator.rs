//! Small authored things seated on a node's surface: a knot, a burl, a thorn, a fruit, a scar.
//!
//! # The one idea
//!
//! **A decorator is a template and three numbers, and no rotation is expressible.** Its template is a small
//! [`shape::Sweep`] authored in its own frame — base at the origin, axis along `+x` — and its seat is the same
//! `(station, slot, sink)` a joint is made of, so the frame it is drawn in is **the node's own surface frame taken
//! whole**: `x` outward, `y` along the parent, `z` their cross product. There is no angle to write down, which is
//! V3's sentence (*"a joint is one subtraction, not a transform"*) made a type: a rotation that cannot be
//! expressed cannot be got wrong.
//!
//! # Why a decorator's template is a sweep
//!
//! The same reason a leaf's is: a knot is a short tube that bulges, a thorn is a thin cone, a burl is a lumpy
//! stub — all of them a spine, a law that thickens it and a section. So a decorator template and a leaf template
//! are the same kind of object, and a scene that can draw one can draw the other: **`k` decorators of one template
//! are one mesh and `k` transforms**, exactly as `k` leaves are.
//!
//! # What is not a decorator
//!
//! A tip cap. An open tube is a hole seen from below, and [`shape::Sweep::capped`] closes it with the sweep's own
//! end ring — a decorator would be a second, slower way to say the same thing, and one that could be left off.

use shape::{Loft, Sweep};

use crate::handle::Handle;
use crate::node::Attachment;
use crate::plant::{Placement, Plant};

/// A decorator's template: a small sweep in its own frame, and the dial it is drawn at.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// The sweep, authored with its base at the origin and its axis along `+x`, which is outward from the surface
    /// it will sit on.
    pub sweep: Sweep,
    /// How many stations the template is sampled at.
    pub stations: usize,
    /// How many slots around its section.
    pub around: usize,
}

impl Template {
    /// The template as the geometry a scene draws once and instances.
    ///
    /// # Invariants
    ///
    /// The loft is in the template's own frame, which is the space an instance's frame expects: a point at the
    /// template's origin lands at the seat's origin, and the template's `+x` lands along the surface's outward
    /// direction.
    pub fn loft(&self) -> Loft {
        self.sweep.sample(self.stations, self.around)
    }
}

/// One decorator: which template, which node, and the three numbers that seat it.
///
/// The numbers are [`Attachment`]'s own — a station along the node, a slot around it, and how far the seat sinks
/// into the surface — so a decorator is placed by the same vocabulary a branch is, and a reader who knows one
/// knows the other.
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    /// The node whose surface the decorator sits on.
    pub node: Handle,
    /// Which template it instances.
    pub template: usize,
    /// Where along the node.
    pub station: f32,
    /// Where around the node's section.
    pub slot: f32,
    /// How far the seat sinks into the surface, so a knot is a knot in the bark rather than a bead on it.
    pub sink: f32,
    /// One uniform number multiplying the template's own size.
    pub scale: f32,
}

impl Seat {
    /// A decorator at a place on a node, with no sink and a scale of one.
    pub fn new(node: Handle, template: usize, station: f32, slot: f32) -> Self {
        Self {
            node,
            template,
            station,
            slot,
            sink: 0.0,
            scale: 1.0,
        }
    }

    /// The same seat sunk into its surface.
    pub fn sunk(mut self, sink: f32) -> Self {
        self.sink = sink;
        self
    }

    /// The same seat drawn larger or smaller.
    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// The attachment these numbers are.
    pub fn attachment(&self) -> Attachment {
        Attachment::new(self.station, self.slot).sunk(self.sink)
    }
}

/// A decorator as a scene draws it: where the template's own frame sits in the world, and how big it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The node it sits on, which is the name a report gives it.
    pub node: Handle,
    /// Which template's mesh it instances.
    pub template: usize,
    /// The frame the template is drawn in.
    pub placement: Placement,
    /// The template's own size, multiplied by this.
    pub scale: f32,
}

/// Places every seat a plant can carry.
///
/// # Invariants
///
/// - **The frame is the node's own surface frame, moved into the world**: `plant.placement(node)` composed with
///   `plant.surface(node, seat.attachment())`, so a decorator sits on the skin the plant actually draws — nudged,
///   baked or bent by a field — and two numbers are the whole of its orientation.
/// - **A seat on a tombstone draws nothing**: the handle still resolves, so this is the one place a decorator can
///   be absent without an error, and it is the same rule the wood follows (`Plant::live`).
/// - **A seat whose template a caller did not supply is still placed**: the template number is the caller's to
///   resolve, and this function does not know how many they have. Resolving it is the scene's job, and a number
///   that names nothing is a scene that will not draw.
pub fn placed(plant: &Plant, seats: &[Seat]) -> Vec<Placed> {
    let mut placed = Vec::new();
    for seat in seats {
        let Ok(node) = plant.get(&seat.node) else {
            continue;
        };
        if node.pruned().is_some() {
            continue;
        }
        let Ok(frame) = plant.surface(&seat.node, seat.attachment()) else {
            continue;
        };
        let Ok(placement) = plant.placement(&seat.node) else {
            continue;
        };
        placed.push(Placed {
            node: seat.node.clone(),
            template: seat.template,
            placement: placement.then(&frame),
            scale: seat.scale,
        });
    }
    placed
}
