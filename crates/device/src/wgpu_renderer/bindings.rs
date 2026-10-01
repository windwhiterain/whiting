//! Turning a pass and an instance into the resources a pipeline needs.
//!
//! # What a binding point becomes
//!
//! A pass declares binding points; the instance holds the payloads. What the backend
//! builds from them is decided by the payload's own kind and usage, not by the name, so an
//! author does not repeat a convention in a document:
//!
//! - a buffer whose usage is `vertex` is a vertex buffer, at the slot of its binding
//!   point, with its layout derived from the schema it carries;
//! - a buffer whose usage is `instance` is the same, stepping once per instance;
//! - a buffer whose usage is `storage` is bound to the shader as a read only storage
//!   buffer, and one that also declares `storage_write` is bound so a shader may write it;
//! - every other buffer and every texture is bound to the shader, and its binding index
//!   is its position among the bound points, in name order.
//!
//! A usage word this module does not know is an error rather than a buffer of some
//! default kind, for the reason the rest of the module takes the same position: a word
//! that was misspelled and bound as a uniform produces a frame that is quietly wrong,
//! which is a worse answer than a frame that is refused.
//!
//! # Invariants
//!
//! - A pass is verified against the interface its own shader declares before anything here
//!   runs, so a pass that reached this module has already been checked against the data it
//!   names. What this module adds is the gpu side of that agreement: a vertex layout
//!   derived from the same schema the bytes were packed with.
//! - A texture is created once per instance entry and shared by every pass that binds it,
//!   which is what lets one pass write a shadow map and the next read it. A bound buffer
//!   is shared the same way, which is what lets a compute pass write one and a later pass
//!   read what it wrote rather than the bytes that were uploaded.

use std::collections::BTreeMap;

use definition::Value;
use graphics::binders::{BinderKind, DATA, FORMAT, HEIGHT, LAYERS, SCHEMA, SIZE_BYTES, USAGE, WIDTH};
use graphics::layout::{VertexLayout, vertex_layout};
use graphics::{
    Declaration, PassView, SamplerAddress, SamplerSettings, is_sampled, verify_pass_inputs,
};

use super::WgpuRenderer;
use crate::render::RenderError;

/// The usage that makes a buffer a vertex buffer.
pub const VERTEX: &str = "vertex";

/// The usage that makes a buffer step once per instance.
pub const INSTANCE: &str = "instance";

/// The usage that names a buffer as a uniform, which is also what a buffer with no usage is.
///
/// The word is redundant, since a payload that declares nothing is read as a uniform, and it is
/// here for the document that would rather say what a buffer is than leave it unstated. Without
/// it the vocabulary can name a vertex buffer, an instance buffer and two kinds of storage
/// buffer but cannot name the one a shader reads most often, and an error listing the words
/// would be asking a document to say nothing at all in order to be understood.
pub const UNIFORM: &str = "uniform";

/// The usage that makes a buffer a read only storage buffer.
///
/// A storage buffer is what a data set that does not fit in a uniform is bound as: the
/// device requests [`wgpu::Limits::downlevel_defaults`], which caps a uniform binding at
/// sixteen kibibytes, and there is no other way to put a large data set where a shader can
/// reach it.
pub const STORAGE: &str = "storage";

/// The usage that lets a shader write a storage buffer.
///
/// It is a second word rather than a third kind because it is a permission on a storage
/// buffer and not a kind of buffer: a read only storage buffer and a writable one are the
/// same buffer with a different answer to one question, which is whether a shader may
/// store into it.
pub const STORAGE_WRITE: &str = "storage_write";

/// How a shader reaches a bound buffer, which the layout has to declare exactly.
///
/// # Invariants
///
/// The three cases are the three ways a backend will let a buffer be bound, and the
/// declaration follows from the document rather than from the shader: a buffer says what
/// it is and a shader says what it reads, and the two are compared as buffer against
/// buffer, so the level a buffer is bound at is a fact about the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    /// A uniform buffer, which the backend caps at sixteen kibibytes here.
    Uniform,
    /// A storage buffer a shader may read but not write.
    ReadOnlyStorage,
    /// A storage buffer a shader may read and write.
    ReadWriteStorage,
}

impl BufferKind {
    /// Returns the binding type the layout declares for this kind of buffer.
    pub fn binding_type(self) -> wgpu::BufferBindingType {
        match self {
            Self::Uniform => wgpu::BufferBindingType::Uniform,
            Self::ReadOnlyStorage => wgpu::BufferBindingType::Storage { read_only: true },
            Self::ReadWriteStorage => wgpu::BufferBindingType::Storage { read_only: false },
        }
    }

    /// Returns the usage a buffer of this kind is created with.
    ///
    /// # Invariants
    ///
    /// A read only and a writable storage buffer are created with the same
    /// [`wgpu::BufferUsages::STORAGE`]: the backend distinguishes the two at the binding,
    /// not at creation, so a buffer that one pass writes and another reads is one buffer
    /// and not two.
    pub fn usages(self) -> wgpu::BufferUsages {
        match self {
            Self::Uniform => wgpu::BufferUsages::UNIFORM,
            Self::ReadOnlyStorage | Self::ReadWriteStorage => wgpu::BufferUsages::STORAGE,
        }
    }

    /// Returns how a kind is named in an error, so a reader is told which buffer it is.
    pub fn name(self) -> &'static str {
        match self {
            Self::Uniform => "a uniform",
            Self::ReadOnlyStorage => "a read only storage buffer",
            Self::ReadWriteStorage => "a writable storage buffer",
        }
    }
}

/// A buffer bound to the vertex stage.
pub struct VertexSlot {
    /// The binding point this buffer answers to.
    pub point: String,
    /// The slot the buffer is bound at, which is its order among the vertex buffers.
    pub slot: u32,
    /// The buffer itself.
    pub buffer: wgpu::Buffer,
    /// How the buffer steps, per vertex or per instance.
    pub step_mode: wgpu::VertexStepMode,
    /// The layout of one element, derived from the schema the buffer carries.
    pub layout: VertexLayout,
    /// How many elements the buffer holds.
    pub count: u64,
    /// The first attribute location this buffer's attributes answer to.
    pub location_base: u32,
    /// How the points are read, which a vertex buffer declares and an instance buffer
    /// does not use.
    pub topology: wgpu::PrimitiveTopology,
}

