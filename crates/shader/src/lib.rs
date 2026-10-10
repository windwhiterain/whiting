//! A surface, described the same way whatever renders it.
//!
//! # What this is for
//!
//! Five of the gallery's twelve scenes compute their surfaces per fragment, so a path tracer
//! is handed five numbers where the rasteriser was handed a shader. Nothing about the target
//! format makes that hard: Cycles takes an arbitrary node graph in a `<shader>` block, and
//! the nodes needed to build a snowline out of height and steepness are all registered.
//! What was missing was a way to *say* the surface without naming any one renderer.
//!
//! This crate is that way. It holds no renderer, no node names from any renderer, and no
//! arithmetic beyond what a surface needs. A backend compiler turns a [`Graph`] into
//! whatever its target speaks: a Cycles node graph today, OSL source if that fallback is
//! ever taken.
//!
//! # The failure this is arranged against
//!
//! Cycles' XML is unforgiving in a way that is easy to mistake for working. Measured against
//! the 5.3.0 executable this repository ships:
//!
//! - an unknown connect **target** raises `Unknown input socket name "x" on "y"`;
//! - an unknown connect **source** raises nothing, and leaves the destination at its
//!   default — in practice zero, which draws a black surface with a specular highlight on
//!   it and looks like a lighting mistake;
//! - an unknown node **attribute** is ignored the same way.
//!
//! A graph can therefore be half connected, render, exit zero, and be wrong. Everything in
//! this crate is shaped to make that impossible to reach by accident: sockets are indexes
//! resolved through [`Op::outputs`] rather than names a caller writes, a wire between two
//! types that do not match is refused here rather than reinterpreted there, and a backend
//! that cannot spell an operation is expected to refuse to emit it.
//!
//! # Shape
//!
//! - [`Op`] is the operation set, and is the one place any socket layout is stated.
//! - [`SocketType`], [`Literal`] and [`Input`] say what flows.
//! - [`Graph`] holds the nodes and reports everything wrong with them.
//!
//! ```no_run
//! // The workspace calls this crate `shader`; its own package name is longer.
//! extern crate whiting_shader as shader;
//! use shader::{GeometryField, Graph, Input, Interpolation, Literal, MapRangeTarget, Op, SocketRef};
//!
//! let mut surface = Graph::new();
//! let position = surface.add(Op::Geometry(GeometryField::Position));
//! let normal = surface.add(Op::Geometry(GeometryField::Normal));
//!
//! // A snowline: how high this point is, remapped so the band is flat at both ends.
//! let snowline = surface.add(Op::MapRange {
//!     target: MapRangeTarget::Value,
//!     interpolation: Interpolation::SmoothStep,
//!     clamp: true,
//! });
//! surface.wire(snowline, 0, Input::Socket(SocketRef { node: position, output: 0 })).unwrap();
//! surface.wire(snowline, 1, Input::Constant(Literal::Float(120.0))).unwrap();
//! surface.wire(snowline, 2, Input::Constant(Literal::Float(180.0))).unwrap();
//! surface.wire(snowline, 3, Input::Constant(Literal::Float(0.0))).unwrap();
//! surface.wire(snowline, 4, Input::Constant(Literal::Float(1.0))).unwrap();
//!
//! let _ = normal;
//! let stone = surface.add(Op::Diffuse);
//! surface.wire(stone, 1, Input::Socket(SocketRef { node: snowline, output: 0 })).unwrap();
//! surface.set_surface(SocketRef { node: stone, output: 0 });
//! assert_eq!(surface.validate(), Vec::new());
//! ```

#![forbid(unsafe_code)]

mod error;
mod graph;
mod op;
mod socket;

pub use error::{GraphError, GraphResult};
pub use graph::{Graph, Node};
pub use op::{
    CameraField, GeometryField, Interpolation, MapRangeTarget, MathOp, ObjectInfoField, Op,
    ResultType, VectorMathOp,
};
pub use socket::{Input, Literal, NodeId, SocketRef, SocketType};
