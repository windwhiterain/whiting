//! Checking that a shader reads its resources and its vertices where the pass put them.
//!
//! # Why this check exists
//!
//! A pass binds its resources at numbers it chooses: the uniform is zero, and the bound
//! resources follow it in name order. A vertex buffer's attributes are numbered the same
//! way, continuing from where the previous buffer's ended. A shader names both, by hand,
//! in its own source.
//!
//! Nothing connects the two. A shader that says `@binding(3)` when the pass bound four
//! resources is a valid shader, the pipeline is valid, and the frame is wrong — or the
//! backend rejects the pipeline with a message about a view dimension that names neither
//! the shader nor the pass, which is the worst of both: a failure that is real and a
//! message that does not say which document to open.
//!
//! The uniform was the same problem and it was closed: the device parses the shader and
//! compares its uniform struct against the interface the pass was packed for
//! ([`super::uniform`]). These are the other two places the same agreement is needed, and
//! they are checked the same way: parse, compare, and report a disagreement as an error
//! naming both sides.
//!
//! The view dimension in the paragraph above used to be the one case this check could not
//! report, because there was only one texture shape to get wrong. There are two now, and
//! which of them a shader reads is a disagreement this module makes on its own rather than
//! leaving to a backend message: see the second invariant.
//!
//! # Invariants
//!
//! - **The shader is the side that is checked against the pass, never the reverse.** A
//!   layout may declare an entry no shader reads; that is legal and common, and reporting
//!   it would make every shader that ignores a binding an error.
//! - **A buffer is compared as a buffer, and a texture as its shape.** A buffer is a buffer
//!   whether it is a uniform or a storage buffer, so its address space is not part of the
//!   comparison. A texture is a different thing rather than a different degree of one:
//!   `texture_depth_2d` and `texture_depth_2d_array` are two WGSL types, and a
//!   `textureLoad` on an array takes an index argument a flat texture has nowhere to put.
//!   A shape that was not compared would let a shader declare an array where the pass bound
//!   a flat texture, and the failure would arrive as a backend message about a view
//!   dimension naming neither document — the outcome the section above says this module
//!   exists to prevent.
//! - **A shape nothing in this binding model builds is left alone.** A one dimensional
//!   image is a WGSL type no document here can declare and the device never creates, so
//!   there is nothing for either side to agree or disagree about, and the comparison skips
//!   it as it skips a resource kind it has no opinion on.
//! - **A shader is only judged on what it actually reads.** A module with no entry point,
//!   or a pass that binds nothing, is not a disagreement: there is nothing to disagree
//!   with.
//! - The parse is the one the renderer already caches against its other parses, so a frame
//!   that changes no shader pays for the comparison and not for the parse.

use graphics::Binder;
use wgpu::naga;

use super::WgpuRenderer;
use crate::render::RenderError;

/// The shape of an image, which is the part of it a shader's *type* names.
///
/// # Why a texture carries one of these and a buffer carries nothing
///
/// The pages of a pool are not the same resource at different sizes, they are two WGSL
/// types, and a shader reads them with different builtins. Comparing the shape is
/// therefore comparing what the shader says it reads rather than how large it is, which is
/// the same grain as comparing a buffer to a buffer. The variants are named as a shader
/// names them, because that is the vocabulary a reader of the document is working in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageShape {
    /// One two dimensional image, which is `texture_2d` or `texture_depth_2d`.
    ///
    /// This is every texture in the project that is not a pool, and a document has to say
    /// so rather than leave it to be inferred, because a reader cannot otherwise tell a
    /// flat texture from an array.
    One,
    /// An array of two dimensional images, `texture_2d_array` or `texture_depth_2d_array`,
    /// whose `textureLoad` takes the layer as an argument.
    Array,
    /// A cube, `texture_cube`.
    Cube,
    /// A volume, `texture_3d`.
    Volume,
}

impl ImageShape {
    /// Returns how a shape is named in an error, so both sides of a disagreement are
    /// described in the same words.
    fn name(self) -> &'static str {
        match self {
            Self::One => "a two dimensional texture",
            Self::Array => "an array of two dimensional textures",
            Self::Cube => "a cube texture",
            Self::Volume => "a volume texture",
        }
    }
}

/// What a resource is, at the coarsest grain a binding number can be compared at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    /// A buffer, uniform or storage: the uniform is one too, and is at binding zero.
    Buffer,
    /// An image, sampled or read, at the shape it is bound as.
    Texture(ImageShape),
    /// A sampler, which reads a texture and is a separate resource from it.
    Sampler,
}

impl ResourceKind {
    /// Returns how a kind is named in an error, so both sides of a disagreement are
    /// described in the same words.
    fn name(self) -> &'static str {
        match self {
            Self::Buffer => "a buffer",
            Self::Texture(shape) => shape.name(),
            Self::Sampler => "a sampler",
        }
    }
}