/// A resource bound to the shader.
pub enum Bound {
    /// A buffer, and the way the shader reaches it.
    ///
    /// The kind travels with the buffer rather than being read back off the layout,
    /// because the layout is derived from this and the two cannot be allowed to disagree
    /// about which of the three buffer bindings a buffer is.
    Buffer(wgpu::Buffer, BufferKind),
    /// A texture, bound as a sampled texture.
    Texture(TextureSlot),
}

/// A texture the instance holds, created once and shared by the passes that bind it.
#[derive(Clone)]
pub struct TextureSlot {
    /// The binding point this texture answers to.
    pub point: String,
    /// The instance entry the texture is in, which is what makes two textures the same
    /// texture however differently the passes that use them name it.
    pub source: String,
    /// The format the instance declared.
    pub format: wgpu::TextureFormat,
    /// The texture itself.
    pub texture: wgpu::Texture,
    /// A view of it, for binding and for attachment.
    pub view: wgpu::TextureView,
    /// The shape the instance declared, which is where the layer count lives.
    pub shape: TextureShape,
    /// Which layer of it this slot is: always the first for a bound texture, and the one
    /// the pass named for an attachment of an array.
    ///
    /// The field exists so that a pass which clears a target clears the page it drew into
    /// and not the whole texture, and so that a frame read back from an array comes from
    /// the layer that was written. See [`TextureSlot::clear_key`].
    pub layer: u32,
    /// The sampler that reads it, when the payload declares that it is sampled.
    ///
    /// The settings belong to the image rather than to the pass, so the sampler is built
    /// once with the texture and cloned out of the cache on every later hit: one instance
    /// entry yields one sampler however many passes bind it.
    pub sampler: Option<wgpu::Sampler>,
    /// Whether this slot is one of the pass's own render targets.
    ///
    /// An attachment is never sampled and is never filtered, so its sample type does not
    /// follow from the declarations a bound texture is judged by. The two are told apart
    /// here rather than at the layout, because only the binding knows which it built.
    pub attachment: bool,
}

impl TextureSlot {
    /// Returns the sample type a shader reads this texture as.
    ///
    /// # Invariants
    ///
    /// The type follows the texture's *role* rather than its format. A depth texture is
    /// read as depth whatever it is for, and a depth *array* is read as depth too: the
    /// shape changes the type a shader names and the arguments a `textureLoad` takes, not
    /// what a texel of it is. An attachment is filtered by nothing: a backend forbids
    /// filtering a texture a pass is writing. A bound colour texture is filterable exactly
    /// when the payload asked for a sampler, which is why the same `rgba8unorm` target
    /// pictures as a filterable image and as an unfilterable one depending on which it is.
    pub fn sample_type(&self) -> wgpu::TextureSampleType {
        if WgpuRenderer::is_depth_format(self.format) {
            wgpu::TextureSampleType::Depth
        } else {
            wgpu::TextureSampleType::Float {
                filterable: !self.attachment && self.sampler.is_some(),
            }
        }
    }

    /// Returns the view dimension a shader must declare to read this texture.
    ///
    /// # Invariants
    ///
    /// The shape is the payload's own and is not restated here, so the layout entry and the
    /// view a pass draws into are derived from one number and cannot disagree about it. A
    /// flat texture is `D2` and an array is `D2Array`, which is the difference the
    /// interface check compares the two sides on.
    pub fn view_dimension(&self) -> wgpu::TextureViewDimension {
        self.shape.view_dimension()
    }

    /// Returns the key that says whether this attachment's page has been written this
    /// render.
    ///
    /// # Invariants
    ///
    /// A pass that clears a target clears the page it draws into and no other, so the key
    /// is the entry **and** the layer. Keying on the entry alone would make the second
    /// pass into a pool *load* the first one's page instead of clearing its own, and two
    /// lights would cast the same shadow.
    pub fn clear_key(&self) -> String {
        format!("{}#{}", self.source, self.layer)
    }
}

/// The shape a texture payload declares: how wide, how tall, and how many layers.
///
/// # Invariants
///
/// - **The layer count comes from the variant, not from a field that might be absent.** A
///   `Texture` payload is one layer and has no count to read; a `TextureArray` payload has
///   one and is that many. Reading a count out of whichever variant declared it is what
///   keeps a document from being able to state a number that is then ignored, which is the
///   disagreement the two kinds exist to make impossible.
/// - **A count this device cannot build is refused by name, never clamped.** A clamped pool
///   is a document that asked for pages it does not have, and the light that lost its page
///   would be lit wrongly with nothing to show for it in the document that caused it.
/// - **The texture dimension is `D2` for every shape here.** An array of two dimensional
///   images *is* a two dimensional texture whose depth is its layer count, which is what
///   `depth_or_array_layers` is for. A volume would be `D3`, and no document in this
///   binding model declares one, so nothing here would know what view to create for it.
/// - **A flat texture produces exactly the descriptors it produced before any of this
///   existed.** A view of one layer is the default view descriptor and the layout dimension
///   is `D2`, so a document that says nothing is byte for byte the document it was.
/// - **The shape follows the variant and never the count.** An array of one layer is still an
///   array: it is a `D2Array` view and a `D2Array` layout entry, because the shader that reads
///   it declared `texture_depth_2d_array` and that is the only thing that can sample it.
///   Reading the shape off `layers == 1` made a pool of a single page — a scene with one
///   shadow casting light, which is a scene any of them may write — bind as a flat texture,
///   and the interface check reported that as the shader declaring a shape the pass did not
///   bind. The count says how deep the array is; the variant says what it *is*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureShape {
    /// The width in pixels that each layer is.
    pub width: u32,
    /// The height in pixels that each layer is.
    pub height: u32,
    /// How many layers the texture has, which is one for a flat texture.
    pub layers: u32,
    /// Whether the payload declared itself an **array**, whatever its layer count.
    ///
    /// # Invariants
    ///
    /// An array of one layer is an array. This is the flag the view and the layout entry read,
    /// and it comes from the payload's variant rather than from `layers`, because a count of
    /// one is what a flat texture *and* the shallowest pool both say.
    pub arrayed: bool,
}

