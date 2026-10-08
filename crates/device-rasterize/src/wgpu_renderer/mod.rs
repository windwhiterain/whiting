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
//! Every pass runs, in the order the document lists them, and what comes back is **what the
//! document declares it produces**: every named output is read back, in the format its target
//! was drawn in. A document that declares nothing means what every document written before the
//! outputs field existed meant, and is answered exactly as it was — the colour target of the
//! *last* pass that drew into one, under `present`. A document that declares an output no pass
//! rendered into is refused rather than answered with a shorter list, because a caller that
//! asked for an image and was handed none would read the missing one as a scene that draws
//! nothing; and a document whose passes write no colour at all has no image, which is reported
//! rather than papered over with a placeholder.
//!
//! # Errors
//!
//! A backend that is handed a pipeline it cannot build says so, rather than drawing
//! nothing: the device is created with an error handler, and whatever the backend reports
//! is drained into the next [`RenderError`]. A shader that does not compile, a bind group
//! that does not match its layout, and a draw with no buffer all arrive as errors with
//! the backend's own words in them.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use definition::Value;
use graphics::{Binder, PassKind, passes};
use scene::DeviceLimits;
use wgpu::naga;
use wgpu::util::DeviceExt;

use bindings::{BufferCache, TextureCache};
use device::render::{RenderError, RenderedImage, Renderer};
use raster::RenderedTarget;

mod bindings;
mod compute;
mod raster;
mod shader_interface;
mod uniform;

pub use bindings::TextureShape;
pub use shader_interface::{check_outputs, check_resources};

/// How many vertex attributes the pipeline asks an adapter for.
///
/// # Invariants
///
/// This is not a number a document chooses: it is what the layout rules imply. A point
/// carries its own fields, and an instance row carries a model matrix **and** the inverse
/// transpose that carries a normal through the same transform — two matrices, eight
/// attributes between them, before a row's own material has said anything. A row of any
/// use at all is therefore within one attribute of the sixteen a backend allows by
/// default, and a pipeline that carries a normal matrix and a material does not fit in a
/// default that was chosen before anything had to be drawn through it.
const VERTEX_ATTRIBUTES: u32 = 32;

/// How many vertex buffers a pass may bind: the points, then the instance rows.
const VERTEX_BUFFERS: u32 = 8;

/// Returns the limits this pipeline needs, clamped to what the adapter has.
///
/// # Invariants
///
/// The request is the lower of what the pipeline needs and what the adapter reports, so a
/// device on an adapter that cannot do this fails to be created — which is a clear error —
/// rather than drawing a frame whose attributes the hardware never received. wgpu takes that
/// lower of the two for us when the device is created, so this asks for the adapter's own
/// numbers and overrides only what the pipeline insists on.
///
/// Only the two vertex counts are overridden, and they are the pipeline's own requirement
/// rather than a ceiling: a document does not choose them, and asking for fewer than a stage
/// needs is the failure this whole function is about.
///
/// There is deliberately **no** `using_resolution` call. It reads as if it raised a floor and
/// in fact it does the opposite: it copies the texture dimensions out of the limits passed to
/// it and discards every other field, so pairing it with any constant re-pins the page size
/// to that constant's numbers whatever the hardware reports. It came in with the vertex
/// override above, as the way to say "the adapter's limits, and these two of them are ours",
/// and the texture dimensions came along for the ride. A scene asking for a shadow page
/// larger than the downlevel floor of 2048 was refused by a request that had pinned the
/// dimension there, which is the bug this removes.
///
/// A scene that asks for more than the adapter has is refused by the device when the resource
/// is created, with the adapter's own numbers in the error, which says more than a request
/// that quietly asked for less and then let a scene's own size limit be the thing that failed.
fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    wgpu::Limits {
        max_vertex_attributes: VERTEX_ATTRIBUTES,
        max_vertex_buffers: VERTEX_BUFFERS,
        ..adapter.limits()
    }
}

/// A renderer that executes passes on a gpu.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    errors: Arc<Mutex<Vec<String>>>,
    /// Shader modules, keyed by their source.
    ///
    /// # Invariants
    ///
    /// A module is keyed by the text it was built from, so two passes naming the same
    /// shader share one module however differently they are labelled, and a scene that
    /// has not changed a shader never rebuilds it. Without this, a frame compiles every
    /// shader it runs — and a range of frames that changes only a number recompiles the
    /// lot on every one of them, which is most of the cost of animating a scene whose
    /// geometry is heavy.
    ///
    /// The map grows with the number of *distinct* shaders, not with the number of frames,
    /// because a scene is a document: it carries the same shader text however many times
    /// it is drawn.
    modules: Mutex<HashMap<String, Arc<wgpu::ShaderModule>>>,
    /// The intermediate form of a shader, keyed by its source, for the uniform layout
    /// check. The same argument as [`WgpuRenderer::modules`]: the check parses a shader on
    /// every pass of every frame, and a shader that has not changed parses to the same
    /// thing every time.
    parsed: Mutex<HashMap<String, Arc<naga::Module>>>,
}

