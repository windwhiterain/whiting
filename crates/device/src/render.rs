//! The renderer abstraction, and the no-operation implementation used without a gpu.

use definition::Value;
use graphics::binders::{BinderKind, FORMAT, HEIGHT, WIDTH};
use graphics::outputs as declared_outputs;
use graphics::{GraphicsError, PassKind, passes};
use scene::DeviceLimits;
use thiserror::Error;

/// One rendered image, read back from the gpu in the format its target was drawn in.
///
/// # Why this is not [`Frame`]
///
/// A frame is what a picture is: eight bits a channel, four channels, ready to be encoded as a
/// png. A debug image is a *measurement*, and quantising a position to eight bits a channel would
/// answer which region a pixel is in rather than where it is. The two are therefore two types,
/// and the one that holds floats is the one whose values are read rather than looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedImage {
    /// The name the graphics document declares the image under.
    pub name: String,
    /// The format it was drawn in, spelled the way a document spells it.
    pub format: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The texels, row major, [`RenderedImage::texel_bytes`] each, with no padding between rows.
    pub bytes: Vec<u8>,
}

impl RenderedImage {
    /// Returns how many bytes one texel occupies, or none for a format this does not read.
    pub fn texel_bytes(&self) -> Option<usize> {
        texel_bytes(&self.format)
    }

    /// Returns one texel as four floats, or none when it is outside the image or the format is
    /// not one this reads.
    ///
    /// # Invariants
    ///
    /// The value is returned as the shader wrote it, up to the format's own precision, and never
    /// scaled or normalised on the way out: an image is a measurement, and a caller that has to
    /// undo an encoding to read a number is a caller that can get the encoding wrong. An 8 bit
    /// image is the one exception, and it is a documented one: its channels are returned over
    /// 255, because that is what the bytes mean.
    pub fn texel(&self, x: u32, y: u32) -> Option<[f32; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let texel = self.texel_bytes()?;
        let start = (y as usize * self.width as usize + x as usize).checked_mul(texel)?;
        let bytes = self.bytes.get(start..start + texel)?;
        decode(&self.format, bytes)
    }

    /// Returns the image as an 8 bit RGBA frame, which is what a png is made of.
    ///
    /// # Invariants
    ///
    /// An `rgba8unorm-srgb` image is returned as it stands: the bytes are the stored ones, which
    /// is what a viewer shows, and decoding them to linear light here would be this function
    /// deciding what the picture means rather than reporting what was drawn.
    pub fn as_frame(&self) -> Option<Frame> {
        matches!(self.format.as_str(), "rgba8unorm" | "rgba8unorm-srgb")
            .then(|| Frame::new(self.width, self.height, self.bytes.clone()))
            .flatten()
    }
}

/// One rendered frame, read back from the gpu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Red, green, blue and alpha bytes, row major, four bytes per pixel.
    pub rgba: Vec<u8>,
}

impl Frame {
    /// Creates a frame, returning `None` when the pixel buffer is the wrong length.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        (rgba.len() == expected).then_some(Self {
            width,
            height,
            rgba,
        })
    }
}

/// Every image one render produced, with the name of the renderer that produced them.
///
/// The name is carried on every result so a caller is never left guessing whether it received
/// real pixels or the no-operation placeholder.
#[derive(Debug, Clone)]
pub struct RenderedFrame {
    /// Name of the renderer that produced the images.
    pub renderer: &'static str,
    /// The images, in the order the graphics document declared them.
    pub images: Vec<RenderedImage>,
}

impl RenderedFrame {
    /// Returns the image declared as the frame, which is the one a picture is made of.
    pub fn present(&self) -> Option<&RenderedImage> {
        self.images
            .iter()
            .find(|image| image.name == graphics::PRESENT_OUTPUT)
    }