impl TextureShape {
    /// Reads the shape a texture payload declares, refusing a layer count the device
    /// cannot build.
    ///
    /// `max_layers` is what this device allows, and it is passed in rather than read from a
    /// constant here so that the number in an error is the number the device was created
    /// with. See [`WgpuRenderer::max_texture_array_layers`].
    pub fn read(source: &str, payload: &Value, max_layers: u32) -> Result<Self, RenderError> {
        let fields = payload
            .variant_fields()
            .ok_or_else(|| RenderError::Backend(format!("`{source}` is not a texture payload")))?;
        let number = |name: &str| {
            fields
                .get(name)
                .and_then(Value::as_int)
                .ok_or_else(|| RenderError::Backend(format!("the texture `{source}` needs a `{name}`")))
        };
        // A size of zero is rounded up rather than refused, as it always has been: a
        // document asking for no pixels is asking for a very small image, and a backend
        // cannot be given a zero extent. A count of layers is a different question — see
        // below — and is refused rather than rounded.
        let width = number(WIDTH)?.max(1) as u32;
        let height = number(HEIGHT)?.max(1) as u32;
        let kind = BinderKind::from_tag(payload.variant_tag().unwrap_or_default());
        let layers = if kind == Some(BinderKind::TextureArray) {
            let declared = number(LAYERS)?;
            if declared < 1 {
                return Err(RenderError::Backend(format!(
                    "the texture array `{source}` declares `layers` of {declared}, and a \
                     texture has at least one layer. A count of zero names an image that \
                     holds nothing, which is not a smaller texture but no texture at all."
                )));
            }
            if declared > i64::from(max_layers) {
                return Err(RenderError::Backend(format!(
                    "the texture array `{source}` declares `layers` of {declared}, and this \
                     device allows {max_layers} at most, which is the \
                     `max_texture_array_layers` it was created with. A deeper pool is refused \
                     rather than clamped, because a clamped one is a document that asked for \
                     pages it does not have."
                )));
            }
            declared as u32
        } else {
            1
        };
        Ok(Self {
            width,
            height,
            arrayed: kind == Some(BinderKind::TextureArray),
            layers,
        })
    }

    /// Returns the descriptor this shape is created with.
    ///
    /// # Invariants
    ///
    /// Every texture in this binding model is created with the same usages, so the set
    /// lives with the descriptor rather than at each call site: a texture is written by a
    /// pass, read by a shader, and copied back for a frame, and a texture missing one of
    /// those would be a texture the model cannot express rather than a document's choice.
    pub fn descriptor<'a>(
        &self,
        label: &'a str,
        format: wgpu::TextureFormat,
        levels: u32,
    ) -> wgpu::TextureDescriptor<'a> {
        wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.layers,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }

    /// Returns the view a shader reads the whole texture through, every layer included.
    ///
    /// # Invariants
    ///
    /// One layer is the *default* descriptor for a **flat** texture, because the default is a
    /// `D2` view of a single layer and that is exactly the resource every document before a
    /// layer count existed described. An array is a `D2Array` view however deep it is, since
    /// a shader declares `texture_depth_2d_array` for a pool of one page as readily as for a
    /// pool of sixty four.
    pub fn view_descriptor(&self) -> wgpu::TextureViewDescriptor<'_> {
        if !self.arrayed {
            return wgpu::TextureViewDescriptor::default();
        }
        wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            base_array_layer: 0,
            array_layer_count: Some(self.layers),
            ..Default::default()
        }
    }

    /// Returns the view of one layer, which is what a pass draws into.
    ///
    /// # Invariants
    ///
    /// A layer is drawn into as a flat image however deep the array is, because a render
    /// pass attaches a two dimensional view and names no layer of its own: the layer is the
    /// view, not an argument to the draw. Naming it in the view rather than in the
    /// attachment is also what lets one cached texture serve every pass that writes a
    /// different page of it.
    pub fn layer_view_descriptor(&self, layer: u32) -> wgpu::TextureViewDescriptor<'_> {
        wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: layer,
            array_layer_count: Some(1),
            ..Default::default()
        }
    }

    /// Returns the view dimension a shader must declare to read this shape.
    pub fn view_dimension(&self) -> wgpu::TextureViewDimension {
        if self.arrayed {
            wgpu::TextureViewDimension::D2Array
        } else {
            wgpu::TextureViewDimension::D2
        }
    }

    /// Checks that a pass may draw into this layer, and returns the index it may use.
    ///
    /// # Invariants
    ///
    /// A layer at or beyond the count is refused rather than clamped, for the same reason
    /// the count itself is: a pass that drew into page four of a pool of four would write
    /// the first page again, and the shadow it thinks it gave a light would be another
    /// light's.
    pub fn check_layer(
        &self,
        source: &str,
        point: &str,
        layer: i64,
    ) -> Result<u32, RenderError> {
        if layer < 0 || layer >= i64::from(self.layers) {
            return Err(RenderError::Backend(format!(
                "the pass renders into `{point}`, which names layer {layer} of the texture \
                 array `{source}`, and that array has {} layers counted from zero.",
                self.layers
            )));
        }
        Ok(layer as u32)
    }
}

/// Returns which layer of a texture a declaration draws into, defaulting to the first.
///
/// # Invariants
///
/// The default is applied here and not in the schema, because the document is not what
/// decides: a flat texture has no layer field and an array's declaration that names none is
/// drawing into page zero, and both of those are the same answer for one reason. A pass
/// that names a layer the pool does not have is refused by [`TextureShape::check_layer`]
/// rather than folded into the first page, which is the case that would be a wrong shadow.
fn layer_of(declaration: &Declaration<'_>) -> i64 {
    declaration.layer().unwrap_or_default()
}

/// Everything one pass needs from the instance.
pub struct Bindings {
    /// The uniform data, as the bytes a uniform buffer carries.
    pub uniform: wgpu::Buffer,
    /// The buffers bound to the vertex stage, in slot order.
    pub vertices: Vec<VertexSlot>,
    /// The resources bound to the shader, in binding index order.
    pub bound: Vec<Bound>,
    /// The textures the pass renders into, which a backend forbids it from binding.
    pub attachments: Vec<TextureSlot>,
    /// The binding points the pass renders into, in attachment order.
    pub targets: Vec<String>,
    /// How many instances to draw, which is the instance buffer's element count.
    pub instances: u64,
}

impl Bindings {
    /// Returns the texture a target binding point names.
    pub fn target(&self, point: &str) -> Option<&TextureSlot> {
        self.attachments
            .iter()
            .find(|texture| texture.point == point)
    }

    /// Returns the number of vertices to draw, which is the vertex buffer's element
    /// count, or none when the pass has no vertex buffer.
    pub fn vertex_count(&self) -> Option<u64> {
        self.vertices
            .iter()
            .find(|slot| slot.step_mode == wgpu::VertexStepMode::Vertex)
            .map(|slot| slot.count)
    }

