//! The renderer abstraction, and the no-operation implementation used without a gpu.

use definition::Value;
use graphics::binders::{BinderKind, HEIGHT, WIDTH};
use graphics::{GraphicsError, PassKind, passes};
use thiserror::Error;

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

/// A frame together with the name of the renderer that produced it.
///
/// The name is carried on every result so a caller is never left guessing whether it
/// received real pixels or the no-operation placeholder.
#[derive(Debug, Clone)]
pub struct RenderedFrame {
    /// Name of the renderer that produced the frame.
    pub renderer: &'static str,
    /// The frame itself.
    pub frame: Frame,
}

impl RenderedFrame {
    /// Encodes the frame as a png, when the `png` feature is enabled.
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, RenderError> {
        crate::image::encode_png(&self.frame)
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

    /// Renders the first rasterize pass that declares a target and reads the framebuffer
    /// back.
    ///
    /// Compute passes contribute no pixels. They are still executed by a backend that
    /// can, because a pass may be a prerequisite for the pixels that follow it.
    fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame, RenderError>;
}

/// A renderer that performs no gpu work.
///
/// This is the fallback when no adapter is present, so the server, the hot reload path
/// and the rpc surface stay exercisable on a machine with no gpu. The frame it returns is
/// transparent black and [`Renderer::available`] reports `false`, so a placeholder can
/// never be mistaken for a real render.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRenderer;

impl Renderer for NoopRenderer {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn available(&self) -> bool {
        false
    }

    fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame, RenderError> {
        let (width, height) = first_target_size(graphics, instance)?;
        let rgba = vec![0u8; (width as usize) * (height as usize) * 4];
        Frame::new(width, height, rgba).ok_or(RenderError::FrameSize)
    }
}

/// Renders with the given renderer, tagging the result with the renderer's name.
pub fn render_with(
    renderer: &dyn Renderer,
    graphics: &Value,
    instance: &Value,
) -> Result<RenderedFrame, RenderError> {
    Ok(RenderedFrame {
        renderer: renderer.name(),
        frame: renderer.render(graphics, instance)?,
    })
}

/// Returns the size of the first rasterize target that resolves to a texture payload,
/// falling back to one by one.
///
/// The size is read from the instance rather than the graphics document, because a
/// target is a named binding point and only the instance knows what it points at.
fn first_target_size(graphics: &Value, instance: &Value) -> Result<(u32, u32), RenderError> {
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
                if let Some(size) = texture_size(binder.value()) {
                    return Ok(size);
                }
            }
        }
        return Ok((1, 1));
    }
    Ok((1, 1))
}

/// Reads a width and height out of a bound texture payload.
pub(crate) fn texture_size(value: &Value) -> Option<(u32, u32)> {
    if BinderKind::from_tag(value.variant_tag()?) != Some(BinderKind::Texture) {
        return None;
    }
    let fields = value.variant_fields()?;
    let width = fields.get(WIDTH)?.as_int()?;
    let height = fields.get(HEIGHT)?.as_int()?;
    (width > 0 && height > 0).then_some((width as u32, height as u32))
}

/// Rendering failed, or could not be attempted.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the graphics document could not be read: {0}")]
    Graphics(#[from] GraphicsError),
    #[error("the graphics document declares no pass that renders into a colour target")]
    NoColourTarget,
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
