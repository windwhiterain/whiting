//! The operations a surface may be built from, and the one table that describes them.
//!
//! # Why a table and not names
//!
//! Every backend spells its sockets differently, and — measured on the executable in
//! `out/probe-sockets.py` — **spells the wrong ones silently**. An unknown connect *target*
//! is an error; an unknown connect *source* is not, and leaves the surface black. An unknown
//! node *attribute* is not either, and leaves the node at a default that is usually zero.
//!
//! So no socket name is ever written by hand anywhere in this repository. Each operation
//! below declares its input and output types once, in [`Op::inputs`] and [`Op::outputs`],
//! and a backend compiler resolves its own names from that one declaration. An operation a
//! compiler does not know how to spell is an error at emit time, not a black render.
//!
//! # Scope
//!
//! These are the operations the gallery's five procedural surfaces need, plus the arithmetic
//! to combine them. This is deliberately not "everything a renderer can do": an operation
//! that no scene uses is an untested promise to every backend that has to implement it.

/// The real-number operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MathOp {
    /// `a + b`
    Add,
    /// `a - b`
    Subtract,
    /// `a * b`
    Multiply,
    /// `a / b`
    Divide,
    /// `a * b + c`
    MultiplyAdd,
    /// `a ^ b`
    Power,
    /// The smaller of two numbers.
    Minimum,
    /// The larger of two numbers.
    Maximum,
    /// `a` held between `low` and `high`.
    Clamp,
    /// The fractional part of `a`, always in zero to one.
    Fraction,
    /// `a` rounded down.
    Floor,
    /// `a` rounded away from zero.
    Round,
    /// `a` without its sign.
    Absolute,
    /// The sine of `a`, in radians.
    Sine,
    /// `a` as a fraction of `b`, zero to one.
    FractionOf,
    /// One when `a >= b`, otherwise zero.
    Step,
    /// Zero to one across `low` to `high`, flat at both ends.
    SmoothStep,
    /// Linear interpolation from `a` to `b` by `t`.
    Mix,
    /// One when `a > b`, otherwise zero.
    GreaterThan,
    /// The reciprocal of `a`.
    Reciprocal,
}

impl MathOp {
    /// How many inputs this operation takes, and what each one is.
    pub const fn arity(self) -> usize {
        match self {
            MathOp::Absolute
            | MathOp::Sine
            | MathOp::Fraction
            | MathOp::Floor
            | MathOp::Round
            | MathOp::Reciprocal => 1,
            MathOp::MultiplyAdd => 3,
            _ => 2,
        }
    }
}

/// The operations on points and directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorMathOp {
    /// Componentwise `a + b`.
    Add,
    /// Componentwise `a - b`.
    Subtract,
    /// Componentwise `a * b`.
    Multiply,
    /// Componentwise `a / b`.
    Divide,
    /// The vector product of two directions.
    CrossProduct,
    /// The scalar product of two vectors.
    DotProduct,
    /// The length of a vector.
    Length,
    /// The same direction at unit length.
    Normalize,
    /// Componentwise `a * b + c`, where `b` is a number.
    ScaleAdd,
    /// The angle between two directions.
    Angle,
    /// Componentwise `a * -b`.
    Reflect,
    /// The componentwise larger of two vectors.
    Maximum,
    /// The componentwise smaller of two vectors.
    Minimum,
    /// Componentwise absolute value.
    Absolute,
    /// `a` scaled by `b`, where `b` is a number.
    Scale,
}

impl VectorMathOp {
    /// How many inputs this operation takes.
    pub const fn arity(self) -> usize {
        match self {
            VectorMathOp::Length | VectorMathOp::Normalize | VectorMathOp::Absolute => 1,
            VectorMathOp::ScaleAdd => 3,
            _ => 2,
        }
    }

    /// What this operation yields. Three operations yield a number rather than a vector,
    /// and getting that wrong is the difference between a surface and a black render, so it
    /// is stated here once instead of being re-derived by each backend.
    pub const fn result(self) -> ResultType {
        match self {
            VectorMathOp::DotProduct | VectorMathOp::Length | VectorMathOp::Angle => {
                ResultType::Float
            }
            _ => ResultType::Vector,
        }
    }
}