    /// Returns how the points a pass draws are read as primitives.
    pub fn topology(&self) -> wgpu::PrimitiveTopology {
        self.vertices
            .iter()
            .find(|slot| slot.step_mode == wgpu::VertexStepMode::Vertex)
            .map_or(wgpu::PrimitiveTopology::TriangleList, |slot| slot.topology)
    }

    /// Returns the binding number of every bound resource, and of its sampler when it has
    /// one, in the order the resources are bound.
    ///
    /// # Invariants
    ///
    /// The uniform is binding zero and each bound resource takes the next number, so a
    /// shader counts to a resource without knowing which pass declared it. A texture whose
    /// payload declares `sampled` then takes a second number, for the sampler that reads it,
    /// at the position immediately after its own. A texture that declares nothing takes one
    /// number, which is what keeps every existing document's numbering unchanged.
    ///
    /// The two arrays are parallel to `self.bound`. This is the only place the arithmetic is
    /// written: the layout and the bind group are both derived from it, so they cannot
    /// disagree about a number. A rule expressed twice is a rule that can be applied once.
    fn binding_numbers(&self) -> (Vec<u32>, Vec<Option<u32>>) {
        let mut numbers = Vec::with_capacity(self.bound.len());
        let mut samplers = Vec::with_capacity(self.bound.len());
        let mut next = 1;
        for resource in self.bound.iter() {
            numbers.push(next);
            next += 1;
            let sampler = match resource {
                Bound::Texture(texture) if texture.sampler.is_some() => {
                    let number = next;
                    next += 1;
                    Some(number)
                }
                _ => None,
            };
            samplers.push(sampler);
        }
        (numbers, samplers)
    }

    /// Describes the bindings to a pipeline.
    ///
    /// The numbering is [`Bindings::binding_numbers`], and this is the layout half of it.
    pub fn layout_entries(
        &self,
        visibility: wgpu::ShaderStages,
    ) -> Vec<wgpu::BindGroupLayoutEntry> {
        let uniform = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let mut entries = vec![uniform];
        let (numbers, samplers) = self.binding_numbers();
        for (index, resource) in self.bound.iter().enumerate() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: numbers[index],
                visibility,
                ty: match resource {
                    Bound::Buffer(_, kind) => wgpu::BindingType::Buffer {
                        ty: kind.binding_type(),
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    Bound::Texture(texture) => wgpu::BindingType::Texture {
                        // A depth texture is sampled as depth, not as float: a shader
                        // that declares `texture_depth_2d` is bound to one, and a
                        // float sample type would not match it. An array is bound as
                        // depth too; the shape is the view dimension below.
                        sample_type: texture.sample_type(),
                        // The shape is the payload's own, so a flat texture is declared
                        // flat and an array is declared as the array a shader reads with
                        // `texture_depth_2d_array` and a `textureLoad` that takes a
                        // layer.
                        view_dimension: texture.view_dimension(),
                        multisampled: false,
                    },
                },
                count: None,
            });
            if let Some(binding) = samplers[index] {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                });
            }
        }
        entries
    }

    /// Fills a bind group with the resources [`Bindings::layout_entries`] declared.
    pub fn group_entries(&self) -> Vec<wgpu::BindGroupEntry<'_>> {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(self.uniform.as_entire_buffer_binding()),
        }];
        let (numbers, samplers) = self.binding_numbers();
        for (index, resource) in self.bound.iter().enumerate() {
            let sampler = match resource {
                Bound::Texture(texture) => texture.sampler.as_ref(),
                Bound::Buffer(_, _) => None,
            };
            entries.push(wgpu::BindGroupEntry {
                binding: numbers[index],
                resource: match resource {
                    Bound::Buffer(buffer, _) => {
                        wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding())
                    }
                    Bound::Texture(texture) => wgpu::BindingResource::TextureView(&texture.view),
                },
            });
            if let (Some(binding), Some(sampler)) = (samplers[index], sampler) {
                entries.push(wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::Sampler(sampler),
                });
            }
        }
        entries
    }

    /// Describes the vertex buffers to a pipeline, deriving each layout from the schema
    /// its bytes were packed with.
    ///
    /// # Invariants
    ///
    /// A vertex buffer's slot is its order among the vertex buffers, and its attributes
    /// continue from where the previous buffer's ended, so two buffers never claim one
    /// location. Per vertex buffers come first and per instance buffers follow, so a
    /// shader reads its own point at the low locations and the instance data above them.
    ///
    /// The attribute lists are written into `attributes` rather than returned alongside the
    /// layouts, because a layout borrows the attributes it points at and the two have to
    /// outlive the pipeline together.
    pub fn vertex_layouts<'attributes>(
        &self,
        attributes: &'attributes mut Vec<Vec<wgpu::VertexAttribute>>,
    ) -> Vec<wgpu::VertexBufferLayout<'attributes>> {
        attributes.clear();
        attributes.extend(self.vertices.iter().map(|vertex| {
            vertex
                .layout
                .attributes
                .iter()
                .map(|attribute| wgpu::VertexAttribute {
                    format: vertex_format(attribute.components),
                    offset: attribute.offset_bytes,
                    shader_location: vertex.location_base + attribute.shader_location,
                })
                .collect()
        }));
        self.vertices
            .iter()
            .zip(attributes.iter())
            .map(|(vertex, attributes)| wgpu::VertexBufferLayout {
                array_stride: vertex.layout.stride_bytes,
                step_mode: vertex.step_mode,
                attributes,
            })
            .collect()
    }

    /// Returns every vertex location this pass binds, as the location and the format it is
    /// read at.
    ///
    /// This is the same numbering [`Bindings::vertex_layouts`] lays out, reported as owned
    /// pairs so it can be compared against a shader's own declarations before the layouts
    /// borrow the attribute lists they point at.
    pub fn vertex_locations(&self) -> Vec<(u32, wgpu::VertexFormat)> {
        self.vertices
            .iter()
            .flat_map(|vertex| {
                vertex.layout.attributes.iter().map(|attribute| {
                    (
                        vertex.location_base + attribute.shader_location,
                        vertex_format(attribute.components),
                    )
                })
            })
            .collect()
    }

    /// Puts the vertex buffers in the order a pipeline reads them, and numbers them.
    ///
    /// A vertex buffer and an instance buffer both start their attributes at zero in the
    /// schema they were packed with, so the numbering has to be shared out across them.
    pub fn order_vertices(&mut self) {
        self.vertices.sort_by(|left, right| {
            (
                left.step_mode == wgpu::VertexStepMode::Instance,
                &left.point,
            )
                .cmp(&(
                    right.step_mode == wgpu::VertexStepMode::Instance,
                    &right.point,
                ))
        });
        let mut location = 0;
        for (slot, vertex) in self.vertices.iter_mut().enumerate() {
            vertex.slot = slot as u32;
            vertex.location_base = location;
            location += vertex.layout.attributes.len() as u32;
        }
    }
}

