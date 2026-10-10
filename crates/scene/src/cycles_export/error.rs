//! What can go wrong, and what it says about the scene that caused it.

use std::fmt;
use std::path::PathBuf;

/// A field the target format has no way to carry.
///
/// # Why this is a type and not a string
///
/// The set of things Cycles' XML loader cannot express is small, bounded and known: five
/// hardcoded vertex attributes and no extension point. Naming them lets the caller match
/// on the case, and lets a test assert that a given scene is rejected for the right reason
/// rather than for any error at all.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedField {
    /// A `point_schema` field with no Cycles attribute to map it to.
    ///
    /// Cycles reads position, vertex normal, UV, UV tangent and tangent sign. Anything else
    /// a mesh declares is dropped or refused here.
    PointAttribute { field: String, mesh: String },
    /// A `custom_schema` field with nowhere to go.
    ///
    /// Cycles' material parameters are compiled into the shader graph, not carried as a
    /// side channel, so a material's custom data has to be folded into a `principled_bsdf`
    /// socket at emit time. Only `tint` has one.
    CustomData { field: String, material: String },
    /// An `instance_schema` field beyond the model matrix.
    InstanceAttribute { field: String },
    /// A `Topology` the loader cannot triangulate.
    ///
    /// The loader's mesh path builds triangles only. There is no curve element in its
    /// element dispatch, so a segment has no representation at all.
    Segments { mesh: String },
    /// A light with no `light_type` mapping.
    Light { kind: String },
    /// An environment map in a projection the loader cannot read.
    ///
    /// Cycles' world lookup is **equirectangular** and its `TextureMapping` projection enum is
    /// flat, cube, tube and sphere — there is no octahedral option to ask for. A map folded
    /// onto an octahedron is therefore not a map Cycles cannot reach, it is a map Cycles reads
    /// as latlong: measured, `foldyard` handed its octahedral sky over and came back with the
    /// left half of the sky black, the right half blue, and a hard vertical seam down the
    /// middle. Exit status zero, no error, a complete frame.
    ///
    /// So this is refused rather than converted. Converting would mean the emitter holding a
    /// second definition of how a map is laid out, which is the failure this whole case exists
    /// to stop: two places that know the answer and eventually disagree.
    ImageProjection { image: String, projection: String },
    /// A shader stage Cycles has no equivalent of.
    ///
    /// `ShadowingShader`, `DepthShader` and `ScreenShader` describe stages of the rasterised
    /// pipeline. Cycles derives shadow rays and depth from its own integrator, so a
    /// `DepthShader` has nothing to be emitted as.
    ShaderStage { stage: String },
}

impl fmt::Display for UnsupportedField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointAttribute { field, mesh } => write!(
                formatter,
                "mesh '{mesh}' declares point attribute '{field}', which the Cycles XML \
                 format cannot carry. The loader reads only position, normal, UV, UV \
                 tangent and tangent sign, and has no way to introduce another"
            ),
            Self::CustomData { field, material } => write!(
                formatter,
                "material '{material}' declares custom data '{field}', which has no \
                 principled_bsdf socket to fold into. Only 'tint' maps"
            ),
            Self::InstanceAttribute { field } => write!(
                formatter,
                "instance attribute '{field}' has no Cycles equivalent. The model matrix is \
                 written as a transform scope"
            ),
            Self::Segments { mesh } => write!(
                formatter,
                "mesh '{mesh}' uses Segments topology. The Cycles XML loader triangulates \
                 polygons and has no curve element, so segments cannot be emitted"
            ),
            Self::Light { kind } => {
                write!(
                    formatter,
                    "light kind '{kind}' has no Cycles light_type mapping"
                )
            }
            Self::ImageProjection { image, projection } => {
                write!(
                    formatter,
                    "the image '{image}' is {projection}, and Cycles reads an environment map \
                     as equirectangular only: its TextureMapping projection enum is flat, cube, \
                     tube and sphere, with no octahedral option. Emitted as it stands it would \
                     load, and the frame would have half a sky"
                )
            }
            Self::ShaderStage { stage } => write!(
                formatter,
                "shader stage '{stage}' describes a stage of the rasterised pipeline. \
                 Cycles derives shadow and depth rays from its own integrator and has no \
                 equivalent pass to emit"
            ),
        }
    }
}

/// Why an emit failed.
#[derive(Debug)]
pub enum EmitError {
    /// The scene asks for something the format cannot express, and
    /// [`crate::cycles_export::LossyMode::Reject`] was in force.
    Unsupported(UnsupportedField),
    /// The same, in [`crate::cycles_export::LossyMode::Degrade`], where the caller asked to
    /// be told rather than to be stopped.
    ///
    /// Kept as its own variant so a caller can log the difference between "this scene
    /// cannot be exported" and "this scene was exported, approximately", which are very
    /// different things to tell an author.
    Degraded(UnsupportedField),
    /// A value was structurally wrong: a light with no colour, a mesh with no points.
    Malformed { detail: String },
    /// The document could not be written to disk.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(field) => write!(formatter, "{field}"),
            Self::Degraded(field) => {
                write!(formatter, "cannot emit: {field}")
            }
            Self::Malformed { detail } => write!(formatter, "malformed scene: {detail}"),
            Self::Write { path, source } => {
                write!(formatter, "writing {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for EmitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl EmitError {
    /// The field this error is about, whichever variant it arrived in.
    pub fn field(&self) -> Option<&UnsupportedField> {
        match self {
            Self::Unsupported(field) | Self::Degraded(field) => Some(field),
            Self::Malformed { .. } | Self::Write { .. } => None,
        }
    }
}

/// The result of an emit.
pub type EmitResult<T> = Result<T, EmitError>;
