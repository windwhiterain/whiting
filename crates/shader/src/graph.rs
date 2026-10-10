//! A surface, as a graph, and the checks that make it worth emitting.
//!
//! # Why this is a typed Rust API and not a document
//!
//! The `graphics` crate in this workspace holds its instruction set as data, so an agent can
//! edit a graphics document with no recompilation. This is the other case. A surface is
//! *built* by code — the gallery's Rust decides what the stone looks like — and the thing it
//! has to beat is a renderer that reports a wrong surface as a correctly rendered one. A
//! document would move the type error to run time and into the render; a typed API moves it
//! to compile time and into the build.
//!
//! # What is checked, and why each check earns its place
//!
//! The cycles emitter previously turned a material into one `principled_bsdf` and had no way
//! to say anything more. Everything below exists because the alternative was measured, not
//! imagined:
//!
//! - **a socket that does not exist** — a backend's wrong socket name is silent, so the
//!   socket an operation yields is an index into [`Op::outputs`], and an index past the end
//!   is refused here;
//! - **a wire between two types that do not match** — refused at the edge, where both nodes
//!   can be named, rather than at emit time where neither can;
//! - **a node nothing reads** — dead weight the backend will happily compile;
//! - **a cycle** — a backend may hang, and a surface that reads its own output is a mistake
//!   in the builder rather than a feature.
//!
//! Every one of these is a *named* failure. That is the whole point: the alternative is a
//! render that is wrong and says nothing.

use crate::error::{GraphError, GraphResult};
use crate::op::Op;
use crate::socket::{Input, Literal, NodeId, SocketRef, SocketType};

/// One operation in a surface, with whatever is wired to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    operation: Op,
    inputs: Vec<Input>,
}

impl Node {
    /// What this node computes.
    pub fn operation(&self) -> &Op {
        &self.operation
    }

    /// What is wired to each of its inputs, in order.
    pub fn inputs(&self) -> &[Input] {
        &self.inputs
    }

    /// What this node yields, in order.
    pub fn outputs(&self) -> Vec<SocketType> {
        self.operation.outputs()
    }

    /// What one of this node's outputs is, if there is one.
    pub fn output_type(&self, output: usize) -> Option<SocketType> {
        self.outputs().get(output).copied()
    }

    /// This node's operation as it reads in an error message.
    pub fn describe(&self) -> String {
        self.operation.describe()
    }
}

/// A surface: a set of nodes, and the one of them that draws the surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    nodes: Vec<Node>,
    surface: Option<SocketRef>,
}