/// What an operation yields, as opposed to a socket that can carry it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResultType {
    /// A number.
    Float,
    /// A point or direction.
    Vector,
    /// A colour.
    Color,
}

impl From<ResultType> for crate::socket::SocketType {
    fn from(value: ResultType) -> Self {
        match value {
            ResultType::Float => crate::socket::SocketType::Float,
            ResultType::Vector => crate::socket::SocketType::Vector,
            ResultType::Color => crate::socket::SocketType::Color,
        }
    }
}

/// How a [`Op::MapRange`] maps one range onto another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interpolation {
    /// A straight line from `to_min` at `from_min` to `to_max` at `from_max`.
    Linear,
    /// The same, but curved by an exponent on the fraction.
    SmoothStep,
    /// The same, but curved by a smootherstep.
    SmootherStep,
}

/// Which number a [`Op::MapRange`] takes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapRangeTarget {
    /// The value to be mapped, in zero to one before the range is applied.
    Factor,
    /// The value to be mapped, untouched.
    Value,
}

/// Which quantity of the shading point a [`Op::Geometry`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeometryField {
    /// Where the shading point is in the world. This is what a surface that must stay put
    /// under camera motion is driven by.
    Position,
    /// The surface normal as it was interpolated.
    Normal,
    /// The true geometric normal of the face, untouched by a bump.
    TrueNormal,
    /// The direction from the shading point toward the viewer.
    Incoming,
    /// The tangent across the surface.
    Tangent,
    /// How curved the surface is here, zero to one. Used for wear at edges.
    Pointiness,
}

/// Which number a [`Op::ObjectInfo`] reports about the object being shaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectInfoField {
    /// Where the object sits in the world.
    Location,
    /// The index of the object within the scene.
    ObjectIndex,
    /// The index of the material within the object.
    MaterialIndex,
    /// A number that is the same for every shading point of one object, and different
    /// between objects. This is the honest way to vary a surface per object without
    /// varying it per fragment, and it is what replaces a per-face authored id.
    Random,
}

/// One operation in a surface.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Arithmetic on one real number.
    Math(MathOp),
    /// Arithmetic on points and directions.
    VectorMath(VectorMathOp),
    /// Remap one range of numbers onto another. The gallery's WGSL reaches for
    /// `smoothstep`, and every backend spells that differently; naming it once here is
    /// what lets one call mean the same curve everywhere.
    MapRange {
        /// Which input carries the number being remapped.
        target: MapRangeTarget,
        /// The shape of the curve between the two ends.
        interpolation: Interpolation,
        /// Whether values outside the range are held at the ends or allowed past them.
        clamp: bool,
    },
    /// Linear interpolation between two numbers.
    MixFloat,
    /// Linear interpolation between two colours. The factor is zero at the first colour.
    MixColor,
    /// Linear interpolation between two vectors.
    MixVector,
    /// Pack three numbers into a vector.
    CombineXyz,
    /// Unpack a vector into three numbers.
    SeparateXyz,
    /// A number computed from the shading point rather than written down.
    Geometry(GeometryField),
    /// A number read off a mesh attribute written when the mesh was emitted.
    Attribute {
        /// The attribute's name, which both backends must be given the same spelling of.
        name: String,
    },
    /// A number about the object being shaded.
    ObjectInfo(ObjectInfoField),
    /// A number about the camera.
    Camera(CameraField),
    /// Perturb a normal by the gradient of a number across the surface. Takes the height
    /// and yields the new normal.
    Bump,
    /// The industry-standard physically based surface: a base colour, a metalness, a
    /// roughness and a dielectric response. Named for the model, not for any renderer's
    /// node, because the model is shared and the spelling is not.
    Principled,
    /// A plain diffuse surface, for the cases that want one and do not want to name a
    /// roughness for it.
    Diffuse,
}

/// Which quantity of the camera a [`Op::Camera`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraField {
    /// Where the eye is in the world. A surface that fades with distance needs this and
    /// gets it from the camera rather than from a uniform.
    Position,
}

