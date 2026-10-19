//! A plant: one root, one seed, and the walk that names every node.
//!
//! # The one idea
//!
//! **A plant is walked, not searched.** A node's placement is composed from the root down — the parent's
//! placement, then the parent's surface frame at the child's station and slot — so no expansion pass looks
//! for a nearest neighbour and no position can disagree with the tree. That is `mountain`'s rule one
//! dimension up: *the lines carry their correspondence*.
//!
//! # A handle is an address, and a stale one is reported
//!
//! [`Plant::get`] answers with the node a [`Handle`] names or with [`HandleOrphaned`], which says how far
//! the path resolved and which child was missing. It never returns the node nearest to the path: a prune
//! leaves its descendants' handles stale, and a stale handle has to be a named fact rather than a silent
//! re-binding to whatever branch took the place.
//!
//! # What a walk derives once: [`Derived`]
//!
//! **A node's placement, its drawn spine and its placed spine are functions of the plant alone** — of the
//! node, of its ancestors' placements, and of the plant's fields — so re-deriving one for a second leaf of
//! the same twig returns the same numbers and costs a rebuilt [`shape::Curve`]. [`Derived`] is the table
//! that keeps them: an entry point that loops over nodes ([`crate::expand::wood`], [`crate::leaf::LeafField`]
//! `groups`, [`crate::decorator::placed`], [`crate::report::report`]) builds one table, threads it through
//! its own walk, and drops it when the walk returns. A leaf field's per-leaf `surface` read — the binding
//! cost of every dense crown — is then one frame evaluation on a curve that was already built.
//!
//! **The table is valid for exactly one immutable borrow of one plant, and that is the whole invariant.**
//! It is a local in the walk that reads it, never a field inside [`Plant`], so it cannot outlive the plant
//! it was derived from and there is no invalidation to miss. The alternative — a cache inside the plant
//! behind interior mutability — would have to be cleared by [`Plant::get_mut`], [`crate::ops::apply`],
//! [`crate::node::Node::push`] and [`crate::envelope::trim`] all at once, and one missed call would be a
//! **wrong picture with no error**: the worst failure this crate has.

use std::collections::HashMap;

use shape::math::{Vec3, clamp, cross3, dot3, length3, normalized3, subtract};
use shape::{Curve, Jet};

use crate::field::Field;
use crate::handle::Handle;
use crate::node::{Attachment, Node};
use crate::rng::Rng;

/// A rigid frame: where one node's authored coordinates sit in its parent's.
///
/// # Invariants
///
/// `(x, y, z)` is orthonormal and right-handed, so a placement never scales or shears what it places — a
/// taper is a [`shape::Scalar`] on the node, not a property of the transform. [`Placement::IDENTITY`] is the
/// root's, which is what makes the root's authored coordinates the world's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Where the frame's origin sits.
    pub origin: Vec3,
    /// The frame's `x`: outward from the parent's surface.
    pub x: Vec3,
    /// The frame's `y`: along the parent toward its tip.
    pub y: Vec3,
    /// `x × y`, which completes the right-handed frame.
    pub z: Vec3,
}

impl Placement {
    /// The frame of the root, and the frame a node with no parent is placed in.
    pub const IDENTITY: Placement = Placement {
        origin: [0.0; 3],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        z: [0.0, 0.0, 1.0],
    };

    /// A point in this frame's coordinates, expressed in its parent's.
    pub fn place(&self, point: Vec3) -> Vec3 {
        let mut placed = self.origin;
        for (axis, out) in placed.iter_mut().enumerate() {
            *out += self.x[axis] * point[0] + self.y[axis] * point[1] + self.z[axis] * point[2];
        }
        placed
    }

    /// A direction in this frame's coordinates, expressed in its parent's: the same as
    /// [`Placement::place`] without the origin.
    pub fn rotate(&self, direction: Vec3) -> Vec3 {
        let mut rotated = [0.0; 3];
        for (axis, out) in rotated.iter_mut().enumerate() {
            *out = self.x[axis] * direction[0]
                + self.y[axis] * direction[1]
                + self.z[axis] * direction[2];
        }
        rotated
    }

