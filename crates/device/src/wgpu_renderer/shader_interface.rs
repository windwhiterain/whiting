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
//! # Invariants
//!
//! - **The shader is the side that is checked against the pass, never the reverse.** A
//!   layout may declare an entry no shader reads; that is legal and common, and reporting
//!   it would make every shader that ignores a binding an error.
//! - **A resource kind is compared, not a resource's shape.** A shader that asks for a
//!   texture where the pass bound a sampler is a mistake whatever the two are sized like,
//!   and a buffer is a buffer whether it is a uniform or a storage buffer.
//! - **A shader is only judged on what it actually reads.** A module with no entry point,
//!   or a pass that binds nothing, is not a disagreement: there is nothing to disagree
//!   with.
//! - The parse is the one the renderer already caches against its other parses, so a frame
//!   that changes no shader pays for the comparison and not for the parse.

use graphics::Binder;
use wgpu::naga;

use super::WgpuRenderer;
use crate::render::RenderError;

/// What a resource is, at the coarsest grain a binding number can be compared at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    /// A buffer, uniform or storage: the uniform is one too, and is at binding zero.
    Buffer,
    /// An image, sampled or read.
    Texture,
    /// A sampler, which reads a texture and is a separate resource from it.
    Sampler,
}

impl ResourceKind {
    /// Returns how a kind is named in an error, so both sides of a disagreement are
    /// described in the same words.
    fn name(self) -> &'static str {
        match self {
            ResourceKind::Buffer => "a buffer",
            ResourceKind::Texture => "a texture",
            ResourceKind::Sampler => "a sampler",
        }
    }
}

impl WgpuRenderer {
    /// Checks that every resource a shader declares in the pass's group is bound, and
    /// bound as the kind the shader says it is.
    ///
    /// The pass is named in every error, because a bound shader entry has no name of its
    /// own and the pass is what a reader is looking at.
    pub(super) fn check_bindings(
        &self,
        binder: &Binder<'_>,
        entries: &[wgpu::BindGroupLayoutEntry],
        label: &str,
    ) -> Result<(), RenderError> {
        let source = self.shader_source(binder)?;
        let module = self.parsed_module(source, label)?;
        for (name, number, declared) in declared_resources(&module) {
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
                     this pass binds {} there.",
                    declared.name(),
                    bound.name()
                )));
            }
        }
        Ok(())
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
/// about; both are left out rather than guessed at.
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
                    naga::TypeInner::Image { .. } => ResourceKind::Texture,
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

/// Returns the kind a layout entry binds, or nothing for a kind this comparison has no
/// opinion about.
///
/// The standard pipeline binds buffers, textures and samplers, and nothing else. A layout
/// entry of some other kind is left alone rather than forced into one of the three, for
/// the same reason a shader resource is: the check judges what both sides are saying, and
/// a side saying something else entirely is not a disagreement.
fn kind_of(ty: &wgpu::BindingType) -> Option<ResourceKind> {
    match ty {
        wgpu::BindingType::Buffer { .. } => Some(ResourceKind::Buffer),
        wgpu::BindingType::Texture { .. } | wgpu::BindingType::StorageTexture { .. } => {
            Some(ResourceKind::Texture)
        }
        wgpu::BindingType::Sampler(_) => Some(ResourceKind::Sampler),
        _ => None,
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