/// Reads a buffer payload's usage words as the kind of buffer it is.
///
/// # Invariants
///
/// A buffer with no words at all is a uniform, which is what every document that predates
/// the `storage` word means and what a payload that declares no usage has always been
/// read as. A vertex or instance buffer is a uniform here too, because the caller has
/// already taken it off this path by the time the words reach the level that matters.
///
/// Every other word must be known. A word that is not is reported rather than taken for
/// a uniform, for the reason the rest of the module takes the same position: a misspelled
/// `storage` would otherwise bind a uniform, and a shader reading a storage buffer that
/// was handed a uniform gets a frame that is quietly wrong. `storage_write` without
/// `storage` is refused for the same reason, since a buffer nothing declared to be a
/// storage buffer cannot be one a shader writes.
///
/// A buffer is one kind, so naming two is a document that contradicts itself and is reported
/// as one. `uniform` beside `storage` is that contradiction: the first says the shader reads
/// a sixteen kibibyte block and the second says it reads a buffer the device sized for a data
/// set, and picking either silently would bind a buffer that is not the one either word names.
fn buffer_kind(usage: &[&str], source: &str) -> Result<BufferKind, RenderError> {
    let storage = usage.contains(&STORAGE);
    let writable = usage.contains(&STORAGE_WRITE);
    let uniform = usage.contains(&UNIFORM);
    if writable && !storage {
        return Err(RenderError::Backend(format!(
            "the buffer `{source}` declares `{STORAGE_WRITE}` without `{STORAGE}`. A buffer a \
             shader writes is a storage buffer, and the two words are what say so. Binding it \
             as a uniform would look right and read the wrong bytes."
        )));
    }
    if uniform && storage {
        return Err(RenderError::Backend(format!(
            "the buffer `{source}` declares both `{UNIFORM}` and `{STORAGE}`. A buffer is bound \
             as one kind or the other: a uniform is capped at sixteen kibibytes and a storage \
             buffer is not, so the two words describe two different buffers under one name."
        )));
    }
    let mut unknown = usage
        .iter()
        .copied()
        .filter(|word| !matches!(*word, VERTEX | INSTANCE | UNIFORM | STORAGE | STORAGE_WRITE));
    let Some(word) = unknown.next() else {
        return Ok(if writable {
            BufferKind::ReadWriteStorage
        } else if storage {
            BufferKind::ReadOnlyStorage
        } else {
            BufferKind::Uniform
        });
    };
    Err(RenderError::Backend(format!(
        "the buffer `{source}` declares the usage `{word}`, which is not a usage a buffer can \
         be bound with. A buffer's usage is `vertex`, `instance`, `{UNIFORM}`, `storage` or \
         `storage_write`, and a buffer that declares none of them is bound as a uniform. \
         Reading an unrecognised word as a uniform would give a frame that is quietly wrong, \
         which is a worse answer than this error."
    )))
}

/// Maps a component count onto the vertex format a backend reads it as.
fn vertex_format(components: usize) -> wgpu::VertexFormat {
    match components {
        2 => wgpu::VertexFormat::Float32x2,
        3 => wgpu::VertexFormat::Float32x3,
        4 => wgpu::VertexFormat::Float32x4,
        _ => wgpu::VertexFormat::Float32,
    }
}

/// Textures created during one render, keyed by the instance entry they hold.
#[derive(Default)]
pub struct TextureCache(BTreeMap<String, TextureSlot>);

/// A buffer created once for an instance entry, and what it was created as.
///
/// # Invariants
///
/// The kind is the kind the *first* pass that named the entry asked for, and it is
/// reported when a later pass asks for a different one. It is not what the later pass
/// binds at: a read only and a writable binding of a storage buffer share this buffer
/// and differ only in the bind group layout entry.
pub struct BufferSlot {
    /// The buffer itself.
    pub buffer: wgpu::Buffer,
    /// The usages the backend gave it at creation, which cannot be widened afterwards.
    pub usages: wgpu::BufferUsages,
    /// The kind the buffer was created for, kept only to name it in a disagreement.
    kind: BufferKind,
}

impl BufferSlot {
    /// Returns the kind the buffer was created for.
    pub fn kind(&self) -> BufferKind {
        self.kind
    }
}

/// Buffers created during one render, keyed by the instance entry they hold.
///
/// # Invariants
///
/// One entry is one buffer however many passes bind it, which is what makes a storage
/// buffer a channel between passes: a compute pass that writes one and a later pass that
/// reads it share the same bytes rather than two uploads of the same initial ones.
///
/// The level a buffer is bound at is *not* part of the cache. It belongs to the pass,
/// because a read only and a writable binding of one buffer are the same buffer: the
/// backend distinguishes them at the bind group, and both are created with
/// [`wgpu::BufferUsages::STORAGE`]. So a buffer bound as read only by one pass and as
/// writable by another needs no reconciliation here, and the one entry serves both.
///
/// What the cache does record is the usage set, because that *is* fixed at creation. One
/// entry cannot be a uniform in one pass and a storage buffer in another, because a
/// backend gives a buffer its usages once and a second set would mean two buffers under
/// one name. That is a document that contradicts itself, and it is reported as one.
#[derive(Default)]
pub struct BufferCache(BTreeMap<String, BufferSlot>);