impl Op {
    /// What each input of this operation is, in the order the inputs are written.
    ///
    /// An empty slice is an operation with nothing wired to it — every parameter it has is
    /// a constant the backend spells in the document instead.
    pub fn inputs(&self) -> Vec<SocketType> {
        match self {
            Op::Math(operation) => vec![SocketType::Float; operation.arity()],
            Op::VectorMath(operation) => match operation {
                // The second input to these three is a number, not a vector.
                VectorMathOp::Scale | VectorMathOp::ScaleAdd => {
                    let mut inputs = vec![SocketType::Vector; operation.arity()];
                    for socket in inputs.iter_mut().skip(1) {
                        *socket = SocketType::Float;
                    }
                    inputs
                }
                _ => vec![SocketType::Vector; operation.arity()],
            },
            Op::MapRange { .. } => {
                // value, from min, from max, to min, to max
                vec![SocketType::Float; 5]
            }
            Op::MixFloat => vec![SocketType::Float, SocketType::Float, SocketType::Float],
            Op::MixColor => vec![SocketType::Float, SocketType::Color, SocketType::Color],
            Op::MixVector => vec![SocketType::Float, SocketType::Vector, SocketType::Vector],
            Op::CombineXyz => vec![SocketType::Float; 3],
            Op::SeparateXyz => vec![SocketType::Vector],
            Op::Geometry(_) | Op::ObjectInfo(_) | Op::Camera(_) => Vec::new(),
            Op::Attribute { .. } => Vec::new(),
            Op::Bump => vec![SocketType::Float, SocketType::Float],
            Op::Principled => vec![
                SocketType::Color,
                SocketType::Float,
                SocketType::Float,
                SocketType::Float,
                SocketType::Vector,
            ],
            Op::Diffuse => vec![SocketType::Color, SocketType::Float],
        }
    }

    /// What this operation yields.
    pub fn outputs(&self) -> Vec<SocketType> {
        match self {
            Op::Math(_) => vec![SocketType::Float],
            Op::VectorMath(operation) => vec![operation.result().into()],
            Op::MapRange { .. } => vec![SocketType::Float],
            Op::MixFloat => vec![SocketType::Float],
            Op::MixColor => vec![SocketType::Color],
            Op::MixVector => vec![SocketType::Vector],
            Op::CombineXyz => vec![SocketType::Vector],
            Op::SeparateXyz => vec![SocketType::Float; 3],
            Op::Geometry(_) | Op::ObjectInfo(_) | Op::Camera(_) | Op::Attribute { .. } => {
                vec![SocketType::Float]
            }
            Op::Bump => vec![SocketType::Vector],
            Op::Principled | Op::Diffuse => vec![SocketType::Float],
        }
    }

    /// A short name for this operation, used when a graph is reported to a person.
    ///
    /// Deliberately not a backend's name: a report that says `map_range` when the IR says
    /// `MapRange` is a report about the wrong graph.
    pub fn describe(&self) -> String {
        match self {
            Op::Math(operation) => format!("math.{operation:?}"),
            Op::VectorMath(operation) => format!("vector_math.{operation:?}"),
            Op::MapRange {
                target,
                interpolation,
                clamp,
            } => format!(
                "map_range.{target:?}.{interpolation:?}.{}",
                if *clamp { "clamped" } else { "unclamped" }
            ),
            Op::MixFloat => "mix_float".to_owned(),
            Op::MixColor => "mix_color".to_owned(),
            Op::MixVector => "mix_vector".to_owned(),
            Op::CombineXyz => "combine_xyz".to_owned(),
            Op::SeparateXyz => "separate_xyz".to_owned(),
            Op::Geometry(field) => format!("geometry.{field:?}"),
            Op::Attribute { name } => format!("attribute.{name}"),
            Op::ObjectInfo(field) => format!("object_info.{field:?}"),
            Op::Camera(field) => format!("camera.{field:?}"),
            Op::Bump => "bump".to_owned(),
            Op::Principled => "principled".to_owned(),
            Op::Diffuse => "diffuse".to_owned(),
        }
    }
}

use crate::socket::SocketType;
