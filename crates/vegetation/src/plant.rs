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

use shape::math::{Vec3, clamp, cross3, length3, normalized3};
use shape::{Curve, Jet};

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
        for axis in 0..3 {
            placed[axis] +=
                self.x[axis] * point[0] + self.y[axis] * point[1] + self.z[axis] * point[2];
        }
        placed
    }

    /// A direction in this frame's coordinates, expressed in its parent's: the same as
    /// [`Placement::place`] without the origin.
    pub fn rotate(&self, direction: Vec3) -> Vec3 {
        let mut rotated = [0.0; 3];
        for axis in 0..3 {
            rotated[axis] = self.x[axis] * direction[0]
                + self.y[axis] * direction[1]
                + self.z[axis] * direction[2];
        }
        rotated
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

/// A plant: a root node and the seed its nodes' randomness is derived from.
#[derive(Debug, Clone, PartialEq)]
pub struct Plant {
    root: Node,
    seed: u64,
}

impl Plant {
    /// A plant whose root is `root` and whose numbers come from `seed`.
    pub fn new(root: Node, seed: u64) -> Self {
        Self { root, seed }
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

    /// Every node in the plant, depth first, each with the handle that names it.
    pub fn walk(&self) -> Walk<'_> {
        Walk {
            stack: vec![(Handle::root(), &self.root)],
        }
    }

    /// The node's own stream of numbers, forked from the plant's seed by its handle.
    pub fn rng(&self, handle: &Handle) -> Rng {
        Rng::for_path(self.seed, handle.path())
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
    pub fn surface(
        &self,
        handle: &Handle,
        attachment: Attachment,
    ) -> Result<Placement, HandleOrphaned> {
        let node = self.get(handle)?;
        let station = clamp(attachment.station, 0.0, 1.0);
        let frame = node.spine().frame(station);
        let scale = node.taper().at(station);
        let point = node.section().at(attachment.slot);
        let mut radial = [0.0; 3];
        for axis in 0..3 {
            radial[axis] =
                frame.normal[axis] * point[0] * scale + frame.binormal[axis] * point[1] * scale;
        }
        let outward = if length3(radial) > 1.0e-6 {
            normalized3(radial)
        } else {
            frame.normal
        };
        let along = node.spine().at(station);
        let mut origin = [0.0; 3];
        for axis in 0..3 {
            origin[axis] = along[axis] + radial[axis] - outward[axis] * attachment.sink;
        }
        Ok(Placement {
            origin,
            x: outward,
            y: frame.tangent,
            z: cross3(outward, frame.tangent),
        })
    }

    /// The node's placement in the world, composed from the root down.
    ///
    /// # Invariants
    ///
    /// Every step is the parent's placement composed with the parent's [`Plant::surface`] frame at the
    /// child's attachment, so the answer is a walk and not a search. The cost is `O(depth²)` in the number
    /// of nodes passed on the way, which for a plant is a handful of `Vec` indexes.
    pub fn placement(&self, handle: &Handle) -> Result<Placement, HandleOrphaned> {
        let mut placement = Placement::IDENTITY;
        let mut prefix = Handle::root();
        for index in handle.path() {
            let child = prefix.child(*index);
            let node = self.get(&child)?;
            placement = placement.then(&self.surface(&prefix, node.attachment())?);
            prefix = child;
        }
        Ok(placement)
    }

    /// The node's spine in the world: its authored jets moved by its placement.
    ///
    /// # Invariants
    ///
    /// The jets are the node's own stored ones, so a rigid transform cannot change the shape: every
    /// segment's parameter and every derivative's magnitude survive, and `Curve::new` reproduces the same
    /// curve. The **initial normal is the node's first frame normal carried into the world**, which is the
    /// one rule that keeps a twig's section oriented continuously with the surface it hangs from
    /// (`shape`'s open question 2, applied by the plant rather than by each sweep).
    pub fn spine(&self, handle: &Handle) -> Result<Curve, HandleOrphaned> {
        let node = self.get(handle)?;
        let placement = self.placement(handle)?;
        let jets: Vec<Jet> = node
            .spine()
            .jets()
            .into_iter()
            .map(|jet| Jet::new(placement.place(jet.position), placement.rotate(jet.tangent)))
            .collect();
        let mut curve =
            Curve::new(jets).expect("a rigid transform of a valid curve is a valid curve");
        curve.set_initial_normal(placement.rotate(node.spine().frame(0.0).normal));
        Ok(curve)
    }
}

/// Every node of a plant, depth first, with the handle that names it.
pub struct Walk<'a> {
    stack: Vec<(Handle, &'a Node)>,
}

impl<'a> Iterator for Walk<'a> {
    type Item = (Handle, &'a Node);

    fn next(&mut self) -> Option<Self::Item> {
        let (handle, node) = self.stack.pop()?;
        for (index, child) in node.children().iter().enumerate().rev() {
            self.stack.push((handle.child(index as u32), child));
        }
        Some((handle, node))
    }
}