impl WgpuRenderer {
    /// Resolves every binding point of a pass against the instance.
    pub(super) fn bind_pass(
        &self,
        pass: &PassView<'_>,
        instance: &Value,
        cache: &mut TextureCache,
        buffers: &mut BufferCache,
    ) -> Result<Bindings, RenderError> {
        // The pass is checked against the interface its shader declares first, so a
        // mismatch is a render error with a message about the data, rather than a layout
        // that was never going to fit.
        verify_pass_inputs(pass, instance)?;

        let packed = uniform_bytes_for(pass, instance)?;
        // This uniform is the pass's own, packed from the fields this pass declares, so it
        // is built fresh for every pass and is deliberately not in the buffer cache: it has
        // no instance entry to key it by, and two passes that packed the same bytes would
        // still be two passes. The uniform a *binder* names is a different thing and is
        // cached, because that one does come from an instance entry.
        let uniform = self.upload(&packed, wgpu::BufferUsages::UNIFORM, "whiting uniform");

        let mut bindings = Bindings {
            uniform,
            vertices: Vec::new(),
            bound: Vec::new(),
            attachments: Vec::new(),
            targets: pass
                .targets()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            instances: 1,
        };

        let declarations = pass.binders()?;
        for declaration in declarations.iter() {
            let payload = declaration.bind(instance)?;
            match payload.kind() {
                Some(BinderKind::Buffer) => {
                    let source = declaration.source()?;
                    self.bind_buffer(
                        source,
                        declaration.point(),
                        payload.value(),
                        &mut bindings,
                        buffers,
                    )?;
                }
                Some(BinderKind::Texture) | Some(BinderKind::TextureArray) => {
                    let texture = self.texture(declaration, instance, cache)?;
                    // A pass that renders into a texture cannot also read it: a backend
                    // treats an attachment as an exclusive use of it. The declaration is
                    // how the pass says what it renders into, so a point that is one of
                    // this pass's targets is an attachment rather than a binding.
                    if bindings.targets.contains(&texture.point) {
                        // An attachment of a texture array is **one layer** of it, named by
                        // the declaration, and the cached view is the whole array. So the
                        // view a pass draws into is built here rather than taken from the
                        // cache: it is a function of this pass's declaration, and one
                        // cached texture serves every pass that writes a different page.
                        // A layer the pool does not have is refused, because a pass that
                        // drew past the end of the pool would write the first page again
                        // and hand one light another's shadow.
                        let layer = texture
                            .shape
                            .check_layer(&texture.source, &texture.point, layer_of(&declaration))?;
                        let view = texture
                            .texture
                            .create_view(&texture.shape.layer_view_descriptor(layer));
                        // An attachment is filtered by nothing whatever its payload said,
                        // because a backend forbids filtering a texture being written.
                        bindings.attachments.push(TextureSlot {
                            sampler: None,
                            attachment: true,
                            view,
                            layer,
                            ..texture
                        });
                    } else if let Some(layer) = declaration.layer() {
                        // A layer is what a pass draws into. A pass that only reads a
                        // texture array does not choose one: the shader indexes the layers
                        // itself, with the page its own data carries.
                        return Err(RenderError::Backend(format!(
                            "the pass binds `{}` and names layer {layer} of it, but it is not \
                             one of the pass's targets. A layer is what a pass draws into, and a \
                             pass that only reads a texture array indexes the layers itself.",
                            texture.point
                        )));
                    } else {
                        bindings.bound.push(Bound::Texture(texture));
                    }
                }
                Some(BinderKind::Uniform) => {
                    let data = field(payload.value(), DATA)
                        .and_then(bytes)
                        .ok_or_else(|| {
                            RenderError::Backend("a uniform payload needs `data`".to_owned())
                        })?;
                    // A uniform is one instance entry like any other, so it is cached under
                    // its own name: two passes binding the same uniform block are binding
                    // the same bytes, and uploading them twice buys nothing.
                    let source = declaration.source()?;
                    let buffer = self.resolve_buffer(source, data, BufferKind::Uniform, buffers)?;
                    bindings
                        .bound
                        .push(Bound::Buffer(buffer, BufferKind::Uniform));
                }
                _ => {
                    return Err(RenderError::Backend(format!(
                        "`{}` is a shader, which a pass names rather than binds",
                        declaration.point()
                    )));
                }
            }
        }
        bindings.order_vertices();
        Ok(bindings)
    }

    /// Resolves a buffer payload into a vertex slot or a bound buffer.
    ///
    /// # Invariants
    ///
    /// Every word the payload declares is one this module knows, checked before anything
    /// is created. A word that fell through to the uniform path would produce a buffer
    /// that is the right size and the wrong kind: a typo would cost a frame that is
    /// quietly wrong rather than a message naming the entry, and the same is true of a
    /// `storage_write` on a buffer that never said it was a storage buffer.
    fn bind_buffer(
        &self,
        source: &str,
        point: &str,
        payload: &Value,
        bindings: &mut Bindings,
        cache: &mut BufferCache,
    ) -> Result<(), RenderError> {
        let usage: Vec<&str> = field(payload, USAGE)
            .and_then(Value::as_list)
            .map(|entries| entries.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let kind = buffer_kind(&usage, source)?;
        let data = field(payload, DATA)
            .and_then(bytes)
            .ok_or_else(|| RenderError::Backend("a buffer payload needs `data`".to_owned()))?;

        if usage.contains(&VERTEX) || usage.contains(&INSTANCE) {
            let per_instance = usage.contains(&INSTANCE);
            let layout = vertex_layout_of(payload)?;
            let count = layout.vertex_count(data.len() as u64).unwrap_or(0);
            if per_instance {
                bindings.instances = count;
            }
            bindings.vertices.push(VertexSlot {
                point: point.to_owned(),
                slot: 0,
                buffer: self.upload(
                    data,
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    "whiting vertex",
                ),
                step_mode: if per_instance {
                    wgpu::VertexStepMode::Instance
                } else {
                    wgpu::VertexStepMode::Vertex
                },
                layout,
                count,
                location_base: 0,
                topology: topology_of(payload),
            });
            return Ok(());
        }

        let buffer = self.resolve_buffer(source, data, kind, cache)?;
        bindings.bound.push(Bound::Buffer(buffer, kind));
        Ok(())
    }

    /// Returns the buffer an instance entry holds, uploading it the first time.
    ///
    /// # Invariants
    ///
    /// One entry is one buffer for the whole frame, so a pass that writes a storage buffer
    /// and a pass that reads it later see the same bytes. The usage set a buffer was
    /// created with is recorded rather than assumed, because a backend grants a buffer its
    /// usages once: an entry that wants a usage the cached buffer does not carry is a
    /// document asking for two different buffers under one name, and is reported rather
    /// than served by quietly making a second one.
    fn resolve_buffer(
        &self,
        source: &str,
        data: &[u8],
        kind: BufferKind,
        cache: &mut BufferCache,
    ) -> Result<wgpu::Buffer, RenderError> {
        // The usage is the kind's alone, so a uniform entry is created exactly as it was
        // before this cache existed. Nothing here needs `COPY_DST`: the upload goes
        // through mapped at creation, and a buffer a shader writes is written by a shader.
        let usages = kind.usages();
        if let Some(existing) = cache.0.get(source) {
            if !existing.usages.contains(usages) {
                return Err(RenderError::Backend(format!(
                    "the buffer `{source}` is bound as {} here and as {} elsewhere in this \
                     frame. A backend gives a buffer its usages once, so one entry cannot be \
                     two kinds of buffer: give the second one its own entry, or bind both \
                     passes to the same kind.",
                    existing.kind().name(),
                    kind.name()
                )));
            }
            // The buffer is shared; the level it is bound at is this pass's own business,
            // and a read only and a writable binding of one buffer are the same buffer.
            return Ok(existing.buffer.clone());
        }
        let buffer = self.upload(data, usages, source);
        cache.0.insert(
            source.to_owned(),
            BufferSlot {
                buffer: buffer.clone(),
                usages,
                kind,
            },
        );
        Ok(buffer)
    }

    /// Returns the texture a declaration names, creating it the first time.
    fn texture(
        &self,
        declaration: &Declaration<'_>,
        instance: &Value,
        cache: &mut TextureCache,
    ) -> Result<TextureSlot, RenderError> {
        let source = declaration.source()?.to_owned();
        if let Some(existing) = cache.0.get(&source) {
            // The texture is shared; only the name it is bound under is this pass's.
            return Ok(TextureSlot {
                point: declaration.point().to_owned(),
                ..existing.clone()
            });
        }

        let payload = declaration.bind(instance)?;
        // The shape is read before anything is created, and a layer count the device cannot
        // build is refused here rather than reaching a driver, where the message would name
        // neither the entry nor the limit.
        let shape = TextureShape::read(&source, payload.value(), self.max_texture_array_layers())?;
        let format = payload
            .value()
            .variant_fields()
            .and_then(|fields| fields.get(FORMAT))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RenderError::Backend(format!("the texture `{source}` needs a `format`"))
            })?;
        let format = WgpuRenderer::format_of(format)?;