    /// A point in the parent's coordinates, expressed in this frame's: the inverse of [`Placement::place`].
    ///
    /// # Invariants
    ///
    /// It is the exact inverse because the frame is orthonormal — `unplace(place(p))` is `p` to a rounding — and
    /// **not** because a placement is a rotation matrix in general: a placement never scales, which is what makes
    /// the inverse a projection rather than a linear solve. A field that pushes a spine in world directions reads
    /// its push back into the frame the spine is authored in, and this is that reading.
    pub fn unplace(&self, point: Vec3) -> Vec3 {
        self.unrotate(subtract(point, self.origin))
    }

    /// A direction in the parent's coordinates, expressed in this frame's: the inverse of [`Placement::rotate`].
    pub fn unrotate(&self, direction: Vec3) -> Vec3 {
        [
            dot3(direction, self.x),
            dot3(direction, self.y),
            dot3(direction, self.z),
        ]
    }

    /// `self` applied **after** `inner`: the composition an expansion walks down the tree with.
    pub fn then(&self, inner: &Placement) -> Placement {
        Placement {
            origin: self.place(inner.origin),
            x: self.rotate(inner.x),
            y: self.rotate(inner.y),
            z: self.rotate(inner.z),
        }
    }
}

/// Why a handle does not name a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleOrphaned {
    /// The last handle on the path that resolved.
    pub from: Handle,
    /// The child index that did not exist under it.
    pub index: u32,
}

impl std::fmt::Display for HandleOrphaned {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} has no child {}", self.from, self.index)
    }
}

/// A plant: a root node, the seed its nodes' randomness is derived from, and the fields that shape it.
#[derive(Debug, Clone, PartialEq)]
pub struct Plant {
    root: Node,
    seed: u64,
    fields: Vec<Field>,
}

/// What one walk of a plant derives once per node, and keeps for the rest of that walk.
///
/// # Invariants
///
/// - **Every entry is a function of the plant alone**, so deriving it twice returns the same numbers:
///   a [`Placement`] is the ancestor chain's placements composed, a drawn spine is [`Plant::shape`]'s, and a
///   placed spine is the drawn one moved by that placement. Nothing about any of them depends on *which*
///   caller is asking, so one entry can answer for a whole node's leaves.
/// - **The table is valid for exactly one immutable borrow of one plant.** It is created by the walk that
///   reads it and dropped when that walk returns, so a mutation cannot be observed through it and there is
///   no entry to invalidate. A caller that edits the plant between two reads — [`crate::ops::apply`], for
///   instance — starts a new table, which is why [`crate::envelope::trim`] takes two.
/// - **Only a successful derivation is kept.** A handle the plant cannot resolve is answered again, which
///   costs a path walk rather than a curve; a stale handle is [`HandleOrphaned`]'s named fact and no cache
///   may hide it.
///
/// It is keyed by [`Handle`] because a handle names a node and survives an edit that renumbers nothing, and
/// because a tombstone is still a row: the walk that includes them and the walk that does not ask for the
/// same key the same way.
#[derive(Debug, Default)]
pub(crate) struct Derived {
    placements: HashMap<Handle, Placement>,
    drawn: HashMap<Handle, Curve>,
    placed: HashMap<Handle, Curve>,
}

impl Plant {
    /// A plant whose root is `root` and whose numbers come from `seed`, with no field shaping it.
    pub fn new(root: Node, seed: u64) -> Self {
        Self {
            root,
            seed,
            fields: Vec::new(),
        }
    }

    /// The same plant under a set of fields: how gravity, a light or a wind shape it.
    ///
    /// The fields are applied where a spine is drawn, in the order they are given, so a plant is shaped the
    /// same way every time it is asked for a spine and two fields compose instead of overwriting each other.
    pub fn with_fields(mut self, fields: Vec<Field>) -> Self {
        self.fields = fields;
        self
    }

    /// The fields shaping this plant, in the order they are applied.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// The root node.
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// The seed every node's randomness is derived from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The node a handle names, or where the path stopped resolving.
    pub fn get(&self, handle: &Handle) -> Result<&Node, HandleOrphaned> {
        let mut node = &self.root;
        let mut reached = Handle::root();
        for index in handle.path() {
            match node.children().get(*index as usize) {
                Some(child) => {
                    node = child;
                    reached = reached.child(*index);
                }
                None => {
                    return Err(HandleOrphaned {
                        from: reached,
                        index: *index,
                    });
                }
            }
        }
        Ok(node)
    }

