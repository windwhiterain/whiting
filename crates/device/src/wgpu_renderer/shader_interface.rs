//! Checking that a shader reads its resources and its vertices where the pass put them.
//!
//! # Why this check exists
//!
//! A pass binds its resources at numbers it chooses: the uniform is zero, and the pass's
//! binding points follow it in name order, each taking a number whether the pass binds it or
//! renders into it. A vertex buffer's attributes are numbered the same
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
//!
//! # Outputs, which is the write side of the same problem
//!
//! Everything above is a shader **reading** what a pass built. A fragment output is the other
//! direction, and it was the one place with no check at all: a pass renders into N colour
//! targets and a shader fills them by `@location(i)`, and a declaration in `outputs` says which
//! image is at which point. Nothing joined the three, so a document could declare an image at a
//! location its shader does not write and get back a target holding whatever the clear left in
//! it — an image of exactly the shape that was asked for, saying nothing whatever.
//!
//! That is the write-side twin of the bug the exposure field found on the read side, where a
//! scene named a uniform field its shader did not declare and the upload dropped it silently.
//! [`check_outputs`] is the same parse, the same comparison and the same reporting for the
//! other end of the pass, and like [`check_resources`] it is a free function over a module, so
//! a machine with no gpu can hold it honest.
//!
//! # A declaration is the abstract layer, and the image is wider than it
//!
//! The first version of this compared a declared type with a written one for **equality**, and
//! equality is the wrong question. A scene says how many numbers its value is, and a document
//! that had to say how a texture lays them out would be carrying a rule from the layer
//! underneath into the layer above — which is the mistake in the opposite direction, and the same
//! one the closed set of channels made by inferring a format from what a channel *was*.
//!
//! The version after that compared **widths** and asked the wrong object the question. A `vec3<f32>`
//! declaration resolves to an image four components wide, so the fragment output that fills it
//! is four components wide too: there is no "declare three, write three" situation, because it
//! does not exist in the hardware. Measuring the write against the *declared* count therefore let
//! a document through that could not be drawn — `vec3` declared, `vec3` written, the check green
//! and the pipeline rejected — which is the one outcome this check exists to prevent.
//!
//! So the comparison is against the **width of the image**, and the two questions are asked
//! separately because they answer different ones. *Can the fragment output fill the image?* is
//! asked against the width the declaration resolves to, and the answer has to be exact: an image
//! is filled one whole texel at a time. *How many numbers does the value have?* is asked against
//! the declared count, and everything from that count onward is padding — written, present in
//! the file, and meaning nothing. A `vec3<f32>` normal is therefore declared with three numbers,
//! drawn as four components, and its fourth channel is padding; a shader that wrote three would
//! leave a number in every texel that the scene never wrote, and that is refused.
//!
//! Both halves are stated in the function's invariants, and neither is stated in terms of
//! anything a scene author has to know about how a target is written.
//!
//! # A `location` is checked against the attachment order it resolves against
//!
//! A declaration names both a point and a location, and for a long time nothing stopped those
//! two from drifting apart: two attachments of the same type, with their locations swapped,
//! passed every check and rendered one image's numbers under the other's name. The check needs
//! the mapping to be *real* before it can be worth anything, and here it is — WGSL numbers
//! `@location(i)` against a pass's colour attachments in order, `raster.rs` builds one state per
//! attachment in that same order, and the order is the document's own `targets` list read as
//! written. So a point's index in that list is the index its declaration must claim, and the
//! check is that the two agree.