        // The declaration decides whether there is a sampler; the payload decides how it
        // filters. A texture the payload does not call sampled is read with `textureLoad`
        // and gets none, whatever its format is. An array is read the same way as the flat
        // texture of the same format, with the layer as one more argument.
        let sampled = is_sampled(payload.value());
        if sampled && WgpuRenderer::is_depth_format(format) {
            return Err(RenderError::Backend(format!(
                "the texture `{source}` declares `sampled`, and it is a depth texture. A \
                 depth texture is read with `textureLoad`; a filtered depth comparison is a \
                 comparison sampler, which is a different resource from the one this \
                 binding model has."
            )));
        }
        if sampled && !WgpuRenderer::is_filterable(format) {
            return Err(RenderError::Backend(format!(
                "the texture `{source}` declares `sampled`, and `{}` cannot be filtered: \
                 filtering it needs the `float32-filterable` feature, which this device does \
                 not request. Declaring `sampled` asks for a linear filter, and silently \
                 giving a blocky one instead would be a worse answer than this error.",
                format_label(format)
            )));
        }

        // A texture with pixels gets the levels they imply. One without gets a single
        // level: a pass is about to write its base level, so a chain built here would be
        // stale, and silently wrong levels are worse than none.
        let data = field(payload.value(), DATA).and_then(bytes);
        if data.is_some() && shape.layers > 1 {
            return Err(RenderError::Backend(format!(
                "the texture array `{source}` carries `data`. Filling a texture averages \
                 neighbouring texels to make the next level down, and it cannot tell a layer \
                 boundary from a row, so an array has no uploaded form here: a page pool is \
                 written by passes and never read from the instance."
            )));
        }
        let levels = if data.is_some() {
            levels_for(shape.width, shape.height)
        } else {
            1
        };

        let texture = self
            .device
            .create_texture(&shape.descriptor(&source, format, levels));
        if let Some(data) = data {
            self.fill_texture(
                &source,
                &texture,
                format,
                shape.width,
                shape.height,
                levels,
                data,
            )?;
        }
        let view = texture.create_view(&shape.view_descriptor());
        let sampler = sampled.then(|| {
            let settings = SamplerSettings::read(payload.value());
            self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(&source),
                address_mode_u: address_mode(settings.address),
                address_mode_v: address_mode(settings.address),
                address_mode_w: address_mode(settings.address),
                mag_filter: filter_mode(settings.linear_filter),
                min_filter: filter_mode(settings.linear_filter),
                mipmap_filter: mipmap_filter_mode(settings.linear_mip_filter),
                ..Default::default()
            })
        });
        let slot = TextureSlot {
            point: declaration.point().to_owned(),
            source: source.clone(),
            format,
            texture: texture.clone(),
            view: view.clone(),
            shape,
            layer: 0,
            sampler,
            attachment: false,
        };
        cache.0.insert(source, slot.clone());
        Ok(slot)
    }

    /// Returns the most layers a texture may have on this device.
    ///
    /// # Invariants
    ///
    /// The number is read from the device rather than written here, because it is a
    /// property of what was asked for when the device was created: the floor this renderer
    /// requests puts it at 256, and a renderer that asked for the adapter's own limits
    /// would read a different number from this same line. A document is refused against
    /// whatever the device in hand allows, which is the only number that means anything.
    fn max_texture_array_layers(&self) -> u32 {
        self.device.limits().max_texture_array_layers
    }

    /// Writes a texture's pixels, and the levels a box filter derives from them.
    ///
    /// # Invariants
    ///
    /// One upload fills the whole chain, so a sampled texture has every level it advertises.
    /// A level is averaged in linear light, which is what stops a tiling surface going grey
    /// as it shrinks: averaging gamma encoded values averages the wrong quantity.
    fn fill_texture(
        &self,
        source: &str,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        levels: u32,
        data: &[u8],
    ) -> Result<(), RenderError> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() != expected {
            return Err(RenderError::Backend(format!(
                "the texture `{source}` declares `data` of {} bytes, and {width} by {height} \
                 texels of four bytes each is {expected}",
                data.len()
            )));
        }
        let encoded = is_srgb(format);
        let mut level = data.to_vec();
        let mut level_width = width;
        let mut level_height = height;
        for index in 0..levels {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: index,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &level,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    // `write_texture` stages into a buffer itself, so a row does not have to
                    // be a multiple of `COPY_BYTES_PER_ROW_ALIGNMENT`. A 300, 100 or 90 wide
                    // texture is then an ordinary shape rather than a padded one.
                    bytes_per_row: Some(level_width * 4),
                    rows_per_image: Some(level_height),
                },
                wgpu::Extent3d {
                    width: level_width,
                    height: level_height,
                    depth_or_array_layers: 1,
                },
            );
            if index + 1 < levels {
                level = filtered_level(&level, level_width, level_height, encoded);
                level_width = (level_width / 2).max(1);
                level_height = (level_height / 2).max(1);
            }
        }
        Ok(())
    }
}

