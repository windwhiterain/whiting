//! Executing a rasterize pass: pipeline, bindings, draw, and attachment handling.
//!
//! # Invariants
//!
//! - A pass draws one mesh, so there is one vertex buffer and one draw call, and the
//!   vertex count comes from the buffer rather than from a document.
//! - A pass whose targets are all depth formats is a depth only pass: it gets a pipeline
//!   with no fragment stage, because a pipeline that declares a colour target it has no
//!   stage to write is rejected by the backend.
//! - A target is cleared by the first pass that writes it and loaded by the rest, or the
//!   second pass into a shared target would erase the first pass's contribution.

use std::collections::BTreeMap;

use definition::Value;
use graphics::{PassView, resolve_for_pass};

use super::WgpuRenderer;
use super::bindings::{Bindings, BufferCache, TextureCache, TextureSlot};
use crate::render::{Frame, RenderError};

/// Vertex entry point every rasterize shader must expose.
pub const VERTEX_ENTRY: &str = "vertex_main";

/// Fragment entry point every rasterize shader must expose.
///
/// The two stages cannot share a name: WGSL has one namespace for functions, so a shader
/// that named both `main` would not compile.
pub const FRAGMENT_ENTRY: &str = "fragment_main";

/// A target resolved out of a pass's declarations, ready to be attached.
struct Attachment<'bindings> {
    slot: &'bindings TextureSlot,
    depth: bool,
}

