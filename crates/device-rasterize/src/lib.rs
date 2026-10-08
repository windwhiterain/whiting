//! Rasterising render backends for the whiting device.
//!
//! # What this crate is
//!
//! The [`device::render::Renderer`] trait is deliberately about *instructions*, not about a
//! scene: it is handed compiled graphics and instance documents and returns images. This crate
//! supplies the backend that draws those instructions on a gpu, plus the placeholder that stands
//! in for it when there is no gpu.
//!
//! # Why this is a crate and not a module
//!
//! `wgpu` is a large dependency with a build-time cost, a system-library story and a driver
//! story of its own. A consumer that only wants to *serve* documents, or to drive the offline
//! path tracer in `whiting-device-cycles`, has no use for any of that, and should not have to
//! resolve it to link. Keeping the rasterising backend behind its own crate is what makes
//! "run the server on a machine with no gpu" a matter of depending on one crate rather than a
//! feature flag inside a crate that otherwise cannot compile there.
//!
//! The split is only possible because the trait itself carries no gpu concept. [`NoopRenderer`]
//! lives in the trait's own crate rather than here, because a device that cannot be constructed
//! without an adapter would not be constructible on the machines this crate exists to let you
//! serve documents from; it is re-exported below so that depending on the backends is still one
//! dependency.

mod wgpu_renderer;

use device::render::Renderer;

pub use wgpu_renderer::{TextureShape, WgpuRenderer, check_outputs, check_resources};

/// The placeholder, re-exported so a caller depending on the backends gets it from the same
/// place as the real ones.
pub use device::render::NoopRenderer;

/// Returns the best renderer this machine has: the gpu backend if an adapter is there, and the
/// named placeholder if it is not.
///
/// # Why the choice lives here
///
/// Every consumer would otherwise write the same probe, and each of them would write it slightly
/// differently — which is how a machine with no gpu ends up reporting a render that silently did
/// nothing under a name that does not say so. The returned renderer always names itself, so a
/// caller that does not check [`Renderer::available`] still cannot mistake one for the other.
///
/// The fallback is a real answer rather than an error: a device that refused to be built would
/// take the server down with it, and the server is what keeps documents observable on exactly the
/// machines that have no gpu.
pub async fn rasterizer() -> Box<dyn Renderer> {
    match WgpuRenderer::new().await {
        Ok(renderer) => Box::new(renderer),
        Err(_) => Box::new(NoopRenderer),
    }
}