    /// Encodes the frame as a png, when the `png` feature is enabled.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, RenderError> {
        let present = self.present().ok_or(RenderError::NoPresentImage)?;
        let frame = present.as_frame().ok_or_else(|| RenderError::FrameFormat {
            format: present.format.clone(),
        })?;
        crate::image::encode_png(&frame)
    }
}

/// Renders a graphics document against an instance document.
pub trait Renderer: Send + Sync {
    /// Returns a stable name for this renderer, reported in every response.
    fn name(&self) -> &'static str;

    /// Returns whether this renderer can actually produce pixels.
    ///
    /// The device reports this so a caller can tell a real render from a placeholder
    /// without inspecting the pixels.
    fn available(&self) -> bool;

    /// Returns what the machine behind this renderer can hold, or `None` where there is no
    /// machine.
    ///
    /// # Why a renderer answers this rather than a caller asking an adapter
    ///
    /// A compile is held to the limits of the **device**, and the device is the one that was
    /// created with the lower of a request and an adapter's numbers. A caller that asked the
    /// adapter separately would have the adapter's answer, which is not the one the resources
    /// are validated against, and a caller that held no adapter at all would have no answer.
    ///
    /// Returning `None` is a real answer rather than a default one: it says this renderer
    /// creates nothing, so nothing it is asked to draw is bounded by it, and a compile made
    /// without a machine is made against the floor.
    fn limits(&self) -> Option<DeviceLimits>;

    /// Renders and reads back every image the graphics document says it produces.
    ///
    /// # Invariants
    ///
    /// A document that **declares** its outputs is held to its own declaration: an output no
    /// pass rendered into is an error rather than an image that is quietly absent, because a
    /// caller that asked for an image and was handed nothing would read the missing one as a
    /// scene that draws nothing. A document that declares none means what every document
    /// written before the field existed meant — the colour target of the last pass that wrote
    /// one, under [`graphics::PRESENT_OUTPUT`] — and is answered that way.
    fn render_outputs(
        &self,
        graphics: &Value,
        instance: &Value,
    ) -> Result<Vec<RenderedImage>, RenderError>;

    /// Renders and returns the frame, which is the image declared as `present`.
    ///
    /// A convenience over [`Renderer::render_outputs`] for the caller that wants the picture and
    /// nothing else. It runs the pass list **once**: this is the same render, read differently.
    fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame, RenderError> {
        let images = self.render_outputs(graphics, instance)?;
        let present = images
            .iter()
            .find(|image| image.name == graphics::PRESENT_OUTPUT)
            .ok_or(RenderError::NoPresentImage)?;
        present.as_frame().ok_or_else(|| RenderError::FrameFormat {
            format: present.format.clone(),
        })
    }
}

/// A renderer that performs no gpu work.
///
/// This is the fallback when no adapter is present, so the server, the hot reload path
/// and the rpc surface stay exercisable on a machine with no gpu. The images it returns are
/// transparent black and [`Renderer::available`] reports `false`, so a placeholder can never be
/// mistaken for a real render.
///
/// # What it answers about a shape
///
/// A document that declares its outputs is answered **at the shapes it declared**, read out of
/// the instance's own texture payloads, so a caller sizing anything from the placeholder gets the
/// size the real renderer would have produced. A document that declares nothing is answered by
/// the first target its passes bind, which is what the placeholder has always answered: it has no
/// frame to resolve, and inventing one would be a size nothing in the document states.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRenderer;

impl Renderer for NoopRenderer {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn available(&self) -> bool {
        false
    }

    fn limits(&self) -> Option<DeviceLimits> {
        None
    }