/// Returns how many levels a texture of this size holds.
///
/// # Invariants
///
/// Halving by integer division rather than taking a logarithm, so the count is exactly the
/// number of times the larger side can be halved before it reaches one texel. This is the
/// count the level chain is built with, and the two cannot disagree about it.
fn levels_for(width: u32, height: u32) -> u32 {
    let mut levels = 1;
    let mut size = width.max(height);
    while size > 1 {
        size /= 2;
        levels += 1;
    }
    levels
}

/// Returns the next level down, as a box filter over each two by two block.
///
/// # Invariants
///
/// The result is exactly `max(source_width / 2, 1)` by `max(source_height / 2, 1)` texels of
/// four bytes, which is the shape the chain was sized for. An odd source size drops the
/// last column or row rather than averaging a partial block, so a level is never a blend of
/// a texel with nothing.
fn filtered_level(source: &[u8], width: u32, height: u32, encoded: bool) -> Vec<u8> {
    let target_width = (width / 2).max(1);
    let target_height = (height / 2).max(1);
    let mut target = vec![0u8; target_width as usize * target_height as usize * 4];
    for y in 0..target_height {
        for x in 0..target_width {
            let mut average = [0.0f32; 4];
            for (offset_y, offset_x) in [(0u32, 0u32), (0, 1), (1, 0), (1, 1)] {
                let source_x = (x * 2 + offset_x).min(width - 1);
                let source_y = (y * 2 + offset_y).min(height - 1);
                let at = ((source_y * width + source_x) * 4) as usize;
                for channel in 0..4 {
                    let value = source[at + channel] as f32 / 255.0;
                    // Alpha is linear whatever the format says about the other three.
                    average[channel] += if encoded && channel < 3 {
                        linearize(value)
                    } else {
                        value
                    };
                }
            }
            let at = ((y * target_width + x) * 4) as usize;
            for channel in 0..4 {
                let value = average[channel] / 4.0;
                let value = if encoded && channel < 3 {
                    encode(value)
                } else {
                    value
                };
                target[at + channel] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
    target
}

/// Decodes one sRGB encoded channel into linear light.
fn linearize(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Encodes one linear light channel into sRGB.
fn encode(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

/// Returns how a sampler setting is spelled by the backend.
fn filter_mode(linear: bool) -> wgpu::FilterMode {
    if linear {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    }
}

/// Returns how a level blending setting is spelled by the backend.
///
/// The backend keeps this as its own type rather than reusing [`wgpu::FilterMode`], because
/// a mip filter is a choice between levels rather than between texels.
fn mipmap_filter_mode(linear: bool) -> wgpu::MipmapFilterMode {
    if linear {
        wgpu::MipmapFilterMode::Linear
    } else {
        wgpu::MipmapFilterMode::Nearest
    }
}

/// Returns how an address setting is spelled by the backend.
fn address_mode(address: SamplerAddress) -> wgpu::AddressMode {
    match address {
        SamplerAddress::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        SamplerAddress::Repeat => wgpu::AddressMode::Repeat,
        SamplerAddress::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    }
}

/// Returns whether a format stores its colour channels sRGB encoded.
///
/// Alpha is linear in every one of them, which is why the box filter linearises three
/// channels and not four.
fn is_srgb(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
    )
}

/// Returns the name of a format as a document spells it, for an error message.
fn format_label(format: wgpu::TextureFormat) -> &'static str {
    match format {
        wgpu::TextureFormat::Rgba8Unorm => "rgba8unorm",
        wgpu::TextureFormat::Rgba8UnormSrgb => "rgba8unorm-srgb",
        wgpu::TextureFormat::Rgba16Float => "rgba16float",
        wgpu::TextureFormat::Bgra8Unorm => "bgra8unorm",
        wgpu::TextureFormat::R32Float => "r32float",
        wgpu::TextureFormat::Rgba32Float => "rgba32float",
        _ => "an unsupported format",
    }
}

/// Derives the layout of one packed buffer from the schema it carries.
fn vertex_layout_of(payload: &Value) -> Result<VertexLayout, RenderError> {
    let declared = field(payload, SCHEMA).ok_or_else(|| {
        RenderError::Backend(
            "a vertex buffer needs the schema it was packed with, so that its layout can \
             be derived from it"
                .to_owned(),
        )
    })?;
    let schema = definition::Schema::from_value(declared)
        .map_err(|error| RenderError::Backend(format!("an unreadable packed schema: {error}")))?;
    let layout = vertex_layout(&schema).map_err(|error| {
        RenderError::Backend(format!("a packed buffer has no vertex layout: {error}"))
    })?;
    let size = field(payload, SIZE_BYTES)
        .and_then(Value::as_int)
        .unwrap_or(0);
    if size > 0 && (size as u64) < layout.stride_bytes {
        return Err(RenderError::Backend(format!(
            "a buffer of {size} bytes is shorter than one vertex of {}",
            layout.stride_bytes
        )));
    }
    Ok(layout)
}

/// Borrows the bytes a payload carries.
fn bytes(value: &Value) -> Option<&[u8]> {
    match value {
        Value::Bytes(bytes) => Some(bytes),
        _ => None,
    }
}

/// Borrows a field of an instance entry.
fn field<'value>(payload: &'value Value, name: &str) -> Option<&'value Value> {
    payload.variant_fields()?.get(name)
}

/// Returns the primitive topology a vertex buffer declares, defaulting to triangles.
fn topology_of(payload: &Value) -> wgpu::PrimitiveTopology {
    match graphics::binders::topology_of(payload) {
        Some("Segments") => wgpu::PrimitiveTopology::LineList,
        _ => wgpu::PrimitiveTopology::TriangleList,
    }
}

/// Packs the uniform of a pass the way its own shader declares it.
///
/// A shader that declares no interface has declared no layout either, so the value is
/// packed tightly and the shader is on its own to match it.
fn uniform_bytes_for(pass: &PassView<'_>, instance: &Value) -> Result<Vec<u8>, RenderError> {
    let uniform = pass.uniform()?;
    let packed = match graphics::uniform_schema(pass, instance)? {
        Some(schema) => graphics::uniform_bytes(&schema, uniform),
        None => graphics::value_bytes(uniform),
    };
    packed.map_err(|error| RenderError::Backend(format!("the uniform cannot be uploaded: {error}")))
}