impl WgpuRenderer {
    /// Requests an adapter and creates a device.
    ///
    /// Returns [`RenderError::NoAdapter`] when no adapter is available, which is the
    /// signal for a caller to fall back to the no-operation renderer.
    pub async fn new() -> Result<Self, RenderError> {
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
                required_limits: required_limits(&adapter),
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
            errors,
            modules: Mutex::new(HashMap::new()),
            parsed: Mutex::new(HashMap::new()),
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

    /// Returns whether a format may be read through a filtering sampler.
    ///
    /// # Invariants
    ///
    /// The device requests [`wgpu::Features::empty`], so the filterability here is the
    /// format's own and not the adapter's: `R32Float` and `Rgba32Float` need the
    /// `float32-filterable` feature, which is why they are excluded rather than assumed to
    /// work. A 32 bit float texture is therefore readable with `textureLoad` and cannot be
    /// declared `sampled`, which is a limit of the device and is reported as one.
    pub(super) const fn is_filterable(format: wgpu::TextureFormat) -> bool {
        matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Rgba16Float
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

    /// Borrows the source of a bound shader document.
    ///
    /// An instruction carries its own text, so nothing here reads a file: a document that
    /// arrived over the wire has everything it needs to be drawn.
    fn shader_source<'binder>(
        &self,
        binder: &'binder Binder<'_>,
    ) -> Result<&'binder str, RenderError> {
        binder
            .text(graphics::binders::SOURCE)
            .ok_or_else(|| RenderError::Backend("a shader binder needs a `source`".to_owned()))
    }

    /// Creates a shader module from a bound shader document.
    ///
    /// The module is labelled with the pass that first needed it, because there is no
    /// filename to name it by and the pass is what a backend diagnostic is about. A
    /// module already built from the same text is shared rather than rebuilt, which is
    /// what keeps a frame that changes nothing from paying to compile its shaders again.
    pub(super) fn shader_module(
        &self,
        binder: &Binder<'_>,
        label: &str,
    ) -> Result<Arc<wgpu::ShaderModule>, RenderError> {
        let source = self.shader_source(binder)?.to_owned();
        if let Some(found) = self
            .modules
            .lock()
            .ok()
            .and_then(|modules| modules.get(&source).cloned())
        {
            return Ok(found);
        }
        let module = Arc::new(
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                }),
        );
        if let Ok(mut modules) = self.modules.lock() {
            modules.insert(source, Arc::clone(&module));
        }
        Ok(module)
    }

    /// Returns the intermediate form of a shader, parsing it the first time only.
    ///
    /// `label` names the pass in a parse failure, because a bound shader entry has no
    /// name of its own and the source of a shader is a wall of text that a reader cannot
    /// find a pass in.
    ///
    /// # Why a parse failure may name a second file
    ///
    /// The source is a module assembled from a template and a scene's own WGSL, so a line
    /// number in it is a line of a string on no disk. An assembler that knew where each
    /// block came from said so in the source, and a line the source cannot account for is
    /// reported exactly as it always was — which is the whole of the message when nothing
    /// declared anything.
    pub(super) fn parsed_module(
        &self,
        source: &str,
        label: &str,
    ) -> Result<Arc<naga::Module>, RenderError> {
        if let Some(found) = self
            .parsed
            .lock()
            .ok()
            .and_then(|parsed| parsed.get(source).cloned())
        {
            return Ok(found);
        }
        let module = Arc::new(naga::front::wgsl::parse_str(source).map_err(|error| {
            let line = error
                .location(source)
                .map_or(0, |where_| where_.line_number);
            RenderError::Backend(format!(
                "`{label}` does not parse as wgsl: {error}{}",
                graphics::provenance::explain(source, line)
            ))
        })?);
        if let Ok(mut parsed) = self.parsed.lock() {
            parsed.insert(source.to_owned(), Arc::clone(&module));
        }
        Ok(module)
    }

    /// Executes every pass, in order, and returns the colour targets each one wrote into.
    ///
    /// # Invariants
    ///
    /// The list is in **write order** and keeps one entry per colour target a pass drew into,
    /// so the same point appears once per pass that wrote it and a pass with several
    /// attachments contributes one entry each. That is what lets a caller read the two rules
    /// apart without either being inferred from the other: the frame under the convention is
    /// the *last* entry, and an image a document declared is the last entry **for that point**.
    /// Collapsing the duplicates here would answer the first question and lose the second.
    ///
    /// **A pass with several attachments and no declared outputs changes which entry is the
    /// frame.** The convention — the last colour target of the last colour pass — is now
    /// answered by the *last* of that pass's attachments rather than the first, because the
    /// list is the attachments in order and the last entry is the last attachment. Every
    /// document that exists today has one colour attachment per pass, so every one of them
    /// answers exactly as it did; the change is only visible to a document that writes a
    /// G-buffer, and the answer a document declares in `outputs` is unaffected either way.
    fn execute<'document>(
        &self,
        graphics: &'document Value,
        instance: &Value,
    ) -> Result<
        (
            Vec<RenderedTarget>,
            Vec<graphics::Output<'document>>,
            Vec<String>,
        ),
        RenderError,
    > {
        let mut cache = TextureCache::default();
        let mut buffers = BufferCache::default();
        let mut cleared: Vec<String> = Vec::new();
        let mut colour: Vec<RenderedTarget> = Vec::new();
        // Read once, here, rather than per pass: the declarations are what the writing pass is
        // checked against, so every pass needs the same list and none of them owns it. The
        // labels are read for the same reason and only used once, but they cannot be read inside
        // the `ok_or_else` below, which is a closure rather than a function.
        let declared = graphics::outputs(graphics)?;
        let labels = written_labels(graphics)?;

        for pass in passes(graphics)? {
            match pass.kind() {
                // A compute pass runs even though it contributes no pixels, because it may
                // be a prerequisite for the passes that follow.
                PassKind::Compute => self.run_compute(&pass, instance, &mut cache, &mut buffers)?,
                PassKind::Rasterize => {
                    let targets = self.run_rasterize(
                        &pass,
                        instance,
                        &mut cache,
                        &mut buffers,
                        &mut cleared,
                        &declared,
                    )?;
                    colour.extend(targets);
                }
            }
        }
        if let Some(error) = self.drain_errors() {
            return Err(error);
        }
        // The labels travel back with the targets so that the one error that needs them is
        // raised against the passes that actually ran rather than against the whole document.
        Ok((colour, declared, labels))
    }
}