impl Graph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node with a zero written into each of its inputs, and returns its id.
    ///
    /// The zeros are a starting point a builder overwrites, not a decision: an input the
    /// caller forgets is left at zero on purpose, because a backend does the same thing
    /// silently and a zero is at least a number a person can look at.
    pub fn add(&mut self, operation: Op) -> NodeId {
        let inputs = vec![Input::Constant(Literal::Float(0.0)); operation.inputs().len()];
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node { operation, inputs });
        id
    }

    /// Wires one input of one node.
    ///
    /// Returns an error rather than replacing the input when the index does not exist, or
    /// when the two sides are of different types. Both are mistakes a caller makes by
    /// forgetting that an operation takes three inputs, and both are invisible at the far
    /// end of a backend that ignores what it does not understand.
    pub fn wire(&mut self, node: NodeId, input: usize, from: Input) -> GraphResult<()> {
        let node_ref = self
            .node(node)
            .ok_or(GraphError::NoSuchNode { node: node.0 })?;
        let arity = node_ref.operation.inputs().len();
        let declared = *node_ref
            .operation
            .inputs()
            .get(input)
            .ok_or(GraphError::NoSuchInput {
                node: node_ref.describe(),
                input,
                arity,
            })?;

        let delivered = match from {
            Input::Constant(value) => value.socket_type(),
            Input::Socket(socket) => self
                .output_type(socket)
                .ok_or(GraphError::NoSuchSocket { socket })?,
        };
        if delivered != declared {
            return Err(GraphError::TypeMismatch {
                node: node_ref.describe(),
                input,
                wanted: declared,
                got: delivered,
            });
        }

        self.node_mut(node).unwrap().inputs[input] = from;
        Ok(())
    }

    /// Names the socket that draws the surface.
    pub fn set_surface(&mut self, socket: SocketRef) {
        self.surface = Some(socket);
    }

    /// The socket that draws the surface, if one has been named.
    pub fn surface(&self) -> Option<SocketRef> {
        self.surface
    }

    /// Every node, in the order they were added.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// One node by its id.
    pub fn node(&self, node: NodeId) -> Option<&Node> {
        self.nodes.get(node.0)
    }

    fn node_mut(&mut self, node: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node.0)
    }

    /// What one socket yields, if it exists.
    pub fn output_type(&self, socket: SocketRef) -> Option<SocketType> {
        self.node(socket.node)?.output_type(socket.output)
    }

    /// Whether anything reads the given socket.
    pub fn socket_is_read(&self, socket: SocketRef) -> bool {
        if self.surface == Some(socket) {
            return true;
        }
        self.nodes
            .iter()
            .any(|node| node.inputs.contains(&Input::Socket(socket)))
    }

    /// Whether any input reads this node, or the surface is drawn by it.
    pub fn node_is_read(&self, node: NodeId) -> bool {
        match self.node(node) {
            None => false,
            Some(found) => (0..found.outputs().len())
                .any(|output| self.socket_is_read(SocketRef { node, output })),
        }
    }

    /// Checks the whole graph and returns every problem found, rather than the first.
    ///
    /// A builder that fixed one wire at a time would report one mistake per run, and a graph
    /// built in one function tends to have several.
    pub fn validate(&self) -> Vec<GraphError> {
        let mut problems = Vec::new();

        let Some(surface) = self.surface else {
            problems.push(GraphError::NoSurface);
            return problems;
        };
        if self.output_type(surface).is_none() {
            problems.push(GraphError::NoSuchSocket { socket: surface });
            return problems;
        }

        for node in &self.nodes {
            for (index, input) in node.inputs.iter().enumerate() {
                let Input::Socket(socket) = *input else {
                    continue;
                };
                match self.output_type(socket) {
                    None => problems.push(GraphError::NoSuchSocket { socket }),
                    Some(got) => {
                        if let Some(wanted) = node.operation.inputs().get(index) {
                            if wanted != &got {
                                problems.push(GraphError::TypeMismatch {
                                    node: node.describe(),
                                    input: index,
                                    wanted: *wanted,
                                    got,
                                });
                            }
                        }
                    }
                }
            }
        }

        for (index, node) in self.nodes.iter().enumerate() {
            if !self.node_is_read(NodeId(index)) {
                problems.push(GraphError::DeadNode {
                    node: node.describe(),
                });
            }
        }

        for cycle in self.cycles() {
            problems.push(GraphError::Cycle {
                nodes: cycle
                    .iter()
                    .map(|id| self.node(*id).map(Node::describe).unwrap_or_default())
                    .collect(),
            });
        }

        problems
    }

    /// Every set of nodes that read each other in a circle.
    fn cycles(&self) -> Vec<Vec<NodeId>> {
        let mut found: Vec<Vec<NodeId>> = Vec::new();
        let mut state = vec![Visit::Unseen; self.nodes.len()];
        for index in 0..self.nodes.len() {
            self.walk(NodeId(index), &mut state, &mut Vec::new(), &mut found);
        }
        found
    }

    fn walk(
        &self,
        node: NodeId,
        state: &mut [Visit],
        path: &mut Vec<NodeId>,
        found: &mut Vec<Vec<NodeId>>,
    ) {
        match state.get(node.0).copied().unwrap_or(Visit::Unseen) {
            Visit::OnPath => {
                let start = path.iter().position(|id| *id == node).unwrap_or(0);
                found.push(path[start..].to_vec());
                return;
            }
            Visit::Done => return,
            Visit::Unseen => {}
        }
        let Some(here) = self.node(node) else {
            return;
        };
        state[node.0] = Visit::OnPath;
        path.push(node);

        for input in &here.inputs {
            if let Input::Socket(socket) = input {
                self.walk(socket.node, state, path, found);
            }
        }

        path.pop();
        state[node.0] = Visit::Done;
    }
}

/// Where a depth-first walk has reached. Written as its own type rather than three booleans so
/// that "on the current path" and "finished" cannot be confused for one another.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Unseen,
    OnPath,
    Done,
}
