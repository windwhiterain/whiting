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

use definition::Value;
use graphics::{PassView, resolve_for_pass};

use super::WgpuRenderer;
use super::bindings::{Bindings, TextureCache, TextureSlot};
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
        cleared: &mut Vec<String>,
    ) -> Result<Option<RenderedTarget>, RenderError> {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        let shader = resolve_for_pass(&label, instance, pass.shader_binder()?)
            .map_err(RenderError::Graphics)?;
        let module = self.shader_module(&shader)?;
        // A shader declares the uniform it reads, and that declaration is what the pass is
        // packed against, so a shader that declares it differently is caught here rather
        // than quietly reading the wrong bytes.
        if let Some(schema) = graphics::uniform_schema(pass, instance)? {
            self.check_uniform_layout(&shader, &schema)?;
        }
        let bindings = self.bind_pass(pass, instance, cache)?;

        let attachments = attachments(&bindings)?;
        let colour = attachments.iter().find(|entry| !entry.depth);
        let depth = attachments.iter().find(|entry| entry.depth);
        let format = colour.map_or(wgpu::TextureFormat::Rgba8Unorm, |entry| entry.slot.format);
        let clear = clear_color(pass);

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{label} bind group layout")),
                    entries: &bindings
                        .layout_entries(wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT),
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
            blend: Some(wgpu::BlendState::REPLACE),
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
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
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
            let colour_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = colour
                .map(|entry| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: &entry.slot.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load(cleared, &entry.slot.source, clear),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                })
                .into_iter()
                .collect();
            let depth_attachment = depth.map(|entry| wgpu::RenderPassDepthStencilAttachment {
                view: &entry.slot.view,
                depth_ops: Some(wgpu::Operations {
                    load: load_depth(cleared, &entry.slot.source),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
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
            width: entry.slot.width,
            height: entry.slot.height,
        }))
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

/// Returns whether an attachment has already been written this render.
///
/// The first pass to write a target clears it, so a clear colour is honoured, and the
/// rest load it, so they accumulate.
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