impl Renderer for WgpuRenderer {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn available(&self) -> bool {
        true
    }

    /// # Invariants
    ///
    /// Read from the **device** rather than from the adapter, because the device is what was
    /// created with the lower of the request above and the adapter's numbers, and a resource
    /// is validated against the numbers the device agreed to rather than the numbers it could
    /// have had. The two are equal today for these two fields, because the request asks for
    /// the adapter's own; they would part company the day an override joined them, which is
    /// exactly the day this would otherwise be the wrong answer.
    fn limits(&self) -> Option<DeviceLimits> {
        let limits = self.device.limits();
        Some(DeviceLimits {
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_texture_array_layers: limits.max_texture_array_layers,
        })
    }

    fn render_outputs(
        &self,
        graphics: &Value,
        instance: &Value,
    ) -> Result<Vec<RenderedImage>, RenderError> {
        let (written, declared, labels) = self.execute(graphics, instance)?;

        // A document that says nothing about its outputs means what every document written
        // before the field existed meant, and is answered exactly as it was: the colour target of
        // the last pass that drew into one.
        if declared.is_empty() {
            let last = written.last().ok_or(RenderError::NoColourTarget)?;
            return Ok(vec![self.read_back(graphics::PRESENT_OUTPUT, last)?]);
        }

        let mut images = Vec::with_capacity(declared.len());
        for output in &declared {
            // The last write to the point, because an image is what the last pass left there. A
            // point no pass wrote is refused rather than skipped, and it is refused **as the
            // other coverage contract**: the writing pass is not there to be asked, so what is
            // missing is a bucket the emitter's enumeration never reached rather than a buffer
            // the pipeline does not have. See `RenderError::UnwrittenOutput`.
            let target = written
                .iter()
                .rev()
                .find(|target| target.point == output.point)
                .ok_or_else(|| RenderError::UnwrittenOutput {
                    name: output.name.to_owned(),
                    point: output.point.to_owned(),
                    ty: output.ty.to_owned(),
                    location: output.location,
                    labels: labels.clone(),
                })?;
            images.push(self.read_back(output.name, target)?);
        }
        if let Some(error) = self.drain_errors() {
            return Err(error);
        }
        Ok(images)
    }
}

/// Returns the labels of the passes that did draw into a colour target, for an error about an
/// output none of them wrote.
///
/// The list is what makes the message say **which** enumeration came up short. A declared
/// output is bounded by the buckets the writing pass enumerated, so "nothing wrote it" is only
/// actionable next to the passes that did run: a scene whose geometry passes all ran and whose
/// declared normal is still missing has a shader that does not write it, and a scene with no
/// geometry pass at all has an emitter that never enumerated one. Those are different defects
/// and they get the same sentence without it.
fn written_labels(graphics: &Value) -> Result<Vec<String>, RenderError> {
    let mut labels = Vec::new();
    for pass in passes(graphics)? {
        if pass.kind() != graphics::PassKind::Rasterize {
            continue;
        }
        if pass
            .targets()
            .map(|targets| !targets.is_empty())
            .unwrap_or(false)
        {
            labels.push(pass.label().unwrap_or("<unlabelled>").to_owned());
        }
    }
    Ok(labels)
}
