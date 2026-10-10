//! What can be wrong with a surface before it is ever handed to a backend.

use crate::socket::{SocketRef, SocketType};

/// The result of a change made to a graph.
pub type GraphResult<T> = Result<T, GraphError>;

/// One thing wrong with a graph, named well enough to act on.
///
/// Every variant names both ends of whatever went wrong. A message that says only "bad
/// socket" leaves the caller with a graph to bisect, which is the situation this crate was
/// written to end.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    /// A wire referred to a node that was never added.
    #[error("node {node} is not in this surface")]
    NoSuchNode {
        /// The position that was asked for.
        node: usize,
    },

    /// A wire referred to a socket that the named node does not have.
    #[error("no output socket {} on node {}", socket.output, socket.node.0)]
    NoSuchSocket {
        /// The socket that was asked for.
        socket: SocketRef,
    },

    /// An input index past the end of what the operation takes.
    #[error("{node} takes {arity} inputs, so there is no input {input}")]
    NoSuchInput {
        /// The node that was wired.
        node: String,
        /// The input index that was asked for.
        input: usize,
        /// How many inputs it really has.
        arity: usize,
    },

    /// Two sockets of different types were wired together.
    #[error("input {input} of {node} takes a {wanted} but was given a {got}")]
    TypeMismatch {
        /// The node that was wired.
        node: String,
        /// Which of its inputs.
        input: usize,
        /// The type that input takes.
        wanted: SocketType,
        /// The type it was given.
        got: SocketType,
    },

    /// Nothing in the surface draws it.
    #[error("this surface has no output socket named as its surface")]
    NoSurface,

    /// A node nothing reads and that does not draw the surface.
    #[error("{node} is computed but nothing reads it")]
    DeadNode {
        /// The node that is wasted.
        node: String,
    },

    /// A set of nodes that read each other in a circle.
    #[error("these nodes read each other in a circle: {}", nodes.join(" -> "))]
    Cycle {
        /// The nodes in the order the wires run.
        nodes: Vec<String>,
    },
}