    fn render_outputs(
        &self,
        graphics: &Value,
        instance: &Value,
    ) -> Result<Vec<RenderedImage>, RenderError> {
        let declared = declared_outputs(graphics)?;
        if declared.is_empty() {
            let (width, height, format) = first_target_shape(graphics, instance)?;
            return Ok(vec![blank(
                graphics::PRESENT_OUTPUT,
                width,
                height,
                &format,
            )?]);
        }

        let mut images = Vec::with_capacity(declared.len());
        for output in &declared {
            let Some((width, height, format)) = target_shape(graphics, instance, output.point)?
            else {
                return Err(RenderError::MissingOutput {
                    name: output.name.to_owned(),
                    point: output.point.to_owned(),
                });
            };
            images.push(blank(output.name, width, height, &format)?);
        }
        Ok(images)
    }
}

/// Returns an image of the given shape with every texel zero, or an error for a format whose
/// texel size is not known.
fn blank(name: &str, width: u32, height: u32, format: &str) -> Result<RenderedImage, RenderError> {
    let texel = texel_bytes(format).ok_or_else(|| RenderError::TexelFormat {
        format: format.to_owned(),
    })?;
    Ok(RenderedImage {
        name: name.to_owned(),
        format: format.to_owned(),
        width,
        height,
        bytes: vec![0u8; texel * width as usize * height as usize],
    })
}

/// Renders with the given renderer, tagging the result with the renderer's name.
pub fn render_with(
    renderer: &dyn Renderer,
    graphics: &Value,
    instance: &Value,
) -> Result<RenderedFrame, RenderError> {
    Ok(RenderedFrame {
        renderer: renderer.name(),
        images: renderer.render_outputs(graphics, instance)?,
    })
}

/// Returns the shape of the first target a graphics document's passes bind, falling back to one
/// by one.
///
/// The shape is read from the instance rather than the graphics document, because a target is a
/// named binding point and only the instance knows what it points at.
fn first_target_shape(
    graphics: &Value,
    instance: &Value,
) -> Result<(u32, u32, String), RenderError> {
    for pass in passes(graphics)? {
        if pass.kind() != PassKind::Rasterize {
            continue;
        }
        let declarations = pass.binders()?;
        for target in pass.targets()? {
            let Some(point) = target.as_str() else {
                continue;
            };
            let Some(declaration) = declarations
                .iter()
                .find(|declaration| declaration.point() == point)
            else {
                continue;
            };
            if let Ok(binder) = declaration.bind(instance) {
                if let Some(shape) = texture_shape(binder.value()) {
                    return Ok(shape);
                }
            }
        }
        return Ok((1, 1, "rgba8unorm".to_owned()));
    }
    Ok((1, 1, "rgba8unorm".to_owned()))
}

/// Returns the shape of the texture a binding point is rendered into, as the instance holds it.
///
/// # Invariants
///
/// The point is resolved through the **last** rasterize pass that names it among its targets,
/// because a target is written by several passes — one lighting pass per group writes
/// `color_target` — and the image is what the last of them left.
fn target_shape(
    graphics: &Value,
    instance: &Value,
    point: &str,
) -> Result<Option<(u32, u32, String)>, RenderError> {
    let mut found = None;
    for pass in passes(graphics)? {
        if pass.kind() != PassKind::Rasterize {
            continue;
        }
        if !pass
            .targets()?
            .iter()
            .any(|target| target.as_str() == Some(point))
        {
            continue;
        }
        let declarations = pass.binders()?;
        let Some(declaration) = declarations
            .iter()
            .find(|declaration| declaration.point() == point)
        else {
            continue;
        };
        if let Ok(binder) = declaration.bind(instance) {
            if let Some(shape) = texture_shape(binder.value()) {
                found = Some(shape);
            }
        }
    }
    Ok(found)
}