impl WgpuRenderer {
    /// Checks that every resource a shader declares in the pass's group is bound, and
    /// bound as the kind and the shape the shader says it is.
    ///
    /// The pass is named in every error, because a bound shader entry has no name of its
    /// own and the pass is what a reader is looking at.
    ///
    /// This is the half that needs a renderer: it finds the shader and parses it, and the
    /// parse is the one this renderer caches against its other parses. The comparison
    /// itself is [`check_resources`], which needs nothing but a module and a layout and is
    /// therefore checkable on a machine with no adapter.
    pub(super) fn check_bindings(
        &self,
        binder: &Binder<'_>,
        entries: &[wgpu::BindGroupLayoutEntry],
        label: &str,
    ) -> Result<(), RenderError> {
        let source = self.shader_source(binder)?;
        let module = self.parsed_module(source, label)?;
        check_resources(&module, entries, label)
    }

    /// Checks that every vertex a shader reads is an attribute this pass binds, read at
    /// the width the shader declares.
    ///
    /// A built-in is not an attribute and is not checked: `@builtin(position)` is supplied
    /// by the pipeline rather than by a buffer, and a shader that read a built-in from a
    /// buffer would not compile in the first place.
    pub(super) fn check_vertex_inputs(
        &self,
        binder: &Binder<'_>,
        locations: &[(u32, wgpu::VertexFormat)],
        label: &str,
    ) -> Result<(), RenderError> {
        let source = self.shader_source(binder)?;
        let module = self.parsed_module(source, label)?;
        for (name, location, declared) in declared_vertices(&module) {
            let Some((_, format)) = locations.iter().find(|(bound, _)| *bound == location) else {
                return Err(RenderError::Backend(format!(
                    "the shader for `{label}` reads `{name}` at vertex location {location}, and \
                     this pass binds no attribute there. A point's fields are laid out from \
                     location zero and the instance row continues from where they ended, so the \
                     locations a shader uses are the ones the two schemas sort into."
                )));
            };
            let Some(bound) = components_of_format(*format) else {
                continue;
            };
            if Some(bound) != declared {
                return Err(RenderError::Backend(format!(
                    "the shader for `{label}` reads `{name}` at vertex location {location} as {} \
                     components, and this pass binds an attribute of {bound} there.",
                    declared.unwrap_or_default()
                )));
            }
        }
        Ok(())
    }
}

/// Returns every resource a shader declares in group zero, as its name, its binding
/// number and what it says the resource is.
///
/// A global with no binding is a module level constant rather than a resource, and a
/// handle that is neither an image nor a sampler is a type this comparison has no opinion
/// about; both are left out rather than guessed at. An image whose shape the binding model
/// never builds is left out on the same grounds, which is why the kind below is read
/// through [`shape_of_image`] rather than matched as `Image { .. }`.
fn declared_resources(module: &naga::Module) -> Vec<(String, u32, ResourceKind)> {
    module
        .global_variables
        .iter()
        .filter_map(|(_, variable)| {
            let binding = variable.binding.as_ref()?;
            if binding.group != 0 {
                return None;
            }
            let kind = match variable.space {
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. } => {
                    ResourceKind::Buffer
                }
                naga::AddressSpace::Handle => match module.types[variable.ty].inner {
                    naga::TypeInner::Image { dim, arrayed, .. } => {
                        ResourceKind::Texture(shape_of_image(dim, arrayed)?)
                    }
                    naga::TypeInner::Sampler { .. } => ResourceKind::Sampler,
                    _ => return None,
                },
                _ => return None,
            };
            Some((
                variable
                    .name
                    .clone()
                    .unwrap_or_else(|| "<unnamed>".to_owned()),
                binding.binding,
                kind,
            ))
        })
        .collect()
}

/// Returns every vertex a shader's vertex entry point reads, as its name, its location and
/// how many components it reads it as.
///
/// A module with no vertex entry point reads no vertices, which is a compute module rather
/// than a disagreement.
fn declared_vertices(module: &naga::Module) -> Vec<(String, u32, Option<u8>)> {
    let Some(entry) = module
        .entry_points
        .iter()
        .find(|entry| entry.stage == naga::ShaderStage::Vertex)
    else {
        return Vec::new();
    };
    // The entry point carries its own function, so there is no handle to resolve and
    // nothing that can fail to resolve.
    entry
        .function
        .arguments
        .iter()
        .filter_map(|argument| {
            let Some(naga::Binding::Location { location, .. }) = argument.binding else {
                return None;
            };
            Some((
                argument
                    .name
                    .clone()
                    .unwrap_or_else(|| "<unnamed>".to_owned()),
                location,
                components_of(&module.types[argument.ty].inner),
            ))
        })
        .collect()
}

