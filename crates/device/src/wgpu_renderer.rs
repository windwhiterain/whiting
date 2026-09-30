//! The `wgpu` backend: headless rasterize and compute with framebuffer readback.
//!
//! The renderer walks the dynamic value tree directly. It never materializes a typed
//! copy of the graphics document, so a schema change needs no recompilation of this file.
//!
//! # Shaders
//!
//! A pass names one shader, which must expose both the vertex and the fragment entry
//! point, so one module serves both stages of a rasterize pass. A shader binder names a
//! path, resolved relative to the configured shader root, which defaults to the process
//! working directory. A compute pass invokes the entry point named in its document.
//!
//! # Geometry
//!
//! A rasterize pass clears its target and issues no draw calls. A buffer payload says
//! how many bytes it holds, which is not the same thing as how many vertices it holds or
//! what those vertices look like, and `wgpu` needs both to describe a vertex buffer. The
//! declaration that would carry them is written up in
//! [proposals/vertex-layout-declaration.md](../../../../.agents/proposals/vertex-layout-declaration.md).

use std::path::{Path, PathBuf};

use definition::Value;
use graphics::binders::{Declaration, FORMAT};
use graphics::{Binder, PassKind, PassView, passes, resolve_for_pass};
use wgpu::util::DeviceExt;

use crate::render::{Frame, RenderError, Renderer, texture_size};

/// Vertex entry point every rasterize shader must expose.
const VERTEX_ENTRY: &str = "vertex_main";

/// Fragment entry point every rasterize shader must expose.
const FRAGMENT_ENTRY: &str = "fragment_main";

/// A renderer that executes passes on a gpu.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader_root: PathBuf,
}

