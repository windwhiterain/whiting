//! A plant authored as a spine tree, and expanded into closed solids and instanced leaves.
//!
//! # The one idea
//!
//! **A plant is a spine tree whose every node names the station and the ring slot it hangs from, and
//! whose manual detail is an offset on a computed value, keyed by a handle that survives the
//! computation.** The tree is the document, the [`handle`] is its address space, and a node's placement
//! is a consequence of four numbers rather than a transform someone typed.
//!
//! # What this crate owns, and what it borrows
//!
//! A spine is [`shape::Curve`], a branch's taper is [`shape::Scalar`], a cross-section is
//! [`shape::Section`], and the walk that turns a spine and a section into triangles is [`shape::Sweep`] —
//! a branch, a leaf blade and an envelope are one primitive there, and this crate restates none of it.
//! What is here is only what a *plant* adds: [`handle`], the derivation path that names a node; [`rng`],
//! one random source per node, forked by that path.
//!
//! # The rules this crate may not break
//!
//! They are [`shape`]'s and `mountain`'s before it, one dimension up: **the lines carry their
//! correspondence**, so an attachment is named rather than searched for and no expansion pass looks for
//! a nearest neighbour; **the output is planar-facet triangles**, and the scene layer has no quads; a
//! check **reports and does not repair**; and every walk is deterministic, so two runs over one document
//! give the same triangles in the same order.
//!
//! # Phasing
//!
//! P1 is one plant and its leaf field; P2 is the hand that selects, offsets and bakes; P3 is the site.
//! The design is
//! [`.agents/proposals/agentic-vegetation-generation.md`](../../../.agents/proposals/agentic-vegetation-generation.md).

pub mod handle;
pub mod node;
pub mod plant;
pub mod rng;