/// The colour texture a render ended on, which is what a caller asked to see.
pub struct RenderedTarget {
    /// The texture the last colour pass drew into.
    pub texture: wgpu::Texture,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

impl WgpuRenderer {
    /// Executes one rasterize pass, returning the colour target it drew into when it drew
    /// into one.
    pub(super) fn run_rasterize(
        &self,
        pass: &PassView<'_>,
        instance: &Value,
        cache: &mut TextureCache,
        buffers: &mut BufferCache,
        cleared: &mut Vec<String>,
    ) -> Result<Option<RenderedTarget>, RenderError> {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        let shader = resolve_for_pass(&label, instance, pass.shader_binder()?)
            .map_err(RenderError::Graphics)?;
        let module = self.shader_module(&shader, &label)?;
        // A shader declares the uniform it reads, and that declaration is what the pass is
        // packed against, so a shader that declares it differently is caught here rather
        // than quietly reading the wrong bytes.
        if let Some(schema) = graphics::uniform_schema(pass, instance)? {
            self.check_uniform_layout(&shader, &schema, &label)?;
        }
        let bindings = self.bind_pass(pass, instance, cache, buffers)?;

        let attachments = attachments(&bindings)?;
        let colour = attachments.iter().find(|entry| !entry.depth);
        let depth = attachments.iter().find(|entry| entry.depth);
        let format = colour.map_or(wgpu::TextureFormat::Rgba8Unorm, |entry| entry.slot.format);
        let clear = clear_color(pass);
        // What a pass does with blending, depth and samples is on the pass, so a document
        // can express an additive pass, a pass that draws over the depth it reads, and an
        // antialiased edge. A pass that names none of them builds the pipeline it always
        // built.
        let state = graphics::PipelineState::read(pass).map_err(RenderError::Graphics)?;
        let samples = state.samples;

        // A shader names its resources and its vertices by hand, in its own source, and
        // the pass numbers both without consulting it. The two are compared here, against
        // the layout the pass actually built, so a disagreement is an error naming the
        // pass rather than a backend message about a view dimension.
        let entries =
            bindings.layout_entries(wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT);
        self.check_bindings(&shader, &entries, &label)?;

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{label} bind group layout")),
                    entries: &entries,
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{label} pipeline layout")),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label} bind group")),
            layout: &bind_group_layout,
            entries: &bindings.group_entries(),
        });

        // The attributes are compared before the layouts borrow them, so the check sees
        // the numbering this pass will hand the pipeline rather than a borrow of it.
        self.check_vertex_inputs(&shader, &bindings.vertex_locations(), &label)?;
        let mut attributes: Vec<Vec<wgpu::VertexAttribute>> = Vec::new();
        let layouts: Vec<Option<wgpu::VertexBufferLayout>> = bindings
            .vertex_layouts(&mut attributes)
            .into_iter()
            .map(Some)
            .collect();
        // The colour target and the module both have to outlive the pipeline, so the
        // descriptor borrows locals rather than building them inline.
        let colour_targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(blend_of(state.blend)),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let fragment = colour.map(|_| wgpu::FragmentState {
            module: &module,
            entry_point: Some(FRAGMENT_ENTRY),
            compilation_options: Default::default(),
            targets: &colour_targets,
        });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(VERTEX_ENTRY),
                    compilation_options: Default::default(),
                    buffers: &layouts,
                },
                primitive: wgpu::PrimitiveState {
                    topology: bindings.topology(),
                    ..Default::default()
                },
                depth_stencil: depth.map(|entry| wgpu::DepthStencilState {
                    format: entry.slot.format,
                    depth_write_enabled: Some(state.depth_write),
                    depth_compare: Some(compare_of(state.depth_compare)),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment,
                multiview_mask: None,
                cache: None,
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(&format!("{label} encoder")),
            });
        {
            // A pass that takes more than one sample renders into a view of the target with
            // that many samples and resolves into the target itself, so the target holds
            // one pixel per fragment rather than one per sample. The views are keyed by the
            // binding point, which is what tells two attachments of one pass apart.
            let multisampled: BTreeMap<String, wgpu::TextureView> = if samples > 1 {
                attachments
                    .iter()
                    .map(|entry| {
                        (
                            entry.slot.point.clone(),
                            self.multisample_view(&label, entry.slot, samples),
                        )
                    })
                    .collect()
            } else {
                BTreeMap::new()
            };
            // The view a pass draws into, and where a multisampled draw resolves to.
            let colour_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = colour
                .map(|entry| {
                    let (view, resolve_target) = drawing_into(&multisampled, &entry.slot);
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target,
                        ops: wgpu::Operations {
                            load: load(cleared, &entry.slot.source, clear),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                })
                .into_iter()
                .collect();
            let depth_attachment = depth.map(|entry| {
                let (view, _) = drawing_into(&multisampled, &entry.slot);
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: load_depth(cleared, &entry.slot.source),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            });

            let mut pass_encoder = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&label),
                color_attachments: &colour_attachments,
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass_encoder.set_pipeline(&pipeline);
            pass_encoder.set_bind_group(0, &bind_group, &[]);
            for vertex in &bindings.vertices {
                pass_encoder.set_vertex_buffer(vertex.slot, vertex.buffer.slice(..));
            }
            if let Some(count) = bindings.vertex_count() {
                if count > 0 {
                    pass_encoder.draw(0..count as u32, 0..bindings.instances as u32);
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));

        Ok(colour.map(|entry| RenderedTarget {
            texture: entry.slot.texture.clone(),
            width: entry.slot.shape.width,
            height: entry.slot.shape.height,
        }))
    }

    /// Creates a view of an attachment's texture with several samples, which a pass that
    /// takes more than one renders into and resolves out of.
    ///
    /// The view is not kept: a pass that samples more than once creates one per draw and
    /// the backend drops it with the pass, which is cheaper than holding a multisampled
    /// copy of every target for the whole frame.
    fn multisample_view(&self, label: &str, slot: &TextureSlot, samples: u32) -> wgpu::TextureView {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label} multisample")),
            size: wgpu::Extent3d {
                width: slot.shape.width,
                height: slot.shape.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: slot.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Reads a texture back into a frame.
    pub(super) fn read_back(&self, target: &RenderedTarget) -> Result<Frame, RenderError> {
        let (width, height) = (target.width.max(1), target.height.max(1));
        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("whiting readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("whiting readback encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| {
                RenderError::Backend(format!("waiting for readback failed: {error}"))
            })?;
        receiver
            .recv()
            .map_err(|_| RenderError::Backend("the readback never completed".to_owned()))?
            .map_err(|error| {
                RenderError::Backend(format!("mapping the readback failed: {error:?}"))
            })?;

        let view = buffer.slice(..).get_mapped_range().map_err(|error| {
            RenderError::Backend(format!("reading the mapped range failed: {error:?}"))
        })?;
        let mut rgba = Vec::with_capacity(unpadded as usize * height as usize);
        for row in 0..height as usize {
            let start = row * padded as usize;
            rgba.extend_from_slice(&view[start..start + unpadded as usize]);
        }
        drop(view);
        buffer.unmap();

        Frame::new(width, height, rgba).ok_or(RenderError::FrameSize)
    }
}

