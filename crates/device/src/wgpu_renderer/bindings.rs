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
//! - every other buffer and every texture is bound to the shader, and its binding index
//!   is its position among the bound points, in name order.
//!
//! # Invariants
//!
//! - A pass is verified against the interface its own shader declares before anything here
//!   runs, so a pass that reached this module has already been checked against the data it
//!   names. What this module adds is the gpu side of that agreement: a vertex layout
//!   derived from the same schema the bytes were packed with.
//! - A texture is created once per instance entry and shared by every pass that binds it,
//!   which is what lets one pass write a shadow map and the next read it.

use std::collections::BTreeMap;

use definition::Value;
use graphics::binders::{BinderKind, DATA, FORMAT, HEIGHT, SCHEMA, SIZE_BYTES, USAGE, WIDTH};
use graphics::layout::{VertexLayout, vertex_layout};
use graphics::{Declaration, PassView, verify_pass_inputs};

use super::WgpuRenderer;
use crate::render::RenderError;

/// The usage that makes a buffer a vertex buffer.
pub const VERTEX: &str = "vertex";

/// The usage that makes a buffer step once per instance.
pub const INSTANCE: &str = "instance";

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
    /// A buffer, bound as a uniform.
    Buffer(wgpu::Buffer),
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
    /// The size the instance declared.
    pub width: u32,
    /// The size the instance declared.
    pub height: u32,
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

    /// Describes the bindings to a pipeline.
    ///
    /// The uniform is binding zero and the bound resources follow it, in name order, so a
    /// shader can name them without knowing which pass declared them. A vertex buffer is
    /// not one of them: it is a vertex buffer, and it is described by
    /// [`Bindings::vertex_layouts`].
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
        entries.extend(self.bound.iter().enumerate().map(|(index, resource)| {
            wgpu::BindGroupLayoutEntry {
                binding: (index + 1) as u32,
                visibility,
                ty: match resource {
                    Bound::Buffer(_) => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    Bound::Texture(texture) => wgpu::BindingType::Texture {
                        // A depth texture is sampled as depth, not as float: a shader
                        // that declares `texture_depth_2d` is bound to one, and a
                        // float sample type would not match it.
                        sample_type: if WgpuRenderer::is_depth_format(texture.format) {
                            wgpu::TextureSampleType::Depth
                        } else {
                            wgpu::TextureSampleType::Float { filterable: false }
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
                count: None,
            }
        }));
        entries
    }

    /// Fills a bind group with the resources [`Bindings::layout_entries`] declared.
    pub fn group_entries(&self) -> Vec<wgpu::BindGroupEntry<'_>> {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(self.uniform.as_entire_buffer_binding()),
        }];
        entries.extend(self.bound.iter().enumerate().map(|(index, resource)| {
            wgpu::BindGroupEntry {
                binding: (index + 1) as u32,
                resource: match resource {
                    Bound::Buffer(buffer) => {
                        wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding())
                    }
                    Bound::Texture(texture) => wgpu::BindingResource::TextureView(&texture.view),
                },
            }
        }));
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

impl WgpuRenderer {
    /// Resolves every binding point of a pass against the instance.
    pub(super) fn bind_pass(
        &self,
        pass: &PassView<'_>,
        instance: &Value,
        cache: &mut TextureCache,
    ) -> Result<Bindings, RenderError> {
        // The pass is checked against the interface its shader declares first, so a
        // mismatch is a render error with a message about the data, rather than a layout
        // that was never going to fit.
        verify_pass_inputs(pass, instance)?;

        let packed = uniform_bytes_for(pass, instance)?;
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
                    self.bind_buffer(declaration.point(), payload.value(), &mut bindings)?;
                }
                Some(BinderKind::Texture) => {
                    let texture = self.texture(declaration, instance, cache)?;
                    // A pass that renders into a texture cannot also read it: a backend
                    // treats an attachment as an exclusive use of it. The declaration is
                    // how the pass says what it renders into, so a point that is one of
                    // this pass's targets is an attachment rather than a binding.
                    if bindings.targets.contains(&texture.point) {
                        bindings.attachments.push(texture);
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
                    bindings.bound.push(Bound::Buffer(self.upload(
                        data,
                        wgpu::BufferUsages::UNIFORM,
                        "whiting uniform",
                    )));
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
    fn bind_buffer(
        &self,
        point: &str,
        payload: &Value,
        bindings: &mut Bindings,
    ) -> Result<(), RenderError> {
        let usage: Vec<&str> = field(payload, USAGE)
            .and_then(Value::as_list)
            .map(|entries| entries.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
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

        bindings.bound.push(Bound::Buffer(self.upload(
            data,
            wgpu::BufferUsages::UNIFORM,
            "whiting storage",
        )));
        Ok(())
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
        let fields = payload
            .value()
            .variant_fields()
            .ok_or_else(|| RenderError::Backend(format!("`{source}` is not a texture payload")))?;
        let number = |name: &str| {
            fields.get(name).and_then(Value::as_int).ok_or_else(|| {
                RenderError::Backend(format!("the texture `{source}` needs a `{name}`"))
            })
        };
        let width = number(WIDTH)?.max(1) as u32;
        let height = number(HEIGHT)?.max(1) as u32;
        let format = fields.get(FORMAT).and_then(Value::as_str).ok_or_else(|| {
            RenderError::Backend(format!("the texture `{source}` needs a `format`"))
        })?;
        let format = WgpuRenderer::format_of(format)?;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&source),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let slot = TextureSlot {
            point: declaration.point().to_owned(),
            source: source.clone(),
            format,
            texture: texture.clone(),
            view: view.clone(),
            width,
            height,
        };
        cache.0.insert(source, slot.clone());
        Ok(slot)
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
