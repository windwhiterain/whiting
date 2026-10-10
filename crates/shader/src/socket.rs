//! What a socket carries, and how one socket refers to another.
//!
//! # Why these three and no more
//!
//! A surface is a function of a real number, a point, or a colour, and a target that cannot be
//! asked for any other type is not a renderer we can target. Every type here is one of those
//! three, so a graph written against this module compiles to every backend, and a backend
//! that wants a fourth has to reject the graph rather than reinterpret it.
//!
//! The alternative — carrying `f64` and `[f64; 3]` in one untyped `Value` — would move the
//! type error from the compiler into the renderer. That is the failure this whole crate
//! exists to prevent; see the note on silent failures in [crate::graph].

/// What flows through one socket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketType {
    /// One real number.
    Float,
    /// A point or direction in three dimensions.
    Vector,
    /// A colour. Stored as three channels; alpha is never a separate socket, because a
    /// surface that decides its own transparency cannot be importance sampled and no
    /// backend in this repository wants it.
    Color,
}

impl std::fmt::Display for SocketType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

impl SocketType {
    /// The name used when a graph is reported to a person.
    pub fn name(self) -> &'static str {
        match self {
            SocketType::Float => "float",
            SocketType::Vector => "vector",
            SocketType::Color => "color",
        }
    }
}

/// Names one node inside one graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// Names one output socket of one node.
///
/// Sockets are positional rather than named here, on purpose. Every backend in this repository
/// spells its sockets differently — and spells the *wrong* ones silently — so a name in the IR
/// would be a name somebody has to get right per backend. An index is resolved through the
/// operation's own table, which has one row per operation, and there is no way to reach a
/// socket that row does not describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketRef {
    /// The node the socket belongs to.
    pub node: NodeId,
    /// Which of that node's outputs this is.
    pub output: usize,
}

/// A literal written into the graph rather than computed by it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Literal {
    /// A real number.
    Float(f64),
    /// Three components, used for both points and colours.
    Triple([f64; 3]),
}

impl Literal {
    /// The socket type this literal can be connected as.
    pub fn socket_type(self) -> SocketType {
        match self {
            Literal::Float(_) => SocketType::Float,
            Literal::Triple(_) => SocketType::Vector,
        }
    }

    /// The three components, if this is not a single number.
    pub fn triple(self) -> Option<[f64; 3]> {
        match self {
            Literal::Float(_) => None,
            Literal::Triple(value) => Some(value),
        }
    }

    /// The single number, if this is not three components.
    pub fn float(self) -> Option<f64> {
        match self {
            Literal::Float(value) => Some(value),
            Literal::Triple(_) => None,
        }
    }

    /// This literal as the given type, when it already is that type.
    ///
    /// A triple is accepted as a colour because a colour and a point are the same three
    /// numbers and nothing in a surface distinguishes them until a backend says so.
    pub fn cast(self, target: SocketType) -> Option<Literal> {
        match (self, target) {
            (value @ Literal::Float(_), SocketType::Float) => Some(value),
            (value @ Literal::Triple(_), SocketType::Vector | SocketType::Color) => Some(value),
            _ => None,
        }
    }
}

/// One input to a node: either a number written down, or a wire back to another node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    /// A constant.
    Constant(Literal),
    /// A socket on another node. Self-reference is refused by [crate::graph::Graph::validate].
    Socket(SocketRef),
}

impl Input {
    /// The type this input delivers, when it delivers one that can be known.
    pub fn socket_type(self, graph: &crate::graph::Graph) -> Option<SocketType> {
        match self {
            Input::Constant(value) => Some(value.socket_type()),
            Input::Socket(socket) => graph.output_type(socket),
        }
    }

    /// This input as a number, when it is one and when it is known.
    pub fn constant_float(self) -> Option<f64> {
        match self {
            Input::Constant(Literal::Float(value)) => Some(value),
            _ => None,
        }
    }
}