/// Returns the view an attachment is drawn into, and where a multisampled draw resolves.
///
/// A pass that takes more than one sample draws into a view of the target with that many
/// samples and resolves into the target itself, so the target holds one pixel per fragment
/// rather than one per sample. A pass that takes one draws into the target directly and
/// has nowhere to resolve to.
fn drawing_into<'view>(
    multisampled: &'view BTreeMap<String, wgpu::TextureView>,
    slot: &'view TextureSlot,
) -> (&'view wgpu::TextureView, Option<&'view wgpu::TextureView>) {
    match multisampled.get(&slot.point) {
        Some(view) => (view, Some(&slot.view)),
        None => (&slot.view, None),
    }
}

/// Returns the blend a pass's declared blend becomes.
///
/// The colour and alpha terms are given separately because a backend blends them with
/// their own equations: replacing colour and adding alpha tints a frame rather than
/// changing it, and the two are the most common way a blend is written down wrong.
fn blend_of(blend: graphics::Blend) -> wgpu::BlendState {
    use wgpu::{BlendComponent, BlendFactor, BlendOperation};
    let (colour, alpha) = match blend {
        // Replacing both is what writing a value means; blending either one leaves a
        // target whose alpha drifts away from its colour.
        graphics::Blend::Replace => (BlendComponent::REPLACE, BlendComponent::REPLACE),
        // Straight alpha: the source contributes its share and the destination the rest,
        // which is the only weighting that keeps a translucent surface from darkening what
        // is behind it twice over.
        graphics::Blend::Alpha => (
            BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        ),
        // One and one: light is gathered by adding to what is already there rather than by
        // replacing it.
        graphics::Blend::Add => (
            BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        ),
    };
    wgpu::BlendState {
        color: colour,
        alpha,
    }
}

/// Returns the depth comparison a pass's declared comparison becomes.
fn compare_of(compare: graphics::Compare) -> wgpu::CompareFunction {
    match compare {
        graphics::Compare::Never => wgpu::CompareFunction::Never,
        graphics::Compare::Less => wgpu::CompareFunction::Less,
        graphics::Compare::LessEqual => wgpu::CompareFunction::LessEqual,
        graphics::Compare::Equal => wgpu::CompareFunction::Equal,
        graphics::Compare::Greater => wgpu::CompareFunction::Greater,
        graphics::Compare::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        graphics::Compare::Always => wgpu::CompareFunction::Always,
    }
}

/// Returns whether an attachment has already been written this render.
fn load(cleared: &mut Vec<String>, source: &str, clear: wgpu::Color) -> wgpu::LoadOp<wgpu::Color> {
    if written(cleared, source) {
        wgpu::LoadOp::Load
    } else {
        wgpu::LoadOp::Clear(clear)
    }
}

/// Returns whether a depth attachment has already been written this render.
fn load_depth(cleared: &mut Vec<String>, source: &str) -> wgpu::LoadOp<f32> {
    if written(cleared, source) {
        wgpu::LoadOp::Load
    } else {
        wgpu::LoadOp::Clear(1.0)
    }
}

/// Records that a target has now been written, and reports whether it had been already.
fn written(cleared: &mut Vec<String>, source: &str) -> bool {
    if cleared.iter().any(|written| written == source) {
        return true;
    }
    cleared.push(source.to_owned());
    false
}

/// Resolves the targets of a pass, in attachment order.
fn attachments(bindings: &Bindings) -> Result<Vec<Attachment<'_>>, RenderError> {
    let mut resolved = Vec::with_capacity(bindings.targets.len());
    for point in &bindings.targets {
        let slot = bindings.target(point).ok_or_else(|| {
            RenderError::Backend(format!(
                "the pass renders into `{point}`, which is not one of its bindings"
            ))
        })?;
        let depth = WgpuRenderer::is_depth_format(slot.format);
        resolved.push(Attachment { slot, depth });
    }
    Ok(resolved)
}

/// Reads the clear colour of a rasterize pass, defaulting to transparent black.
fn clear_color(pass: &PassView<'_>) -> wgpu::Color {
    let Ok(components) = pass.clear_color() else {
        return wgpu::Color::TRANSPARENT;
    };
    let component = |index: usize| -> f64 {
        components
            .get(index)
            .and_then(Value::as_float)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    };
    wgpu::Color {
        r: component(0),
        g: component(1),
        b: component(2),
        a: component(3),
    }
}