impl WgpuRenderer {
    /// Requests an adapter and creates a device.
    ///
    /// Returns [`RenderError::NoAdapter`] when no adapter is available, which is the
    /// signal for a caller to fall back to the no-operation renderer.
    pub async fn new(shader_root: impl Into<PathBuf>) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| RenderError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| RenderError::Backend(error.to_string()))?;

        Ok(Self {
            device,
            queue,
            shader_root: shader_root.into(),
        })
    }

    /// Returns whether an adapter is present on this machine.
    pub async fn probe() -> bool {
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .is_ok()
    }

    /// Maps a colour target format name onto a texture format.
    fn target_format(name: &str) -> Result<wgpu::TextureFormat, RenderError> {
        Ok(match name {
            "rgba8unorm" => wgpu::TextureFormat::Rgba8Unorm,
            "rgba8unorm-srgb" => wgpu::TextureFormat::Rgba8UnormSrgb,
            "rgba16float" => wgpu::TextureFormat::Rgba16Float,
            "bgra8unorm" => wgpu::TextureFormat::Bgra8Unorm,
            "r32float" => wgpu::TextureFormat::R32Float,
            "rgba32float" => wgpu::TextureFormat::Rgba32Float,
            other => {
                return Err(RenderError::Backend(format!(
                    "`{other}` is not a supported colour target format"
                )));
            }
        })
    }

    /// Reads the source of a bound shader document.
    fn shader_source(&self, binder: &Binder<'_>) -> Result<(String, String), RenderError> {
        let path = binder
            .text("path")
            .ok_or_else(|| RenderError::Backend("a shader binder needs a `path`".to_owned()))?
            .to_owned();
        let full_path = self.shader_root.join(Path::new(&path));
        let source =
            std::fs::read_to_string(&full_path).map_err(|error| RenderError::ShaderRead {
                path: full_path.display().to_string(),
                reason: error.to_string(),
            })?;
        Ok((path, source))
    }

    /// Creates a shader module from a bound shader document.
    fn shader_module(&self, binder: &Binder<'_>) -> Result<wgpu::ShaderModule, RenderError> {
        let (path, source) = self.shader_source(binder)?;
        Ok(self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&path),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            }))
    }

    /// Creates the colour target a rasterize pass renders into.
    fn target_texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("whiting colour target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    /// Executes one compute pass into a storage texture.
    fn run_compute(&self, pass: &PassView<'_>, instance: &Value) -> Result<(), RenderError> {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        let binder = resolve_for_pass(&label, instance, pass.shader_binder()?)?;
        let module = self.shader_module(&binder)?;

        let numbers: Vec<u32> = pass
            .workgroup()?
            .iter()
            .filter_map(|value| value.as_int())
            .map(|value| value.max(0) as u32)
            .collect();
        let (workgroup_size, dispatch) = numbers.split_at_checked(3).ok_or_else(|| {
            RenderError::Backend("a compute pass needs a six component workgroup".to_owned())
        })?;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("whiting compute target"),
            size: wgpu::Extent3d {
                width: dispatch.first().copied().unwrap_or(1).max(1),
                height: dispatch.get(1).copied().unwrap_or(1).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("whiting compute"),
                layout: None,
                module: &module,
                entry_point: Some(pass.entry_point().unwrap_or("main")),
                compilation_options: Default::default(),
                cache: None,
            });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("whiting compute bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("whiting compute encoder"),
            });
        {
            let mut pass_encoder = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("whiting compute pass"),
                timestamp_writes: None,
            });
            pass_encoder.set_pipeline(&pipeline);
            pass_encoder.set_bind_group(0, &bind_group, &[]);
            pass_encoder.dispatch_workgroups(
                workgroup_size.first().copied().unwrap_or(1).max(1),
                workgroup_size.get(1).copied().unwrap_or(1).max(1),
                workgroup_size.get(2).copied().unwrap_or(1).max(1),
            );
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Executes one rasterize pass and returns the frame it produced.
    fn run_rasterize(&self, pass: &PassView<'_>, instance: &Value) -> Result<Frame, RenderError> {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        // A pass binds one shader, which must expose both stages, so one module serves
        // both of them.
        let shader = resolve_for_pass(&label, instance, pass.shader_binder()?)?;
        let module = self.shader_module(&shader)?;

        let declarations = pass.binders()?;
        let target = first_target(&declarations, pass.targets()?, instance)
            .ok_or_else(|| RenderError::Backend("a rasterize pass needs a target".to_owned()))?;
        let format = Self::target_format(&target.format)?;
        let (width, height) = (target.width.max(1), target.height.max(1));

        let clear = clear_color(pass)?;
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("whiting rasterize layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("whiting rasterize"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(VERTEX_ENTRY),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(FRAGMENT_ENTRY),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let (texture, target_view) = self.target_texture(width, height, format);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("whiting rasterize encoder"),
            });
        {
            let mut pass_encoder = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("whiting rasterize pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass_encoder.set_pipeline(&pipeline);
            // No draw call is issued yet. A buffer says how many bytes it holds, not how
            // many vertices or what attributes they have, and wgpu needs both to describe
            // a vertex buffer. See proposals/vertex-layout-declaration.md.
        }

        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("whiting readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
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

impl Renderer for WgpuRenderer {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn available(&self) -> bool {
        true
    }

    fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame, RenderError> {
        let mut rendered: Option<Frame> = None;
        for pass in passes(graphics)? {
            match pass.kind() {
                // A compute pass runs even though it contributes no pixels, because it may
                // be a prerequisite for the passes that follow.
                PassKind::Compute => self.run_compute(&pass, instance)?,
                PassKind::Rasterize => {
                    if rendered.is_none() {
                        rendered = Some(self.run_rasterize(&pass, instance)?);
                    }
                }
            }
        }
        rendered.ok_or_else(|| {
            RenderError::Backend("the graphics document declares no rasterize pass".to_owned())
        })
    }
}

/// Reads the clear colour of a rasterize pass, defaulting to transparent black.
fn clear_color(pass: &PassView<'_>) -> Result<wgpu::Color, RenderError> {
    let components = pass.clear_color()?;
    let component = |index: usize| -> f64 {
        components
            .get(index)
            .and_then(Value::as_float)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    };
    Ok(wgpu::Color {
        r: component(0),
        g: component(1),
        b: component(2),
        a: component(3),
    })
}

/// A target resolved out of a pass's declarations and the instance.
struct Target {
    width: u32,
    height: u32,
    format: String,
}

/// Resolves the first named target of a pass to the texture the instance holds.
///
/// A pass names its attachments as binding points, and the format and the size live with
/// the payload, so a renderer never has to guess what a target is.
fn first_target(
    declarations: &[Declaration<'_>],
    targets: &[Value],
    instance: &Value,
) -> Option<Target> {
    targets.iter().filter_map(Value::as_str).find_map(|point| {
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.point() == point)?;
        let payload = declaration.bind(instance).ok()?;
        let fields = payload.value().variant_fields()?;
        let format = fields.get(FORMAT)?.as_str()?.to_owned();
        let (width, height) = texture_size(payload.value())?;
        Some(Target {
            width,
            height,
            format,
        })
    })
}

/// Creates a storage buffer, used by callers that bind one outside the pass path.
#[allow(dead_code)]
fn storage_buffer(device: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("whiting storage"),
        contents: data,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}