use graphics::{Binder, Output};
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

    /// Checks that every output declared at a point this pass renders into is written by the
    /// pass's fragment shader, at the location the declaration names and as the type it
    /// declares.
    ///
    /// This is the half that needs a renderer: it finds the shader and parses it, against the
    /// parse this renderer already caches for the other two checks. The comparison itself is
    /// [`check_outputs`], which needs nothing but a module, a list of declarations and a list
    /// of points — which is what makes it checkable on a machine with no adapter, where a
    /// scene's fragment shader can be broken three ways and each way named.
    pub(super) fn check_outputs(
        &self,
        binder: &Binder<'_>,
        declared: &[Output<'_>],
        points: &[&str],
        label: &str,
    ) -> Result<(), RenderError> {
        if declared.is_empty() {
            return Ok(());
        }
        let source = self.shader_source(binder)?;
        let module = self.parsed_module(source, label)?;
        check_outputs(&module, declared, points, label)
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
                 pass binds nothing there. The uniform is binding zero and the pass's \
                 binding points follow it in name order, so the numbers a shader uses are \
                 the positions the document gave its binders — a point the pass renders \
                 into takes a position as well and has no entry to read."
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

/// One location the writing shader's fragment entry point fills, as it declares it.
struct FragmentOutput {
    /// The name the shader gives the value, which is what a reader opens the shader to find.
    name: String,
    /// The `@location` it is written at.
    location: u32,
    /// How many `f32` components it writes, or nothing for a type that is not a float scalar or
    /// vector at all — which is a disagreement to report rather than an absence to skip.
    ///
    /// It is a count rather than a [`graphics::TexelType`] because **the image's vocabulary is
    /// not the shader's limit and the declaration's is neither**: the fragment output has to be
    /// exactly as wide as the image the declaration resolves to, which is a property of the row
    /// beneath the declaration rather than of the declaration. A lookup keyed on the declaration
    /// would make a three component write answerable — and a three component write is exactly
    /// what cannot fill a four wide image.
    floats: Option<u8>,
    /// The type as WGSL spells it, whatever the workspace makes of it. Both sides of every
    /// disagreement are reported in both spellings, because a reader holding the document and a
    /// reader holding the shader are holding different vocabularies.
    spelled: String,
}

/// Returns every output a shader's fragment entry point writes, as its name, its location and
/// its type.
///
/// A fragment output is the entry point's **return value** rather than one of its arguments,
/// and it comes in two shapes: a single value carrying `@location(n)` directly, or a struct
/// whose members each carry one. Both are read here, because a G-buffer is written the second
/// way and a single-target pass the first, and a check that only understood one of them would
/// pass on half the documents it exists for.
///
/// A module with no fragment entry point writes no output, which is a depth-only pass rather
/// than a disagreement. A struct member with no `@location` is a value the shader keeps to
/// itself and is left out for the reason a vertex argument without one is: it is not an output.
fn declared_outputs(module: &naga::Module) -> Vec<FragmentOutput> {
    let Some(entry) = module
        .entry_points
        .iter()
        .find(|entry| entry.stage == naga::ShaderStage::Fragment)
    else {
        return Vec::new();
    };
    // The entry point carries its own function, so there is no handle to resolve and nothing
    // that can fail to resolve.
    let Some(result) = entry.function.result.as_ref() else {
        return Vec::new();
    };
    match &module.types[result.ty].inner {
        naga::TypeInner::Struct { members, .. } => members
            .iter()
            .filter_map(|member| {
                let Some(naga::Binding::Location { location, .. }) = member.binding else {
                    return None;
                };
                Some(describe_output(
                    module,
                    member.name.as_deref(),
                    location,
                    &module.types[member.ty].inner,
                ))
            })
            .collect(),
        // A fragment stage with no colour target writes nothing, so a return type that is not
        // a struct is a single value at the location the entry point's own result carries.
        inner => match result.binding {
            Some(naga::Binding::Location { location, .. }) => {
                // A single value carries no name of its own — it is a `return`, not a member —
                // so the entry point is what a reader is looking at, and it is named rather
                // than reported as `<unnamed>`.
                vec![describe_output(module, Some(&entry.name), location, inner)]
            }
            _ => Vec::new(),
        },
    }
}

/// Reads one fragment output out of a module, in both vocabularies.
fn describe_output(
    module: &naga::Module,
    name: Option<&str>,
    location: u32,
    inner: &naga::TypeInner,
) -> FragmentOutput {
    FragmentOutput {
        name: name.unwrap_or("<unnamed>").to_owned(),
        location,
        floats: float_components(inner),
        spelled: spelling(module, inner),
    }
}

/// Returns how many `f32` components a WGSL type writes, or nothing for a type that is neither
/// a `f32` scalar nor a vector of them.
///
/// **A half float is not a `f32` and is counted as nothing**, because the surplus components rule
/// is about numbers a reader can read back rather than about widths: a `vec3<f16>` beside a
/// `vec3<f32>` declaration agrees on how many components there are and disagrees about what one
/// of them is, and only the second of those is a disagreement worth a message. A whole number
/// and a matrix are left alone for the same reason — there is no width to compare, so the type
/// disagreement that follows names both spellings instead.
fn float_components(inner: &naga::TypeInner) -> Option<u8> {
    match inner {
        naga::TypeInner::Scalar(scalar) => {
            (scalar.kind == naga::ScalarKind::Float && scalar.width == 4).then_some(1)
        }
        naga::TypeInner::Vector { size, scalar } => {
            (scalar.kind == naga::ScalarKind::Float && scalar.width == 4).then_some(*size as u8)
        }
        _ => None,
    }
}

/// Returns how a WGSL type spells itself, for the side of a disagreement that is the shader's.
///
/// A type the workspace has no row for is still spelled rather than described, because
/// "the shader writes `vec2<f32>`" and "the shader writes something else" are not the same
/// report and only the first one is actionable. An array is the one type that recurses, so the
/// module travels with it; every other type is spelled from itself.
fn spelling(module: &naga::Module, inner: &naga::TypeInner) -> String {
    match inner {
        naga::TypeInner::Scalar(scalar) => scalar_name(scalar),
        naga::TypeInner::Vector { size, scalar } => {
            format!("vec{}<{}>", *size as u8, scalar_name(scalar))
        }
        naga::TypeInner::Matrix {
            columns,
            rows,
            scalar,
        } => format!(
            "mat{}x{}<{}>",
            *columns as u8,
            *rows as u8,
            scalar_name(scalar)
        ),
        naga::TypeInner::Array { base, size, .. } => format!(
            "array<{}, {size:?}>",
            spelling(module, &module.types[*base].inner)
        ),
        naga::TypeInner::Struct { members, .. } => format!("a struct of {} fields", members.len()),
        naga::TypeInner::Image { .. } => "a texture".to_owned(),
        naga::TypeInner::Sampler { .. } => "a sampler".to_owned(),
        naga::TypeInner::Atomic(scalar) => format!("atomic<{}>", scalar_name(scalar)),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// Returns how one scalar spells itself in WGSL.
fn scalar_name(scalar: &naga::Scalar) -> String {
    let prefix = match scalar.kind {
        naga::ScalarKind::Float => "f",
        naga::ScalarKind::Sint => "i",
        naga::ScalarKind::Uint => "u",
        naga::ScalarKind::Bool => "bool",
        naga::ScalarKind::AbstractInt => return "an abstract integer".to_owned(),
        naga::ScalarKind::AbstractFloat => return "an abstract float".to_owned(),
    };
    format!("{prefix}{}", scalar.width * 8)
}

/// Checks that every output a document declares **at a point this pass renders into** is
/// written by the pass's fragment shader, at the location the declaration names and at the width
/// of the image the declaration resolves to.
///
/// # Invariants
///
/// - **Only the outputs this pass writes are compared, and each is compared by the pass that
///   writes it.** A point no pass renders into is a different failure with a different owner —
///   it is [`crate::RenderError::UnwrittenOutput`], and it is raised by the renderer that ran
///   every pass rather than by any one of them. This function is the *writing* pass's half of
///   the agreement and has nothing to say about the others.
/// - **Two questions about a declaration and its image, and they are asked against two different
///   numbers.** *Can the shader's fragment output fill the image?* is asked against the width the
///   declaration resolves to ([`graphics::TexelType::image_width`]) and the answer must be exact,
///   because an image is filled one whole texel at a time. *How many numbers does the value
///   have?* is asked against the declared count ([`graphics::TexelType::components`]), and it is
///   never more than the image holds; everything from that count onward is padding. Asking the
///   first question against the second number is the defect this rule was corrected for: a
///   `vec3<f32>` declaration filled by a `vec3<f32>` fragment output passes a check against the
///   declared count and cannot be drawn, because the image it resolves to is four wide.
/// - **The order of the failures is the order of what each one makes moot.** Where the point
///   sits in this pass's attachment order, and whether a format can hold the declared type, and
///   whether the declared count fits the image it resolves to, are all facts about the
///   declaration alone, so they are asked first and nothing downstream can rescue them. Whether
///   the shader writes that location is asked next, because a width cannot be compared against a
///   value that is not there. The width itself is last.
/// - **A location means this pass's attachment order, and the check says so rather than
///   assuming it.** `points` arrives in colour attachment order, which is the order the
///   document's own `targets` list is written in, and that is the numbering a `@location(i)`
///   resolves against. The check therefore compares a declaration's point against the index its
///   location claims, which is what stops the two from drifting apart; it is not a second
///   opinion about what a location means, and it would be a different check if that list were
///   ever sorted or deduplicated on the way in.
/// - **No message names a rule of the layer underneath.** A document states what a value *is*,
///   and every refusal here is phrased as a fact about the image the two documents disagree
///   about: how wide it is drawn, and how many components the shader supplies. The single place
///   a format appears is the refusal for a type this device cannot read back, and that one is
///   already phrased about the device.
/// - **Every message names the shader's own variable and the location.** A bound shader entry
///   has no name and its source is a wall of text; the variable and the number are the two
///   things a reader can go and look at.
/// - **Nothing here needs a device, a queue or an adapter.** It is a module, a list of
///   declarations and a list of points, and that is what lets a scene's fragment shader be
///   broken four ways on a laptop with no gpu in it.
pub fn check_outputs(
    module: &naga::Module,
    declared: &[Output<'_>],
    points: &[&str],
    label: &str,
) -> Result<(), RenderError> {
    if declared.is_empty() {
        return Ok(());
    }
    let written = declared_outputs(module);
    for output in declared
        .iter()
        .filter(|output| points.contains(&output.point))
    {
        // The two facts about the declaration itself, before a word of the shader is consulted.
        //
        // **A location is this pass's colour attachment order, index to index.** The list handed
        // in is that order: the pass's own `targets`, in the order the document wrote them, with
        // the depth attachment out of it — which is the same numbering a shader's `@location(i)`
        // resolves against, and is why it is the one list a positional location can be checked
        // against at all. Nothing sorts that list, deduplicates it, or filters it beyond dropping
        // the depth attachment, so the index a point holds in it is the index its fragment output
        // has to be written at.
        let written_at = points
            .iter()
            .position(|point| *point == output.point)
            .map(|index| index as u32)
            .expect("the filter above kept only the points this pass renders into");
        if written_at != output.location {
            return Err(RenderError::Backend(format!(
                "the output `{name}` is declared at location {location} and names the point \
                 `{point}`, which the pass `{label}` renders into at location {written_at}. A \
                 pass's colour outputs are numbered from zero in the order of its `targets`, so \
                 these are two names for one fact and they have to be the same one: an image read \
                 back from `{point}` would be filled from whatever else this pass writes at \
                 location {location}, and both documents would be valid. A point more than one \
                 pass writes has to sit at the same location in each of them, because a single \
                 declaration has to name all of them.",
                name = output.name,
                point = output.point,
                location = output.location,
                written_at = written_at,
                label = label,
            )));
        }

        let at = written
            .iter()
            .find(|found| found.location == output.location);

        // The type next, because a type no format can hold is refused whether or not a shader
        // agrees to write it. Checking it later would report a width or a location disagreement
        // inside a declaration that is already unrenderable.
        let declared_type = match graphics::TexelType::parse(output.ty) {
            Ok(ty) => ty,
            Err(error) => {
                return Err(RenderError::Backend(format!(
                    "the output `{name}` is declared at location {location} as `{spelled}`, and \
                     no format this device can create and read back can hold it: {reason} The \
                     shader for `{label}` {shader_says}",
                    name = output.name,
                    location = output.location,
                    spelled = output.ty,
                    reason = error.reason(),
                    shader_says = sentence_about(written.as_slice(), at, "writes"),
                )));
            }
        };

        // **How many numbers does this declaration claim, against how many the image it resolves
        // to holds.** The first of the two questions to be asked, because it is a fact about the
        // declaration alone and nothing downstream can rescue it. An image has no component for a
        // number beyond its own width, so a declaration claiming more can never be filled — and it
        // is a different error from a width disagreement because the edit is in the type rather
        // than in the WGSL, which is why it is not folded into the width check below.
        let width = declared_type.image_width();
        if declared_type.components() > width {
            return Err(RenderError::Backend(format!(
                "the output `{name}` is declared at location {location} as `{declared}`, \
                 {claim}, and a declaration of that type is drawn in an image {width} wide. An \
                 image has no component for a number beyond its own width, so no fragment output \
                 can fill this one and the shader is not what is wrong: {written} The declared \
                 type is what decides how wide the image is, so it is the declared type that has \
                 to change.",
                name = output.name,
                location = output.location,
                declared = declared_type.name(),
                claim = components(declared_type.components()),
                width = components(width),
                written = sentence_about(written.as_slice(), at, "writes"),
            )));
        }

        let Some(found) = at else {
            return Err(RenderError::Backend(format!(
                "the output `{name}` is declared at location {location}, and the shader for \
                 `{label}` writes nothing there: {written_at} A fragment output is a \
                 `@location(n)` on the value the entry point returns, and the image this \
                 declaration names would hold whatever the image was cleared to and nothing \
                 else: it would be the right shape and say nothing at all.",
                name = output.name,
                location = output.location,
                label = label,
                written_at = sentence_about(written.as_slice(), None, "writes"),
            )));
        };

        // **Can this fragment output fill the image the declaration resolves to?** The second of
        // the two questions, and the one a reader cannot see. The image is filled one whole texel
        // at a time, so a fragment output has to be **exactly** as wide as that image. Not as wide
        // as the declaration — that says how many numbers the value *is*, which is the question
        // above — and not "at least" as wide either, because nothing in this table can be filled
        // by a narrower write and nothing by a wider one. A `vec3<f32>` declaration resolves to a
        // four wide image and the shader writes four, of which the fourth is padding the
        // declaration never asked for; the same declaration with a three component write satisfies
        // a check against the declared count and then cannot be drawn at all, which is the one
        // outcome this check exists to prevent.
        let Some(found_width) = found.floats else {
            return Err(RenderError::Backend(format!(
                "the output `{name}` is declared at location {location} as `{declared}`, and the \
                 shader for `{label}` writes `{variable}` there as `{found}`. This image is drawn \
                 {width} wide and is filled {width} at a time, and a `{found}` is not a number of \
                 components at all: there is no width here to compare and no surplus to call \
                 padding, and a reader of this image would find {width} in it, none of which is \
                 the value the shader wrote.",
                name = output.name,
                location = output.location,
                declared = declared_type.name(),
                label = label,
                variable = found.name,
                found = found.spelled,
                width = components(width),
            )));
        };
        if found_width != width {
            return Err(RenderError::Backend(format!(
                "the output `{name}` is declared at location {location} as `{declared}`, \
                 {claim}, and the shader for `{label}` writes `{variable}` there as `{found}`. \
                 {sentence}",
                name = output.name,
                location = output.location,
                declared = declared_type.name(),
                claim = components(declared_type.components()),
                label = label,
                variable = found.name,
                found = found.spelled,
                sentence = width_sentence(found_width, width, declared_type.name()),
            )));
        }
    }
    Ok(())
}

/// Returns a count of components as it is written in a sentence, which is not the same string for
/// one of them as for two.
fn components(count: u8) -> String {
    if count == 1 {
        "1 component".to_owned()
    } else {
        format!("{count} components")
    }
}

/// Returns the sentence that says why a fragment output cannot fill the image a declaration
/// resolves to, and what to write instead.
///
/// # Invariants
///
/// **It is a fact about the image and not about the declaration.** The image is drawn a whole
/// number of components wide and it is filled a whole number of components at a time, so the two
/// numbers a reader needs — how wide the image is and how many the shader supplies — are the
/// whole of the disagreement. The declared count does not appear in either half, because it is
/// the other question: it says how many numbers the value is, and everything from that count
/// onward is padding.
///
/// **The two directions are two sentences, because the two edits are two edits.** A shortfall
/// leaves numbers in an image that nothing supplied, and a surplus has nowhere to go and is
/// dropped rather than refused. Neither is reachable from the other, and a message that said
/// only "the widths disagree" would leave the reader to work out which of the two they had.
fn width_sentence(supplied: u8, width: u8, declared: &str) -> String {
    let surplus = supplied.saturating_sub(width);
    if surplus == 0 {
        let missing = width - supplied;
        return format!(
            "This image is drawn {drawn_wide} wide, because that is the width a `{declared}` \
             declaration resolves to, and an image is filled one whole texel at a time, so the \
             shader supplies {supplied} of the {width}. A reader would find {missing} \
             number{plural} the scene never wrote, and a number nobody wrote reads as though it \
             had been: a worse failure than an error, because it is a plausible picture. The \
             remedy is the surplus — write {width} components at this location, and everything \
             from the declared count onward is padding, which is written and means nothing. A \
             declaration naming fewer numbers is drawn in a narrower image and is filled with a \
             narrower write.",
            drawn_wide = components(width),
            plural = if missing == 1 { "" } else { "s" },
        );
    }
    format!(
        "This image is drawn {drawn_wide} wide, because that is the width a `{declared}` \
         declaration resolves to, and an image is filled one whole texel at a time, so {surplus} \
         of the {supplied} components the shader writes have nowhere to go: the image would hold \
         the first {width} of them and drop the rest, which is a wrong picture rather than an \
         error. The declared type is what decides how wide this image is, so a declaration of \
         fewer numbers is the other edit — and one number is drawn as a scalar, while three and \
         four are both drawn four wide.",
        drawn_wide = components(width),
    )
}

/// Returns the sentence naming what a shader writes, for a message about a location it does.
///
/// The one case it has to get right is the **agreement**: a declaration whose type is refused
/// and a shader that writes exactly that type is the case where a reader's first suspicion is
/// the shader, and the sentence has to say plainly that the two agree and the type is still
/// refused — otherwise the message reads as a mistake in the WGSL and sends the reader to edit
/// the one file that is correct.
///
/// Every branch is a whole sentence with its own full stop, because each is dropped into the
/// middle of a longer message and a clause that runs straight into the next one is a message
/// nobody reads to the end.
fn sentence_about(written: &[FragmentOutput], at: Option<&FragmentOutput>, verb: &str) -> String {
    match at {
        Some(found) => format!(
            "{verb} `{variable}` at location {location}, as `{spelled}`.",
            variable = found.name,
            location = found.location,
            spelled = found.spelled,
        ),
        None if written.is_empty() => {
            "has no fragment output at all, so the pass fills nothing.".to_owned()
        }
        None => {
            let list = written
                .iter()
                .map(|found| {
                    format!(
                        "`{variable}` at location {location}",
                        variable = found.name,
                        location = found.location
                    )
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!("{verb} {list}, and nothing at any other location.")
        }
    }
}