/// Checks that every resource a shader declares in the pass's group is bound, and bound as
/// the kind and the shape the shader says it is.
///
/// # Invariants
///
/// - Only what a shader declares is compared, and only against the entry at the same
///   number. A layout entry no shader reads is legal and is never reported.
/// - A disagreement is named on both sides and the pass is named, because a bound shader
///   entry has no name of its own and the pass is what a reader is looking at.
/// - Nothing here needs a device, a queue or an adapter: it is a module and a list of
///   entries. That is what lets it be checked on a machine with no gpu, and it is why
///   [`WgpuRenderer::check_bindings`] is a thin wrapper over it rather than the other way
///   round.
pub fn check_resources(
    module: &naga::Module,
    entries: &[wgpu::BindGroupLayoutEntry],
    label: &str,
) -> Result<(), RenderError> {
    for (name, number, declared) in declared_resources(module) {
        let Some(entry) = entries.iter().find(|entry| entry.binding == number) else {
            return Err(RenderError::Backend(format!(
                "the shader for `{label}` declares `{name}` at binding {number}, and this \
                 pass binds nothing there. The uniform is binding zero and the bound \
                 resources follow it in name order, so the numbers a shader uses are the \
                 ones the document gave its binders."
            )));
        };
        let Some(bound) = kind_of(&entry.ty) else {
            continue;
        };
        if bound != declared {
            return Err(RenderError::Backend(format!(
                "the shader for `{label}` declares `{name}` at binding {number} as {}, and \
                 this pass binds {} there. A flat texture and an array of them are different \
                 types to a shader, and a `textureLoad` on an array takes the layer as an \
                 argument a flat texture has no place for, so a pass that binds one where a \
                 shader reads the other is a document that cannot be drawn.",
                declared.name(),
                bound.name()
            )));
        }
    }
    Ok(())
}

/// Returns the kind a layout entry binds, or nothing for a kind this comparison has no
/// opinion about.
///
/// The standard pipeline binds buffers, textures and samplers, and nothing else. A layout
/// entry of some other kind is left alone rather than forced into one of the three, for
/// the same reason a shader resource is: the check judges what both sides are saying, and
/// a side saying something else entirely is not a disagreement.
///
/// A texture entry's `view_dimension` is **not** one of the things that is discarded. It is
/// the shape the shader's own type has to agree with, so it is read out and compared.
fn kind_of(ty: &wgpu::BindingType) -> Option<ResourceKind> {
    match ty {
        wgpu::BindingType::Buffer { .. } => Some(ResourceKind::Buffer),
        wgpu::BindingType::Texture { view_dimension, .. }
        | wgpu::BindingType::StorageTexture { view_dimension, .. } => {
            Some(ResourceKind::Texture(shape_of_view(*view_dimension)?))
        }
        wgpu::BindingType::Sampler(_) => Some(ResourceKind::Sampler),
        _ => None,
    }
}

/// Returns the shape a layout entry binds a texture as, or nothing for a shape this
/// binding model never builds.
///
/// A one dimensional view is the one that has no counterpart here: no document declares a
/// one dimensional texture and the device never creates one, so there is nothing to compare
/// it against and it is left alone rather than forced into a shape it is not.
fn shape_of_view(dimension: wgpu::TextureViewDimension) -> Option<ImageShape> {
    match dimension {
        wgpu::TextureViewDimension::D2 => Some(ImageShape::One),
        wgpu::TextureViewDimension::D2Array => Some(ImageShape::Array),
        wgpu::TextureViewDimension::Cube => Some(ImageShape::Cube),
        wgpu::TextureViewDimension::D3 => Some(ImageShape::Volume),
        wgpu::TextureViewDimension::D1 | wgpu::TextureViewDimension::CubeArray => None,
    }
}

/// Returns the shape a shader's image type reads, or nothing for a shape this binding model
/// never builds.
///
/// # Invariants
///
/// An array is a `D2` image with `arrayed` set, which is how WGSL spells
/// `texture_depth_2d_array`, so the two are read together and a `D2` that is not arrayed
/// is the flat texture rather than a member of a family. A one dimensional image is left
/// alone for the reason [`shape_of_view`] gives.
fn shape_of_image(dimension: naga::ImageDimension, arrayed: bool) -> Option<ImageShape> {
    match (dimension, arrayed) {
        (naga::ImageDimension::D2, false) => Some(ImageShape::One),
        (naga::ImageDimension::D2, true) => Some(ImageShape::Array),
        (naga::ImageDimension::Cube, _) => Some(ImageShape::Cube),
        (naga::ImageDimension::D3, _) => Some(ImageShape::Volume),
        (naga::ImageDimension::D1, _) => None,
    }
}

/// Returns how many components a vertex type reads, or nothing for a type that is neither
/// a scalar nor a vector.
fn components_of(inner: &naga::TypeInner) -> Option<u8> {
    match inner {
        naga::TypeInner::Scalar { .. } => Some(1),
        naga::TypeInner::Vector { size, .. } => Some(*size as u8),
        _ => None,
    }
}

/// Returns how many components a vertex format reads, or nothing for a format the
/// standard pipeline does not emit.
fn components_of_format(format: wgpu::VertexFormat) -> Option<u8> {
    match format {
        wgpu::VertexFormat::Float32 => Some(1),
        wgpu::VertexFormat::Float32x2 => Some(2),
        wgpu::VertexFormat::Float32x3 => Some(3),
        wgpu::VertexFormat::Float32x4 => Some(4),
        _ => None,
    }
}