    /// The node a handle names, mutably: how the hand writes an edit.
    pub fn get_mut(&mut self, handle: &Handle) -> Result<&mut Node, HandleOrphaned> {
        let mut node = &mut self.root;
        let mut reached = Handle::root();
        for index in handle.path() {
            match node.children_mut().get_mut(*index as usize) {
                Some(child) => {
                    node = child;
                    reached = reached.child(*index);
                }
                None => {
                    return Err(HandleOrphaned {
                        from: reached,
                        index: *index,
                    });
                }
            }
        }
        Ok(node)
    }

    /// Every node in the plant, depth first, each with the handle that names it.
    ///
    /// # Invariants
    ///
    /// **This walk includes tombstones**, because a handle has to go on naming what it named: a prune is
    /// reported at [`crate::node::Node::pruned`] rather than by renumbering its siblings. Use
    /// [`Plant::live`] for what a plant draws.
    pub fn walk(&self) -> Walk<'_> {
        Walk {
            stack: vec![(Handle::root(), &self.root)],
            skip_pruned: false,
        }
    }

    /// Every node a plant **draws**, depth first: the walk above, without the tombstones or their subtrees.
    pub fn live(&self) -> Walk<'_> {
        Walk {
            stack: vec![(Handle::root(), &self.root)],
            skip_pruned: true,
        }
    }

    /// The node's own stream of numbers, forked from the plant's seed by its handle.
    pub fn rng(&self, handle: &Handle) -> Rng {
        Rng::for_path(self.seed, handle.path())
    }

    /// The node's spine **as it is drawn**: the authored one, as long as a length nudge says, bent by the
    /// plant's fields.
    ///
    /// # Invariants
    ///
    /// **The fields are applied here and nowhere else**, in the order the plant holds them, each reading the
    /// spine the last one left. A spine an author baked is passed through untouched — the design's third tier
    /// says an authored spine is authored from here on — and that is the one place the marker is read.
    fn shape(&self, node: &Node, placement: &Placement) -> Curve {
        let drawn = node.spine_now();
        if node.baked() {
            return drawn;
        }
        self.fields
            .iter()
            .fold(drawn, |curve, field| field.shape(&curve, placement))
    }

    /// The node's spine **as it is drawn**, held in `derived` for the rest of the walk.
    ///
    /// # Invariants
    ///
    /// **It is [`Plant::shape`]'s answer, once.** The drawn spine is a function of the node and of the
    /// placement it is drawn in, and every attachment on that node reads the same curve — so a leaf field's
    /// hundred leaves of one twig rebuild it once instead of a hundred times. The frame is then read off the
    /// curve analytically, at the attachment's own station, which is what keeps a cached answer and a fresh
    /// one **the same numbers**: nothing is sampled and interpolated between rings here.
    fn drawn_in<'a>(
        &self,
        handle: &Handle,
        node: &Node,
        placement: &Placement,
        derived: &'a mut Derived,
    ) -> &'a Curve {
        if derived.drawn.contains_key(handle) {
            return derived
                .drawn
                .get(handle)
                .expect("the drawn spine is under this handle");
        }
        derived
            .drawn
            .insert(handle.clone(), self.shape(node, placement));
        derived
            .drawn
            .get(handle)
            .expect("the drawn spine was just placed under this handle")
    }

    /// The frame on `handle`'s surface at an attachment: the joint a child hangs from.
    ///
    /// # Invariants
    ///
    /// The frame's `x` is the **outward** direction through the attachment's point, so it is perpendicular
    /// to the parent's tangent and `x × y` is a unit vector. `sink` moves the origin inward along `x`, which
    /// is what makes a joint a smaller joint rather than a moved one. Where the parent's taper is zero the
    /// point has no direction from the spine, and `x` falls back to the section's own `+x` rather than to a
    /// zero vector that would collapse every child.
    ///
    /// **The skin is the drawn one**: the frame is read from the spine [`Plant::shape`] returns and from
    /// [`crate::node::Node::taper_now`], so a node an author nudged carries its children on the surface they
    /// see, and a node a field bent carries them on the bend.
    pub fn surface(
        &self,
        handle: &Handle,
        attachment: Attachment,
    ) -> Result<Placement, HandleOrphaned> {
        self.surface_in(handle, attachment, &mut Derived::default())
    }

    /// [`Plant::surface`] with the walk's table, so a caller placing many attachments on one node derives
    /// its drawn spine once.
    pub(crate) fn surface_in(
        &self,
        handle: &Handle,
        attachment: Attachment,
        derived: &mut Derived,
    ) -> Result<Placement, HandleOrphaned> {
        let node = self.get(handle)?;
        let placement = self.placement_in(handle, derived)?;
        Ok(self.surface_at_in(handle, node, &placement, attachment, derived))
    }

    /// The surface frame of a node whose placement is already known: one step of the walk
    /// [`Plant::placement`] makes, kept as a function so the walk and a caller cannot disagree about where a
    /// joint is. The only thing the table buys here is the drawn spine the frame is read from.
    fn surface_at_in(
        &self,
        handle: &Handle,
        node: &Node,
        placement: &Placement,
        attachment: Attachment,
        derived: &mut Derived,
    ) -> Placement {
        let spine = self.drawn_in(handle, node, placement, derived);
        let station = clamp(attachment.station, 0.0, 1.0);
        let frame = spine.frame(station);
        let scale = node.taper_now().at(station);
        let point = node.section().at(attachment.slot);
        let mut radial = [0.0; 3];
        for (axis, out) in radial.iter_mut().enumerate() {
            *out = frame.normal[axis] * point[0] * scale + frame.binormal[axis] * point[1] * scale;
        }
        let outward = if length3(radial) > 1.0e-6 {
            normalized3(radial)
        } else {
            frame.normal
        };
        let along = spine.at(station);
        let mut origin = [0.0; 3];
        for axis in 0..3 {
            origin[axis] = along[axis] + radial[axis] - outward[axis] * attachment.sink;
        }
        Placement {
            origin,
            x: outward,
            y: frame.tangent,
            z: cross3(outward, frame.tangent),
        }
    }

    /// The node's placement in the world, composed from the root down.
    ///
    /// # Invariants
    ///
    /// Every step is the parent's placement composed with the parent's [`Plant::surface`] frame at the
    /// child's **effective** attachment — the authored one plus its station and slot nudges — so an edit that
    /// slides a bough along the trunk moves the bough and everything it carries, and nothing has to renumber.
    /// A node's own `shift` is added last, in the node's frame, so it moves that node's origin and every
    /// placement below it. The cost is `O(depth)` in the number of nodes passed on the way **with a table**
    /// and `O(depth²)` without one, because every ancestor's placement is already derived.
    pub fn placement(&self, handle: &Handle) -> Result<Placement, HandleOrphaned> {
        self.placement_in(handle, &mut Derived::default())
    }

    /// [`Plant::placement`] with the walk's table, so a walk that places many nodes pays for each ancestor
    /// chain once rather than once per node that passes through it.
    ///
    /// # Invariants
    ///
    /// **The composition is the walk's own, step for step**: the parent's placement, then the parent's
    /// surface frame at the child's effective attachment, then the child's own shift in the frame that
    /// results. Only the *reuse* is new — a cached ancestor's placement is the number the walk would have
    /// composed again — so a node's placement does not depend on whether an ancestor was asked before it.
    /// The root is the base case and is never asked for a surface: it hangs from nothing.
    pub(crate) fn placement_in(
        &self,
        handle: &Handle,
        derived: &mut Derived,
    ) -> Result<Placement, HandleOrphaned> {
        if let Some(held) = derived.placements.get(handle) {
            return Ok(*held);
        }
        if handle.is_root() {
            derived
                .placements
                .insert(Handle::root(), Placement::IDENTITY);
            return Ok(Placement::IDENTITY);
        }
        let parent = handle
            .parent()
            .expect("a handle that is not the root has a parent");
        let held = self.placement_in(&parent, derived)?;
        let node = self.get(handle)?;
        let attachment = node.effective_attachment();
        let ancestor = self.get(&parent)?;
        let surface = self.surface_at_in(&parent, ancestor, &held, attachment, derived);
        let mut placement = held.then(&surface);
        let shift = node.shift();
        if shift != [0.0; 3] {
            let moved = placement.rotate(shift);
            for (axis, out) in placement.origin.iter_mut().enumerate() {
                *out += moved[axis];
            }
        }
        derived.placements.insert(handle.clone(), placement);
        Ok(placement)
    }

    /// The node's spine in the world: its drawn jets moved by its placement.
    ///
    /// # Invariants
    ///
    /// The jets are [`crate::node::Node::spine_now`]'s, so a length nudge is in the geometry and a rigid
    /// transform cannot change the shape: every segment's parameter and every derivative's magnitude survive,
    /// and `Curve::new` reproduces the same curve. The **initial normal is the node's first frame normal
    /// carried into the world**, which is the one rule that keeps a twig's section oriented continuously with
    /// the surface it hangs from (`shape`'s open question 2, applied by the plant rather than by each sweep).
    pub fn spine(&self, handle: &Handle) -> Result<Curve, HandleOrphaned> {
        self.spine_in(handle, &mut Derived::default())
    }

    /// The node's tip in the world: where its drawn spine's last knot lands under its placement.
    ///
    /// # Invariants
    ///
    /// **This is `spine_in(handle).at(1.0)`, bit for bit.** `shape` states that `Curve::at`'s two ends are
    /// the two ends and not one bisection step away from them — `t = 1` answers the last segment at
    /// `u = 1`, so `at` returns the last jet's position exactly — and [`Plant::place_spine`] maps every
    /// jet, last included, with the same [`Placement::place`]. So a caller that wants the tip pays for one
    /// knot rather than for a whole rebuilt curve, and gets the same point.
    ///
    /// The placed spine is still what a caller that needs the curve's *length* asks for: an arc length is a
    /// quadrature over the rotated derivatives, and this is not a way to answer one.
    pub(crate) fn tip_in(
        &self,
        handle: &Handle,
        derived: &mut Derived,
    ) -> Result<Vec3, HandleOrphaned> {
        let node = self.get(handle)?;
        let placement = self.placement_in(handle, derived)?;
        let drawn = self.drawn_in(handle, node, &placement, derived);
        let tip = drawn
            .jets()
            .pop()
            .expect("a curve holds at least two knots")
            .position;
        Ok(placement.place(tip))
    }

    /// [`Plant::spine`] with the walk's table, so a walk that places many nodes derives each placed spine
    /// once — a [`shape::Curve`] rebuild is the single most expensive thing this crate does per node.
    ///
    /// # Invariants
    ///
    /// **The placed spine is a function of the node and its placement**, both of which the table already
    /// holds, so a cached answer is the curve the walk would have built again. The construction below is
    /// [`Plant::spine`]'s own arithmetic, unchanged: a rigid transform of the drawn jets, then
    /// `Curve::new`, then the initial normal carried into the world. Nothing here is re-parameterised or
    /// approximated, because a placed spine is **geometry** and moving one vertex is the failure this crate
    /// cannot report.
    pub(crate) fn spine_in(
        &self,
        handle: &Handle,
        derived: &mut Derived,
    ) -> Result<Curve, HandleOrphaned> {
        if let Some(held) = derived.placed.get(handle) {
            return Ok(held.clone());
        }
        let node = self.get(handle)?;
        let placement = self.placement_in(handle, derived)?;
        let curve = self.place_spine(handle, node, &placement, derived);
        derived.placed.insert(handle.clone(), curve.clone());
        Ok(curve)
    }

    /// The world curve of a node whose placement is already known, read off the table's drawn spine.
    fn place_spine(
        &self,
        handle: &Handle,
        node: &Node,
        placement: &Placement,
        derived: &mut Derived,
    ) -> Curve {
        let drawn = self.drawn_in(handle, node, placement, derived);
        let jets: Vec<Jet> = drawn
            .jets()
            .into_iter()
            .map(|jet| Jet::new(placement.place(jet.position), placement.rotate(jet.tangent)))
            .collect();
        let mut curve =
            Curve::new(jets).expect("a rigid transform of a valid curve is a valid curve");
        curve.set_initial_normal(placement.rotate(drawn.frame(0.0).normal));
        curve
    }
}

/// Every node of a plant, depth first, with the handle that names it.
///
/// It yields tombstones unless `skip_pruned` is set, and in that case it does not descend into them either:
/// a pruned node's children are as dead as it is, and a caller that wants them asks [`Plant::walk`].
pub struct Walk<'a> {
    stack: Vec<(Handle, &'a Node)>,
    skip_pruned: bool,
}

impl<'a> Iterator for Walk<'a> {
    type Item = (Handle, &'a Node);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (handle, node) = self.stack.pop()?;
            if self.skip_pruned && node.pruned().is_some() {
                continue;
            }
            for (index, child) in node.children().iter().enumerate().rev() {
                self.stack.push((handle.child(index as u32), child));
            }
            return Some((handle, node));
        }
    }
}
