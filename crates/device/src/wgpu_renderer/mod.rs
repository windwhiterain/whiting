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
//! # What comes back
//!
//! Every pass runs, in the order the document lists them, and what comes back is the
//! colour target of the *last* pass that drew into one. A standard scene ends on its
//! lighting pass, so that is the image; a scene whose last pass writes only a depth map
//! has no image, which is reported rather than papered over with a placeholder.
//!
//! # Errors
//!
//! A backend that is handed a pipeline it cannot build says so, rather than drawing
//! nothing: the device is created with an error handler, and whatever the backend reports
//! is drained into the next [`RenderError`]. A shader that does not compile, a bind group
//! that does not match its layout, and a draw with no buffer all arrive as errors with
//! the backend's own words in them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use definition::Value;
use graphics::{Binder, PassKind, passes};
use wgpu::util::DeviceExt;

use crate::render::{Frame, RenderError, Renderer};
use bindings::TextureCache;
use raster::RenderedTarget;

mod bindings;
mod compute;
mod raster;
mod uniform;

/// A renderer that executes passes on a gpu.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader_root: PathBuf,
    errors: Arc<Mutex<Vec<String>>>,
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
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("whiting device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(|error| RenderError::Backend(error.to_string()))?;

        // Without a handler a backend that is handed something it cannot build is
        // silent, and a silent renderer is indistinguishable from one that drew nothing.
        let errors = Arc::new(Mutex::new(Vec::new()));
        let collected = Arc::clone(&errors);
        device.on_uncaptured_error(Arc::new(move |error| {
            if let Ok(mut collected) = collected.lock() {
                collected.push(error.to_string());
            }
        }));
        Ok(Self {
            device,
            queue,
            shader_root: shader_root.into(),
            errors,
        })
    }

    /// Returns whether an adapter is present on this machine.
    pub async fn probe() -> bool {
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .is_ok()
    }

    /// Maps a format name onto a texture format.
    pub(super) fn format_of(name: &str) -> Result<wgpu::TextureFormat, RenderError> {
        Ok(match name {
            "rgba8unorm" => wgpu::TextureFormat::Rgba8Unorm,
            "rgba8unorm-srgb" => wgpu::TextureFormat::Rgba8UnormSrgb,
            "rgba16float" => wgpu::TextureFormat::Rgba16Float,
            "bgra8unorm" => wgpu::TextureFormat::Bgra8Unorm,
            "r32float" => wgpu::TextureFormat::R32Float,
            "rgba32float" => wgpu::TextureFormat::Rgba32Float,
            "depth16unorm" => wgpu::TextureFormat::Depth16Unorm,
            "depth24plus" => wgpu::TextureFormat::Depth24Plus,
            "depth24plus-stencil8" => wgpu::TextureFormat::Depth24PlusStencil8,
            "depth32float" => wgpu::TextureFormat::Depth32Float,
            other => {
                return Err(RenderError::Backend(format!(
                    "`{other}` is not a supported texture format"
                )));
            }
        })
    }

    /// Returns whether a format is a depth format, which is what makes a target an
    /// attachment rather than a colour.
    pub(super) const fn is_depth_format(format: wgpu::TextureFormat) -> bool {
        matches!(
            format,
            wgpu::TextureFormat::Depth16Unorm
                | wgpu::TextureFormat::Depth24Plus
                | wgpu::TextureFormat::Depth24PlusStencil8
                | wgpu::TextureFormat::Depth32Float
                | wgpu::TextureFormat::Depth32FloatStencil8
        )
    }

    /// Uploads bytes into a buffer.
    pub(super) fn upload(
        &self,
        bytes: &[u8],
        usage: wgpu::BufferUsages,
        label: &str,
    ) -> wgpu::Buffer {
        // A zero sized buffer is not something a backend can bind, so an empty upload
        // still carries four bytes. What a shader reads past them is never read, because
        // a draw of zero elements reads nothing at all.
        let empty;
        let contents = if bytes.is_empty() {
            empty = [0u8; 4];
            &empty[..]
        } else {
            bytes
        };
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
    }

    /// Takes whatever the backend has complained about since the last time.
    pub(super) fn drain_errors(&self) -> Option<RenderError> {
        let errors = self.errors.lock().ok()?;
        (!errors.is_empty()).then(|| {
            RenderError::Backend(format!(
                "the backend rejected this frame:\n{}",
                errors.join("\n")
            ))
        })
    }

    /// Reads the source of a bound shader document.
    fn shader_source(&self, binder: &Binder<'_>) -> Result<(String, String), RenderError> {
        let path = binder
            .text(graphics::binders::PATH)
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
    pub(super) fn shader_module(
        &self,
        binder: &Binder<'_>,
    ) -> Result<wgpu::ShaderModule, RenderError> {
        let (path, source) = self.shader_source(binder)?;
        Ok(self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&path),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            }))
    }

    /// Executes every pass, in order, and reads back the colour target of the last one.
    fn execute(
        &self,
        graphics: &Value,
        instance: &Value,
    ) -> Result<Option<RenderedTarget>, RenderError> {
        let mut cache = TextureCache::default();
        let mut cleared: Vec<String> = Vec::new();
        let mut colour: Option<RenderedTarget> = None;

        for pass in passes(graphics)? {
            match pass.kind() {
                // A compute pass runs even though it contributes no pixels, because it may
                // be a prerequisite for the passes that follow.
                PassKind::Compute => self.run_compute(&pass, instance, &mut cache)?,
                PassKind::Rasterize => {
                    let target = self.run_rasterize(&pass, instance, &mut cache, &mut cleared)?;
                    if target.is_some() {
                        colour = target;
                    }
                }
            }
        }
        if let Some(error) = self.drain_errors() {
            return Err(error);
        }
        Ok(colour)
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
        let Some(target) = self.execute(graphics, instance)? else {
            return Err(RenderError::NoColourTarget);
        };
        let frame = self.read_back(&target)?;
        if let Some(error) = self.drain_errors() {
            return Err(error);
        }
        Ok(frame)
    }
}