/// Reads a width, a height and a format out of a bound texture payload.
///
/// # Invariants
///
/// A texture array answers this the same way a flat texture does, because the size of an image is
/// the size of each of its layers: the question is which image this entry holds, and both shapes
/// hold one. The kind test is therefore [`BinderKind::is_texture`] rather than one variant, so
/// that a pool reports the size of a page and a frame is not sized from a default.
pub(crate) fn texture_shape(value: &Value) -> Option<(u32, u32, String)> {
    if !BinderKind::from_tag(value.variant_tag()?)?.is_texture() {
        return None;
    }
    let fields = value.variant_fields()?;
    let width = fields.get(WIDTH)?.as_int()?;
    let height = fields.get(HEIGHT)?.as_int()?;
    let format = fields.get(FORMAT)?.as_str()?;
    (width > 0 && height > 0).then(|| (width as u32, height as u32, format.to_owned()))
}

/// Returns how many bytes one texel occupies, or none for a format no image read here carries.
///
/// # Invariants
///
/// This is the byte width of a copy, not the meaning of the texels: `depth32float` and `r32float`
/// are both four bytes and only one of them is a colour. A format that is absent is refused
/// rather than given a default, because a default of four would silently mis-size every texel of
/// a format it did not know.
pub fn texel_bytes(format: &str) -> Option<usize> {
    Some(match format {
        "rgba8unorm" | "rgba8unorm-srgb" | "bgra8unorm" | "r32float" | "depth32float" => 4,
        "rgba16float" => 8,
        "rgba32float" => 16,
        _ => return None,
    })
}

/// Reads one texel of a format as four floats.
fn decode(format: &str, bytes: &[u8]) -> Option<[f32; 4]> {
    let bytes: [u8; 16] = {
        let mut padded = [0u8; 16];
        padded[..bytes.len()].copy_from_slice(bytes);
        padded
    };
    let f32_at = |index: usize| {
        f32::from_le_bytes([
            bytes[index],
            bytes[index + 1],
            bytes[index + 2],
            bytes[index + 3],
        ])
    };
    let f16_at = |index: usize| {
        half::f16::from_bits(u16::from_le_bytes([bytes[index], bytes[index + 1]])).to_f32()
    };
    Some(match format {
        // The stored value over 255, in the order the bytes are in. An sRGB image is reported as
        // it is stored rather than decoded, because the bytes are what a viewer shows.
        "rgba8unorm" | "rgba8unorm-srgb" => [
            f32::from(bytes[0]) / 255.0,
            f32::from(bytes[1]) / 255.0,
            f32::from(bytes[2]) / 255.0,
            f32::from(bytes[3]) / 255.0,
        ],
        "bgra8unorm" => [
            f32::from(bytes[2]) / 255.0,
            f32::from(bytes[1]) / 255.0,
            f32::from(bytes[0]) / 255.0,
            f32::from(bytes[3]) / 255.0,
        ],
        "rgba16float" => [f16_at(0), f16_at(2), f16_at(4), f16_at(6)],
        "rgba32float" => [f32_at(0), f32_at(4), f32_at(8), f32_at(12)],
        "r32float" => [f32_at(0), 0.0, 0.0, 1.0],
        _ => return None,
    })
}

/// Rendering failed, or could not be attempted.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the graphics document could not be read: {0}")]
    Graphics(#[from] GraphicsError),
    #[error("the graphics document declares no pass that renders into a colour target")]
    NoColourTarget,
    #[error(
        "the graphics document declares no image named `{}`",
        graphics::PRESENT_OUTPUT
    )]
    NoPresentImage,
    #[error("the output `{name}` renders into `{point}`, and no pass in this document writes it")]
    MissingOutput { name: String, point: String },
    #[error("the frame is drawn in `{format}`, and a frame is read as an 8 bit rgba image")]
    FrameFormat { format: String },
    #[error("`{format}` is not a texture format a texel can be read out of")]
    TexelFormat { format: String },
    #[error("no gpu adapter is available")]
    NoAdapter,
    #[error("the framebuffer size does not match its pixel buffer")]
    FrameSize,
    #[error("the renderer backend failed: {0}")]
    Backend(String),
    #[cfg_attr(not(feature = "png"), allow(dead_code))]
    #[error("failed to encode the frame as a png: {0}")]
    Encoding(String),
}
